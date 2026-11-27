//! daram2_view_plus — daram2 「뷰 플러스」 6종 통합판(밴픽뷰 제외 · 2026-11-25 유저 지시 · 밴픽뷰는 단독판만) v1.0.0 (게임 0.7.4 재빌드 2026-11-25 · 최초 0.7.2 2026-11-11 · stable ABI).
//! 왜: 스팀 창작마당 공식 업로더(TFM2ModUploader)는 모드 폴더 하나 = 항목 하나라서 6개를 한 폴더·dll 1개·mod.mod_info 1개로 묶는다.
//! 모듈(= 원본 크레이트 · 각자 단독판으로도 그대로 빌드·배포된다):
//!   training_view_plus · facility_view_plus · custom_tier_assignment · roster_view_plus · coaching_staff_view_plus · recruitment_view_plus
//! 구조:
//!   · 원본을 rlib + feature "embedded" 로 경로 의존 → 원본의 진입점 export(`declare_stable_mod!`)는 꺼지고
//!     `<원본>::embedded::init(host, 통합판 폴더)` 가 (클라 확장, 서버 확장) 을 돌려준다.
//!     이 크레이트의 진입점 하나가 그 훅(on_init·pre/post_update·on_end·pre/post_render · 서버 4훅)을 모듈 순서대로 부른다 — 모듈마다 catch_unwind.
//!   · 서버 이벤트 큐는 모드당 하나 → 모듈이 각자 `ctx.take_events()` 하면 남의 이벤트까지 비운다. 그래서 post_update 머리에서 한 번만 비우고
//!     이벤트를 쓰는 모듈(training·facility·custom_tier)에 `feed_events` 로 나눠 준다(각 모듈은 자기 이벤트 이름만 보관).
//!   · 명령(send_command)은 모듈마다 이름이 달라(tvp_apply·fvp_produce·cta_apply) 서버 handle_command 는 Pass 가 아닌 첫 결과를 쓴다.
//!   · 같은 기능의 단독판이 같이 로드돼 있으면(`GetModuleHandleW("<원본>.dll")`) 그 모듈만 끈다(훅·UI·게임 코드 패치 이중 방지).
//!     1차 = init 때 이미 로드된 것(그 모듈은 init 부수효과도 안 한다) · 확정 = 첫 프레임 훅 때(늦게 로드된 것까지) · 이후 프로세스 동안 고정.
//! 파일(통합판 dll 폴더 = 로컬 `mods\daram2_view_plus\` 또는 워크숍 폴더):
//!   · `daram2_view_plus.log` — 실행마다 새로 씀(시작·설정 이관·모듈 on/off·서버 이벤트 분배·모듈 패닉). `mods\_log_root.txt` 가 있으면 그 아래.
//!   · 모듈 파일 = 원본 이름 그대로(roster_view_plus.cfg 등). 겹치는 debug.log 만 `<원본>_debug.log`(진단 빌드·flag 때만 생김).
//!   · 통합판 폴더에 없는 설정 파일은 시작 때 `<게임>\mods\<원본>\` 의 단독판 것을 복사해 온다(덮어쓰기 없음 · 원본 파일은 그대로).
//! 메모리 안전(CLAUDE.md §3): 모듈 호출은 전부 catch_unwind · 이 크레이트는 게임 메모리를 직접 건드리지 않는다(원본 모듈 안의 것은 원본 규약 그대로).
use mod_api_stable::{
    declare_stable_mod, CommandResultV1, LogLevel, StableClient, StableCommand, StableExtension, StableHost, StableMod, StableModEvent,
    StableServerCtx, StableServerExtension,
};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::OnceLock;
#[path = "../../common/log_dir.rs"]
mod log_dir; // 로그 폴더(`mods\_log_root.txt` 있으면 그 아래) — 원본 모듈들과 같은 규칙

const MOD_ID: &str = "daram2_view_plus";
const VERSION: &str = env!("CARGO_PKG_VERSION");
/// 모듈 하나당 패닉을 로그에 남기는 최대 횟수(매 프레임 패닉이면 로그가 불어난다).
const PANIC_LOG_MAX: u32 = 5;

