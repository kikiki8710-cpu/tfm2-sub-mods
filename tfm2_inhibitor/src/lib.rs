//! tfm2_inhibitor v0.1.0 — 롤식 억제기(게임 0.7.4 · stable 껍데기 + exe RE 네이티브 훅).
//! 라인마다 2차 포탑 뒤에 공격하지 않는 억제기를 세운다. 억제기가 깨지면 그 라인 상대 미니언이 강화되고,
//! 일정 시간 뒤 억제기가 다시 생긴다. 쌍둥이 포탑은 억제기가 하나라도 깨져 있어야, 넥서스는 쌍둥이 둘이 깨지면 공격 가능.
//! 설계·RE 원문 = F:\claude\tfm2\mods_report\tfm2_inhibitor\ (RVA 근거 = RE\2026-11-26_*.md).
//! 안전: 모든 detour 본문 catch_unwind · 메모리는 VEH 보호 읽기/쓰기 · 프롤로그 검증 · 외부 훅 체인(serpen 과 MOBATICK 공유).
#![allow(dead_code)]
use mod_api_stable::{declare_stable_mod, LogLevel, StableClient, StableExtension, StableHost, StableMod};
use std::io::Write as _;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::Mutex;

mod ai;
mod game;
mod hook;
mod inhib;
mod mem;
mod view;
mod perf;
#[path = "../../common/log_dir.rs"]
mod log_dir;

use game::*;

pub const MOD_ID: &str = "tfm2_inhibitor";
pub const VERSION: &str = "0.1.7";
/// 진단 로그(설치 결과·경기별 생성/재생성·주기 카운터). ⚠배포 전 /deploy 디버그 OFF 점검 대상.
/// v0.1.6~(릴리스) = false. 시험할 때만 true 로 다시 빌드 (cfg perf_log 도 이 로그로 나오므로 false 면 출력 없음).
pub const LOG_ENABLED: bool = false;
const LOG_LINE_CAP: u64 = 20000; // v0.1.5: 파괴·재생성 줄이 늘어 2000 → 20000 (~2MB)

#[link(name = "kernel32")]
extern "system" {
    fn GetModuleFileNameW(module: usize, buf: *mut u16, size: u32) -> u32;
    fn GetModuleHandleExW(flags: u32, addr: *const u16, h: *mut usize) -> i32;
    fn GetModuleHandleW(name: *const u16) -> usize;
}
pub fn mod_dir() -> Option<String> {
    let mut h: usize = 0;
    if unsafe { GetModuleHandleExW(0x4 | 0x2, mod_dir as *const () as *const u16, &mut h) } == 0 || h == 0 { return None; }
    let mut buf = [0u16; 1024];
    let n = unsafe { GetModuleFileNameW(h, buf.as_mut_ptr(), buf.len() as u32) } as usize;
    if n == 0 || n >= buf.len() { return None; }
    let p = String::from_utf16_lossy(&buf[..n]);
    p.rfind(|c| c == '\\' || c == '/').map(|i| p[..i].to_string())
}
fn ts() -> String {
    let ms = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0);
    let s = ms / 1000 + 9 * 3600;
    format!("{:02}:{:02}:{:02}.{:03}", (s / 3600) % 24, (s / 60) % 60, s % 60, ms % 1000)
}
static LOG_N: AtomicU64 = AtomicU64::new(0);
static LOG_LOCK: Mutex<()> = Mutex::new(());
pub fn log(msg: &str) {
    if !LOG_ENABLED { return; }
    if LOG_N.fetch_add(1, Ordering::Relaxed) >= LOG_LINE_CAP { return; }
    let _g = LOG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(d) = mod_dir() {
        let dir = log_dir::log_dir(&d);
        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(format!("{}\\inhibitor.log", dir)) {
            let _ = writeln!(f, "[{}] {}", ts(), msg);
        }
    }
}

