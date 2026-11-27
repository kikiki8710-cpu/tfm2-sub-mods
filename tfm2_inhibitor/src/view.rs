//! 외형 교체 — KEYRES(그림 키 리졸버) post-detour 로 억제기·슈퍼미니언만 모드 시트 키로 제자리 치환.
//! 근거 = mods_report\tfm2_inhibitor\RE\2026-11-26_외형교체경로-KEYRES.md (0.7.4 · 정적)
//!   · fn(rcx = out String(sret), rdx = 뷰 개체 VE) -> out. 뷰 스레드 전용 · sim 무영향.
//!   · 억제기 = 출력 키가 {blue|red}_tower ∧ VE 위치(+0x17C/+0x180 f32 = sim×0.001)가 억제기 좌표(POS_CACHE)
//!   · 슈퍼미니언 = 출력 키가 UI_aseprite/minion ∧ VE 버프 목록(+0xB8 ptr/+0xC0 len, 원소 0x28, 이름 +8/+0x10)에 "inhibitor_super_minion"
//!   · 키는 cap 이하일 때만 바이트 덮고 len 만 고친다(serpen keyres_swap 과 같은 방식). ptr·cap 불변.
//!   ⚠`{키}#anim` 에셋이 없으면 게임이 패닉 → 모드 폴더에 파일이 있을 때만 켠다.
//!   ⚠serpen 이 KEYRES 를 체인 비인식으로 먼저 설치 → 우리는 post_update 에서 늦게·외부 훅 대기 뒤 1회 체인.
#![allow(dead_code)]
use crate::mem::*;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};

pub const RVA_KEYRES: usize = 0x238b840;
pub const PRO_KEYRES: [u8; 16] = [0x55, 0x56, 0x57, 0x48, 0x83, 0xec, 0x70, 0x48, 0x8d, 0x6c, 0x24, 0x70, 0x48, 0xc7, 0x45, 0xf8];

const K_BLUE_TOWER: &[u8] = b"asset/base/aseprite_resources/ingame/blue_tower";
const K_RED_TOWER: &[u8] = b"asset/base/aseprite_resources/ingame/red_tower";
const K_MINION: &[u8] = b"asset/base/aseprite_resources/UI_aseprite/minion";
const K_BI: &[u8] = b"asset/tfm2_inhibitor/s/bi";
const K_RI: &[u8] = b"asset/tfm2_inhibitor/s/ri";
const K_SM: &[u8] = b"asset/tfm2_inhibitor/s/sm";
const SUPER_BUFF: &[u8] = b"inhibitor_super_minion";

const VE_X: usize = 0x17C;
const VE_Y: usize = 0x180;
const VE_BUFF_PTR: usize = 0xB8;
const VE_BUFF_LEN: usize = 0xC0;
const VE_BUFF_STRIDE: usize = 0x28;

pub static INHIB_ART: AtomicBool = AtomicBool::new(false);
pub static SUPER_ART: AtomicBool = AtomicBool::new(false);
pub static N_SWAP_INHIB: AtomicU64 = AtomicU64::new(0);
pub static N_SWAP_SUPER: AtomicU64 = AtomicU64::new(0);
static NEXT: AtomicUsize = AtomicUsize::new(0);
type FnKeyRes = extern "win64" fn(usize, usize) -> usize;

extern "win64" fn keyres_detour(out: usize, ve: usize) -> usize {
    let next = NEXT.load(Ordering::Acquire);
    if next == 0 { return out; }
    let r = unsafe { core::mem::transmute::<usize, FnKeyRes>(next)(out, ve) };
    let _ = catch_unwind(AssertUnwindSafe(|| unsafe { swap(out, ve) }));
    r
}

unsafe fn swap(out: usize, ve: usize) {
    let (Some(cap), Some(p), Some(len)) = (rd_u64(out), rd_u64(out + 8), rd_u64(out + 0x10)) else { return };
    let (cap, p, len) = (cap as usize, p as usize, len as usize);
    if len < K_RED_TOWER.len() || len > 64 || !ptr_ok(p) { return; }
    let mut buf = [0u8; 64];
    if !rd_bytes(p, &mut buf[..len]) { return; }
    let key = &buf[..len];
    let new: &[u8] = if key == K_BLUE_TOWER || key == K_RED_TOWER {
        if !INHIB_ART.load(Ordering::Relaxed) { return; }
        let team = if key == K_BLUE_TOWER { 0 } else { 1 };
        if !is_inhib_ve(ve, team) { return; }
        N_SWAP_INHIB.fetch_add(1, Ordering::Relaxed);
        if team == 0 { K_BI } else { K_RI }
    } else if key == K_MINION {
        if !SUPER_ART.load(Ordering::Relaxed) || !has_super_buff(ve) { return; }
        N_SWAP_SUPER.fetch_add(1, Ordering::Relaxed);
        K_SM
    } else {
        return;
    };
    if new.len() > cap { return; }
    if wr_bytes(p, new) { wr_u64(out + 0x10, new.len() as u64); }
}