// ───────── 모듈 표 ─────────
type Parts = (Box<dyn StableExtension>, Option<Box<dyn StableServerExtension>>);
struct Spec {
    /// 단독판 mod_id(= 단독판 dll 이름 · 단독판 폴더 이름)
    id: &'static str,
    init: fn(&StableHost, &str) -> Parts,
    /// 서버 이벤트를 쓰는 모듈만
    feed: Option<fn(&[StableModEvent])>,
}
const SPECS: [Spec; 6] = [
    Spec { id: "training_view_plus", init: training_view_plus::embedded::init, feed: Some(training_view_plus::embedded::feed_events) },
    Spec { id: "facility_view_plus", init: facility_view_plus::embedded::init, feed: Some(facility_view_plus::embedded::feed_events) },
    Spec { id: "custom_tier_assignment", init: custom_tier_assignment::embedded::init, feed: Some(custom_tier_assignment::embedded::feed_events) },
    Spec { id: "roster_view_plus", init: roster_view_plus::embedded::init, feed: None },
    Spec { id: "coaching_staff_view_plus", init: coaching_staff_view_plus::embedded::init, feed: None },
    Spec { id: "recruitment_view_plus", init: recruitment_view_plus::embedded::init, feed: None },
];
/// 단독판 → 통합판 설정 이관 대상(단독판 폴더 `<게임>\mods\<id>\` 안 파일 · 통합판에서도 같은 이름). coaching 은 설정 파일 없음.
const LEGACY_CFG: [(&str, &str); 5] = [
    ("training_view_plus", "training_view_plus.cfg"),
    ("facility_view_plus", "facility_view_plus.cfg"),
    ("custom_tier_assignment", "custom_tier_assignment.cfg"),
    ("roster_view_plus", "roster_view_plus.cfg"),
    ("recruitment_view_plus", "recruitment_view_plus.cfg"),
];

struct Slot {
    id: &'static str,
    ext: Option<Box<dyn StableExtension>>,
    srv: Option<Box<dyn StableServerExtension>>,
    feed: Option<fn(&[StableModEvent])>,
    /// init 을 안 한 이유(None = init 함)
    skip: Option<&'static str>,
    panics: AtomicU32,
}
static SLOTS: OnceLock<Vec<Slot>> = OnceLock::new();
/// 모듈 on/off 확정값(첫 프레임 훅에서 1회).
static ON: OnceLock<Vec<bool>> = OnceLock::new();

fn slots() -> &'static [Slot] { SLOTS.get().map(Vec::as_slice).unwrap_or(&[]) }
/// 모듈 on/off 확정 — 첫 프레임(또는 서버) 훅에서 처음 불릴 때 1회. 그때는 모든 모드 dll 이 로드돼 있다. 이후 고정.
fn on() -> &'static [bool] {
    ON.get_or_init(|| {
        let v: Vec<bool> = slots().iter().map(|s| s.ext.is_some() && !standalone_loaded(s.id)).collect();
        let desc: Vec<String> = slots()
            .iter()
            .zip(&v)
            .map(|(s, on)| {
                let st = if *on { "on" } else if let Some(r) = s.skip { r } else { "off(standalone loaded)" };
                format!("{}={}", s.id, st)
            })
            .collect();
        log(&format!("모듈 확정: {}", desc.join(" · ")));
        v
    })
}
fn is_on(on: &[bool], i: usize) -> bool { on.get(i).copied().unwrap_or(false) }

/// 모듈 호출 하나를 패닉 격리로 감싼다(한 모듈의 패닉이 다음 모듈을 막지 않게).
fn guard(i: usize, hook: &str, f: impl FnOnce()) {
    if catch_unwind(AssertUnwindSafe(f)).is_err() { note_panic(i, hook); }
}
fn note_panic(i: usize, hook: &str) {
    let Some(s) = slots().get(i) else { return };
    let n = s.panics.fetch_add(1, Ordering::Relaxed).saturating_add(1);
    if n <= PANIC_LOG_MAX {
        log(&format!("패닉 {}.{} ({}번째{})", s.id, hook, n, if n == PANIC_LOG_MAX { " — 이후 이 모듈 패닉 기록 생략" } else { "" }));
    }
}