// ── 설정 파일 ─────────────────────────────────────────────────────────────
const CFG_DEFAULT: &str = "# tfm2_inhibitor settings (key = value). Restart the game after editing.\n\
# tfm2_inhibitor 설정 (키 = 값). 고친 뒤 게임을 다시 켜야 적용된다.\n\
# Delete this file to get the defaults back (the mod writes a new one on the next start).\n\
# 이 파일을 지우면 다음 실행 때 기본값으로 다시 만들어진다.\n\
\n\
# Whole mod on/off. 0 = vanilla matches.\n\
# 모드 전체 켜기/끄기. 0 = 원래 게임과 같음.\n\
enabled = 1\n\
\n\
# ── Inhibitors / 억제기 ──\n\
# Lanes that get an inhibitor: all, or any of top,mid,bot (e.g. lanes = mid  /  lanes = top,bot).\n\
#   A lane without an inhibitor works like vanilla: once its two towers fall, the nexus towers open.\n\
# 억제기를 둘 라인: all 또는 top,mid,bot 중 골라서 (예: lanes = mid  /  lanes = top,bot).\n\
#   억제기가 없는 라인은 원래 게임처럼 1·2차 포탑이 다 깨지면 쌍둥이 포탑이 열린다.\n\
lanes = all\n\
# Respawn time in seconds. 0 = destroyed inhibitors never come back.\n\
# 부활 시간(초). 0 = 깨진 억제기가 다시 생기지 않음.\n\
respawn_sec = 300\n\
# Health as a percent of a nexus tower's health (100 = same, 200 = double).\n\
# 체력: 쌍둥이 포탑 체력 대비 % (100 = 같음, 200 = 2배).\n\
hp_pct = 100\n\
# Position on the lane, in percent: 0 = at the lane's second tower, 100 = at the minion spawn point.\n\
# 위치(%): 0 = 그 라인 2차 포탑 자리, 100 = 미니언 출발점.\n\
position_pct = 50\n\
# 1 = the nexus towers can only be attacked while at least one of their team's inhibitors is down (LoL rule).\n\
#   0 = vanilla rule.\n\
# 1 = 쌍둥이 포탑은 자기 팀 억제기가 하나라도 깨져 있어야 공격할 수 있다(롤 규칙). 0 = 원래 게임 규칙.\n\
lock_twin = 1\n\
# 1 = once both nexus towers are destroyed, the nexus can be attacked even if inhibitors are alive (LoL rule).\n\
#   0 = vanilla rule.\n\
# 1 = 쌍둥이 포탑 둘이 깨지면 억제기가 살아 있어도 넥서스를 공격할 수 있다(롤 규칙). 0 = 원래 게임 규칙.\n\
lol_nexus = 1\n\
\n\
# ── Super minions / 슈퍼미니언 ──\n\
# 1 = while an inhibitor is down, every enemy minion wave in that lane brings 1 extra super minion. 0 = off.\n\
# 1 = 억제기가 깨진 라인은 상대 미니언 웨이브마다 슈퍼미니언이 1마리 더 나온다. 0 = 끔.\n\
super_minion = 1\n\
# Attack / health bonus over a melee minion, in percent (150 = 2.5x, 300 = 4x).\n\
# 근접 미니언보다 공격력·체력을 몇 % 더 (150 = 2.5배, 300 = 4배).\n\
super_attack_pct = 150\n\
super_hp_pct = 300\n\
# Defense / magic resist added on top of a melee minion's (flat values).\n\
# 근접 미니언 방어·마법저항에 더하는 값.\n\
super_defence = 40\n\
super_magic_resist = 40\n\
# Collision size bonus in percent (does not change the sprite size).\n\
# 충돌 크기를 몇 % 더 (그림 크기는 그대로).\n\
super_size_pct = 30\n\
# Minimum seconds between two super minions in the same lane (keeps it at one per wave).\n\
# 같은 라인 슈퍼미니언 사이 최소 간격(초) — 웨이브마다 1마리로 맞추는 용도.\n\
super_gap_sec = 10\n\
\n\
# ── Diagnostics (leave as is) / 진단 (그대로 둘 것) ──\n\
# fast_path = 1 uses the optimized code path (cached positions, updates only when something changes).\n\
#   0 = the older, slower path with the same result.\n\
# fast_path = 1 이면 최적화 경로(좌표 캐시, 바뀔 때만 갱신). 0 = 예전 경로(더 느리고 결과는 같음).\n\
fast_path = 1\n\
# perf_log = 1 writes timing to the mod log. Release builds have the log turned off, so it does nothing.\n\
# perf_log = 1 이면 시간 측정을 로그에 쓴다. 배포판은 로그가 꺼져 있어 아무것도 안 나온다.\n\
perf_log = 0\n";

