//! 억제기 핵심 로직 — 생성·잠금·공격 차단·파괴 감지·재생성·슈퍼미니언.
//! 설계 = mods_report\tfm2_inhibitor\01_구조.md / 02_구현정보.md (0.7.4 · 롤 규칙).
//!   억제기 = 쌍둥이 타입(ty 3) Tower 엔티티, 라인×팀 6개. 좌표 = 그 라인 2차 포탑 ↔ 미니언 스폰점 사이(pos_pct %).
//!   식별은 상태가 아니라 엔티티 특징(kind 4 ∧ ty 3/4 ∧ 억제기 좌표)으로 한다 — World 복제·주소 재사용에도 안전.
//! 규칙(매 틱 MOBATICK 뒤 재기록 · 게임은 can_target 에 1 만 쓴다):
//!   억제기 공격 가능 ⇔ 그 라인 1·2차 포탑 전멸
//!   쌍둥이 공격 가능 ⇔ 자기 팀 억제기 하나 이상 파괴 상태
//!   넥서스 공격 가능 ⇔ 진짜 쌍둥이 2개 전멸 (게임 원래 조건은 ty 3/4 전멸이라 억제기가 막으므로 모드가 1 을 씀)
//! 결정성: 판단은 World 내용과 틱만 쓴다(난수·시각·스레드 무관).
#![allow(dead_code)]
use crate::game::*;
use crate::mem::*;
use std::cell::Cell;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Mutex;

// ── 설정 ─────────────────────────────────────────────────────────────────
#[derive(Clone, Copy, Debug)]
pub struct Cfg {
    pub enabled: bool,
    pub respawn_ticks: u64, // 0 = 재생성 안 함
    pub pos_pct: u64,       // 2차 포탑(0) → 스폰점(100) 사이 위치
    pub hp_pct: u64,        // 쌍둥이 포탑 체력 대비 %
    pub lock_twin: bool,    // 쌍둥이를 억제기 파괴 전까지 잠글지(롤 규칙)
    pub lol_nexus: bool,    // 진짜 쌍둥이만 깨지면 넥서스 열기
    pub super_minion: bool,
    pub super_attack_mult: i32, // %
    pub super_hp_mult: i32,     // %
    pub super_defence: i32,
    pub super_mr: i32,
    pub super_radius_mult: i32, // %
    pub super_gap_ticks: u64,   // 같은 라인·팀 슈퍼미니언 최소 간격(= 웨이브 1마리 판정)
    pub lanes: u8,              // 억제기를 둘 라인 비트(1 탑·2 미드·4 봇) — 기본 7 = 세 라인
    pub fast_path: bool,        // v0.1.3 최적화 경로(좌표 캐시·포탑 목록 스캔·변경 시에만 잠금). 0 = v0.1.2 경로(전후 비교용)
    pub perf_log: bool,         // 처리 시간 계측(rdtsc) — 진단용
}
impl Default for Cfg {
    fn default() -> Self {
        Cfg { enabled: true, respawn_ticks: 300 * 60, pos_pct: 50, hp_pct: 100, lock_twin: true, lol_nexus: true,
              super_minion: true, super_attack_mult: 150, super_hp_mult: 300, super_defence: 40, super_mr: 40, super_radius_mult: 30,
              super_gap_ticks: 10 * 60, lanes: 7, fast_path: true, perf_log: false }
    }
}
/// 설정은 init 에서 한 번 정한다(매 틱·포탑마다 읽으므로 잠금 없이).
pub static CFG: std::sync::OnceLock<Cfg> = std::sync::OnceLock::new();
pub fn cfg() -> Cfg { CFG.get().copied().unwrap_or_default() }

// ── 진단 카운터 ───────────────────────────────────────────────────────────
pub static N_WORLDS: AtomicU64 = AtomicU64::new(0);
pub static N_CREATED: AtomicU64 = AtomicU64::new(0);
pub static N_RESPAWN: AtomicU64 = AtomicU64::new(0);
pub static N_SUPER: AtomicU64 = AtomicU64::new(0);
pub static N_SKIP_ATK: AtomicU64 = AtomicU64::new(0);
pub static N_LOCKW: AtomicU64 = AtomicU64::new(0);
pub static N_ERR: AtomicU64 = AtomicU64::new(0);
pub static N_ADOPT: AtomicU64 = AtomicU64::new(0);
pub static N_TICKS: AtomicU64 = AtomicU64::new(0);

// ── 게임 함수 주소(설치 시 채움) ───────────────────────────────────────────
pub static FN_CLONE: AtomicUsize = AtomicUsize::new(0);
pub static FN_ADD_ENTITY: AtomicUsize = AtomicUsize::new(0);
pub static FN_ADD_BUFF: AtomicUsize = AtomicUsize::new(0);
pub static FN_RECALC: AtomicUsize = AtomicUsize::new(0);
pub static VT_MOBA_A: AtomicUsize = AtomicUsize::new(0);

