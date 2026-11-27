mod log;
mod draw;
mod monitor;
use anyhow::{bail,Context,Result};
use openxr as xr;
use std::{path::Path,sync::{Arc,Mutex,atomic::{AtomicBool,Ordering}},time::{Duration,Instant},thread};
use windows::{core::Interface,Win32::{Foundation::LUID,Graphics::{Direct3D::D3D_FEATURE_LEVEL_11_0,Direct3D11::ID3D11Texture2D,Dxgi::Common::*}}};
use log::Log;

const LOADER:&str="C:/Program Files (x86)/Steam/steamapps/common/SteamVR/bin/win64/openxr_loader.dll";
const VIEW:xr::ViewConfigurationType=xr::ViewConfigurationType::PRIMARY_STEREO;
fn instance(log:&Log)->Result<xr::Instance>{
 log.line(format!("설치된 로더 절대경로={LOADER} 활성 런타임 설정은 변경하지 않습니다"));
 let start=Instant::now();let entry=unsafe{xr::Entry::load_from(Path::new(LOADER),&())}.context("설치된 OpenXR 로더 불러오기 실패")?;log.line(format!("OpenXR 로더 불러오기 {:.3}ms",start.elapsed().as_secs_f64()*1000.0));
 let available=log.xr("xrEnumerateInstanceExtensionProperties",||entry.enumerate_extensions())?;if !available.khr_d3d11_enable{bail!("활성 런타임에 XR_KHR_D3D11_enable이 없습니다");}
 let mut extensions=xr::ExtensionSet::default();extensions.khr_d3d11_enable=true;
 let instance=log.xr("xrCreateInstance",||entry.create_instance(&xr::ApplicationInfo{application_name:"tfm2_vr_cube",application_version:1,engine_name:"tfm2_vr diagnostic",engine_version:1,api_version:xr::Version::new(1,0,0)},&extensions,&[],&()))?;
 let props=log.xr("xrGetInstanceProperties",||instance.properties())?;log.line(format!("OpenXR 런타임 이름={} 버전={} D3D11확장=true",props.runtime_name,props.runtime_version));Ok(instance)
}

