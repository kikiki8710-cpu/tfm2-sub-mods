//! This application owns the only HWND touched by this module.
use anyhow::{Result,bail};
use windows::{core::w,Win32::{Foundation::*,Graphics::Gdi::*,System::LibraryLoader::GetModuleHandleW,UI::{WindowsAndMessaging::*,Input::KeyboardAndMouse::{SetCapture,ReleaseCapture,GetCapture}}}};
#[derive(Clone,Copy,Default)]pub struct Actions{pub toggle:bool,pub back:bool,pub forward:bool,pub recenter:bool,pub speed:bool,pub height:f32,pub scale:f32,pub orbit_x:f32,pub orbit_y:f32,pub wheel:f32}
struct State{play:bool,quit:bool,drag:bool,last:(i16,i16),pub width:u32,pub height:u32,pub resized:bool,actions:Actions}
pub struct Window{pub hwnd:HWND,state:Box<State>}
unsafe extern "system" fn procedure(hwnd:HWND,message:u32,wparam:WPARAM,lparam:LPARAM)->LRESULT{
 let ptr=GetWindowLongPtrW(hwnd,GWLP_USERDATA)as *mut State;if ptr.is_null(){return DefWindowProcW(hwnd,message,wparam,lparam);}let state=&mut *ptr;
 match message{
  WM_CLOSE=>{state.quit=true;let _=DestroyWindow(hwnd);return LRESULT(0);},WM_DESTROY=>{state.quit=true;return LRESULT(0);},
  WM_SIZE=>{let(w,h)=((lparam.0 as u32&0xffff),(lparam.0 as u32>>16)&0xffff);if w>0&&h>0{state.width=w;state.height=h;state.resized=true;}return LRESULT(0);},
  WM_LBUTTONDOWN=>{state.drag=true;state.last=(lparam.0 as i16,(lparam.0>>16)as i16);SetCapture(hwnd);return LRESULT(0);},
  WM_LBUTTONUP=>{state.drag=false;if GetCapture()==hwnd{let _=ReleaseCapture();}return LRESULT(0);},
  WM_MOUSEMOVE if state.drag=>{let p=(lparam.0 as i16,(lparam.0>>16)as i16);state.actions.orbit_x+=(p.0-state.last.0)as f32*0.005;state.actions.orbit_y+=(p.1-state.last.1)as f32*0.005;state.last=p;return LRESULT(0);},
  WM_MOUSEWHEEL=>{state.actions.wheel+=((wparam.0>>16)as i16)as f32/120.0;return LRESULT(0);},
  WM_KEYDOWN if lparam.0&(1<<30)==0=>{match wparam.0 as u32{0x20=>state.actions.toggle=true,0x25=>state.actions.back=true,0x27=>state.actions.forward=true,0x52=>state.actions.recenter=true,0x42=>state.actions.speed=true,0x21=>state.actions.height+=0.05,0x22=>state.actions.height-=0.05,0xbb|0x6b=>state.actions.scale+=0.1,0xbd|0x6d=>state.actions.scale-=0.1,0x1b=>{state.quit=true;let _=DestroyWindow(hwnd);},_=>{}}return LRESULT(0);},
  WM_PAINT=>{let mut paint=PAINTSTRUCT::default();let dc=BeginPaint(hwnd,&mut paint);let mut text:Vec<u16>=(if state.play{crate::lang::t("TFM2 VR 이 실행 중입니다.\n이 창이나 게임을 닫으면 VR 도 끝납니다.")}else{"로컬 자료를 준비하고 있습니다.\nVR 종료: 창 닫기 또는 Ctrl+C\n일반 화면: 드래그·휠, Space 재생, ←/→ 탐색"}).encode_utf16().collect();DrawTextW(dc,&mut text,&mut RECT{left:20,top:20,right:800,bottom:200},DT_LEFT);let _=EndPaint(hwnd,&paint);return LRESULT(0);},_=>{}}
 DefWindowProcW(hwnd,message,wparam,lparam)
}
impl Window{
 pub fn new(minimized:bool)->Result<Self>{unsafe{let instance=HINSTANCE(GetModuleHandleW(None)?.0);let class=WNDCLASSW{lpfnWndProc:Some(procedure),hInstance:instance,lpszClassName:w!("TFM2VRNativeViewer"),hCursor:LoadCursorW(None,IDC_ARROW)?,..Default::default()};if RegisterClassW(&class)==0{bail!("자체 창 클래스 등록 실패");}let args:Vec<_>=std::env::args().collect();let diagnostic=!args.iter().any(|a|a=="--session")&&args.iter().any(|a|matches!(a.as_str(),"--validate"|"--soak-seconds"|"--headset-pair"));let title=if diagnostic{w!("[검사] TFM2 VR · 성능·화면 검증")}else if args.iter().any(|a|a=="--play"){w!("TFM2 VR")}else{w!("TFM2 VR · 로컬 경기 뷰어")};let hwnd=CreateWindowExW(WINDOW_EX_STYLE::default(),class.lpszClassName,title,WS_OVERLAPPEDWINDOW,80,80,1280,900,None,None,Some(instance),None)?;let mut rect=RECT::default();GetClientRect(hwnd,&mut rect)?;let state=Box::new(State{play:args.iter().any(|a|a=="--play"),quit:false,drag:false,last:(0,0),width:(rect.right-rect.left)as u32,height:(rect.bottom-rect.top)as u32,resized:false,actions:Actions::default()});let mut this=Self{hwnd,state};SetWindowLongPtrW(hwnd,GWLP_USERDATA,(&mut *this.state as *mut State)as isize);let _=ShowWindow(hwnd,if minimized{SW_SHOWMINNOACTIVE}else{SW_SHOWNOACTIVATE});Ok(this)}}
 pub fn poll(&mut self)->bool{unsafe{let mut message=MSG::default();while PeekMessageW(&mut message,None,0,0,PM_REMOVE).as_bool(){if message.message==WM_QUIT{self.state.quit=true;}let _=TranslateMessage(&message);DispatchMessageW(&message);}}!self.state.quit}
 pub fn actions(&mut self)->Actions{std::mem::take(&mut self.state.actions)}
 pub fn size(&self)->(u32,u32){(self.state.width,self.state.height)}
 pub fn resized(&mut self)->bool{std::mem::replace(&mut self.state.resized,false)}
}
impl Drop for Window{fn drop(&mut self){unsafe{let ptr=(&mut *self.state as *mut State)as isize;if GetWindowLongPtrW(self.hwnd,GWLP_USERDATA)==ptr{let _=DestroyWindow(self.hwnd);}}}}