// ── MOBATICK → spawn_minion 전달(같은 스레드 · const 초기화 · Drop 없음) ──
thread_local! {
    pub static CUR_W: Cell<usize> = const { Cell::new(0) };
}

// ── World 별 상태(작게 · 짧게 잠금) ────────────────────────────────────────
#[derive(Clone, Copy, Default)]
struct WState {
    last_tick: u64,
    first_tick: u64,
    managed: bool,          // 억제기를 만들었거나(신규) 이미 있는 것을 찾았음(복제 World)
    decided: bool,          // managed 판정 끝
    template: [usize; 2],   // 팀별 억제기 템플릿 엔티티(0x6B8, 누수 보관 — Arc refcount 만 남음)
    dead_since: [[u64; 3]; 2], // u64::MAX = 살아 있음
    inhib_dead: [[bool; 3]; 2],
    super_tick: [[u64; 3]; 2],   // 마지막 슈퍼미니언 예약 틱(+1 저장 — 0 = 없음)
    pending: [[usize; 3]; 2],    // 다음 틱 pre 에 넣을 슈퍼미니언 버퍼(우리 Box · 0 = 없음)
    last_lens: (u64, u64),       // fast: 지난 post 의 (타워 목록 len, 넥서스 목록 len)
    dirty: u8,                   // fast: 남은 강제 재계산 틱 수(len 변화 → 이번+다음 틱 — 게임 언락은 다음 틱에 1 을 쓴다)
}
static WORLDS: Mutex<Option<HashMap<usize, WState>>> = Mutex::new(None);
fn with_state<R>(w: usize, f: impl FnOnce(&mut WState) -> R) -> R {
    let mut g = WORLDS.lock().unwrap_or_else(|e| e.into_inner());
    let m = g.get_or_insert_with(HashMap::new);
    if m.len() > 4096 { m.clear(); } // 끝난 World 정리(주소만 키라 무한 누적 방지)
    f(m.entry(w).or_default())
}

// ── 좌표 ─────────────────────────────────────────────────────────────────
pub unsafe fn inhib_pos(ctx: usize, team: usize, lane: usize, pct: u64) -> Option<(u64, u64)> {
    let (x2, y2) = mapdef_tower_pos(ctx, 5 + lane as u8, team)?;
    let (sx, sy) = MINION_SPAWN[team][lane];
    let lerp = |a: u64, b: u64| -> u64 { (a as i128 + (b as i128 - a as i128) * pct as i128 / 100) as u64 };
    Some((lerp(x2, sx), lerp(y2, sy)))
}
pub unsafe fn inhib_positions(ctx: usize, pct: u64) -> Option<[[(u64, u64); 3]; 2]> {
    let mut r = [[(0u64, 0u64); 3]; 2];
    for t in 0..2 { for l in 0..3 { r[t][l] = inhib_pos(ctx, t, l, pct)?; } }
    Some(r)
}
const POS_TOL: u64 = 1000;
fn near(a: (u64, u64), b: (u64, u64)) -> bool { a.0.abs_diff(b.0) <= POS_TOL && a.1.abs_diff(b.1) <= POS_TOL }

/// 억제기 좌표 전역 캐시 — 타워 update·AI 빌더(ctx 없음/매 호출)에서 쓴다. MOBATICK 이 매 틱 갱신.
static POS_CACHE: [AtomicU64; 12] = [const { AtomicU64::new(0) }; 12];
static POS_VALID: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
fn cache_store(p: &[[(u64, u64); 3]; 2]) {
    if POS_VALID.load(Ordering::Relaxed) && cache_load().map(|c| c == *p).unwrap_or(false) { return; }
    for t in 0..2 { for l in 0..3 {
        POS_CACHE[(t * 3 + l) * 2].store(p[t][l].0, Ordering::Relaxed);
        POS_CACHE[(t * 3 + l) * 2 + 1].store(p[t][l].1, Ordering::Relaxed);
    } }
    POS_VALID.store(true, Ordering::Release);
}
fn cache_load() -> Option<[[(u64, u64); 3]; 2]> {
    if !POS_VALID.load(Ordering::Acquire) { return None; }
    let mut r = [[(0u64, 0u64); 3]; 2];
    for t in 0..2 { for l in 0..3 {
        r[t][l] = (POS_CACHE[(t * 3 + l) * 2].load(Ordering::Relaxed), POS_CACHE[(t * 3 + l) * 2 + 1].load(Ordering::Relaxed));
    } }
    Some(r)
}
/// 좌표가 그 팀 억제기 자리인가(전역 캐시 · 뷰 스레드에서 읽기 전용)
pub fn is_inhib_pos(team: usize, p: (u64, u64)) -> bool {
    if team > 1 { return false; }
    match cache_load() { Some(c) => (0..3).any(|l| near(p, c[team][l])), None => false }
}
/// 엔티티가 억제기면 Some((team, lane)) — kind 4 ∧ ty 3/4 ∧ 억제기 좌표(전역 캐시).
pub unsafe fn inhib_of(e: usize) -> Option<(usize, usize)> {
    if rd_u32(e + E_KIND)? != KIND_TOWER { return None; }
    let ty = rd_u8(e + E_TW_TY)?;
    if ty != TY_TWIN_A && ty != TY_TWIN_B { return None; }
    let team = rd_u64(e + E_TEAM)? as usize;
    if team > 1 { return None; }
    let p = (rd_u64(e + E_POS_X)?, rd_u64(e + E_POS_Y)?);
    let c = cache_load()?;
    (0..3).find(|&l| near(p, c[team][l])).map(|l| (team, l))
}