/// `lanes = all | top,mid,bot` → 비트(1 탑·2 미드·4 봇). 못 읽으면 None(기본값 유지).
fn parse_lanes(v: &str) -> Option<u8> {
    let mut m = 0u8;
    for tok in v.split(|ch: char| ch == ',' || ch == ' ' || ch == '/' || ch == '+').map(|s| s.trim().to_ascii_lowercase()).filter(|s| !s.is_empty()) {
        m |= match tok.as_str() {
            "all" | "전부" | "모두" | "3" => 7,
            "top" | "탑" => 1,
            "mid" | "미드" => 2,
            "bot" | "bottom" | "봇" | "바텀" => 4,
            "none" | "0" => 0,
            _ => return None,
        };
    }
    Some(m)
}

fn load_cfg() -> inhib::Cfg {
    let mut c = inhib::Cfg::default();
    let Some(d) = mod_dir() else { return c };
    let dir = format!("{}\\config", d);
    let p = format!("{}\\inhibitor.cfg", dir);
    let text = match std::fs::read_to_string(&p) {
        Ok(t) => t,
        Err(_) => { let _ = std::fs::create_dir_all(&dir); let _ = std::fs::write(&p, CFG_DEFAULT); CFG_DEFAULT.to_string() }
    };
    // 옛 판(v0.1.3 이전)이 만든 cfg 에는 lanes 줄이 없다 — 보이게 덧붙인다(값은 기본 all · 다른 줄은 건드리지 않음)
    if !text.lines().any(|l| l.trim_start().starts_with("lanes")) {
        if let Ok(mut f) = std::fs::OpenOptions::new().append(true).open(&p) {
            let _ = f.write_all("\n# Lanes that get an inhibitor: all, or any of top,mid,bot (e.g. lanes = mid).\n# 억제기를 둘 라인: all 또는 top,mid,bot 중 골라서 (예: lanes = mid).\nlanes = all\n".as_bytes());
        }
    }
    for line in text.trim_start_matches('\u{feff}').lines() {
        let line = line.split('#').next().unwrap_or("").trim();
        let Some((k, v)) = line.split_once('=') else { continue };
        let (k, v) = (k.trim(), v.trim());
        if k == "lanes" {
            match parse_lanes(v) { Some(m) => c.lanes = m, None => log(&format!("cfg: lanes = '{}' 를 못 읽음 — 기본(all) 유지", v)) }
            continue;
        }
        let n: i64 = match v.parse() { Ok(n) => n, Err(_) => continue };
        match k {
            "enabled" => c.enabled = n != 0,
            "respawn_sec" => c.respawn_ticks = (n.max(0) as u64) * 60,
            "position_pct" => c.pos_pct = n.clamp(0, 100) as u64,
            "hp_pct" => c.hp_pct = n.clamp(1, 1000) as u64,
            "lock_twin" => c.lock_twin = n != 0,
            "lol_nexus" => c.lol_nexus = n != 0,
            "super_minion" => c.super_minion = n != 0,
            "super_attack_pct" => c.super_attack_mult = n.clamp(-90, 1000) as i32,
            "super_hp_pct" => c.super_hp_mult = n.clamp(-90, 1000) as i32,
            "super_defence" => c.super_defence = n.clamp(-1000, 10000) as i32,
            "super_magic_resist" => c.super_mr = n.clamp(-1000, 10000) as i32,
            "super_size_pct" => c.super_radius_mult = n.clamp(-50, 200) as i32,
            "super_gap_sec" => c.super_gap_ticks = (n.clamp(1, 600) as u64) * 60,
            "fast_path" => c.fast_path = n != 0,
            "perf_log" => c.perf_log = n != 0,
            _ => {}
        }
    }
    c
}

