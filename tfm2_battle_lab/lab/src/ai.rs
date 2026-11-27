//! ai — 게임 AI 의 think 슬롯(vtable 0x4138fa8 +0x78)을 이 프로세스 안에서만 바꿔 입력을 기록·교체한다.
//! think(out: *mut [u8;32] Option<Input>, self, world, player, ctx) -> out · out 첫 i64 == -1 = 입력 없음 (0.7.3 0x1695ad0 호출부 판독 · 0.7.4 슬롯 함수 본문 같음)
use crate::rd;
use std::sync::atomic::{AtomicUsize, Ordering::Relaxed};

pub const VT_AI: usize = 0x4138fa8;
static ORIG: AtomicUsize = AtomicUsize::new(0);
static NSEEN: AtomicUsize = AtomicUsize::new(0);
pub static PTRS: [AtomicUsize; 16] = [const { AtomicUsize::new(0) }; 16]; // 선수 번호 → Player*
pub static MISS: AtomicUsize = AtomicUsize::new(0);
/// 선수별 덮어쓸 입력(32B). 첫 워드 u64::MAX = 덮지 않음 · u64::MAX-1 = 입력 없음 강제(허수아비)
pub static OVERRIDE: [[AtomicUsize; 4]; 16] = [const { [const { AtomicUsize::new(usize::MAX) }; 4] }; 16];
pub static LAST: [[AtomicUsize; 4]; 16] = [const { [const { AtomicUsize::new(0) }; 4] }; 16];


type Think = unsafe extern "win64" fn(usize, usize, usize, usize, usize, usize, usize, usize) -> usize;

pub static MUTE: [AtomicUsize; 16] = [const { AtomicUsize::new(0) }; 16]; // 1 = 원본 AI 를 부르지 않음(허수아비)

unsafe extern "win64" fn think_hook(out: usize, me: usize, a: usize, b: usize, c: usize, d: usize, e: usize, f: usize) -> usize {
    // 선수 번호 = 인자 b(Player*) 를 PTRS(선수 목록 순서)와 대조
    let mut idx = usize::MAX;
    for i in 0..16 { if PTRS[i].load(Relaxed) == b && b != 0 { idx = i; break; } }
    NSEEN.fetch_add(1, Relaxed);
    if idx == usize::MAX { MISS.fetch_add(1, Relaxed); }
    let mut r = out;
    if idx < 16 && MUTE[idx].load(Relaxed) == 1 {
        *(out as *mut usize) = usize::MAX;
    } else {
        let orig: Think = std::mem::transmute(ORIG.load(Relaxed));
        r = orig(out, me, a, b, c, d, e, f);
    }
    if idx < 16 {
        for k in 0..4 { LAST[idx][k].store(rd::<usize>(out + k * 8), Relaxed); }
        let o0 = OVERRIDE[idx][0].load(Relaxed);
        if o0 == usize::MAX - 1 { *(out as *mut usize) = usize::MAX; }
        else if o0 != usize::MAX { for k in 0..4 { *((out + k * 8) as *mut usize) = OVERRIDE[idx][k].load(Relaxed); } }
    }
    r
}

pub unsafe fn install(base: usize) -> bool {
    #[link(name = "kernel32")]
    extern "system" { fn VirtualProtect(a: usize, n: usize, p: u32, o: *mut u32) -> i32; }
    let slot = base + VT_AI + 0x78;
    let cur: usize = rd(slot);
    if cur != base + 0x10ea500 { println!("AI 훅 실패: 슬롯 값 game+0x{:x} (기대 0x10ea500)", cur.wrapping_sub(base)); return false; }
    ORIG.store(cur, Relaxed);
    let mut old = 0u32;
    if VirtualProtect(slot, 8, 4, &mut old) == 0 { return false; }
    *(slot as *mut usize) = think_hook as usize;
    VirtualProtect(slot, 8, old, &mut old);
    true
}
pub fn last(i: usize) -> [usize; 4] { [LAST[i][0].load(Relaxed), LAST[i][1].load(Relaxed), LAST[i][2].load(Relaxed), LAST[i][3].load(Relaxed)] }
pub fn set_override(i: usize, v: [usize; 4]) { for k in (0..4).rev() { OVERRIDE[i][k].store(v[k], Relaxed); } }
pub fn clear_override(i: usize) { OVERRIDE[i][0].store(usize::MAX, Relaxed); }
pub fn freeze(i: usize) { OVERRIDE[i][0].store(usize::MAX - 1, Relaxed); }
pub fn count() -> usize { NSEEN.load(Relaxed) }
