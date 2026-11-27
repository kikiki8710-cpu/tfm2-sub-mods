//! SEH(VEH) 보호 메모리 읽기·쓰기 — `tfm2_elemental_serpen` / `tfm2_item_tactics` 검증본 이식(스레드별 TLS, 락 없음).
//! VEH 핸들러 안에는 할당·락·패닉 경로가 없다(CLAUDE.md §3).
#![allow(dead_code)]
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

type BOOL = i32;
#[repr(C)]
struct ExceptionRecord { code: u32, flags: u32, rec: usize, addr: usize, nparams: u32, _p: u32, params: [usize; 15] }
#[repr(C)]
struct ExceptionPointers { rec: *mut ExceptionRecord, ctx: *mut core::ffi::c_void }
type VehHandler = extern "system" fn(*mut ExceptionPointers) -> i32;
extern "system" {
    fn AddVectoredExceptionHandler(first: u32, handler: VehHandler) -> usize;
    fn VirtualProtect(addr: usize, size: usize, new_protect: u32, old_protect: *mut u32) -> BOOL;
}

#[repr(C)]
struct SehTls { v: [core::cell::Cell<u64>; 8] }
thread_local! {
    static SEH_T: SehTls = const { SehTls { v: [const { core::cell::Cell::new(0) }; 8] } };
}
#[inline(always)]
fn seh_ptr() -> *mut u64 { SEH_T.with(|s| s.v.as_ptr() as *mut u64) }
static SEH_INSTALLED: AtomicBool = AtomicBool::new(false);
pub static SEH_FAULTS: AtomicU64 = AtomicU64::new(0);

extern "system" fn seh_veh(p: *mut ExceptionPointers) -> i32 {
    const CONTINUE_EXECUTION: i32 = -1;
    const CONTINUE_SEARCH: i32 = 0;
    unsafe {
        if p.is_null() { return CONTINUE_SEARCH; }
        let rec = (*p).rec;
        if rec.is_null() || (*rec).code != 0xC0000005 { return CONTINUE_SEARCH; }
        let Ok(g) = SEH_T.try_with(|s| s.v.as_ptr() as *mut u64) else { return CONTINUE_SEARCH; };
        if *g.add(0) == 0 { return CONTINUE_SEARCH; }
        let ctx = (*p).ctx as usize;
        if ctx == 0 { return CONTINUE_SEARCH; }
        let rip = *((ctx + 0xF8) as *const u64);
        if rip < *g.add(5) || rip >= *g.add(6) { return CONTINUE_SEARCH; }
        *((ctx + 0xF8) as *mut u64) = *g.add(2);
        *((ctx + 0x98) as *mut u64) = *g.add(3);
        *((ctx + 0xA0) as *mut u64) = *g.add(4);
        SEH_FAULTS.fetch_add(1, Ordering::Relaxed);
        CONTINUE_EXECUTION
    }
}
pub fn seh_install() {
    if SEH_INSTALLED.swap(true, Ordering::Relaxed) { return; }
    unsafe { AddVectoredExceptionHandler(1, seh_veh); }
}

#[inline(never)]
pub unsafe fn safe_copy(dst: *mut u8, src: *const u8, len: usize) -> bool {
    if !SEH_INSTALLED.load(Ordering::Relaxed) { return false; }
    let g = seh_ptr();
    let ok: u64;
    core::arch::asm!(
        "lea rax, [rip + 200f]",
        "mov [{g} + 40], rax",
        "lea rax, [rip + 201f]",
        "mov [{g} + 48], rax",
        "lea rax, [rip + 202f]",
        "mov [{g} + 16], rax",
        "mov [{g} + 24], rsp",
        "mov [{g} + 32], rbp",
        "mov qword ptr [{g} + 0], 1",
        "cld",
        "200:",
        "rep movsb",
        "201:",
        "mov {ok}, 1",
        "jmp 203f",
        "202:",
        "mov {ok}, 0",
        "203:",
        "mov qword ptr [{g} + 0], 0",
        g = in(reg) g,
        ok = out(reg) ok,
        inout("rcx") len => _,
        inout("rdi") dst => _,
        inout("rsi") src => _,
        out("rax") _,
    );
    ok != 0
}

#[inline]
pub fn ptr_ok(a: usize) -> bool { a >= 0x10000 && a < (1usize << 48) }

pub unsafe fn rd_u64(addr: usize) -> Option<u64> {
    if !ptr_ok(addr) { return None; }
    let mut b = [0u8; 8];
    if safe_copy(b.as_mut_ptr(), addr as *const u8, 8) { Some(u64::from_le_bytes(b)) } else { None }
}
pub unsafe fn rd_u32(addr: usize) -> Option<u32> {
    if !ptr_ok(addr) { return None; }
    let mut b = [0u8; 4];
    if safe_copy(b.as_mut_ptr(), addr as *const u8, 4) { Some(u32::from_le_bytes(b)) } else { None }
}
pub unsafe fn rd_u8(addr: usize) -> Option<u8> {
    if !ptr_ok(addr) { return None; }
    let mut b = [0u8; 1];
    if safe_copy(b.as_mut_ptr(), addr as *const u8, 1) { Some(b[0]) } else { None }
}
pub unsafe fn rd_bytes(addr: usize, out: &mut [u8]) -> bool {
    if !ptr_ok(addr) || out.is_empty() { return false; }
    safe_copy(out.as_mut_ptr(), addr as *const u8, out.len())
}
/// 게임 힙 쓰기(이미 쓰기 가능) — 실패 시에만 VirtualProtect 폴백.
pub unsafe fn wr_bytes(addr: usize, data: &[u8]) -> bool {
    if data.is_empty() || data.len() > 0x1000 || !ptr_ok(addr) { return false; }
    if safe_copy(addr as *mut u8, data.as_ptr(), data.len()) { return true; }
    let mut old = 0u32;
    if VirtualProtect(addr, data.len(), 0x04, &mut old) == 0 { return false; }
    let ok = safe_copy(addr as *mut u8, data.as_ptr(), data.len());
    let mut o2 = 0u32;
    VirtualProtect(addr, data.len(), old, &mut o2);
    ok
}
pub unsafe fn wr_u64(addr: usize, v: u64) -> bool { wr_bytes(addr, &v.to_le_bytes()) }
pub unsafe fn wr_u8(addr: usize, v: u8) -> bool { wr_bytes(addr, &[v]) }
