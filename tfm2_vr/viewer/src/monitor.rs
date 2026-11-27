use std::{sync::{Arc,Mutex,atomic::{AtomicBool,AtomicU64,Ordering}},thread::{self,JoinHandle},time::{Duration,Instant}};
use crate::log::Log;
pub type SessionSlot=Arc<Mutex<Option<openxr::Session<openxr::D3D11>>>>;
#[derive(Default)] pub struct Stats {pub frames:AtomicU64,pub rendered:AtomicU64,pub should_render:AtomicBool,pub phase:Mutex<&'static str>,pub first_display:Mutex<Option<Instant>>,pub hresult:Mutex<Option<i32>>}
pub fn expired(startup:Duration,display:Option<Duration>)->bool {display.map(|t|t>=Duration::from_secs(60)).unwrap_or(startup>=Duration::from_secs(120))}
pub struct Monitor {pub stats:Arc<Stats>,done:Arc<AtomicBool>,thread:Option<JoinHandle<()>>,log:Log}
impl Monitor {
 pub fn new(log:&Log,stop:Arc<AtomicBool>,exit_sent:Arc<AtomicBool>,session:SessionSlot)->Self {
  let stats=Arc::new(Stats::default());*stats.phase.lock().unwrap()="OpenXR 초기화";let done=Arc::new(AtomicBool::new(false));let(s,d,l)=(stats.clone(),done.clone(),log.clone());
  let worker=thread::spawn(move||{let startup=Instant::now();let mut metrics=Instant::now();while !d.load(Ordering::SeqCst){thread::park_timeout(Duration::from_secs(1));if d.load(Ordering::SeqCst){break;}let first=*s.first_display.lock().unwrap();if metrics.elapsed()>=Duration::from_secs(5){let hr=s.hresult.lock().unwrap().map(|v|format!("0x{:08X}",v as u32)).unwrap_or("미조회".to_string());l.line(format!("5초 독립 진단 frames={} rendered={} shouldRender={} 단계={} 표시경과={:.1}초 최근HRESULT={hr}",s.frames.load(Ordering::SeqCst),s.rendered.load(Ordering::SeqCst),s.should_render.load(Ordering::SeqCst),*s.phase.lock().unwrap(),first.map(|t|t.elapsed().as_secs_f64()).unwrap_or(0.0)));metrics=Instant::now();}
   if expired(startup.elapsed(),first.map(|t|t.elapsed())){stop.store(true,Ordering::SeqCst);if !exit_sent.swap(true,Ordering::SeqCst){l.line("독립 타이머: 표시60초/표시전120초 종료 조건 도달");let active=session.lock().ok().and_then(|s|s.clone());if let Some(active)=active{let _=l.xr("xrRequestExitSession(타이머)",||active.request_exit());}}}
  }});Self{stats,done,thread:Some(worker),log:log.clone()}
 }
}
impl Drop for Monitor {fn drop(&mut self){self.done.store(true,Ordering::SeqCst);if let Some(t)=self.thread.take(){t.thread().unpark();let start=Instant::now();while !t.is_finished()&&start.elapsed()<Duration::from_secs(1){thread::sleep(Duration::from_millis(5));}if t.is_finished(){let _=t.join();}else{self.log.line("종료 요청의 런타임 응답이 지연돼 진단 스레드 합류를 기다리지 않습니다");}}}}
pub struct SlotGuard(pub SessionSlot);
impl Drop for SlotGuard {fn drop(&mut self){if let Ok(mut s)=self.0.lock(){*s=None;}}}
#[cfg(test)] mod tests {use super::*;#[test] fn display_and_startup_deadlines(){assert!(!expired(Duration::from_secs(119),None));assert!(expired(Duration::from_secs(120),None));assert!(!expired(Duration::from_secs(180),Some(Duration::from_secs(59))));assert!(expired(Duration::from_secs(180),Some(Duration::from_secs(60))));}}
