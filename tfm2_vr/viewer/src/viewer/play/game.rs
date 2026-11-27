//! 게임 프로세스·창·입력. 게임 메모리는 건드리지 않는다(창 찾기, 실행, 마우스 입력만).
use anyhow::Result;
use std::{sync::atomic::{AtomicBool, Ordering}, time::{Duration, Instant}};

use windows::{core::{BOOL, PCWSTR}, Win32::{Foundation::*, Graphics::{Dwm::{DwmGetWindowAttribute, DWMWA_EXTENDED_FRAME_BOUNDS}, Gdi::ClientToScreen}, System::{ProcessStatus::K32GetProcessImageFileNameW, Threading::*, Power::*}, UI::{Input::KeyboardAndMouse::*, Shell::ShellExecuteW, WindowsAndMessaging::*}}};

static FOCUSING: AtomicBool = AtomicBool::new(false);
const EXE: &str = "TeamfightManager2.exe";
pub const APP_ID: &str = "3009300";
struct Search { found: Option<HWND>, area: i64 }
unsafe extern "system" fn each(hwnd: HWND, param: LPARAM) -> BOOL {
    let s = &mut *(param.0 as *mut Search);
    if !IsWindowVisible(hwnd).as_bool() { return TRUE; }
    let mut pid = 0;
    GetWindowThreadProcessId(hwnd, Some(&mut pid));
    let Ok(p) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) else { return TRUE };
    let mut b = [0u16; 1024];
    let n = K32GetProcessImageFileNameW(p, &mut b) as usize;
    let _ = CloseHandle(p);
    if String::from_utf16_lossy(&b[..n]).ends_with(EXE) {
        let mut r = RECT::default();
        let _ = GetClientRect(hwnd, &mut r);
        let area = (r.right as i64) * r.bottom as i64;
        if area > s.area { s.area = area; s.found = Some(hwnd); }
    }
    TRUE
}
pub fn find() -> Option<HWND> { let mut s = Search { found: None, area: 320 * 200 }; unsafe { let _ = EnumWindows(Some(each), LPARAM(&mut s as *mut _ as isize)); } s.found }
pub fn alive(hwnd: HWND) -> bool { unsafe { IsWindow(Some(hwnd)).as_bool() } }
fn wide(s: &str) -> Vec<u16> { s.encode_utf16().chain([0]).collect() }
/// 스팀으로 실행한다. 경로를 주면 그 exe 를 직접 실행한다(시험용).
pub fn launch(direct: Option<&str>) -> Result<()> {
    unsafe {
        match direct {
            Some(path) => { let dir = std::path::Path::new(path).parent().map(|p| p.to_string_lossy().into_owned()).unwrap_or_default(); std::process::Command::new(path).current_dir(dir).stdin(std::process::Stdio::null()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).spawn()?; }
            None => { let (op, url) = (wide("open"), wide(&format!("steam://rungameid/{APP_ID}"))); ShellExecuteW(None, PCWSTR(op.as_ptr()), PCWSTR(url.as_ptr()), None, None, SW_SHOWNORMAL); }
        }
    }
    Ok(())
}
/// 캡처 그림(창 전체) 안에서 게임 화면(클라이언트 영역)이 차지하는 비율 사각형.
pub fn client_uv(hwnd: HWND) -> [f32; 4] {
    unsafe {
        let (mut frame, mut client, mut origin) = (RECT::default(), RECT::default(), POINT::default());
        if DwmGetWindowAttribute(hwnd, DWMWA_EXTENDED_FRAME_BOUNDS, &mut frame as *mut _ as *mut _, std::mem::size_of::<RECT>() as u32).is_err() || GetClientRect(hwnd, &mut client).is_err() { return [0.0, 0.0, 1.0, 1.0]; }
        let _ = ClientToScreen(hwnd, &mut origin);
        let (w, h) = ((frame.right - frame.left).max(1) as f32, (frame.bottom - frame.top).max(1) as f32);
        let uv = [(origin.x - frame.left) as f32 / w, (origin.y - frame.top) as f32 / h, client.right as f32 / w, client.bottom as f32 / h];
        if uv.iter().all(|v| v.is_finite()) && uv[2] > 0.2 && uv[3] > 0.2 { [uv[0].clamp(0.0, 0.8), uv[1].clamp(0.0, 0.8), uv[2].min(1.0), uv[3].min(1.0)] } else { [0.0, 0.0, 1.0, 1.0] }
    }
}
pub struct Mouse { hwnd: HWND, focus: Instant, last: (i32, i32), pub down: bool }
impl Mouse {
    pub fn new(hwnd: HWND) -> Self { Self { hwnd, focus: Instant::now() - Duration::from_secs(10), last: (i32::MIN, 0), down: false } }
    pub fn retarget(&mut self, hwnd: HWND) { self.hwnd = hwnd; self.last = (i32::MIN, 0); }
    fn send(flags: MOUSE_EVENT_FLAGS, dx: i32, dy: i32, data: i32) { unsafe { let input = INPUT { r#type: INPUT_MOUSE, Anonymous: INPUT_0 { mi: MOUSEINPUT { dx, dy, mouseData: data as u32, dwFlags: flags, time: 0, dwExtraInfo: 0 } } }; SendInput(&[input], std::mem::size_of::<INPUT>() as i32); } }
    /// 게임 창이 앞에 있어야 입력이 간다. 레이저가 화면을 가리킬 때만 부른다.
    /// 게임이 불러오는 중이면 이 호출이 오래 막힐 수 있어 따로 돌리고, 끝날 때까지 다시 부르지 않는다.
    pub fn focus(&mut self) {
        unsafe { if GetForegroundWindow() == self.hwnd || self.focus.elapsed() < Duration::from_millis(2000) || IsHungAppWindow(self.hwnd).as_bool() { return; } }
        self.focus = Instant::now();
        if FOCUSING.swap(true, Ordering::AcqRel) { return; }
        let target = self.hwnd.0 as isize;
        std::thread::spawn(move || {
            unsafe {
                let target = HWND(target as *mut _);
                // 지금 앞에 있는 창의 입력 흐름에 잠깐 붙어 전환이 막히지 않게 한다.
                let front = GetWindowThreadProcessId(GetForegroundWindow(), None);
                let me = GetCurrentThreadId();
                let joined = front != 0 && front != me && AttachThreadInput(front, me, true).as_bool();
                let _ = BringWindowToTop(target);
                let _ = SetForegroundWindow(target);
                if joined { let _ = AttachThreadInput(front, me, false); }
            }
            FOCUSING.store(false, Ordering::Release);
        });
    }
    /// u, v = 게임 화면 안 비율.
    pub fn move_to(&mut self, u: f32, v: f32) {
        unsafe {
            let (mut client, mut p) = (RECT::default(), POINT::default());
            if GetClientRect(self.hwnd, &mut client).is_err() { return; }
            let _ = ClientToScreen(self.hwnd, &mut p);
            let (x, y) = (p.x + (u.clamp(0.0, 0.999) * client.right as f32) as i32, p.y + (v.clamp(0.0, 0.999) * client.bottom as f32) as i32);
            if (x, y) == self.last { return; }
            self.last = (x, y);
            let (l, t, w, h) = (GetSystemMetrics(SM_XVIRTUALSCREEN), GetSystemMetrics(SM_YVIRTUALSCREEN), GetSystemMetrics(SM_CXVIRTUALSCREEN).max(2), GetSystemMetrics(SM_CYVIRTUALSCREEN).max(2));
            Self::send(MOUSEEVENTF_MOVE | MOUSEEVENTF_ABSOLUTE | MOUSEEVENTF_VIRTUALDESK, ((x - l) as i64 * 65536 / (w as i64) + 32) as i32, ((y - t) as i64 * 65536 / (h as i64) + 32) as i32, 0);
        }
    }
    pub fn button(&mut self, down: bool) { if down == self.down { return; } unsafe { if down && GetForegroundWindow() != self.hwnd { return; } } self.down = down; Self::send(if down { MOUSEEVENTF_LEFTDOWN } else { MOUSEEVENTF_LEFTUP }, 0, 0, 0); }
    pub fn wheel(&mut self, amount: i32) { unsafe { if GetForegroundWindow() != self.hwnd { return; } } Self::send(MOUSEEVENTF_WHEEL, 0, 0, amount); }
    pub fn key(&mut self, vk: u16) { unsafe { if GetForegroundWindow() != self.hwnd { return; } for up in [false, true] { let input = INPUT { r#type: INPUT_KEYBOARD, Anonymous: INPUT_0 { ki: KEYBDINPUT { wVk: VIRTUAL_KEY(vk), wScan: 0, dwFlags: if up { KEYEVENTF_KEYUP } else { KEYBD_EVENT_FLAGS(0) }, time: 0, dwExtraInfo: 0 } } }; SendInput(&[input], std::mem::size_of::<INPUT>() as i32); } } }
}
impl Drop for Mouse { fn drop(&mut self) { if self.down { Self::send(MOUSEEVENTF_LEFTUP, 0, 0, 0); } } }

/// 게임 창 귀퉁이에 얹는 거의 투명한 작은 창. 전체 화면 직통 출력으로 바뀌면 화면 받기가 멈추므로 항상 합성되게 한다.
pub struct Cover { hwnd: HWND, at: (i32, i32) }
unsafe extern "system" fn cover_proc(h: HWND, m: u32, w: WPARAM, l: LPARAM) -> LRESULT { if m == WM_NCHITTEST { return LRESULT(-1); } DefWindowProcW(h, m, w, l) }
impl Cover {
    pub fn new() -> Option<Self> {
        unsafe {
            let instance = HINSTANCE(windows::Win32::System::LibraryLoader::GetModuleHandleW(None).ok()?.0);
            let name = windows::core::w!("TFM2VRCover");
            let class = WNDCLASSW { lpfnWndProc: Some(cover_proc), hInstance: instance, lpszClassName: name, ..Default::default() };
            RegisterClassW(&class);
            let hwnd = CreateWindowExW(WS_EX_TOPMOST | WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE, name, windows::core::w!("TFM2 VR"), WS_POPUP, 0, 0, 4, 4, None, None, Some(instance), None).ok()?;
            let _ = SetLayeredWindowAttributes(hwnd, COLORREF(0), 2, LWA_ALPHA);
            let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
            Some(Self { hwnd, at: (i32::MIN, 0) })
        }
    }
    pub fn follow(&mut self, game: HWND) {
        unsafe {
            let mut p = POINT { x: 6, y: 6 };
            let _ = ClientToScreen(game, &mut p);
            // 게임이 앞으로 올라와도 위에 남도록 주기적으로 다시 얹는다.
            let _ = SetWindowPos(self.hwnd, Some(HWND_TOPMOST), p.x, p.y, 4, 4, SWP_NOACTIVATE | SWP_SHOWWINDOW);
            self.at = (p.x, p.y);
        }
    }
}
impl Drop for Cover { fn drop(&mut self) { unsafe { let _ = DestroyWindow(self.hwnd); } } }

/// 다른 프로그램이 바빠도 헤드셋 장면이 밀리지 않게 이 프로그램의 우선순위를 올린다.
pub fn priority() { unsafe { let _ = SetPriorityClass(GetCurrentProcess(), HIGH_PRIORITY_CLASS); let _ = SetThreadPriority(GetCurrentThread(), THREAD_PRIORITY_HIGHEST);
    // 모니터가 절전으로 꺼지면 게임 화면 받기가 멈춘다.
    let _ = SetThreadExecutionState(ES_CONTINUOUS | ES_DISPLAY_REQUIRED | ES_SYSTEM_REQUIRED); } }

/// 게임 창 상태 한 줄(기록용).
pub fn state(hwnd: HWND) -> String {
    unsafe {
        let mut pid = 0;
        GetWindowThreadProcessId(GetForegroundWindow(), Some(&mut pid));
        let mut name = String::new();
        if let Ok(p) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) { let mut b = [0u16; 512]; let n = K32GetProcessImageFileNameW(p, &mut b) as usize; let _ = CloseHandle(p); name = String::from_utf16_lossy(&b[..n]).rsplit('\\').next().unwrap_or("").to_string(); }
        format!("응답없음={} 최소화={} 앞창={}", IsHungAppWindow(hwnd).as_bool(), IsIconic(hwnd).as_bool(), name)
    }
}
/// VR 을 하는 동안 게임이 다른 작업에 밀리지 않게 한 단계 올린다.
pub fn favor(hwnd: HWND) { unsafe { let mut pid = 0; GetWindowThreadProcessId(hwnd, Some(&mut pid)); if let Ok(p) = OpenProcess(PROCESS_SET_INFORMATION, false, pid) { let _ = SetPriorityClass(p, ABOVE_NORMAL_PRIORITY_CLASS); let _ = CloseHandle(p); } } }