// ── detour ───────────────────────────────────────────────────────────────
type FnMobaTick = extern "win64" fn(usize, usize, usize, usize);
type FnTowerUpd = extern "win64" fn(usize, usize, usize, usize, u64, usize);
type FnSpawnMinion = extern "win64" fn(usize, usize, u64, u64, u64, u64, u64, u64) -> usize;
static NEXT_MOBATICK: AtomicUsize = AtomicUsize::new(0);
static NEXT_TOWER: AtomicUsize = AtomicUsize::new(0);
static NEXT_SPAWN: AtomicUsize = AtomicUsize::new(0);
static PANICS: AtomicU64 = AtomicU64::new(0);

extern "win64" fn mobatick_detour(w: usize, ctx: usize, rng: usize, ev: usize) {
    let next = NEXT_MOBATICK.load(Ordering::Acquire);
    if next == 0 { return; }
    let pf = perf::on();
    let t0 = if pf { perf::now() } else { 0 };
    // 생성·재생성은 원본 전(같은 틱 EntityInfo 스캔에 잡히게)
    if catch_unwind(AssertUnwindSafe(|| unsafe { inhib::pre_tick(w, ctx) })).is_err() { PANICS.fetch_add(1, Ordering::Relaxed); }
    let t1 = if pf { perf::now() } else { 0 };
    inhib::CUR_W.with(|v| v.set(w));
    unsafe { core::mem::transmute::<usize, FnMobaTick>(next)(w, ctx, rng, ev) };
    inhib::CUR_W.with(|v| v.set(0));
    let t2 = if pf { perf::now() } else { 0 };
    if catch_unwind(AssertUnwindSafe(|| unsafe { inhib::post_tick(w, ctx) })).is_err() { PANICS.fetch_add(1, Ordering::Relaxed); }
    if pf {
        let t3 = perf::now();
        perf::T_PRE.fetch_add(t1.wrapping_sub(t0), Ordering::Relaxed);
        perf::T_ORIG.fetch_add(t2.wrapping_sub(t1), Ordering::Relaxed);
        perf::T_POST.fetch_add(t3.wrapping_sub(t2), Ordering::Relaxed);
        perf::N_MOBA.fetch_add(1, Ordering::Relaxed);
    }
}
extern "win64" fn tower_detour(w: usize, vt: usize, rng: usize, ctx: usize, id: u64, ev: usize) {
    let pf = perf::on();
    let t0 = if pf { perf::now() } else { 0 };
    let skip = catch_unwind(AssertUnwindSafe(|| unsafe { inhib::tower_should_skip(w, vt, ctx, id) })).unwrap_or(false);
    if pf { perf::T_TOWER.fetch_add(perf::now().wrapping_sub(t0), Ordering::Relaxed); perf::N_TOWER.fetch_add(1, Ordering::Relaxed); }
    if skip { return; }
    let next = NEXT_TOWER.load(Ordering::Acquire);
    if next == 0 { return; }
    unsafe { core::mem::transmute::<usize, FnTowerUpd>(next)(w, vt, rng, ctx, id, ev) }
}
extern "win64" fn spawn_detour(out: usize, setting: usize, tut: u64, tick: u64, team: u64, line: u64, buffed: u64, melee: u64) -> usize {
    let next = NEXT_SPAWN.load(Ordering::Acquire);
    if next == 0 { return 0; }
    let r = unsafe { core::mem::transmute::<usize, FnSpawnMinion>(next)(out, setting, tut, tick, team, line, buffed, melee) };
    if catch_unwind(AssertUnwindSafe(|| unsafe { inhib::after_spawn_minion(out, tick) })).is_err() { PANICS.fetch_add(1, Ordering::Relaxed); }
    r
}