/// 모드가 다룰 경기인가(5v5 풀맵 · 미니언 있음)
pub unsafe fn ctx_ok(ctx: usize) -> bool {
    if !ptr_ok(ctx) { return false; }
    let mode = match rd_u8(ctx + C_MODE) { Some(v) => v, None => return false };
    if mode != 0 && mode != 8 { return false; }
    matches!(rd_u8(ctx + C_NO_MINION), Some(0))
}

// ── 한 틱 스캔 결과 ───────────────────────────────────────────────────────
#[derive(Default)]
struct Scan {
    outer: [[bool; 3]; 2],
    inner: [[bool; 3]; 2],
    twins: [Vec<usize>; 2],      // 진짜 쌍둥이 엔티티 ptr
    inhib: [[usize; 3]; 2],      // 억제기 엔티티 ptr (0 = 없음)
    nexus: [usize; 2],
}
unsafe fn scan(w: usize, pos: &[[(u64, u64); 3]; 2]) -> Option<Scan> {
    let (dp, dn) = dense(w)?;
    let mut s = Scan::default();
    for i in 0..dn {
        let e = dp + i * ENT_STRIDE;
        let kind = rd_u32(e + E_KIND)?;
        if kind != KIND_TOWER && kind != KIND_NEXUS { continue; }
        if rd_u8(e + E_FACTION)? != 0 { continue; }
        let team = rd_u64(e + E_TEAM)? as usize;
        if team > 1 { continue; }
        if kind == KIND_NEXUS { s.nexus[team] = e; continue; }
        let ty = rd_u8(e + E_TW_TY)?;
        match ty {
            0..=2 => s.outer[team][ty as usize] = true,
            5..=7 => s.inner[team][ty as usize - 5] = true,
            3 | 4 => {
                let p = (rd_u64(e + E_POS_X)?, rd_u64(e + E_POS_Y)?);
                match (0..3).find(|&l| near(p, pos[team][l])) {
                    Some(l) => s.inhib[team][l] = e,
                    None => s.twins[team].push(e),
                }
            }
            _ => {}
        }
    }
    Some(s)
}

/// 쌍둥이(또는 기존 억제기)로 팀 템플릿을 만든다. 반환 = 누수 보관 버퍼 주소.
unsafe fn make_template(src: usize, from_twin: bool, hp_pct: u64) -> Option<usize> {
    let f_clone = FN_CLONE.load(Ordering::Relaxed);
    let f_recalc = FN_RECALC.load(Ordering::Relaxed);
    if f_clone == 0 || f_recalc == 0 { return None; }
    let buf: Box<[u64; ENT_STRIDE / 8]> = Box::new([0u64; ENT_STRIDE / 8]);
    let b = Box::into_raw(buf) as usize; // 의도적 누수(엔티티 drop 을 돌리지 않는다)
    core::mem::transmute::<usize, FnClone>(f_clone)(b as *mut u8, src as *const u8);
    if from_twin {
        let ok = wr_u8(b + E_TW_TY, TY_TWIN_A)
            && wr_u64(b + E_TW_STATE, 0) && wr_u64(b + E_TW_NEAREST, 0) && wr_u64(b + E_TW_CD, 0)
            && wr_u64(b + E_TW_LASTHIT, u64::MAX) && wr_u64(b + E_LAST_ATK_TAG, 0)
            && wr_u64(b + E_DMG_WIN, 0) && wr_u64(b + E_DMG_WIN + 8, 0) && wr_u64(b + E_DMG_WIN + 0x10, 0) && wr_u64(b + E_DMG_WIN + 0x18, 0)
            && wr_bytes(b + E_ATK_TYPE, &[0u8; 4]);
        if !ok { return None; }
        if hp_pct != 100 && hp_pct > 0 {
            let base = rd_u64(b + E_BASE_HP)?;
            let st = rd_u64(b + E_TW_STAT_HP)?;
            wr_u64(b + E_BASE_HP, base.saturating_mul(hp_pct) / 100);
            wr_u64(b + E_TW_STAT_HP, st.saturating_mul(hp_pct) / 100);
        }
    }
    wr_u8(b + E_CAN_TARGET, 0);
    wr_u8(b + E_DIRTY, 1);
    core::mem::transmute::<usize, FnRecalc>(f_recalc)(b);
    let mx = rd_u64(b + E_MAX_HP)?;
    wr_u64(b + E_HP, mx);
    Some(b)
}