struct Eye { targets:Vec<draw::Target>,swapchain:xr::Swapchain<xr::D3D11>,width:u32,height:u32 }
fn session_mode(log:&Log,instance:xr::Instance,stop:Arc<AtomicBool>,exit_sent:Arc<AtomicBool>,shared:Arc<Mutex<Option<xr::Session<xr::D3D11>>>>,stats:Arc<monitor::Stats>)->Result<()> {
 let startup=Instant::now();let mut status=Instant::now()-Duration::from_secs(5);
 let system=loop{
  if stop.load(Ordering::SeqCst){log.line("헤드셋 대기 중 Ctrl+C 정상 종료");return Ok(());}
  let t=Instant::now();log.line("OpenXR 시작 xrGetSystem");let result=instance.system(xr::FormFactor::HEAD_MOUNTED_DISPLAY);log.line(format!("OpenXR 완료 xrGetSystem {:.3}ms {}",t.elapsed().as_secs_f64()*1000.0,match &result{Ok(_)=>"성공".to_string(),Err(e)=>format!("결과={e:?}")}));
  match result {Ok(id)=>break id,Err(e) if e==xr::sys::Result::ERROR_FORM_FACTOR_UNAVAILABLE=>{},Err(e)=>bail!("xrGetSystem: {e:?}")}
  if startup.elapsed()>=Duration::from_secs(120){log.line("헤드셋을 120초 기다렸습니다. 세션 없이 종료합니다.");return Ok(());}
  if status.elapsed()>=Duration::from_secs(5){log.line(format!("헤드셋 연결 대기 {}/120초 · Ctrl+C로 종료",startup.elapsed().as_secs()));status=Instant::now();}thread::sleep(Duration::from_millis(250));
 };
 let props=log.xr("xrGetSystemProperties",||instance.system_properties(system))?;log.line(format!("헤드셋 시스템={}",props.system_name));
 let requirements=log.xr("xrGetD3D11GraphicsRequirementsKHR",||instance.graphics_requirements::<xr::D3D11>(system))?;let luid=LUID{LowPart:requirements.adapter_luid.LowPart,HighPart:requirements.adapter_luid.HighPart};log.line(format!("OpenXR 요구 LUID={},{} 최소기능수준=0x{:X}",luid.HighPart,luid.LowPart,requirements.min_feature_level));
 if stop.load(Ordering::SeqCst){log.line("초기화 도중 종료 요청: 세션을 생성하지 않습니다");return Ok(());}
 let renderer=draw::Draw::new(log,Some(luid),requirements.min_feature_level)?;*stats.hresult.lock().unwrap()=Some(0);
 *stats.phase.lock().unwrap()="xrCreateSession";let (session,mut waiter,mut stream)=log.xr("xrCreateSession(D3D11)",||unsafe{instance.create_session::<xr::D3D11>(system,&xr::d3d::SessionCreateInfoD3D11{device:renderer.device.as_raw().cast()})})?;
 let _slot=monitor::SlotGuard(shared.clone());*shared.lock().unwrap()=Some(session.clone());
 let spaces=log.xr("xrEnumerateReferenceSpaces",||session.enumerate_reference_spaces())?;let stage=spaces.contains(&xr::ReferenceSpaceType::STAGE);let reference=if stage{xr::ReferenceSpaceType::STAGE}else{xr::ReferenceSpaceType::LOCAL};let space=log.xr("xrCreateReferenceSpace",||session.create_reference_space(reference,xr::Posef::IDENTITY))?;log.line(format!("기준공간={reference:?} LOCAL일 때 장면을 눈높이 아래1.6m로 배치"));
 let configs=log.xr("xrEnumerateViewConfigurationViews",||instance.enumerate_view_configuration_views(system,VIEW))?;if configs.len()!=2{bail!("스테레오 뷰 수가2가 아닙니다: {}",configs.len());}
 let formats=log.xr("xrEnumerateSwapchainFormats",||session.enumerate_swapchain_formats())?;let format=[DXGI_FORMAT_R8G8B8A8_UNORM_SRGB,DXGI_FORMAT_B8G8R8A8_UNORM_SRGB,DXGI_FORMAT_R8G8B8A8_UNORM,DXGI_FORMAT_B8G8R8A8_UNORM].into_iter().find(|f|formats.contains(&(f.0 as u32))).context("지원되는 RGBA8/BGRA8 스왑체인 형식 없음")?;
 let blends=log.xr("xrEnumerateEnvironmentBlendModes",||instance.enumerate_environment_blend_modes(system,VIEW))?;if !blends.contains(&xr::EnvironmentBlendMode::OPAQUE){bail!("불투명 PC VR 합성 모드가 없습니다");}let blend=xr::EnvironmentBlendMode::OPAQUE;
 let mut eyes=Vec::new();for (i,c) in configs.iter().enumerate(){let(w,h)=(c.recommended_image_rect_width,c.recommended_image_rect_height);log.line(format!("눈{i} 권장={w}x{h} 형식={format:?} sample=1"));let swapchain=log.xr("xrCreateSwapchain",||session.create_swapchain(&xr::SwapchainCreateInfo{create_flags:xr::SwapchainCreateFlags::EMPTY,usage_flags:xr::SwapchainUsageFlags::COLOR_ATTACHMENT,format:format.0 as u32,sample_count:1,width:w,height:h,face_count:1,array_size:1,mip_count:1}))?;let images=log.xr("xrEnumerateSwapchainImages",||swapchain.enumerate_images())?;let mut targets=Vec::new();for raw in images {let p=raw.cast();let texture=unsafe{ID3D11Texture2D::from_raw_borrowed(&p)}.context("런타임 D3D11Texture 포인터 없음")?;targets.push(renderer.target(texture,w,h,format)?);}eyes.push(Eye{swapchain,targets,width:w,height:h});}
 let mut events=xr::EventDataBuffer::new();let(mut running,mut frames,mut rendered,mut should_render)=(false,0u64,0u64,false);let mut scene_model:Option<glam::Mat4>=None;let mut first_display:Option<Instant>=None;let mut metrics=Instant::now();let mut shutdown:Option<Instant>=None;let mut first_begin=false;let mut first_end=false;
 *stats.phase.lock().unwrap()="세션 READY 대기";log.line("세션 READY 대기 · 최초 표시 후60초 자동 종료 · Ctrl+C 정상 종료");
 'session:loop{
  while let Some(event)=log.xr("xrPollEvent",||instance.poll_event(&mut events))? {match event{
   xr::Event::SessionStateChanged(e)=>{log.line(format!("세션 상태={:?} OpenXR시간={:?}",e.state(),e.time()));match e.state(){
    xr::SessionState::READY=>{log.xr("xrBeginSession",||session.begin(VIEW))?;running=true;},
    xr::SessionState::STOPPING=>{if running{log.xr("xrEndSession",||session.end())?;}break 'session;},
    xr::SessionState::EXITING|xr::SessionState::LOSS_PENDING=>break 'session,_=>{}}},
   xr::Event::InstanceLossPending(_)=>{log.line("인스턴스 손실 예정");break 'session;},_=>{}}
  }
  let timed_out=monitor::expired(startup.elapsed(),first_display.map(|t|t.elapsed()));
  if stop.load(Ordering::SeqCst)||timed_out {if !running{log.line("표시 전 대기 종료");break;}if !exit_sent.swap(true,Ordering::SeqCst){log.line(if timed_out{"표시60초 또는 준비120초 도달: 정상종료 요청"}else{"Ctrl+C: 정상종료 요청"});log.xr("xrRequestExitSession",||session.request_exit())?;}if shutdown.is_none(){shutdown=Some(Instant::now());}}
  if shutdown.map(|t|t.elapsed()>Duration::from_secs(10)).unwrap_or(false){log.line("종료 전이10초 지연: 소유한 OpenXR 자원을 해제합니다");break;}
  if !running {if metrics.elapsed()>=Duration::from_secs(5){log.line(format!("세션 상태 대기 {}/120초 frames={frames}",startup.elapsed().as_secs()));metrics=Instant::now();}thread::sleep(Duration::from_millis(10));continue;}
  if frames==0{log.line("최초 xrWaitFrame 진입");}*stats.phase.lock().unwrap()="xrWaitFrame";let frame=log.xr("xrWaitFrame",||waiter.wait())?;frames+=1;should_render=frame.should_render;stats.frames.store(frames,Ordering::SeqCst);stats.should_render.store(should_render,Ordering::SeqCst);*stats.phase.lock().unwrap()="xrBeginFrame/그리기";
  if !first_begin{log.line("최초 xrBeginFrame 진입");first_begin=true;}log.xr("xrBeginFrame",||stream.begin())?;
  let mut submit=false;let mut views=Vec::new();
  if frame.should_render {let(flags,located)=log.xr("xrLocateViews",||session.locate_views(VIEW,frame.predicted_display_time,&space))?;if flags.contains(xr::ViewStateFlags::POSITION_VALID|xr::ViewStateFlags::ORIENTATION_VALID)&&located.len()==2 {views=located;if scene_model.is_none(){let model=draw::anchor(&views,stage);log.line(format!("첫 유효 눈 위치 기준 장면 배치={:?}",model.to_cols_array()));scene_model=Some(model);}for (eye,view) in eyes.iter_mut().zip(&views){let index=log.xr("xrAcquireSwapchainImage",||eye.swapchain.acquire_image())?;log.xr("xrWaitSwapchainImage",||eye.swapchain.wait_image(xr::Duration::INFINITE))?;let model=scene_model.unwrap();renderer.render(&eye.targets[index as usize],draw::view_projection(view)*model);renderer.flush();log.xr("xrReleaseSwapchainImage",||eye.swapchain.release_image())?;}submit=true;}}
  if !first_end{log.line("최초 xrEndFrame 진입");first_end=true;}*stats.phase.lock().unwrap()="xrEndFrame";
  if submit {let projections:Vec<_>=eyes.iter().zip(&views).map(|(e,v)|xr::CompositionLayerProjectionView::new().pose(v.pose).fov(v.fov).sub_image(xr::SwapchainSubImage::new().swapchain(&e.swapchain).image_array_index(0).image_rect(xr::Rect2Di{offset:xr::Offset2Di{x:0,y:0},extent:xr::Extent2Di{width:e.width as i32,height:e.height as i32}}))).collect();let layer=xr::CompositionLayerProjection::new().space(&space).views(&projections);log.xr("xrEndFrame(스테레오 투영)",||stream.end(frame.predicted_display_time,blend,&[&layer]))?;rendered+=1;if first_display.is_none(){first_display=Some(Instant::now());log.line("최초 유효 스테레오 프레임 제출 · 60초 타이머 시작");}}
  else{log.xr("xrEndFrame(빈 레이어)",||stream.end(frame.predicted_display_time,blend,&[]))?;}
  stats.rendered.store(rendered,Ordering::SeqCst);*stats.first_display.lock().unwrap()=first_display;
  if metrics.elapsed()>=Duration::from_secs(5){log.line(format!("5초 지표 frames={frames} rendered={rendered} shouldRender={should_render} 표시경과={:.1}초",first_display.map(|t|t.elapsed().as_secs_f64()).unwrap_or(0.0)));let check=renderer.check();*stats.hresult.lock().unwrap()=Some(check.as_ref().err().and_then(|e|e.downcast_ref::<windows::core::Error>()).map(|e|e.code().0).unwrap_or(0));check?;metrics=Instant::now();}
 }
 log.line(format!("최종 frames={frames} rendered={rendered} shouldRender={should_render}"));*shared.lock().unwrap()=None;drop(eyes);drop(space);drop(stream);drop(waiter);let t=Instant::now();drop(session);log.line(format!("OpenXR xrDestroySession {:.3}ms",t.elapsed().as_secs_f64()*1000.0));renderer.check()?;drop(renderer);let t=Instant::now();drop(instance);log.line(format!("OpenXR xrDestroyInstance {:.3}ms",t.elapsed().as_secs_f64()*1000.0));Ok(())
}
fn run(log:&Log)->Result<()> {
 let mode=std::env::args().nth(1).unwrap_or_default();
 if mode=="--offscreen" {let d=draw::Draw::new(log,None,D3D_FEATURE_LEVEL_11_0.0)?;return d.offscreen();}
 if mode!="--probe"&&mode!="--session" {println!("사용법: tfm2_vr_cube.exe --probe | --offscreen | --session\n--session은 사용자가 헤드셋을 착용하고 직접 실행합니다.");return Ok(());}
 let stop=Arc::new(AtomicBool::new(false));let exit_sent=Arc::new(AtomicBool::new(false));let shared:Arc<Mutex<Option<xr::Session<xr::D3D11>>>>=Arc::new(Mutex::new(None));let(s,e,h,l)=(stop.clone(),exit_sent.clone(),shared.clone(),log.clone());ctrlc::set_handler(move||{s.store(true,Ordering::SeqCst);l.line("Ctrl+C 수신");let session=h.lock().ok().and_then(|x|x.clone());if let Some(session)=session{if !e.swap(true,Ordering::SeqCst){let _=l.xr("xrRequestExitSession(Ctrl+C)",||session.request_exit());}}})?;
 let monitor=if mode=="--session"{Some(monitor::Monitor::new(log,stop.clone(),exit_sent.clone(),shared.clone()))}else{None};
 let instance=instance(log)?;if mode=="--probe"{log.line("probe 완료 · OpenXR 세션/스왑체인/헤드셋 프레임은 만들지 않았습니다");let t=Instant::now();drop(instance);log.line(format!("OpenXR xrDestroyInstance {:.3}ms",t.elapsed().as_secs_f64()*1000.0));return Ok(());}
 session_mode(log,instance,stop,exit_sent,shared,monitor.as_ref().unwrap().stats.clone())
}
fn main(){let log=match Log::new(){Ok(l)=>l,Err(e)=>{eprintln!("로그 생성 실패: {e:#}");std::process::exit(1)}};match run(&log){Ok(())=>log.line("프로그램 정상 종료"),Err(e)=>{log.line(format!("프로그램 오류: {e:#}"));std::process::exit(1);}}}