// ───────── Win32 ─────────
#[link(name = "kernel32")]
extern "system" {
    fn GetModuleHandleW(name: *const u16) -> usize;
    fn GetModuleHandleExW(flags: u32, addr: *const u16, h: *mut usize) -> i32;
    fn GetModuleFileNameW(module: usize, buf: *mut u16, size: u32) -> u32;
}
fn module_file(h: usize) -> Option<String> {
    let mut buf = vec![0u16; 1024];
    let n = unsafe { GetModuleFileNameW(h, buf.as_mut_ptr(), buf.len() as u32) } as usize;
    if n == 0 || n >= buf.len() { return None; }
    Some(String::from_utf16_lossy(&buf[..n]))
}
fn parent(p: &str) -> Option<String> { p.rfind(|c| c == '\\' || c == '/').map(|i| p[..i].to_string()) }
/// 이 dll 이 있는 폴더(로컬 `mods\daram2_view_plus` 또는 워크숍 폴더 — exe 기준으로 잡지 않는다).
fn dll_dir() -> Option<String> {
    let mut h = 0usize;
    // 0x4 = GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS · 0x2 = UNCHANGED_REFCOUNT
    if unsafe { GetModuleHandleExW(0x4 | 0x2, dll_dir as *const () as *const u16, &mut h) } == 0 || h == 0 { return None; }
    parent(&module_file(h)?)
}
fn exe_dir() -> Option<String> { parent(&module_file(0)?) }
/// 같은 기능의 단독판 dll(`mods\<id>\<id>.dll`)이 이 프로세스에 로드돼 있나. 로더는 모드 dll 을 그 자리에서 이름 그대로 올린다.
fn standalone_loaded(id: &str) -> bool {
    let w: Vec<u16> = format!("{}.dll", id).encode_utf16().chain(std::iter::once(0)).collect();
    unsafe { GetModuleHandleW(w.as_ptr()) != 0 }
}

// ───────── 로그(daram2_view_plus.log · 실행마다 새로) ─────────
static LOG_FILE: OnceLock<String> = OnceLock::new();
static T0: OnceLock<std::time::Instant> = OnceLock::new();
fn log_start(dir: &str) {
    let p = format!(r"{}\{}.log", log_dir::log_dir(dir), MOD_ID);
    if std::fs::write(&p, b"").is_ok() { let _ = LOG_FILE.set(p); }
}
fn log(s: &str) {
    let Some(p) = LOG_FILE.get() else { return };
    let t = T0.get().map(|t| t.elapsed().as_secs_f32()).unwrap_or(0.0);
    use std::io::Write;
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(p) {
        let _ = writeln!(f, "[{:8.1}s] {}", t, s);
    }
}

/// 단독판 → 통합판 설정 이관: 통합판 폴더에 없을 때만 `<게임>\mods\<원본>\<파일>` 을 복사(덮어쓰기 없음 · 원본 파일은 그대로).
fn migrate_legacy(dir: &str) {
    let Some(exe) = exe_dir() else { return };
    for (id, file) in LEGACY_CFG {
        let dst = format!(r"{}\{}", dir, file);
        if std::path::Path::new(&dst).exists() { continue; }
        let src = format!(r"{}\mods\{}\{}", exe, id, file);
        if !std::path::Path::new(&src).is_file() { continue; }
        match std::fs::copy(&src, &dst) {
            Ok(n) => log(&format!("설정 이관 mods\\{}\\{} → 통합판 폴더 ({}B)", id, file, n)),
            Err(e) => log(&format!("설정 이관 실패 mods\\{}\\{}: {}", id, file, e)),
        }
    }
}