fn prologue16(tail: &[u8; 4]) -> [u8; 16] {
    let mut p = [0u8; 16];
    p[..12].copy_from_slice(&PRO_PUSH8);
    p[12..].copy_from_slice(tail);
    p
}
/// 진입부 16B 를 검증하고 앞 12B(push×8)만 옮긴다.
unsafe fn install_one(name: &str, rva: usize, tail: &[u8; 4], detour: usize, next: &AtomicUsize) -> bool {
    let full = prologue16(tail);
    // 깨끗하면 16B 전부 일치해야(버전 검증) — 옮기는 건 12B
    let st = hook::entry_state(rva, &full);
    if let hook::Entry::Mismatch(cur) = st {
        log(&format!("HOOK {} SKIP: prologue mismatch rva={:#x} found={:02x?}", name, rva, cur));
        return false;
    }
    match hook::install(rva, &PRO_PUSH8, detour) {
        Ok((n, mode)) => { next.store(n, Ordering::Release); log(&format!("HOOK {} OK rva={:#x} mode={}", name, rva, mode)); true }
        Err(e) => { log(&format!("HOOK {} FAIL: {}", name, e)); false }
    }
}

// MOBATICK 은 serpen 이 먼저 직접(체인 비인식) 설치하므로 늦게·외부훅 대기 뒤 1회 설치.
static MOBA_INSTALLED: AtomicBool = AtomicBool::new(false);
static MOBA_WAIT: AtomicU64 = AtomicU64::new(0);
const MOBA_WAIT_LIMIT: u64 = 600;
fn mobatick_install_tick() {
    if MOBA_INSTALLED.load(Ordering::Relaxed) { return; }
    let w = MOBA_WAIT.fetch_add(1, Ordering::Relaxed);
    let full = prologue16(&PRO_MOBATICK_TAIL);
    let st = unsafe { hook::entry_state(RVA_MOBATICK, &full) };
    let serpen_loaded = serpen_loaded();
    let go = match st {
        hook::Entry::Foreign(_) => true,
        hook::Entry::Clean => !serpen_loaded || w >= MOBA_WAIT_LIMIT,
        hook::Entry::Mismatch(_) => true, // install_one 이 로그 남기고 포기
    };
    if !go { return; }
    MOBA_INSTALLED.store(true, Ordering::Relaxed);
    let ok = unsafe { install_one("MOBATICK", RVA_MOBATICK, &PRO_MOBATICK_TAIL, mobatick_detour as *const () as usize, &NEXT_MOBATICK) };
    log(&format!("MOBATICK install after {} frames (serpen dll {}) = {}", w, if serpen_loaded { "loaded" } else { "absent" }, ok));
}

fn serpen_loaded() -> bool {
    let n: Vec<u16> = "tfm2_elemental_serpen.dll\0".encode_utf16().collect();
    unsafe { GetModuleHandleW(n.as_ptr()) != 0 }
}

static SETUP: AtomicBool = AtomicBool::new(false);
fn setup() {
    if SETUP.swap(true, Ordering::Relaxed) { return; }
    mem::seh_install();
    let c = load_cfg();
    let _ = inhib::CFG.set(c);
    if c.perf_log { perf::start(); }
    log(&format!("init v{} cfg={:?}", VERSION, c));
    let base = hook::exe_base();
    inhib::FN_CLONE.store(base + RVA_ENTITY_CLONE, Ordering::Relaxed);
    inhib::FN_ADD_ENTITY.store(base + RVA_ADD_ENTITY, Ordering::Relaxed);
    inhib::FN_ADD_BUFF.store(base + RVA_ADD_BUFF, Ordering::Relaxed);
    inhib::FN_RECALC.store(base + RVA_RECALC, Ordering::Relaxed);
    inhib::VT_MOBA_A.store(base + RVA_VT_MOBA_A, Ordering::Relaxed);
    if !c.enabled { log("disabled by cfg"); MOBA_INSTALLED.store(true, Ordering::Relaxed); return; }
    unsafe {
        install_one("TOWER_UPDATE", RVA_TOWER_UPDATE, &PRO_TOWER_TAIL, tower_detour as *const () as usize, &NEXT_TOWER);
        install_one("SPAWN_MINION", RVA_SPAWN_MINION, &PRO_SPAWN_TAIL, spawn_detour as *const () as usize, &NEXT_SPAWN);
        ai::install(c);
    }
    if let Some(d) = mod_dir() { view::check_assets(&d); }
}

