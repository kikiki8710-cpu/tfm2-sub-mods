//! 진입부 12B 트램폴린 설치 + 외부 훅 체인 (champion_exclude/hook.rs · serpen launcher_install_tick 패턴).
//! detour 는 "다음 함수(next)"를 불러 원본(또는 앞선 외부 훅)을 실행한다.
//!   · 깨끗한 프롤로그: next = 트램폴린(원본 앞 N바이트 + jmp fn+N)
//!   · 외부 훅(`48 b8 <tgt> ff e0`): next = tgt (함수 진입 상태로 들어가는 코드라 그대로 호출 가능)
//! ⛔매 프레임 재검증·재체인 금지(DONE: 상호 체인 사이클 사고) — 설치는 1회 확정.
#![allow(dead_code)]
type BOOL = i32;
extern "system" {
    fn GetModuleHandleW(name: *const u16) -> usize;
    fn VirtualProtect(addr: usize, size: usize, new_protect: u32, old: *mut u32) -> BOOL;
    fn VirtualAlloc(addr: usize, size: usize, alloc_type: u32, protect: u32) -> usize;
    fn FlushInstructionCache(proc: usize, addr: usize, sz: usize) -> BOOL;
    fn GetCurrentProcess() -> usize;
}

pub fn exe_base() -> usize { unsafe { GetModuleHandleW(core::ptr::null()) } }

pub fn jmp_abs(target: usize) -> [u8; 12] {
    let mut b = [0u8; 12];
    b[0] = 0x48; b[1] = 0xB8; b[2..10].copy_from_slice(&(target as u64).to_le_bytes()); b[10] = 0xFF; b[11] = 0xE0;
    b
}

pub enum Entry { Clean, Foreign(usize), Mismatch([u8; 16]) }

/// 진입부 상태 판정. `prologue` = 원본 바이트(≥12, 재배치 가능한 명령 경계까지).
pub unsafe fn entry_state(rva: usize, prologue: &[u8]) -> Entry {
    let base = exe_base();
    let addr = base + rva;
    let mut cur = [0u8; 16];
    core::ptr::copy_nonoverlapping(addr as *const u8, cur.as_mut_ptr(), 16);
    if &cur[..prologue.len()] == prologue { return Entry::Clean; }
    if cur[0] == 0x48 && cur[1] == 0xB8 && cur[10] == 0xFF && cur[11] == 0xE0 {
        return Entry::Foreign(u64::from_le_bytes(cur[2..10].try_into().unwrap()) as usize);
    }
    Entry::Mismatch(cur)
}

/// 설치. 성공 시 next(호출할 원본/외부 훅 주소) 반환.
pub unsafe fn install(rva: usize, prologue: &[u8], detour: usize) -> Result<(usize, String), String> {
    if prologue.len() < 12 { return Err("prologue < 12".into()); }
    let base = exe_base();
    if base == 0 { return Err("exe base 0".into()); }
    let addr = base + rva;
    let (next, mode) = match entry_state(rva, prologue) {
        Entry::Clean => {
            let tramp = VirtualAlloc(0, 0x100, 0x3000, 0x40);
            if tramp == 0 { return Err("VirtualAlloc".into()); }
            core::ptr::copy_nonoverlapping(addr as *const u8, tramp as *mut u8, prologue.len());
            let j = jmp_abs(addr + prologue.len());
            core::ptr::copy_nonoverlapping(j.as_ptr(), (tramp + prologue.len()) as *mut u8, 12);
            (tramp, "clean".to_string())
        }
        Entry::Foreign(tgt) => (tgt, format!("chain(ext={:#x})", tgt)),
        Entry::Mismatch(cur) => return Err(format!("prologue mismatch rva={:#x} found={:02x?}", rva, cur)),
    };
    // 외부 훅이면 그 12B 만 덮는다(뒤 바이트는 외부 패치 잔여일 수 있음). 깨끗하면 프롤로그 길이만큼(나머지 nop).
    let plen = if mode == "clean" { prologue.len() } else { 12 };
    let mut patch = vec![0x90u8; plen];
    patch[..12].copy_from_slice(&jmp_abs(detour));
    let mut old = 0u32;
    if VirtualProtect(addr, plen, 0x40, &mut old) == 0 { return Err("VirtualProtect".into()); }
    core::ptr::copy_nonoverlapping(patch.as_ptr(), addr as *mut u8, plen);
    let mut o2 = 0u32;
    VirtualProtect(addr, plen, old, &mut o2);
    FlushInstructionCache(GetCurrentProcess(), addr, plen);
    Ok((next, mode))
}