// ───────── 클라 확장(모듈 순서대로) ─────────
struct Hub;
impl StableExtension for Hub {
    fn on_init(&self, ctx: &mut StableClient<'_>) {
        // on_init 시점엔 모든 모드가 로드됐다는 보장이 없어 확정(on())하지 않는다 — 지금 단독판이 안 보이는 모듈에만 전달
        //   (원본 모듈은 on_init 을 쓰지 않는다 · 2026-11-11 기준).
        for (i, s) in slots().iter().enumerate() {
            let Some(e) = &s.ext else { continue };
            let ok = match ON.get() { Some(v) => is_on(v, i), None => !standalone_loaded(s.id) };
            if ok { guard(i, "on_init", || e.on_init(ctx)); }
        }
    }
    fn pre_update(&self, ctx: &mut StableClient<'_>, dt: u64) {
        let on = on();
        for (i, s) in slots().iter().enumerate() {
            let (true, Some(e)) = (is_on(on, i), &s.ext) else { continue };
            guard(i, "pre_update", || e.pre_update(ctx, dt));
        }
    }
    fn post_update(&self, ctx: &mut StableClient<'_>, dt: u64) {
        let on = on();
        // 서버 이벤트 = 모드 큐 하나 → 여기서 한 번만 비우고 나눠 준다(각 모듈은 자기 이벤트 이름만 보관 · 꺼진 모듈엔 안 줌).
        let evs = ctx.take_events();
        if !evs.is_empty() {
            log(&format!("서버 이벤트 {}개: {:?}", evs.len(), evs.iter().map(|e| e.event.as_str()).collect::<Vec<_>>()));
            for (i, s) in slots().iter().enumerate() {
                let (true, Some(feed)) = (is_on(on, i), s.feed) else { continue };
                guard(i, "feed_events", || feed(&evs));
            }
        }
        for (i, s) in slots().iter().enumerate() {
            let (true, Some(e)) = (is_on(on, i), &s.ext) else { continue };
            guard(i, "post_update", || e.post_update(ctx, dt));
        }
    }
    fn on_end(&self) {
        let on = on();
        for (i, s) in slots().iter().enumerate() {
            let (true, Some(e)) = (is_on(on, i), &s.ext) else { continue };
            guard(i, "on_end", || e.on_end());
        }
    }
    fn pre_render(&self, ctx: &mut StableClient<'_>) {
        let on = on();
        for (i, s) in slots().iter().enumerate() {
            let (true, Some(e)) = (is_on(on, i), &s.ext) else { continue };
            guard(i, "pre_render", || e.pre_render(ctx));
        }
    }
    fn post_render(&self, ctx: &mut StableClient<'_>) {
        let on = on();
        for (i, s) in slots().iter().enumerate() {
            let (true, Some(e)) = (is_on(on, i), &s.ext) else { continue };
            guard(i, "post_render", || e.post_render(ctx));
        }
    }
}

// ───────── 서버 확장(서버를 쓰는 모듈 = training·facility·custom_tier) ─────────
struct HubSrv;
impl StableServerExtension for HubSrv {
    fn on_server_start(&self, ctx: &mut StableServerCtx<'_>) {
        let on = on();
        for (i, s) in slots().iter().enumerate() {
            let (true, Some(v)) = (is_on(on, i), &s.srv) else { continue };
            guard(i, "on_server_start", || v.on_server_start(ctx));
        }
    }
    fn before_management_tick(&self, ctx: &mut StableServerCtx<'_>) {
        let on = on();
        for (i, s) in slots().iter().enumerate() {
            let (true, Some(v)) = (is_on(on, i), &s.srv) else { continue };
            guard(i, "before_management_tick", || v.before_management_tick(ctx));
        }
    }
    fn after_management_tick(&self, ctx: &mut StableServerCtx<'_>) {
        let on = on();
        for (i, s) in slots().iter().enumerate() {
            let (true, Some(v)) = (is_on(on, i), &s.srv) else { continue };
            guard(i, "after_management_tick", || v.after_management_tick(ctx));
        }
    }
    /// 명령 이름은 모듈마다 다르다 — Pass 가 아닌 첫 결과를 쓴다(패닉한 모듈은 Pass 로 보고 다음 모듈로).
    fn handle_command(&self, ctx: &mut StableServerCtx<'_>, cmd: &StableCommand<'_>) -> CommandResultV1 {
        let on = on();
        for (i, s) in slots().iter().enumerate() {
            let (true, Some(v)) = (is_on(on, i), &s.srv) else { continue };
            match catch_unwind(AssertUnwindSafe(|| v.handle_command(ctx, cmd))) {
                Ok(CommandResultV1::Pass) => {}
                Ok(r) => return r,
                Err(_) => note_panic(i, "handle_command"),
            }
        }
        CommandResultV1::Pass
    }
}