/// 템플릿을 복제해 좌표를 주고 World 에 넣는다(add_entity = 생성 틱·효과 캐시·삽입). 반환 = 새 id.
unsafe fn spawn_from(w: usize, tmpl: usize, pos: (u64, u64)) -> Option<u64> {
    let f_clone = FN_CLONE.load(Ordering::Relaxed);
    let f_add = FN_ADD_ENTITY.load(Ordering::Relaxed);
    if f_clone == 0 || f_add == 0 || !ptr_ok(tmpl) { return None; }
    let mut buf: Box<[u64; ENT_STRIDE / 8]> = Box::new([0u64; ENT_STRIDE / 8]);
    let b = buf.as_mut_ptr() as usize;
    core::mem::transmute::<usize, FnClone>(f_clone)(b as *mut u8, tmpl as *const u8);
    wr_u64(b + E_POS_X, pos.0);
    wr_u64(b + E_POS_Y, pos.1);
    wr_u8(b + E_CAN_TARGET, 0);
    let mx = rd_u64(b + E_MAX_HP)?;
    wr_u64(b + E_HP, mx);
    let id = core::mem::transmute::<usize, FnAddEntity>(f_add)(w, b as *mut u8);
    // add_entity 가 move 했다 — buf 는 바이트만 해제([u64] 는 drop 동작 없음)
    drop(buf);
    Some(id)
}

/// 공통 가드 + 좌표. 반환 = (tick, 좌표)
unsafe fn guard(w: usize, ctx: usize, c: &Cfg) -> Option<(u64, [[(u64, u64); 3]; 2])> {
    if !c.enabled || !ptr_ok(w) || !ctx_ok(ctx) { return None; }
    if rd_u8(w + W_INIT_DONE)? != 1 { return None; }
    let tick = rd_u64(w + W_TICK)?;
    let pos = if c.fast_path {
        // 좌표는 MapDef 가 같으면 같다 — 스레드별로 (MapDef ptr → 좌표) 1칸 캐시
        let m = rd_u64(ctx + C_MAPDEF)? as usize;
        let hit = POS_TL.with(|v| { let (k, p) = v.get(); if k == m && m != 0 { Some(p) } else { None } });
        match hit {
            Some(p) => p,
            None => { let p = inhib_positions(ctx, c.pos_pct)?; POS_TL.with(|v| v.set((m, p))); cache_store(&p); p }
        }
    } else {
        let p = inhib_positions(ctx, c.pos_pct)?;
        cache_store(&p);
        p
    };
    Some((tick, pos))
}
thread_local! {
    static POS_TL: Cell<(usize, [[(u64, u64); 3]; 2])> = const { Cell::new((0, [[(0, 0); 3]; 2])) };
}

/// v0.1.3 진입점 — cfg.fast_path 로 최적화 경로 / v0.1.2 경로를 고른다(같은 dll 로 전후 비교).
pub unsafe fn pre_tick(w: usize, ctx: usize) {
    if cfg().fast_path { pre_tick_fast(w, ctx) } else { pre_tick_slow(w, ctx) }
}
pub unsafe fn post_tick(w: usize, ctx: usize) {
    if cfg().fast_path { post_tick_fast(w, ctx) } else { post_tick_slow(w, ctx) }
}

/// 포탑·넥서스 id 목록만 훑는 스캔(엔티티 전체 대신 ~24개). 목록이 이상하면 None → 호출자가 전체 스캔으로 폴백.
unsafe fn scan_lists(w: usize, pos: &[[(u64, u64); 3]; 2]) -> Option<Scan> {
    let mut s = Scan::default();
    for (pa, la, want) in [(W_TOWERS_PTR, W_TOWERS_LEN, KIND_TOWER), (W_NEXUS_PTR, W_NEXUS_LEN, KIND_NEXUS)] {
        let n = rd_u64(w + la)? as usize;
        if n > 256 { return None; }
        if n == 0 { continue; }
        let p = rd_u64(w + pa)? as usize;
        if !ptr_ok(p) { return None; }
        for i in 0..n {
            let id = rd_u64(p + 8 * i)?;
            let e = entity_by_id(w, id)?;
            if rd_u32(e + E_KIND)? != want { return None; } // 목록 해석이 틀렸다 = 폴백
            if rd_u8(e + E_FACTION)? != 0 { continue; }
            let team = rd_u64(e + E_TEAM)? as usize;
            if team > 1 { continue; }
            if want == KIND_NEXUS { s.nexus[team] = e; continue; }
            match rd_u8(e + E_TW_TY)? {
                ty @ 0..=2 => s.outer[team][ty as usize] = true,
                ty @ 5..=7 => s.inner[team][ty as usize - 5] = true,
                3 | 4 => {
                    let q = (rd_u64(e + E_POS_X)?, rd_u64(e + E_POS_Y)?);
                    match (0..3).find(|&l| near(q, pos[team][l])) { Some(l) => s.inhib[team][l] = e, None => s.twins[team].push(e) }
                }
                _ => {}
            }
        }
    }
    Some(s)
}
pub static N_LIST_FALLBACK: AtomicU64 = AtomicU64::new(0);
pub static N_POST_SKIP: AtomicU64 = AtomicU64::new(0);
pub static N_POST_RUN: AtomicU64 = AtomicU64::new(0);
unsafe fn scan_fast(w: usize, pos: &[[(u64, u64); 3]; 2]) -> Option<Scan> {
    match scan_lists(w, pos) {
        Some(s) => Some(s),
        None => { N_LIST_FALLBACK.fetch_add(1, Ordering::Relaxed); scan(w, pos) }
    }
}
unsafe fn list_lens(w: usize) -> Option<(u64, u64)> { Some((rd_u64(w + W_TOWERS_LEN)?, rd_u64(w + W_NEXUS_LEN)?)) }

