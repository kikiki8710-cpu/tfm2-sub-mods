use anyhow::Result;
use std::{fs::{self, File}, io::Write, path::PathBuf, sync::{Arc,Mutex}, time::Instant};
use windows::Win32::System::SystemInformation::GetSystemTime;

#[derive(Clone)]
pub struct Log { file: Arc<Mutex<File>>, start: Instant }
/// 배포본(옆에 tfm2_vr.portable 이 있음)은 exe 폴더, 개발본은 프로젝트 폴더.
pub fn root() -> PathBuf { if let Some(d)=std::env::current_exe().ok().and_then(|p|p.parent().map(|d|d.to_owned())){if d.join("tfm2_vr.portable").exists(){return d;}} PathBuf::from(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_owned() }
pub fn stamp() -> String { let t=unsafe {GetSystemTime()};format!("{:04}{:02}{:02}_{:02}{:02}{:02}_{:03}",t.wYear,t.wMonth,t.wDay,t.wHour,t.wMinute,t.wSecond,t.wMilliseconds) }
pub fn utc() -> String { let t=unsafe{GetSystemTime()};format!("{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:03}Z",t.wYear,t.wMonth,t.wDay,t.wHour,t.wMinute,t.wSecond,t.wMilliseconds) }
impl Log {
 pub fn new() -> Result<Self> {Self::named("native_cube")}
 pub fn named(prefix:&str) -> Result<Self> { let dir=root().join("_tmp/logs");fs::create_dir_all(&dir)?;let path=dir.join(format!("{prefix}_{}_{}.log",stamp(),std::process::id()));let log=Self {file:Arc::new(Mutex::new(File::create(&path)?)),start:Instant::now()};log.line(format!("로그={} PID={}",path.display(),std::process::id()));Ok(log) }
 pub fn line(&self,s:impl AsRef<str>) { let text=format!("{} +{:.3}초 {}",utc(),self.start.elapsed().as_secs_f64(),s.as_ref());println!("{text}");if let Ok(mut f)=self.file.lock(){let _=writeln!(f,"{text}");let _=f.flush();} }
 pub fn xr<T>(&self,name:&str,f:impl FnOnce()->openxr::Result<T>) -> Result<T> {self.line(format!("OpenXR 시작 {name}"));let t=Instant::now();match f(){Ok(v)=>{self.line(format!("OpenXR 완료 {name} {:.3}ms 성공",t.elapsed().as_secs_f64()*1000.));Ok(v)},Err(e)=>{self.line(format!("OpenXR 완료 {name} {:.3}ms 결과={e:?}",t.elapsed().as_secs_f64()*1000.));Err(anyhow::anyhow!("{name}: {e:?}"))}} }
 pub fn dx<T>(&self,name:&str,f:impl FnOnce()->windows::core::Result<T>) -> Result<T> {let t=Instant::now();match f(){Ok(v)=>{self.line(format!("D3D11 {name} {:.3}ms HRESULT=0x00000000",t.elapsed().as_secs_f64()*1000.));Ok(v)},Err(e)=>{self.line(format!("D3D11 {name} {:.3}ms HRESULT=0x{:08X} {e}",t.elapsed().as_secs_f64()*1000.,e.code().0 as u32));Err(e.into())}} }
}