unsafe fn is_inhib_ve(ve: usize, team: usize) -> bool {
    let (Some(xb), Some(yb)) = (rd_u32(ve + VE_X), rd_u32(ve + VE_Y)) else { return false };
    let (x, y) = (f32::from_bits(xb) as f64 * 1000.0, f32::from_bits(yb) as f64 * 1000.0);
    if !x.is_finite() || !y.is_finite() || x < 0.0 || y < 0.0 { return false; }
    crate::inhib::is_inhib_pos(team, (x.round() as u64, y.round() as u64))
}

unsafe fn has_super_buff(ve: usize) -> bool {
    let (Some(p), Some(n)) = (rd_u64(ve + VE_BUFF_PTR), rd_u64(ve + VE_BUFF_LEN)) else { return false };
    if n == 0 || n > 64 || !ptr_ok(p as usize) { return false; }
    for i in 0..n as usize {
        let e = p as usize + i * VE_BUFF_STRIDE;
        let (Some(np), Some(nl)) = (rd_u64(e + 8), rd_u64(e + 0x10)) else { return false };
        if nl as usize != SUPER_BUFF.len() || !ptr_ok(np as usize) { continue; }
        let mut b = [0u8; 32];
        if rd_bytes(np as usize, &mut b[..SUPER_BUFF.len()]) && &b[..SUPER_BUFF.len()] == SUPER_BUFF { return true; }
    }
    false
}

/// 모드 폴더에 시트·fanim 이 다 있을 때만 해당 교체를 켠다(없는 키로 바꾸면 게임 패닉).
pub fn check_assets(mod_dir: &str) {
    let has = |k: &str| ["#sheet.png", "#anim.fanim"].iter().all(|s| std::path::Path::new(&format!("{}\\s\\{}{}", mod_dir, k, s)).is_file());
    INHIB_ART.store(has("bi") && has("ri"), Ordering::Relaxed);
    SUPER_ART.store(has("sm"), Ordering::Relaxed);
    crate::log(&format!("view assets: inhibitor={} super_minion={}", INHIB_ART.load(Ordering::Relaxed), SUPER_ART.load(Ordering::Relaxed)));
}

static INSTALLED: AtomicBool = AtomicBool::new(false);
static WAIT: AtomicU64 = AtomicU64::new(0);
/// post_update 에서 매 프레임 호출 — 1회 설치 확정 뒤 no-op.
pub fn install_tick(serpen_loaded: bool) {
    if INSTALLED.load(Ordering::Relaxed) { return; }
    if !INHIB_ART.load(Ordering::Relaxed) && !SUPER_ART.load(Ordering::Relaxed) { INSTALLED.store(true, Ordering::Relaxed); return; }
    let w = WAIT.fetch_add(1, Ordering::Relaxed);
    let st = unsafe { crate::hook::entry_state(RVA_KEYRES, &PRO_KEYRES) };
    let go = match st {
        crate::hook::Entry::Foreign(_) => true,
        crate::hook::Entry::Clean => !serpen_loaded || w >= 600,
        crate::hook::Entry::Mismatch(cur) => { crate::log(&format!("HOOK KEYRES SKIP: prologue mismatch found={:02x?}", cur)); INSTALLED.store(true, Ordering::Relaxed); return; }
    };
    if !go { return; }
    INSTALLED.store(true, Ordering::Relaxed);
    match unsafe { crate::hook::install(RVA_KEYRES, &PRO_KEYRES[..12], keyres_detour as *const () as usize) } {
        Ok((n, mode)) => { NEXT.store(n, Ordering::Release); crate::log(&format!("HOOK KEYRES OK mode={} after {} frames", mode, w)); }
        Err(e) => crate::log(&format!("HOOK KEYRES FAIL: {}", e)),
    }
}