/// fast pre: 결정(첫 틱)·재생성 시각 도래·슈퍼미니언 예약이 있을 때만 일을 한다. 파괴 감지는 post 가 한다.
unsafe fn pre_tick_fast(w: usize, ctx: usize) {
    let c = cfg();
    let Some((tick, pos)) = guard(w, ctx, &c) else { return };
    N_TICKS.fetch_add(1, Ordering::Relaxed);
    let mut st = with_state(w, |s| {
        if !s.decided && s.last_tick == 0 && s.first_tick == 0 {
            s.first_tick = tick; s.last_tick = tick; N_WORLDS.fetch_add(1, Ordering::Relaxed);
        } else if tick < s.last_tick {
            *s = WState { first_tick: tick, last_tick: tick, ..Default::default() };
            N_WORLDS.fetch_add(1, Ordering::Relaxed);
        }
        s.last_tick = tick;
        *s
    });
    if !st.decided {
        // 결정 로직은 v0.1.2 와 같다(첫 틱 1회) — 그대로 위임
        return pre_tick_slow(w, ctx);
    }
    if !st.managed { return; }
    // 재생성 시각이 된 라인이 있을 때만 스캔
    let due = c.respawn_ticks != 0 && (0..2).any(|t| (0..3).any(|l| {
        let d = st.dead_since[t][l];
        d != u64::MAX && tick.saturating_sub(d) >= c.respawn_ticks
    }));
    if due {
        if let Some(sc) = scan_fast(w, &pos) {
            for t in 0..2 { for l in 0..3 {
                let d = st.dead_since[t][l];
                if d == u64::MAX || tick.saturating_sub(d) < c.respawn_ticks { continue; }
                if sc.inhib[t][l] != 0 { st.dead_since[t][l] = u64::MAX; continue; } // 이미 있음(복제 World 등)
                if st.template[t] == 0 {
                    let src = (0..3).map(|k| sc.inhib[t][k]).find(|&e| e != 0);
                    st.template[t] = match src {
                        Some(e) => make_template(e, false, 100).unwrap_or(0),
                        None => sc.twins[t].first().and_then(|&e| make_template(e, true, c.hp_pct)).unwrap_or(0),
                    };
                }
                if st.template[t] == 0 { continue; }
                if spawn_from(w, st.template[t], pos[t][l]).is_some() {
                    N_RESPAWN.fetch_add(1, Ordering::Relaxed);
                    st.dead_since[t][l] = u64::MAX;
                    st.dirty = 2;
                    crate::log(&format!("world {:#x} tick {}: inhibitor respawn team{} lane{}", w, tick, t, l));
                }
            } }
        } else { N_ERR.fetch_add(1, Ordering::Relaxed); }
        let s2 = st;
        with_state(w, |s| { s.dead_since = s2.dead_since; s.template = s2.template; s.dirty = s.dirty.max(s2.dirty); });
    }
    let pend = with_state(w, |s| { let p = s.pending; s.pending = [[0; 3]; 2]; p });
    for t in 0..2 { for l in 0..3 {
        let b = pend[t][l];
        if b == 0 { continue; }
        if spawn_super(w, b, &c) { N_SUPER.fetch_add(1, Ordering::Relaxed); } else { N_ERR.fetch_add(1, Ordering::Relaxed); }
    } }
}