struct Ext;
static FRAME: AtomicU64 = AtomicU64::new(0);
impl StableExtension for Ext {
    fn on_init(&self, _ctx: &mut StableClient<'_>) { let _ = catch_unwind(setup); }
    fn post_update(&self, _ctx: &mut StableClient<'_>, _dt: u64) {
        let _ = catch_unwind(AssertUnwindSafe(|| {
            setup();
            let f = FRAME.fetch_add(1, Ordering::Relaxed) + 1;
            if f >= 5 { mobatick_install_tick(); view::install_tick(serpen_loaded()); }
            if LOG_ENABLED && f % 1800 == 0 {
                use inhib::*;
                let r = Ordering::Relaxed;
                log(&format!("stats worlds={} adopt={} created={} respawn={} super={} skip_atk={} lockw={} ticks={} err={} panics={} seh={} | {}",
                    N_WORLDS.load(r), N_ADOPT.load(r), N_CREATED.load(r), N_RESPAWN.load(r), N_SUPER.load(r), N_SKIP_ATK.load(r),
                    N_LOCKW.load(r), N_TICKS.load(r), N_ERR.load(r), PANICS.load(r), mem::SEH_FAULTS.load(r), ai::stats()));
                log(&format!("view swap inhib={} super={} | fast={} post run={} skip={} list_fallback={}",
                    view::N_SWAP_INHIB.load(r), view::N_SWAP_SUPER.load(r), cfg_fast(), N_POST_RUN.load(r), N_POST_SKIP.load(r), N_LIST_FALLBACK.load(r)));
                log(&perf::report());
            }
        }));
    }
}

fn init(host: &StableHost) -> StableMod {
    host.log(LogLevel::Info, &format!("{} v{}", MOD_ID, VERSION));
    let _ = catch_unwind(setup);
    let mut d = StableMod::new(MOD_ID);
    d.set_extension(Ext);
    d
}
declare_stable_mod!(init);
fn cfg_fast() -> bool { inhib::cfg().fast_path }

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn lanes_parse() {
        assert_eq!(parse_lanes("all"), Some(7));
        assert_eq!(parse_lanes("mid"), Some(2));
        assert_eq!(parse_lanes("top,bot"), Some(5));
        assert_eq!(parse_lanes("Top, Mid"), Some(3));
        assert_eq!(parse_lanes("미드"), Some(2));
        assert_eq!(parse_lanes("none"), Some(0));
        assert_eq!(parse_lanes("middle"), None);
    }
    #[test]
    fn default_cfg_text_parses_to_defaults() {
        // CFG_DEFAULT 의 모든 키=값이 기본 Cfg 와 같은 값으로 읽히는지(문서와 코드 기본값 어긋남 방지)
        let mut c = inhib::Cfg { lanes: 0, respawn_ticks: 1, hp_pct: 1, super_attack_mult: 0, super_hp_mult: 0, ..inhib::Cfg::default() };
        for line in CFG_DEFAULT.lines() {
            let line = line.split('#').next().unwrap_or("").trim();
            let Some((k, v)) = line.split_once('=') else { continue };
            let (k, v) = (k.trim(), v.trim());
            match k {
                "lanes" => c.lanes = parse_lanes(v).unwrap(),
                "respawn_sec" => c.respawn_ticks = v.parse::<u64>().unwrap() * 60,
                "hp_pct" => c.hp_pct = v.parse().unwrap(),
                "super_attack_pct" => c.super_attack_mult = v.parse().unwrap(),
                "super_hp_pct" => c.super_hp_mult = v.parse().unwrap(),
                _ => {}
            }
        }
        let d = inhib::Cfg::default();
        assert_eq!((c.lanes, c.respawn_ticks, c.hp_pct, c.super_attack_mult, c.super_hp_mult),
                   (d.lanes, d.respawn_ticks, d.hp_pct, d.super_attack_mult, d.super_hp_mult));
    }
}
