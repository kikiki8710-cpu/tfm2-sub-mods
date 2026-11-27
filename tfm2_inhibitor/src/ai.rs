//! AI 월드뷰 보정 — 롤 넥서스 규칙 (RE 2026-11-26_AI쌍둥이소비처·포탑렌더 §3 (b')).
//! 빌더 0x19b5420(simulation.rs L1556) post-detour 한 곳:
//!   ① 쌍둥이 Vec(od+0x140+0x20·side ptr / +0x158 len)에서 억제기를 빼고 len = 진짜 쌍둥이 수
//!      → attack_nexus·fc50e0·본진 위협·방어 관문이 고친 곳 없이 「진짜 쌍둥이 둘 파괴 = 넥서스」가 된다.
//!   ② 그 라인 2차 슬롯(Top2 +0x1C0 / Mid2 +0x1E0 / Bot2 +0x200, +8·team)이 비었으면 억제기를 넣는다
//!      → AI 에겐 1차 → 2차 → 억제기 → 쌍둥이 → 넥서스 순서로 보이고, 열린(+0x6B1==1) 억제기를 공격 대상으로 고른다.
//! od 는 호출마다 새로 만드는 스택 지역(단일 스레드) — 제자리 수정이라 일시 상태·복원이 없다.
#![allow(dead_code)]
use crate::inhib::{inhib_of, Cfg};
use crate::mem::*;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

pub const RVA_AI_BUILDER: usize = 0x19b5420;
const PRO_BUILDER_TAIL: [u8; 4] = [0x48, 0x81, 0xec, 0xc8];
const OD_TWIN_PTR: usize = 0x140;
const OD_TWIN_LEN: usize = 0x158;
const OD_SLOT2: [usize; 3] = [0x1C0, 0x1E0, 0x200];

static NEXT: AtomicUsize = AtomicUsize::new(0);
static N_CALL: AtomicU64 = AtomicU64::new(0);
static N_FIX: AtomicU64 = AtomicU64::new(0);
static N_SLOT: AtomicU64 = AtomicU64::new(0);
static ON: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

type FnBuilder = extern "win64" fn(usize, usize, usize, usize, usize, usize, usize) -> usize;

extern "win64" fn builder_detour(a1: usize, a2: usize, a3: usize, a4: usize, a5: usize, a6: usize, a7: usize) -> usize {
    let next = NEXT.load(Ordering::Acquire);
    if next == 0 { return a1; }
    let r = unsafe { core::mem::transmute::<usize, FnBuilder>(next)(a1, a2, a3, a4, a5, a6, a7) };
    if ON.load(Ordering::Relaxed) {
        let pf = crate::perf::on();
        let t0 = if pf { crate::perf::now() } else { 0 };
        let od = if ptr_ok(r) { r } else { a1 };
        let _ = catch_unwind(AssertUnwindSafe(|| unsafe { fix_od(od) }));
        if pf { crate::perf::T_AI.fetch_add(crate::perf::now().wrapping_sub(t0), Ordering::Relaxed); crate::perf::N_AI.fetch_add(1, Ordering::Relaxed); }
    }
    r
}

unsafe fn fix_od(od: usize) {
    N_CALL.fetch_add(1, Ordering::Relaxed);
    for side in 0..2usize {
        let pa = od + OD_TWIN_PTR + 0x20 * side;
        let la = od + OD_TWIN_LEN + 0x20 * side;
        let Some(len) = rd_u64(la) else { continue };
        if len == 0 || len > 64 { continue; }
        let Some(vp) = rd_u64(pa) else { continue };
        let vp = vp as usize;
        if !ptr_ok(vp) { continue; }
        let mut real: Vec<u64> = Vec::with_capacity(len as usize);
        let mut inh: Vec<(u64, usize, usize)> = Vec::new();
        for i in 0..len as usize {
            let Some(e) = rd_u64(vp + 8 * i) else { return };
            match inhib_of(e as usize) {
                Some((team, lane)) => inh.push((e, team, lane)),
                None => real.push(e),
            }
        }
        if inh.is_empty() { continue; }
        // 진짜 쌍둥이 앞, 억제기 뒤(포인터 제자리 교체) → len 축소
        let mut k = 0usize;
        for &e in real.iter() { wr_u64(vp + 8 * k, e); k += 1; }
        for &(e, _, _) in inh.iter() { wr_u64(vp + 8 * k, e); k += 1; }
        wr_u64(la, real.len() as u64);
        N_FIX.fetch_add(1, Ordering::Relaxed);
        for &(e, team, lane) in inh.iter() {
            let slot = od + OD_SLOT2[lane] + 8 * team;
            if rd_u64(slot) == Some(0) { if wr_u64(slot, e) { N_SLOT.fetch_add(1, Ordering::Relaxed); } }
        }
    }
}

pub unsafe fn install(c: Cfg) {
    if !c.lol_nexus { crate::log("AI builder hook: off (lol_nexus=0)"); return; }
    let mut full = [0u8; 16];
    full[..12].copy_from_slice(&crate::game::PRO_PUSH8);
    full[12..].copy_from_slice(&PRO_BUILDER_TAIL);
    if let crate::hook::Entry::Mismatch(cur) = crate::hook::entry_state(RVA_AI_BUILDER, &full) {
        crate::log(&format!("HOOK AI_BUILDER SKIP: prologue mismatch found={:02x?}", cur));
        return;
    }
    match crate::hook::install(RVA_AI_BUILDER, &crate::game::PRO_PUSH8, builder_detour as *const () as usize) {
        Ok((n, mode)) => { NEXT.store(n, Ordering::Release); ON.store(true, Ordering::Relaxed); crate::log(&format!("HOOK AI_BUILDER OK mode={}", mode)); }
        Err(e) => crate::log(&format!("HOOK AI_BUILDER FAIL: {}", e)),
    }
}
pub fn stats() -> String {
    let r = Ordering::Relaxed;
    format!("ai calls={} fixed={} slot={}", N_CALL.load(r), N_FIX.load(r), N_SLOT.load(r))
}