/// fast post: 타워·넥서스 목록 길이가 바뀐 틱과 그다음 틱(게임 언락이 다음 틱에 1 을 쓴다)에만 잠금·파괴 상태를 다시 계산.
/// 안전망으로 600틱마다 한 번은 무조건.
unsafe fn post_tick_fast(w: usize, ctx: usize) {
    let c = cfg();
    let Some((tick, pos)) = guard(w, ctx, &c) else { return };
    let Some(lens) = list_lens(w) else { return };
    let run = with_state(w, |s| {
        if !s.managed { return false; }
        if s.last_lens != lens { s.last_lens = lens; s.dirty = 2; }
        if s.dirty == 0 && tick % 600 != 0 { return false; }
        s.dirty = s.dirty.saturating_sub(1);
        true
    });
    if !run { N_POST_SKIP.fetch_add(1, Ordering::Relaxed); return; }
    N_POST_RUN.fetch_add(1, Ordering::Relaxed);
    let Some(sc) = scan_fast(w, &pos) else { N_ERR.fetch_add(1, Ordering::Relaxed); return };
    let dead = apply_locks(&c, &sc);
    let mut newly = [[false; 3]; 2];
    with_state(w, |s| {
        s.inhib_dead = dead;
        for t in 0..2 { for l in 0..3 {
            if !dead[t][l] { s.dead_since[t][l] = u64::MAX; }
            else if s.dead_since[t][l] == u64::MAX { s.dead_since[t][l] = tick; newly[t][l] = true; }
        } }
    });
    // 파괴 틱 기록(재생성 간격을 로그로 검증 — respawn 줄의 틱과 빼면 = respawn_sec×60). 잠금 밖에서 쓴다.
    for t in 0..2 { for l in 0..3 {
        if newly[t][l] { crate::log(&format!("world {:#x} tick {}: inhibitor down team{} lane{} (respawn at tick {})", w, tick, t, l, tick + c.respawn_ticks)); }
    } }
}

/// MOBATICK 원본 **전**(틱 사이). 생성·재생성(삽입)은 여기서만 한다:
///   add_entity 가 +0x670 = 현재 틱을 쓰고, 같은 MOBATICK 의 EntityInfo 스캔(+0x670 == 틱)이 뷰 개체를 만든다.
///   (post 에서 넣으면 틱 증가 위치에 따라 화면에 안 나올 수 있다 — RE AI·뷰 §4)
unsafe fn pre_tick_slow(w: usize, ctx: usize) {
    let c = cfg();
    let Some((tick, pos)) = guard(w, ctx, &c) else { return };
    N_TICKS.fetch_add(1, Ordering::Relaxed);

    let mut st = with_state(w, |s| {
        if !s.decided && s.last_tick == 0 && s.first_tick == 0 {
            s.first_tick = tick; s.last_tick = tick; N_WORLDS.fetch_add(1, Ordering::Relaxed);
        } else if tick < s.last_tick {
            *s = WState { first_tick: tick, last_tick: tick, ..Default::default() }; // 주소 재사용(새 경기)
            N_WORLDS.fetch_add(1, Ordering::Relaxed);
        }
        s.last_tick = tick;
        *s
    });
    let Some(sc) = scan(w, &pos) else { N_ERR.fetch_add(1, Ordering::Relaxed); return };
    let any_inhib = sc.inhib.iter().flatten().any(|&e| e != 0);

    // ① 관리 여부 판정(처음 한 번)
    if !st.decided {
        st.decided = true;
        st.dead_since = [[u64::MAX; 3]; 2];
        if any_inhib {
            st.managed = true; N_ADOPT.fetch_add(1, Ordering::Relaxed); // 복제 World — 이미 있는 억제기를 이어받음
            crate::log(&format!("world {:#x} tick {}: adopted (inhibitors already present)", w, tick));
        } else if st.first_tick <= 3 {
            // 신규 경기(첫 틱에 L2053 이 포탑·넥서스·챔피언을 만든 직후): 팀마다 쌍둥이를 템플릿으로 6개
            let mut ok = true;
            for t in 0..2 {
                let Some(&tw) = sc.twins[t].first() else { ok = false; break };
                match make_template(tw, true, c.hp_pct) { Some(tp) => st.template[t] = tp, None => { ok = false; break } }
            }
            if ok {
                let (mut n, mut want) = (0, 0);
                for t in 0..2 { for l in 0..3 {
                    if !lane_on(&c, l) { continue; }
                    want += 1;
                    if spawn_from(w, st.template[t], pos[t][l]).is_some() { n += 1; N_CREATED.fetch_add(1, Ordering::Relaxed); }
                    else { N_ERR.fetch_add(1, Ordering::Relaxed); }
                } }
                st.managed = true;
                crate::log(&format!("world {:#x} tick {}: inhibitors created {}/{} lanes={:03b} pos={:?}", w, tick, n, want, c.lanes, pos));
            } else {
                N_ERR.fetch_add(1, Ordering::Relaxed);
                crate::log(&format!("world {:#x} tick {}: create FAILED (twins {}/{})", w, tick, sc.twins[0].len(), sc.twins[1].len()));
            }
        } // 그 밖(훅 설치 전에 시작한 경기) = 관리 안 함(바닐라)
        let s2 = st;
        with_state(w, |s| *s = s2);
        return;
    }
    if !st.managed { return; }

    // ② 파괴 감지·재생성
    for t in 0..2 { for l in 0..3 {
        if !lane_on(&c, l) { continue; }
        if sc.inhib[t][l] != 0 { st.dead_since[t][l] = u64::MAX; continue; }
        if st.dead_since[t][l] == u64::MAX { st.dead_since[t][l] = tick; }
        if c.respawn_ticks == 0 || tick.saturating_sub(st.dead_since[t][l]) < c.respawn_ticks { continue; }
        // 템플릿 없으면(복제 World) 살아 있는 같은 팀 억제기 → 없으면 쌍둥이로 만든다
        if st.template[t] == 0 {
            let src = (0..3).map(|k| sc.inhib[t][k]).find(|&e| e != 0);
            st.template[t] = match src {
                Some(e) => make_template(e, false, 100).unwrap_or(0),
                None => sc.twins[t].first().and_then(|&e| make_template(e, true, c.hp_pct)).unwrap_or(0),
            };
        }
        if st.template[t] == 0 { continue; }
        // ⚠삽입하면 dense 가 재할당될 수 있다 — 이번 틱의 sc 포인터는 더 쓰지 않는다(템플릿은 우리 버퍼라 안전)
        if spawn_from(w, st.template[t], pos[t][l]).is_some() {
            N_RESPAWN.fetch_add(1, Ordering::Relaxed);
            st.dead_since[t][l] = u64::MAX;
            crate::log(&format!("world {:#x} tick {}: inhibitor respawn team{} lane{}", w, tick, t, l));
        }
    } }
    let s2 = st;
    with_state(w, |s| *s = s2);

    // ③ 슈퍼미니언 삽입(지난 틱 spawn_minion 이 복제해 둔 것) — 삽입은 마지막(앞의 sc 포인터를 더 안 쓴다)
    let pend = with_state(w, |s| { let p = s.pending; s.pending = [[0; 3]; 2]; p });
    for t in 0..2 { for l in 0..3 {
        let b = pend[t][l];
        if b == 0 { continue; }
        if spawn_super(w, b, &c) { N_SUPER.fetch_add(1, Ordering::Relaxed); } else { N_ERR.fetch_add(1, Ordering::Relaxed); }
    } }
}