// ───────── 진입점 ─────────
fn init(host: &StableHost) -> StableMod {
    let _ = T0.set(std::time::Instant::now());
    // 통합판 폴더 = 이 dll 폴더(못 구하면 exe 기준 mods\daram2_view_plus · 그것도 안 되면 상대 경로)
    let dir = dll_dir()
        .or_else(|| exe_dir().map(|e| format!(r"{}\mods\{}", e, MOD_ID)))
        .unwrap_or_else(|| format!(r"mods\{}", MOD_ID));
    log_start(&dir);
    let v = host.game_version();
    host.log(LogLevel::Info, &format!("{} {} (View Plus integrated · {} modules)", MOD_ID, VERSION, SPECS.len()));
    log(&format!("INIT {} {} game {}.{}.{} host_abi={} dir={}", MOD_ID, VERSION, v.major, v.minor, v.patch, host.abi_level(), dir));
    migrate_legacy(&dir);
    let mut slots = Vec::with_capacity(SPECS.len());
    for sp in SPECS.iter() {
        let mut slot = Slot { id: sp.id, ext: None, srv: None, feed: sp.feed, skip: None, panics: AtomicU32::new(0) };
        if standalone_loaded(sp.id) {
            // 단독판이 먼저 로드돼 있다 → 이 모듈은 init 부수효과(설정 읽기 등)부터 하지 않는다
            slot.skip = Some("off(standalone loaded at init)");
            log(&format!("  {} : 끔 — 단독판 {}.dll 이 이미 로드됨", sp.id, sp.id));
            host.log(LogLevel::Info, &format!("{}: module {} off (standalone {}.dll loaded)", MOD_ID, sp.id, sp.id));
        } else {
            match catch_unwind(AssertUnwindSafe(|| (sp.init)(host, &dir))) {
                Ok((e, s)) => {
                    log(&format!("  {} : init 완료{}", sp.id, if s.is_some() { " (서버 확장 포함)" } else { "" }));
                    slot.ext = Some(e);
                    slot.srv = s;
                }
                Err(_) => {
                    slot.skip = Some("off(init panic)");
                    log(&format!("  {} : init 패닉 — 이 모듈 끔", sp.id));
                    host.log(LogLevel::Warn, &format!("{}: module {} init panicked (module off)", MOD_ID, sp.id));
                }
            }
        }
        slots.push(slot);
    }
    let _ = SLOTS.set(slots);
    let mut d = StableMod::new(MOD_ID);
    d.set_extension(Hub);
    d.set_server_extension(HubSrv);
    d
}
declare_stable_mod!(init);

#[cfg(test)]
mod tests {
    use super::*;
    /// 모듈 id 중복 없음 · 이관 대상은 모듈 표 안의 id 만.
    #[test]
    fn specs_consistent() {
        let ids: Vec<&str> = SPECS.iter().map(|s| s.id).collect();
        let mut u = ids.clone();
        u.sort();
        u.dedup();
        assert_eq!(u.len(), ids.len(), "모듈 id 중복");
        for (id, _) in LEGACY_CFG { assert!(ids.contains(&id), "이관 대상 {} 가 모듈 표에 없음", id); }
        // 서버 이벤트를 쓰는 3개만 feed 가 있다
        let fed: Vec<&str> = SPECS.iter().filter(|s| s.feed.is_some()).map(|s| s.id).collect();
        assert_eq!(fed, ["training_view_plus", "facility_view_plus", "custom_tier_assignment"]);
    }
    /// 각 모듈 feed 는 자기 이벤트만 보관한다(다른 모듈 이벤트·모르는 이벤트는 버림) — 분배가 서로를 지우지 않는지.
    #[test]
    fn feed_keeps_only_own_events() {
        let ev = |n: &str| StableModEvent { event: n.to_string(), payload: n.as_bytes().to_vec() };
        let all = [ev("tvp_result"), ev("fvp_result"), ev("cta_result"), ev("unknown")];
        for sp in SPECS.iter() { if let Some(f) = sp.feed { f(&all); } }
        // 모듈 내부 큐는 비공개 → 같은 분배를 한 번 더 해도 패닉 없이 누적되는지만 확인(내용 대조는 원본 크레이트 쪽 책임)
        for sp in SPECS.iter() { if let Some(f) = sp.feed { f(&all); } }
    }
    /// 단독판 dll 이 없는 테스트 프로세스에선 전부 "안 로드됨".
    #[test]
    fn standalone_not_loaded_in_test() {
        for sp in SPECS.iter() { assert!(!standalone_loaded(sp.id), "{}", sp.id); }
        assert!(standalone_loaded("kernel32"), "GetModuleHandleW 이름 조회 자체가 동작해야 한다");
    }
}