/// 복제해 둔 미니언 버퍼에 슈퍼미니언 버프를 붙이고 World 에 넣는다. 버퍼는 add_entity 가 move 하므로 바이트만 해제.
unsafe fn spawn_super(w: usize, b: usize, c: &Cfg) -> bool {
    let f_buff = FN_ADD_BUFF.load(Ordering::Relaxed);
    let f_add = FN_ADD_ENTITY.load(Ordering::Relaxed);
    if f_buff == 0 || f_add == 0 || !ptr_ok(b) { return false; }
    let buf = super_buff(c);
    core::mem::transmute::<usize, FnAddBuff>(f_buff)(b, buf.as_ptr()); // push + 재계산 + HP 비례
    if let Some(mx) = rd_u64(b + E_MAX_HP) { wr_u64(b + E_HP, mx); }
    core::mem::transmute::<usize, FnAddEntity>(f_add)(w, b as *mut u8);
    drop(Box::from_raw(b as *mut [u64; ENT_STRIDE / 8]));
    true
}
fn super_buff(c: &Cfg) -> [u8; BUFF_SIZE] {
    let mut b = [0u8; BUFF_SIZE];
    let name = b"inhibitor_super_minion";
    b[B_NAME_LEN..B_NAME_LEN + 4].copy_from_slice(&(name.len() as u32).to_le_bytes());
    b[B_NAME..B_NAME + name.len()].copy_from_slice(name);
    b[B_DURATION..B_DURATION + 4].copy_from_slice(&0u32.to_le_bytes()); // Permanent
    let mut put = |off: usize, v: i32| b[off..off + 4].copy_from_slice(&v.to_le_bytes());
    put(B_ATTACK_MULT, c.super_attack_mult);
    put(B_HP_MULT, c.super_hp_mult);
    put(B_DEFENCE, c.super_defence);
    put(B_MR, c.super_mr);
    put(B_RADIUS_MULT, c.super_radius_mult);
    b
}

/// MOBATICK 원본 **뒤**. 잠금 재기록(게임 언락 단계가 이번 틱에 1 을 썼을 수 있다) + 슈퍼미니언용 파괴 상태 갱신.
unsafe fn post_tick_slow(w: usize, ctx: usize) {
    let c = cfg();
    let Some((_tick, pos)) = guard(w, ctx, &c) else { return };
    let managed = with_state(w, |s| s.managed);
    if !managed { return; }
    let Some(sc) = scan(w, &pos) else { N_ERR.fetch_add(1, Ordering::Relaxed); return };
    let dead = apply_locks(&c, &sc);
    with_state(w, |s| s.inhib_dead = dead);
}

/// 라인 l(0 탑·1 미드·2 봇)에 억제기를 두는가(cfg `lanes`)
pub fn lane_on(c: &Cfg, l: usize) -> bool { c.lanes & (1 << l) != 0 }

/// 잠금 재기록(정확히 1 을 쓴다 — AI 는 == 1 로 비교). 반환 = 억제기가 깨진 라인(슈퍼미니언·재생성용 · 억제기 없는 라인은 false).
///   억제기 = 그 라인 1·2차 전멸 뒤 공격 가능.
///   쌍둥이 = 「열린 라인」이 하나라도 있으면 공격 가능 — 억제기 라인은 억제기 파괴, 억제기 없는 라인(cfg lanes 밖)은 바닐라처럼 1·2차 전멸.
///   넥서스 = 진짜 쌍둥이 둘 파괴(lol_nexus).
unsafe fn apply_locks(c: &Cfg, sc: &Scan) -> [[bool; 3]; 2] {
    let mut dead = [[false; 3]; 2];
    for t in 0..2 {
        let mut any_open = false;
        for l in 0..3 {
            let cleared = !sc.outer[t][l] && !sc.inner[t][l];
            if !lane_on(c, l) { if cleared { any_open = true; } continue; }
            let e = sc.inhib[t][l];
            if e == 0 { any_open = true; dead[t][l] = true; continue; }
            set_ct(e, cleared as u8);
        }
        if c.lock_twin { for &e in sc.twins[t].iter() { set_ct(e, any_open as u8); } }
        if c.lol_nexus && sc.twins[t].is_empty() && sc.nexus[t] != 0 { set_ct(sc.nexus[t], 1); }
    }
    dead
}

unsafe fn set_ct(e: usize, v: u8) {
    if rd_u8(e + E_CAN_TARGET) != Some(v) { if wr_u8(e + E_CAN_TARGET, v) { N_LOCKW.fetch_add(1, Ordering::Relaxed); } }
}

/// 타워 update 진입: 억제기면 true(=원본 건너뜀). 상태를 Idle·대상 없음으로 고정(AI 의 「포탑이 나를 노림」 판정도 막는다).
pub unsafe fn tower_should_skip(w: usize, vt: usize, _ctx: usize, id: u64) -> bool {
    if !cfg().enabled { return false; }
    let va = VT_MOBA_A.load(Ordering::Relaxed);
    if va == 0 || vt != va { return false; }
    let Some(e) = entity_by_id(w, id) else { return false };
    if inhib_of(e).is_none() { return false; }
    if rd_u64(e + E_TW_STATE) != Some(0) { wr_u64(e + E_TW_STATE, 0); }
    if rd_u64(e + E_TW_NEAREST) != Some(0) { wr_u64(e + E_TW_NEAREST, 0); }
    N_SKIP_ATK.fetch_add(1, Ordering::Relaxed);
    true
}

/// spawn_minion 원본이 out 을 다 쓴 뒤: 상대 억제기가 깨진 라인이면 슈퍼미니언 버프.
pub unsafe fn after_spawn_minion(out: usize, tick: u64) {
    let c = cfg();
    if !c.enabled || !c.super_minion { return; }
    let w = CUR_W.with(|v| v.get());
    if w == 0 || !ptr_ok(out) { return; }
    if rd_u32(out + E_KIND) != Some(KIND_MINION) { return; }
    let (Some(team), Some(line)) = (rd_u64(out + E_TEAM), rd_u8(out + E_MN_LINE)) else { return };
    if team > 1 || line > 2 { return; }
    if rd_u8(out + E_MN_IS_RANGE) != Some(0) { return; } // 근접 미니언만 원본으로(웨이브 첫 미니언 = 근접)
    let (t, l) = (team as usize, line as usize);
    let enemy = 1 - t;
    // 웨이브당 1마리: 상대 억제기가 파괴 상태 ∧ 같은 라인·팀 직전 예약에서 gap 이상 지남 ∧ 대기 중인 것 없음
    let go = with_state(w, |s| {
        if !s.managed || !s.inhib_dead[enemy][l] || s.pending[t][l] != 0 { return false; }
        if s.super_tick[t][l] != 0 && tick.saturating_sub(s.super_tick[t][l] - 1) < c.super_gap_ticks { return false; }
        s.super_tick[t][l] = tick + 1;
        true
    });
    if !go { return; }
    let f_clone = FN_CLONE.load(Ordering::Relaxed);
    if f_clone == 0 { return; }
    // out 은 원본이 다 쓴 완성 엔티티 — 그대로 복제해 두고(원본 미니언은 손대지 않음) 다음 틱 pre 에 버프 붙여 삽입
    let buf: Box<[u64; ENT_STRIDE / 8]> = Box::new([0u64; ENT_STRIDE / 8]);
    let b = Box::into_raw(buf) as usize;
    core::mem::transmute::<usize, FnClone>(f_clone)(b as *mut u8, out as *const u8);
    with_state(w, |s| s.pending[t][l] = b);
}
