//! battle_lab_host 0.0.2 — 게임 exe 를 라이브러리로 불러오는 시제품 (0.7.4 · 2026-11-23)
//!
//! 단계(02_구현정보 검증표):
//!   V2a  ① exe 올리기 ② 함수표 제자리 ③ 전역 상태 없는 leaf 실행
//!   V3   ④ TLS 슬롯 확인 ⑤ 게임 세이브 로더로 세이브 → Database ⑥ match_replays 순회·필드 판독
//!
//! 사용: battle_lab_host [세이브 경로] [exe 경로]
//! 주소는 0.7.4 RVA — 정본 = `re\syms_0.7.4.json`. exe 크기가 다르면 아무것도 부르지 않는다.
//! 세이브는 읽기만 한다(게임 로더에 바이트만 넘김 · 파일 쓰기 없음).
mod sim;
mod lab;
mod ai;
mod view;
mod modhost;
use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;

#[link(name = "kernel32")]
extern "system" {
    fn LoadLibraryExW(name: *const u16, file: usize, flags: u32) -> usize;
    fn GetLastError() -> u32;
    fn GetProcessHeap() -> usize;
    fn HeapAlloc(heap: usize, flags: u32, size: usize) -> usize;
    fn K32EnumProcessModules(proc_: usize, mods: *mut usize, cb: u32, needed: *mut u32) -> i32;
    fn GetCurrentProcess() -> usize;
    fn SuspendThread(h: usize) -> u32;
    fn ResumeThread(h: usize) -> u32;
    fn GetThreadContext(h: usize, ctx: *mut u8) -> i32;
}

#[link(name = "kernel32")]
extern "system" {
    fn LoadLibraryW(name: *const u16) -> usize;
    fn GetProcAddress(m: usize, name: *const u8) -> usize;
    fn VirtualProtect(addr: usize, size: usize, newp: u32, old: *mut u32) -> i32;
}
const PAGE_READWRITE: u32 = 0x04;

/// ★LoadLibrary 로 올린 EXE 는 **임포트가 해석되지 않는다**(MS 문서: 실행 모듈은 DONT_RESOLVE_DLL_REFERENCES 처럼 로드).
///   실측 0.7.3: IAT 가 비어 할당자(GetProcessHeap)·예외 처리기(__CxxFrameHandler3) 호출이 무효 → 예외 재귀로 스택 소진.
/// ⟹ 임포트 디렉터리를 직접 돌며 DLL 을 올리고 IAT 를 채운다. DLL 은 게임 폴더 먼저(steam_api64 등), 없으면 시스템 검색.
unsafe fn resolve_imports(base: usize, game_dir: &str) -> Result<(usize, usize), String> {
    let pe = base + rd::<u32>(base + 0x3c) as usize;
    let imp_rva: u32 = rd(pe + 24 + 112 + 8);
    if imp_rva == 0 { return Ok((0, 0)); }
    let mut desc = base + imp_rva as usize;
    let (mut ndll, mut nfn) = (0usize, 0usize);
    loop {
        let oft: u32 = rd(desc);
        let name_rva: u32 = rd(desc + 12);
        let ft: u32 = rd(desc + 16);
        if name_rva == 0 { break; }
        let name_ptr = (base + name_rva as usize) as *const u8;
        let dll = std::ffi::CStr::from_ptr(name_ptr as *const i8).to_string_lossy().into_owned();
        let local = format!("{game_dir}\\{dll}");
        let mut h = if std::path::Path::new(&local).exists() { LoadLibraryW(wide(&local).as_ptr()) } else { 0 };
        if h == 0 { h = LoadLibraryW(wide(&dll).as_ptr()); }
        if h == 0 { return Err(format!("{dll} 로드 실패 GetLastError={}", GetLastError())); }
        ndll += 1;
        let lookup = if oft != 0 { oft } else { ft } as usize;
        let mut i = 0usize;
        loop {
            let th: u64 = rd(base + lookup + i * 8);
            if th == 0 { break; }
            let addr = if th >> 63 == 1 {
                GetProcAddress(h, (th & 0xffff) as usize as *const u8)
            } else {
                GetProcAddress(h, (base + (th & 0x7fff_ffff) as usize + 2) as *const u8)
            };
            if addr == 0 {
                let what = if th >> 63 == 1 { format!("ord{}", th & 0xffff) } else {
                    std::ffi::CStr::from_ptr((base + (th & 0x7fff_ffff) as usize + 2) as *const i8).to_string_lossy().into_owned()
                };
                return Err(format!("{dll}!{what} 해석 실패"));
            }
            let slot = base + ft as usize + i * 8;
            let mut old = 0u32;
            VirtualProtect(slot, 8, PAGE_READWRITE, &mut old);
            *(slot as *mut usize) = addr;
            VirtualProtect(slot, 8, old, &mut old);
            nfn += 1;
            i += 1;
        }
        desc += 20;
    }
    Ok((ndll, nfn))
}

pub static SNAP: std::sync::Mutex<Option<modhost::Snap>> = std::sync::Mutex::new(None);
pub static MOD_DLLS: std::sync::Mutex<Vec<(String, usize)>> = std::sync::Mutex::new(Vec::new());
static GAME_BASE: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
const GAME_IMAGE_SIZE: usize = 0x5a31000;

/// 스택 끝을 넘을 수 있는 주소를 안전하게 읽는다(못 읽으면 None). 진단기가 스택 끝을 넘어 읽다 프로세스를 죽인 사고(2026-11-25 · 세그폴트 139 · VEH 무기록) 방지.
pub fn read_word(a: usize) -> Option<usize> {
    #[link(name = "kernel32")]
    extern "system" { fn ReadProcessMemory(h: isize, a: usize, b: *mut usize, n: usize, r: *mut usize) -> i32; }
    let (mut v, mut n) = (0usize, 0usize);
    if unsafe { ReadProcessMemory(-1, a, &mut v, 8, &mut n) } != 0 && n == 8 { Some(v) } else { None }
}

/// 감시견: 작업 스레드를 잠깐 멈추고 RIP·스택 위 게임 이미지 복귀주소(RVA)를 찍는다. 멈춤 진단용.
fn watchdog(h: usize, rounds: u32, every_s: u64) {
    for r in 0..rounds {
        std::thread::sleep(std::time::Duration::from_millis(every_s));
        let base = GAME_BASE.load(std::sync::atomic::Ordering::Relaxed);
        unsafe {
            // ⚠스레드를 멈춘 동안에는 할당·출력 금지: 멈춘 스레드가 힙/stdout 락을 쥐고 있으면 교착(2026-11-25 실측 2/3).
            let mut buf = vec![0u8; 1232 + 16];
            let ctx = ((buf.as_mut_ptr() as usize + 15) & !15) as *mut u8;
            *(ctx.add(0x30) as *mut u32) = 0x100003; // CONTEXT_CONTROL|CONTEXT_INTEGER
            let (mut addrs, mut na) = ([0usize; 16], 0usize);
            if SuspendThread(h) == u32::MAX { loop { std::thread::sleep(std::time::Duration::from_secs(3600)); } }
            let ok = GetThreadContext(h, ctx) != 0;
            let rip = *(ctx.add(0xf8) as *const usize);
            let rsp = *(ctx.add(0x98) as *const usize);
            if ok && base != 0 {
                let mut p = rsp;
                while p < rsp + 0x40000 && na < 16 {
                    let Some(v) = read_word(p) else { break };
                    if v > base && v < base + GAME_IMAGE_SIZE { addrs[na] = v - base; na += 1; }
                    p += 8;
                }
            }
            ResumeThread(h);
            let rets: Vec<String> = addrs[..na].iter().map(|v| format!("0x{v:x}")).collect();
            let rel = if base != 0 && rip > base && rip < base + GAME_IMAGE_SIZE { format!("game+0x{:x}", rip - base) } else { format!("0x{rip:x}(이미지 밖)") };
            println!("[watchdog {r}] ok={ok} rip={rel} rsp=0x{rsp:x} 스택의 게임 주소={}", rets.join(" "));
        }
    }
}

/// 지금 프로세스에 올라온 모듈 중 정적 TLS 가 있는 것들의 `_tls_index` 최댓값(게임 이미지 제외).
/// ⟹ TEB.ThreadLocalStoragePointer 배열은 최소 (최댓값+1) 칸이다. (배열은 로더 전용 힙 소유라 HeapSize 로 못 잰다 — 실측 크래시)
unsafe fn max_static_tls_index(skip_base: usize) -> Option<u32> {
    let mut mods = vec![0usize; 1024];
    let mut need = 0u32;
    if K32EnumProcessModules(GetCurrentProcess(), mods.as_mut_ptr(), (mods.len() * 8) as u32, &mut need) == 0 {
        return None;
    }
    let mut mx: Option<u32> = None;
    for &m in mods.iter().take(need as usize / 8) {
        if m == 0 || m == skip_base { continue; }
        let pe = m + rd::<u32>(m + 0x3c) as usize;
        let tls_rva: u32 = rd(pe + 24 + 112 + 8 * 9);
        let tls_sz: u32 = rd(pe + 24 + 112 + 8 * 9 + 4);
        if tls_rva == 0 || tls_sz == 0 { continue; }
        let addr_of_index: usize = rd(m + tls_rva as usize + 16);
        if addr_of_index == 0 { continue; }
        let idx: u32 = rd(addr_of_index);
        mx = Some(mx.map_or(idx, |v| v.max(idx)));
    }
    mx
}
const HEAP_ZERO_MEMORY: u32 = 0x8;
/// 게임 이미지 TLS 디렉터리를 머리말에서 읽는다(재배치된 주소): (템플릿 시작, 끝, _tls_index 주소). 정렬 16 · zerofill 0 전제(0.7.3·0.7.4 확인).
unsafe fn tls_dir(base: usize) -> (usize, usize, usize) {
    let pe = base + rd::<u32>(base + 0x3c) as usize;
    let d = base + rd::<u32>(pe + 24 + 112 + 8 * 9) as usize;
    (rd(d), rd(d + 8), rd(d + 16))
}

/// LoadLibrary 로 올린 EXE 는 로더가 정적 TLS 를 잡아 주지 않는다(실측 0.7.3: `_tls_index` 가 0 = 호스트 자신의 칸).
/// ⟹ **이 스레드에만** 게임 이미지용 TLS 블록을 만들어 TEB.ThreadLocalStoragePointer 배열 끝에 붙이고 인덱스를 고친다.
/// ⚠게임 코드를 부르는 스레드마다 한 번씩 해야 한다. 게임 코드가 스스로 스레드를 만들면 그 스레드엔 블록이 없다(미대응).
unsafe fn attach_game_tls(base: usize) -> Result<(u32, usize), String> {
    let teb: usize;
    std::arch::asm!("mov {}, gs:[0x30]", out(reg) teb);
    let old: usize = rd(teb + 0x58);
    let heap = GetProcessHeap();
    let n = match max_static_tls_index(base) {
        Some(m) => m as usize + 1,
        None => 0,
    };
    if old == 0 && n > 0 {
        return Err("TEB TLS 배열이 비었는데 TLS 모듈이 있다".into());
    }
    let (t0, t1, tidx) = tls_dir(base);
    let size = t1 - t0;
    let block = HeapAlloc(heap, HEAP_ZERO_MEMORY, size + 16);
    if block == 0 { return Err("TLS 블록 할당 실패".into()); }
    let block = (block + 15) & !15; // 16 정렬(앞 여유 16B 안에서)
    std::ptr::copy_nonoverlapping(t0 as *const u8, block as *mut u8, size);
    let arr = HeapAlloc(heap, HEAP_ZERO_MEMORY, (n + 1) * 8);
    if arr == 0 { return Err("TLS 배열 할당 실패".into()); }
    if n > 0 { std::ptr::copy_nonoverlapping(old as *const u8, arr as *mut u8, n * 8); }
    *((arr + n * 8) as *mut usize) = block;
    *((teb + 0x58) as *mut usize) = arr;
    *(tidx as *mut u32) = n as u32;
    ORIG_TLS_ARRAY.with(|c| c.set(old));
    Ok((n as u32, block))
}

thread_local! { static ORIG_TLS_ARRAY: std::cell::Cell<usize> = const { std::cell::Cell::new(0) }; }
/// 스레드가 끝나기 전에 원래 TLS 배열로 되돌린다 — 로더는 스레드 종료 때 이 배열을 **자기 힙으로** 해제하므로
/// 우리가 프로세스 힙에 만든 배열이 남아 있으면 힙이 깨진다(실측: 결과 출력 전에 프로세스가 죽음 · 추정 원인).
unsafe fn detach_game_tls() {
    let old = ORIG_TLS_ARRAY.with(|c| c.get());
    if old == 0 { return; }
    let teb: usize;
    std::arch::asm!("mov {}, gs:[0x30]", out(reg) teb);
    *((teb + 0x58) as *mut usize) = old;
}
const LOAD_WITH_ALTERED_SEARCH_PATH: u32 = 0x8;

const DEFAULT_EXE: &str = r"C:\Program Files (x86)\Steam\steamapps\common\Teamfight Manager2\TeamfightManager2.exe";
const EXPECT_SIZE: u64 = 94_511_104; // 0.7.4

// ── 0.7.4 RVA (re\syms_0.7.4.json) ──
const SIMVT_HOST: usize = 0x4385b90;
const FIELD_VISIT_LEAF: usize = 0x3503580;
const SAVE_DECODE: usize = 0x137b580; // save_container__decode
const DB_DECODE: usize = 0x1380490; // Database__decode_from_bytes
// ── Database(0x180d0) 오프셋 ──
const DB_SIZE: usize = 0x180d0;
const DB_REPLAYS: usize = 0x17b30; // LinkedHashMap: +0x18 items · +0x20 head 센티넬
// ── MatchReplayData 오프셋 (02 §3) / 노드 ──
const NODE_PREV: usize = 0x330;
const NODE_KEY: usize = 0x338;

type FieldVisit = unsafe extern "win64" fn(out: *mut [u8; 16], s: *const u8, len: usize) -> *mut [u8; 16];
type SaveDecode = unsafe extern "win64" fn(out: *mut [u64; 4], data: *const u8, len: usize, label: *const u8, label_len: usize) -> *mut [u64; 4];
type DbDecode = unsafe extern "win64" fn(out: *mut u8, ptr: *const u8, len: usize, pos: *mut usize) -> *mut u8;

fn wide(s: &str) -> Vec<u16> {
    OsStr::new(s).encode_wide().chain(Some(0)).collect()
}
pub unsafe fn rd<T: Copy>(a: usize) -> T {
    std::ptr::read_unaligned(a as *const T)
}
/// Rust String {cap, ptr, len} 를 읽는다(게임 쪽 객체).
pub unsafe fn rstr(a: usize) -> String {
    let ptr: usize = rd(a + 8);
    let len: usize = rd(a + 16);
    if ptr == 0 || len > 4096 {
        return format!("<bad ptr=0x{ptr:x} len={len}>");
    }
    String::from_utf8_lossy(std::slice::from_raw_parts(ptr as *const u8, len)).into_owned()
}
fn tls_slot(index: u32) -> usize {
    // TEB+0x58 = ThreadLocalStoragePointer
    let teb: usize;
    unsafe { std::arch::asm!("mov {}, gs:[0x30]", out(reg) teb) };
    let arr: usize = unsafe { rd(teb + 0x58) };
    if arr == 0 { 0 } else { unsafe { rd(arr + index as usize * 8) } }
}

fn run(save: Option<String>, exe: String) -> bool {
    let size = std::fs::metadata(&exe).map(|m| m.len()).unwrap_or(0);
    println!("exe = {exe}\nsize = {size}");
    if size != EXPECT_SIZE {
        println!("STOP: exe 크기가 0.7.4 기대값({EXPECT_SIZE})과 다르다 — 주소표를 다시 만들 것");
        return false;
    }
    // ① 불러오기
    let w = wide(&exe);
    let base = unsafe { LoadLibraryExW(w.as_ptr(), 0, LOAD_WITH_ALTERED_SEARCH_PATH) };
    if base == 0 {
        println!("FAIL ① LoadLibraryExW 실패 GetLastError={}", unsafe { GetLastError() });
        return false;
    }
    println!("OK ① loaded base = 0x{base:x}");
    GAME_BASE.store(base, std::sync::atomic::Ordering::Relaxed);
    let game_dir = std::path::Path::new(&exe).parent().map(|p| p.to_string_lossy().into_owned()).unwrap_or_default();
    match unsafe { resolve_imports(base, &game_dir) } {
        Ok((d, f)) => println!("OK ①b 임포트 직접 해석: DLL {d}개 · 함수 {f}개"),
        Err(e) => { println!("FAIL ①b 임포트 해석: {e}"); return false; }
    }
    // ② 함수표
    let sz: u64 = unsafe { rd(base + SIMVT_HOST) };
    let tick = unsafe { rd::<usize>(base + SIMVT_HOST + 8) }.wrapping_sub(base);
    let ok2 = sz == 720 && tick == 0x35119f0;
    println!("{} ② SimVtableV1_host size={sz} tick 슬롯=+0x{tick:x}", if ok2 { "OK" } else { "FAIL" });
    // ③ leaf
    let f: FieldVisit = unsafe { std::mem::transmute(base + FIELD_VISIT_LEAF) };
    let mut ok3 = true;
    for (name, want) in [("cs", 6u8), ("champion", 2), ("kill", 3)] {
        let mut out = [0xEEu8; 16];
        unsafe { f(&mut out, name.as_ptr(), name.len()) };
        ok3 &= out[0] == 0 && out[1] == want;
    }
    println!("{} ③ field_visit leaf 3건", if ok3 { "OK" } else { "FAIL" });
    // ④ TLS — 로더가 준 인덱스를 먼저 보고(진단), 이 스레드에 게임 전용 블록을 붙인다.
    let idx0: u32 = unsafe { rd(tls_dir(base).2) };
    println!("    로더가 남긴 게임 _tls_index = {idx0} (0 = 호스트 칸과 겹침 = 미할당)");
    // ③b 모드 dll 은 반드시 게임 TLS 부착보다 먼저 올린다(정적 TLS 칸이 겹치는 것 방지 · RE 2026-11-25 로더 §4).
    if std::env::var("LAB_MODS").map_or(false, |v| !v.is_empty()) {
        let snap = unsafe { modhost::snapshot(base) };
        *SNAP.lock().unwrap() = Some(snap);
        let old = unsafe { modhost::become_game(base, &exe) };
        println!("LAB 본체 주소 0x{old:x} → 게임 0x{base:x}");
    }
    if let Ok(list) = std::env::var("LAB_MODS") {
        for path in list.split('|').filter(|x| !x.is_empty()) {
            let w = wide(path);
            let h = unsafe { LoadLibraryExW(w.as_ptr(), 0, 0) };
            println!("LAB 모드 dll {path} → {}", if h == 0 { format!("LoadLibrary 실패 {}", unsafe { GetLastError() }) } else { "로드".into() });
            if h != 0 && std::env::var("LAB_IAT").is_ok() { let n = unsafe { redirect_gmh(h) }; println!("LAB 모드 dll GetModuleHandle(NULL) → 게임 주소: {n} 칸 바꿈"); }
            if h != 0 { MOD_DLLS.lock().unwrap().push((path.to_string(), h)); }
        }
    }
    let (idx, block) = match unsafe { attach_game_tls(base) } {
        Ok(v) => v,
        Err(e) => { println!("FAIL ④ {e}"); return false; }
    };
    let slot = tls_slot(idx);
    let ok4 = slot == block && slot != 0;
    println!("{} ④ 게임 TLS 블록 부착 index={idx} 블록=0x{block:x} (TEB 조회=0x{slot:x})", if ok4 { "OK" } else { "FAIL" });
    if !ok4 { return false; }
    let Some(save) = save else {
        println!("(세이브 경로 없음 — ⑤⑥ 건너뜀)");
        return ok2 && ok3;
    };
    // ⑤ 세이브 → Database
    let bytes = match std::fs::read(&save) {
        Ok(b) => b,
        Err(e) => { println!("FAIL ⑤ 세이브 읽기 {save}: {e}"); return false; }
    };
    println!("    세이브 = {save} ({} B)", bytes.len());
    // 큰 세이브(여러 해 진행)는 해독에 세이브 크기의 약 14배 메모리를 쓴다(2년·44MB → 약 620MB 실측) — 실패 원인 판독용
    let (avail, total, commit) = mem_mb();
    println!("LAB 메모리 / Memory: 세이브 {} MB · 남은 실메모리 {avail} / {total} MB · 남은 커밋 {commit} MB", bytes.len() >> 20);
    let dec: SaveDecode = unsafe { std::mem::transmute(base + SAVE_DECODE) };
    let label = b"save file";
    let mut o = [0u64; 4];
    let t0 = std::time::Instant::now();
    unsafe { dec(&mut o, bytes.as_ptr(), bytes.len(), label.as_ptr(), label.len()) };
    if o[0] != 0 {
        let msg = unsafe { rstr(&o[1] as *const u64 as usize - 8 + 8) };
        println!("FAIL ⑤a 컨테이너 해제 tag={} msg={msg}", o[0]);
        return false;
    }
    let (pcap, pptr, plen) = (o[1] as usize, o[2] as usize, o[3] as usize);
    println!("OK ⑤a 컨테이너 해제 → 페이로드 {plen} B (cap {pcap}) · {:?}", t0.elapsed());
    let layout = std::alloc::Layout::from_size_align(DB_SIZE, 16).unwrap();
    let db = unsafe { std::alloc::alloc_zeroed(layout) } as usize;
    let dbdec: DbDecode = unsafe { std::mem::transmute(base + DB_DECODE) };
    let mut pos = 0usize;
    let t1 = std::time::Instant::now();
    unsafe { dbdec(db as *mut u8, pptr as *const u8, plen, &mut pos) };
    let tag: u64 = unsafe { rd(db) };
    if tag == u64::MAX {
        let msg = unsafe { rstr(db + 8) };
        println!("FAIL ⑤b Database 해독 Err: {msg}");
        return false;
    }
    println!("OK ⑤b Database 해독 · pos={pos}/{plen} · {:?}", t1.elapsed());
    // ⑥ match_replays 순회
    let items: usize = unsafe { rd(db + DB_REPLAYS + 0x18) };
    let head: usize = unsafe { rd(db + DB_REPLAYS + 0x20) };
    println!("    match_replays items={items} head=0x{head:x}");
    if head == 0 {
        println!("FAIL ⑥ head 센티넬이 없다");
        return false;
    }
    let mut n: usize = unsafe { rd(head + NODE_PREV) };
    let mut count = 0usize;
    let mut brief = 0usize;
    let mut shown = 0usize;
    while n != head && count < 1_000_000 {
        count += 1;
        let is_brief: u8 = unsafe { rd(n + 0x321) };
        if is_brief != 0 { brief += 1; }
        if shown < 3 || (shown < 6 && is_brief == 0) {
            shown += 1;
            unsafe {
                let id: usize = rd(n + NODE_KEY);
                let rid: usize = rd(n + 0x2f8);
                let seed: u64 = rd(n + 0x310);
                let tick: usize = rd(n + 0x318);
                let win: u8 = rd(n + 0x320);
                let bt: usize = rd(n + 0x300);
                let rt: usize = rd(n + 0x308);
                let blen: usize = rd(n + 0x40);
                let rlen: usize = rd(n + 0x58);
                let ver = rstr(n + 0x60);
                println!(
                    "    리플레이 key={id} id={rid} seed=0x{seed:016x} game_tick={tick} blue_win={win} is_brief={is_brief} 팀 {bt} vs {rt} 인원 {blen}/{rlen} version={ver:?}"
                );
                // blue 첫 선수(stride 0x270): 챔피언 String @+0 · athlete_id @+0x238 · position @+0x268
                let bptr: usize = rd(n + 0x38);
                if bptr != 0 && blen > 0 {
                    for i in 0..blen.min(5) {
                        let a = bptr + i * 0x270;
                        println!("        blue[{i}] champ={:?} athlete_id={} pos={}", rstr(a), rd::<usize>(a + 0x238), rd::<u32>(a + 0x268));
                    }
                }
            }
        }
        n = unsafe { rd(n + NODE_PREV) };
    }
    // LAB_PROBE="0x17900,0x17890,…" → Database 의 DataTable 후보(+0x18 개수 · +0x20 헤드)마다 노드 링크 칸 S 를 찾는다(개발용)
    if let Ok(list) = std::env::var("LAB_PROBE") {
        for t in list.split(',').filter_map(|x| usize::from_str_radix(x.trim().trim_start_matches("0x"), 16).ok()) {
            let (len, hd) = (read_word(db + t + 0x18).unwrap_or(0), read_word(db + t + 0x20).unwrap_or(0));
            let mut found = vec![];
            if hd > 0x10000 && len < 2_000_000 {
                for s in (8..0x2000).step_by(8) {
                    let mut n = read_word(hd + s).unwrap_or(0); let mut k = 0usize;
                    while n != hd && n > 0x10000 && k <= len { n = read_word(n + s).unwrap_or(0); k += 1; }
                    if n == hd && k == len && len > 0 { found.push(s); }
                }
            }
            println!("PROBE +0x{t:x} len={len} head=0x{hd:x} link={:x?}", found);
        }
    }
    // LAB_DUMP="표:링크:개수,…" → 앞 노드들의 8바이트 칸 전부(문자열로 보이면 글자도)
    if let Ok(list) = std::env::var("LAB_DUMP") {
        for spec in list.split(',') {
            let v: Vec<usize> = spec.split(':').filter_map(|x| usize::from_str_radix(x.trim(), 16).ok()).collect();
            if v.len() < 3 { continue; }
            let (t, s, cnt) = (v[0], v[1], v[2]);
            let hd = read_word(db + t + 0x20).unwrap_or(0);
            let mut n = read_word(hd + s).unwrap_or(0);
            for i in 0..cnt {
                if n == hd || n < 0x10000 { break; }
                let mut line = format!("DUMP +0x{t:x}[{i}]");
                for o in (0..s).step_by(8) {
                    let w = read_word(n + o).unwrap_or(0);
                    let (p, l) = (read_word(n + o + 8).unwrap_or(0), read_word(n + o + 16).unwrap_or(0));
                    let mut txt = String::new();
                    if o + 24 <= s && l > 0 && l < 80 && w >= l && w < 1 << 20 && p > 0x10000 {
                        let b: Vec<u8> = (0..l).map(|k| { let q = read_word((p + k) & !7).unwrap_or(0); (q >> (((p + k) & 7) * 8)) as u8 }).collect();
                        if let Ok(x) = String::from_utf8(b) { if x.chars().all(|c| !c.is_control()) { txt = format!("\"{x}\""); } }
                    }
                    line += &format!(" {o:x}={w:x}{txt}");
                }
                println!("{line}");
                n = read_word(n + s).unwrap_or(0);
            }
        }
    }
    if let Ok(path) = std::env::var("LAB_INSPECT") {
        return unsafe { view::inspect(db, head, &path, &save) };
    }
    // ---- 실험실: LAB_SCENARIO=<시나리오 파일> ----
    if let Ok(path) = std::env::var("LAB_SCENARIO") {
        return unsafe { lab::run(base, db, head, &path) };
    }
    // ---- 모드 챔피언 시험(사본 전용) ----
    // LAB_ADD="경로|경로" : .data_champion 을 게임 파서로 읽어 항목 추가 · LAB_REPLACE=1 : 같은 이름 항목을 새 정의로 통째 교체 · LAB_PATCH=1 : 행동 패치 채움
    unsafe {
        let sheets = || -> Vec<(String, usize)> {
            let mut v = vec![("live".to_string(), db + 0x9308)];
            let pp = db + 0x17cb0;
            let (ctrl, mask): (usize, usize) = (rd(pp), rd(pp + 8));
            if ctrl != 0 { for i in 0..=mask { if rd::<u8>(ctrl + i) & 0x80 == 0 { let b = ctrl - (i + 1) * 0x9030; v.push((rstr(b), b + 0x18 + 0x1538)); } } }
            v
        };
        let find = |ci: usize, id: &str| -> usize { let (ptr, len): (usize, usize) = (rd(ci + 8), rd(ci + 16)); for i in 0..len { let e = ptr + i * 0xb38; if rstr(e + 0x10) == id { return e; } } 0 };
        let zalloc = |n: usize| std::alloc::alloc_zeroed(std::alloc::Layout::from_size_align(n, 16).unwrap()) as usize;
        let halloc = |n: usize| { #[link(name = "kernel32")] extern "system" { fn GetProcessHeap() -> usize; fn HeapAlloc(h: usize, f: u32, n: usize) -> usize; } HeapAlloc(GetProcessHeap(), 8, n) };
        if let Ok(list) = std::env::var("LAB_ADD") {
            let replace = std::env::var("LAB_REPLACE").is_ok();
            for path in list.split('|') {
                let mut raw = std::fs::read(path).expect("read data_champion");
                if raw.starts_with(&[0xef, 0xbb, 0xbf]) { raw.drain(..3); }
                let out = zalloc(0x9f0);
                let sr: [usize; 3] = [raw.as_ptr() as usize, raw.len(), 0];
                let parse: unsafe extern "system" fn(usize, usize) -> usize = std::mem::transmute(base + 0x3ce990);
                parse(out, sr.as_ptr() as usize);
                if rd::<i64>(out) == -1 {
                    let eb: usize = rd(out + 8);
                    let msg = if eb != 0 && rd::<usize>(eb) == 0 { let (mp, ml): (usize, usize) = (rd(eb + 8), rd(eb + 0x10)); if ml < 4096 { String::from_utf8_lossy(std::slice::from_raw_parts(mp as *const u8, ml)).into_owned() } else { "?".into() } } else { format!("code {}", rd::<usize>(eb)) };
                    println!("ADD FAIL parse {path}: {msg} (line {} col {})", rd::<usize>(eb + 0x18), rd::<usize>(eb + 0x20));
                    continue;
                }
                let id = rstr(out + 0x10);
                let arc = halloc(0xa00);
                *(arc as *mut usize) = 1 << 20; *((arc + 8) as *mut usize) = 1; // strong 넉넉히 = 해제 안 됨
                std::ptr::copy_nonoverlapping(out as *const u8, (arc + 0x10) as *mut u8, 0x9f0);
                let d = arc + 0x10; let vt = base + 0x42440d0;
                let conv: unsafe extern "system" fn(usize, usize, usize, usize) -> usize = std::mem::transmute(base + 0x1dd65d0);
                let fill: unsafe extern "system" fn(usize, usize) = std::mem::transmute(base + 0x1dd7720);
                let grow: unsafe extern "system" fn(usize) = std::mem::transmute(base + 0x3fb4a20);
                let push = |ci: usize| { let tmp = zalloc(0xb38); conv(tmp, d, arc, vt);
                    let (cap, len): (usize, usize) = (rd(ci), rd(ci + 16)); if cap == len { grow(ci); }
                    std::ptr::copy_nonoverlapping(tmp as *const u8, (rd::<usize>(ci + 8) + len * 0xb38) as *mut u8, 0xb38); *((ci + 16) as *mut usize) = len + 1; };
                for (tag, ci) in sheets() {
                    if !replace && tag != "live" { continue; }
                    let e = find(ci, &id);
                    let act = if e == 0 { push(ci); "push" } else if replace { let tmp = zalloc(0xb38); conv(tmp, d, arc, vt); std::ptr::copy_nonoverlapping(tmp as *const u8, e as *mut u8, 0xb38); "replace" }
                        else { fill(e, d); if rd::<u8>(e + 0xb34) == 0 { *((e + 0x9a0) as *mut usize) = arc; *((e + 0x9a8) as *mut usize) = vt; } "fill" };
                    let f = find(ci, &id);
                    let (a, v) = sim::call3_ret2(base + 0x1dc1a30, ci, id.as_ptr() as usize, id.len());
                    println!("ADD {id:?} {tag}: {act} len={} +9a0={} 조회={} vt=+0x{:x}", rd::<usize>(ci + 16), rd::<usize>(f + 0x9a0) != 0, a != 0, v.wrapping_sub(base));
                }
                if !replace {
                    let past: unsafe extern "system" fn(usize, usize) = std::mem::transmute(base + 0x13b5280);
                    past(db, d);
                    let n = sheets().iter().filter(|(t, ci)| t != "live" && find(*ci, &id) != 0).count();
                    println!("ADD {id:?} 과거 버전 시트 {n}개에 존재");
                }
            }
        }
        if std::env::var("LAB_PATCH").is_ok() {
            let pm = db + 0x17c80;
            println!("PATCH 현행 맵 items={}", rd::<usize>(pm + 0x18));
            let f1: unsafe extern "system" fn(usize, usize) = std::mem::transmute(base + 0x1dd57b0);
            let f2: unsafe extern "system" fn(usize, usize) = std::mem::transmute(base + 0x13dbf30);
            f1(db + 0x9308, pm); f2(db + 0x17cb0, pm);
            for (tag, ci) in sheets() { let (ptr, len): (usize, usize) = (rd(ci + 8), rd(ci + 16));
                let n = (0..len).filter(|i| rd::<usize>(ptr + i * 0xb38 + 0x9b0) != 0).count();
                println!("PATCH {tag}: 항목 {len} 중 +9b0 채움 {n}"); }
        }
    }
    if std::env::var("LAB_MODCHAMP").is_ok() {
        unsafe {
            let show = |tag: &str, ci: usize| {
                let (cap, ptr, len): (usize, usize, usize) = (rd(ci), rd(ci + 8), rd(ci + 16));
                println!("MODCHAMP {tag} cap={cap} len={len}");
                if len < 4096 { for i in 0..len { let e = ptr + i * 0xb38;
                    println!("   [{i}] id={:?} +0={} +8={:x} +9a0={:x} +9b0={:x}", rstr(e + 0x10), rd::<u64>(e), rd::<u64>(e + 8), rd::<u64>(e + 0x9a0), rd::<u64>(e + 0x9b0));
                    if tag == "live" { let id = rstr(e + 0x10);
                        let (arc, vt) = sim::call3_ret2(base + 0x1dc1a30, ci, id.as_ptr() as usize, id.len());
                        println!("   LOOKUP {id:?} {} vt=+0x{:x}", if arc == 0 { "NULL" } else { "FOUND" }, vt.wrapping_sub(base)); } } }
            };
            show("live", db + 0x9308);
            let pp = db + 0x17cb0;
            let (ctrl, mask): (usize, usize) = (rd(pp), rd(pp + 8));
            if ctrl != 0 { for i in 0..=mask { if rd::<u8>(ctrl + i) & 0x80 == 0 { let b = ctrl - (i + 1) * 0x9030; show(&rstr(b), b + 0x18 + 0x1538); } } }
        }
    }
    // LAB_VERS=1 → pre_patch_data 키 목록 + 리플레이 버전 문자열 분포
    if std::env::var("LAB_VERS").is_ok() {
        unsafe {
            let pp = db + 0x17cb0;
            let (ctrl, mask, items): (usize, usize, usize) = (rd(pp), rd(pp + 8), rd(pp + 0x18));
            let mut keys = Vec::new();
            if ctrl != 0 { for i in 0..=mask { if rd::<u8>(ctrl + i) & 0x80 == 0 { keys.push(rstr(ctrl - (i + 1) * 0x9030)); } } }
            println!("VERS pre_patch_data items={items} 키={keys:?}");
            let mut hist: std::collections::BTreeMap<String, (usize, usize)> = Default::default();
            let mut m: usize = rd(head + NODE_PREV);
            while m != head { let e = hist.entry(rstr(m + 0x60)).or_default(); if rd::<u8>(m + 0x321) == 0 { e.0 += 1 } else { e.1 += 1 }; m = rd(m + NODE_PREV); }
            for (v, (full, brief)) in hist { println!("VERS 리플레이 version={v:?} 재생가능 {full} · 요약 {brief}"); }
        }
    }
    // LAB_ITEMS=1 → 기록 items 의 성격 판정: 길이 분포 · 최종템(id%5==4) 비율 · 경기 길이와의 관계
    if std::env::var("LAB_ITEMS").is_ok() {
        unsafe {
            let mut len_hist: std::collections::BTreeMap<usize, usize> = Default::default();
            let (mut n_items, mut n_final) = (0usize, 0usize);
            let mut short: Vec<(usize, usize, Vec<u64>)> = Vec::new(); // (game_tick, key, items) 짧은 경기 표본
            let mut nonfinal: Vec<(usize, Vec<u64>)> = Vec::new();
            let mut m: usize = rd(head + NODE_PREV);
            let mut ticks: Vec<usize> = Vec::new();
            while m != head {
                if rd::<u8>(m + 0x321) == 0 {
                    let tick: usize = rd(m + 0x318);
                    ticks.push(tick);
                    for vec_off in [0x30usize, 0x48] {
                        let ptr: usize = rd(m + vec_off + 8);
                        let len: usize = rd(m + vec_off + 16);
                        for i in 0..len {
                            let a = ptr + i * 0x270;
                            let ip: usize = rd(a + 0x20);
                            let il: usize = rd(a + 0x28);
                            let items: Vec<u64> = if ip != 0 && il < 64 { (0..il).map(|k| rd::<u64>(ip + k * 8)).collect() } else { vec![] };
                            *len_hist.entry(items.len()).or_default() += 1;
                            n_items += items.len();
                            n_final += items.iter().filter(|x| **x % 5 == 4).count();
                            if items.iter().any(|x| *x % 5 != 4) && nonfinal.len() < 8 { nonfinal.push((tick, items.clone())); }
                            if i == 0 && vec_off == 0x30 { short.push((tick, rd::<usize>(m + NODE_KEY), items)); }
                        }
                    }
                }
                m = rd(m + NODE_PREV);
            }
            short.sort();
            println!("ITEMS 선수 수(길이별) {len_hist:?} · 아이템 {n_items}개 중 최종템(id%5==4) {n_final}개");
            println!("ITEMS 최종템 아닌 것이 섞인 예: {nonfinal:?}");
            println!("ITEMS 가장 짧은 경기 5건(game_tick, key, blue 첫 선수 items): {:?}", &short[..short.len().min(5)]);
            ticks.sort();
            if !ticks.is_empty() { println!("ITEMS game_tick 최소 {} · 중앙 {} · 최대 {}", ticks[0], ticks[ticks.len() / 2], ticks[ticks.len() - 1]); }
        }
    }
    // LAB_LIST=1 → 재생 가능한 리플레이 전수 요약(대상 고르기용)
    if std::env::var("LAB_LIST").is_ok() {
        let mut m: usize = unsafe { rd(head + NODE_PREV) };
        while m != head {
            unsafe {
                if rd::<u8>(m + 0x321) == 0 {
                    let k = |p: usize| (0..5).map(|i| rd::<u64>(p + 0x50 + i * 8)).sum::<u64>();
                    let champs = |off: usize| -> String {
                        let ptr: usize = rd(m + off + 8); let len: usize = rd(m + off + 16);
                        let mut v: Vec<(u32, String)> = (0..len).map(|i| { let a = ptr + i * 0x270; (rd::<u32>(a + 0x268), rstr(a)) }).collect();
                        v.sort(); v.into_iter().map(|x| x.1).collect::<Vec<_>>().join(",")
                    };
                    println!("LIST key={} 팀 {}vs{} seed=0x{:016x} tick={} blue_win={} 킬 {}:{} | {} | {}",
                        rd::<usize>(m + NODE_KEY), rd::<usize>(m + 0x300), rd::<usize>(m + 0x308), rd::<u64>(m + 0x310),
                        rd::<usize>(m + 0x318), rd::<u8>(m + 0x320), k(m + 0xa8), k(m + 0x1a0), champs(0x30), champs(0x48));
                }
                m = rd(m + NODE_PREV);
            }
        }
    }
    let ok6 = count == items;
    println!("{} ⑥ 순회 {count}건(items {items} 와 {}) · is_brief {brief}건", if ok6 { "OK" } else { "FAIL" }, if ok6 { "일치" } else { "불일치" });
    // ⑦ 재시뮬 — 재생 가능한(is_brief=0) 리플레이 앞에서 SIM_N 건
    let sim_n: usize = std::env::var("LAB_SIM_N").ok().and_then(|v| v.parse().ok()).unwrap_or(3);
    unsafe { arm_hw_bp(base) };
    let mut n: usize = unsafe { rd(head + NODE_PREV) };
    let mut done = 0usize;
    let mut agree = 0usize;
    while n != head && done < sim_n {
        let brief: u8 = unsafe { rd(n + 0x321) };
        // LAB_KEYS=1769,1770 → 그 키만
        let keys: Option<Vec<usize>> = std::env::var("LAB_KEYS").ok().map(|s| s.split(',').filter_map(|x| x.trim().parse().ok()).collect());
        let key_ok = keys.as_ref().map_or(true, |ks| ks.contains(&unsafe { rd::<usize>(n + NODE_KEY) }));
        if brief == 0 && key_ok {
            done += 1;
            unsafe {
                let key: usize = rd(n + NODE_KEY);
                let rec_tick: usize = rd(n + 0x318);
                let rec_win: u8 = rd(n + 0x320);
                // MatchTeamPerformance(0xf8) 추정 배치: kills[5] @+0x50 · total_kills @+0xd0 — kills 합으로 자가검증
                let perf = |p: usize| -> (u64, u64) {
                    let s: u64 = (0..5).map(|i| rd::<u64>(p + 0x50 + i * 8)).sum();
                    (s, rd::<u64>(p + 0xd0))
                };
                let (bk_sum, bk) = perf(n + 0xa8);
                let (rk_sum, rk) = perf(n + 0x1a0);
                println!("  ⑦ 리플레이 key={key} 기록: game_tick={rec_tick} blue_win={rec_win} 킬 {bk}:{rk} (kills 배열 합 {bk_sum}:{rk_sum} {})",
                    if bk_sum == bk && rk_sum == rk { "배치 확인" } else { "⚠배치 불일치" });
                let repeat: usize = std::env::var("LAB_REPEAT").ok().and_then(|v| v.parse().ok()).unwrap_or(1);
                let mut first: Option<(usize, (u64, u64))> = None;
                for rep in 0..repeat {
                    match sim::simulate(base, db, n, done == 1 && rep == 0) {
                        Ok(r) => {
                            let same_k = r.pkills == (bk, rk);
                            if same_k && rep == 0 { agree += 1; }
                            let vs_first = match first { None => { first = Some((r.ticks, r.kills)); String::new() }
                                Some(f) => if f == (r.ticks, r.kills) { " · 1회차와 동일".into() } else { " · ⚠1회차와 다름".into() } };
                            println!("     재시뮬#{rep}: 오라클 {}회 · World tick {} · 선수킬합 {}:{} (팀카운터 {}:{}) · seed {} · {:?} → 킬 {}{vs_first}",
                                r.ticks, r.world_tick, r.pkills.0, r.pkills.1, r.kills.0, r.kills.1, if r.seed_ok { "일치" } else { "불일치" }, r.wall,
                                if same_k { "★기록과 일치" } else { "기록과 다름" });
                        }
                        Err(e) => println!("     재시뮬#{rep} FAIL: {e}"),
                    }
                }
            }
        }
        n = unsafe { rd(n + NODE_PREV) };
    }
    if done > 0 { println!("{} ⑦ 재시뮬 {done}건 중 킬 스코어 기록 일치 {agree}건", if agree == done { "OK" } else { "DIFF" }); }
    report_hw_bp();
    // Database 는 게임 drop glue 가 필요하므로 일부러 해제하지 않는다(RE 권장 · 프로세스 종료로 회수).
    ok2 && ok3 && ok6
}

#[link(name = "kernel32")]
extern "system" {
    fn AddVectoredExceptionHandler(first: u32, h: unsafe extern "system" fn(*mut [usize; 2]) -> i32) -> usize;
    fn SetErrorMode(mode: u32) -> u32;
    fn GlobalMemoryStatusEx(p: *mut MemStat) -> i32;
}
#[repr(C)]
struct MemStat { len: u32, load: u32, total: u64, avail: u64, page_total: u64, page_avail: u64, virt_total: u64, virt_avail: u64, ext: u64 }
/// (실메모리 여유, 전체, 커밋 여유) MB
fn mem_mb() -> (u64, u64, u64) {
    let mut m = MemStat { len: std::mem::size_of::<MemStat>() as u32, load: 0, total: 0, avail: 0, page_total: 0, page_avail: 0, virt_total: 0, virt_avail: 0, ext: 0 };
    if unsafe { GlobalMemoryStatusEx(&mut m) } == 0 { return (0, 0, 0); }
    (m.avail >> 20, m.total >> 20, m.page_avail >> 20)
}
/// 진단용 VEH: 치명 예외(C0000005 등)의 코드·위치(게임 RVA)·스택의 게임 복귀주소를 한 번 찍는다. 처리는 하지 않는다(0 = 계속 검색).
unsafe extern "system" fn veh(info: *mut [usize; 2]) -> i32 {
    use std::sync::atomic::{AtomicBool, Ordering};
    static ONCE: AtomicBool = AtomicBool::new(false);
    let rec = (*info)[0];
    let ctx = (*info)[1];
    let code: u32 = rd(rec);
    let addr: usize = rd(rec + 0x10);
    // C++/Rust panic(0xE06D7363) 와 디버그 출력(0x40010006 등)은 넘긴다
    if code < 0xC0000000 || code == 0xE06D7363 { return 0; }
    if ONCE.swap(true, Ordering::Relaxed) { return 0; }
    let base = GAME_BASE.load(Ordering::Relaxed);
    let rel = |v: usize| if base != 0 && v > base && v < base + GAME_IMAGE_SIZE { format!("game+0x{:x}", v - base) } else { format!("0x{v:x}") };
    let rsp: usize = rd(ctx + 0x98);
    let mut rets = Vec::new();
    let mut p = rsp;
    while p < rsp + 0x4000 && rets.len() < 12 {
        let Some(v) = read_word(p) else { break };
        if base != 0 && v > base && v < base + GAME_IMAGE_SIZE { rets.push(rel(v)); }
        p += 8;
    }
    let info1: usize = rd(rec + 0x20 + 8); // 접근 주소(AV 일 때)
    eprintln!("[VEH] 예외 0x{code:08X} at {} (접근 주소 0x{info1:x}) · 스택: {}", rel(addr), rets.join(" "));
    0
}

// ── 하드웨어 브레이크포인트 호출 계수(LAB_BP=rva,rva,… 최대 4) — 코드를 안 바꾸므로 시뮬에 영향 없음 ──
static BP_RVA: [std::sync::atomic::AtomicUsize; 4] = [const { std::sync::atomic::AtomicUsize::new(0) }; 4];
static BP_HITS: [std::sync::atomic::AtomicU64; 4] = [const { std::sync::atomic::AtomicU64::new(0) }; 4];
#[link(name = "kernel32")]
extern "system" {
    fn SetThreadContext(h: usize, ctx: *const u8) -> i32;
    fn GetCurrentThread() -> usize;
    fn DuplicateHandle(sp: usize, sh: usize, tp: usize, th: *mut usize, access: u32, inherit: i32, opt: u32) -> i32;
}
static BP_RET: [[std::sync::atomic::AtomicUsize; 8]; 4] = [const { [const { std::sync::atomic::AtomicUsize::new(0) }; 8] }; 4];
static BP_RETN: [[std::sync::atomic::AtomicUsize; 8]; 4] = [const { [const { std::sync::atomic::AtomicUsize::new(0) }; 8] }; 4];
static BP_ARG: [[std::sync::atomic::AtomicUsize; 24]; 4] = [const { [const { std::sync::atomic::AtomicUsize::new(0) }; 24] }; 4];
unsafe extern "system" fn veh_bp(info: *mut [usize; 2]) -> i32 {
    let rec = (*info)[0];
    let ctx = (*info)[1];
    if rd::<u32>(rec) != 0x80000004 { return 0; } // EXCEPTION_SINGLE_STEP
    let dr6: u64 = rd(ctx + 0x68);
    let mut ours = false;
    for i in 0..4 { if dr6 & (1 << i) != 0 {
        let n = BP_HITS[i].fetch_add(1, std::sync::atomic::Ordering::Relaxed); ours = true;
        let rel = std::sync::atomic::Ordering::Relaxed;
        let rsp: usize = rd(ctx + 0x98);
        let ret = read_word(rsp).unwrap_or(0);
        // 반환 주소별 횟수(서로 다른 것 8개까지)
        for k in 0..8 { let cur = BP_RET[i][k].load(rel); if cur == ret || cur == 0 { BP_RET[i][k].store(ret, rel); BP_RETN[i][k].fetch_add(1, rel); break; } }
        if n < 3 { // 처음 3회: rcx rdx r8 r9 · [rsp+0x28..0x48)
            for (k, o) in [0x80usize, 0x88, 0xb8, 0xc0].iter().enumerate() { BP_ARG[i][n as usize * 8 + k].store(rd::<u64>(ctx + o) as usize, rel); }
            for k in 0..4 { BP_ARG[i][n as usize * 8 + 4 + k].store(read_word(rsp + 0x28 + k * 8).unwrap_or(0), rel); }
        }
    } }
    if !ours { return 0; }
    *((ctx + 0x68) as *mut u64) = 0;
    let ef = (ctx + 0x44) as *mut u32;
    *ef |= 0x10000; // RF: 같은 명령에서 다시 걸리지 않게
    -1 // EXCEPTION_CONTINUE_EXECUTION
}
/// 현재 스레드에 DR0~3 실행 브레이크포인트를 건다(다른 스레드가 이 스레드를 멈추고 설정).
pub unsafe fn arm_hw_bp(base: usize) {
    let Ok(list) = std::env::var("LAB_BP") else { return };
    let rvas: Vec<usize> = list.split(',').filter_map(|s| usize::from_str_radix(s.trim().trim_start_matches("0x"), 16).ok()).take(4).collect();
    if rvas.is_empty() { return; }
    for (i, r) in rvas.iter().enumerate() { BP_RVA[i].store(*r, std::sync::atomic::Ordering::Relaxed); }
    let mut me = 0usize;
    DuplicateHandle(GetCurrentProcess(), GetCurrentThread(), GetCurrentProcess(), &mut me, 0, 0, 2);
    let rv = rvas.clone();
    std::thread::spawn(move || {
        SuspendThread(me);
        let mut buf = vec![0u8; 1232 + 16];
        let c = ((buf.as_mut_ptr() as usize + 15) & !15) as *mut u8;
        *(c.add(0x30) as *mut u32) = 0x100010; // CONTEXT_DEBUG_REGISTERS
        GetThreadContext(me, c);
        let mut dr7: u64 = 0;
        for (i, r) in rv.iter().enumerate() {
            *(c.add(0x48 + i * 8) as *mut u64) = (base + r) as u64;
            dr7 |= 1 << (i * 2); // Li 활성, RW=00(실행) LEN=00
        }
        *(c.add(0x70) as *mut u64) = dr7;
        SetThreadContext(me, c);
        ResumeThread(me);
    }).join().ok();
    println!("    [BP] 실행 브레이크포인트 {} 개 설치: {}", rvas.len(), rvas.iter().map(|r| format!("0x{r:x}")).collect::<Vec<_>>().join(" "));
}
pub fn report_hw_bp() {
    for i in 0..4 {
        let r = BP_RVA[i].load(std::sync::atomic::Ordering::Relaxed);
        if r != 0 {
            let rel = std::sync::atomic::Ordering::Relaxed; let base = GAME_BASE.load(rel);
            println!("    [BP] game+0x{r:x} 호출 {}회", BP_HITS[i].load(rel));
            for k in 0..8 { let a = BP_RET[i][k].load(rel); if a != 0 { println!("        복귀 game+0x{:x} × {}", a.wrapping_sub(base), BP_RETN[i][k].load(rel)); } }
            for h in 0..3 { println!("        인자 {}", (0..8).map(|k| format!("{:x}", BP_ARG[i][h * 8 + k].load(rel))).collect::<Vec<_>>().join(" ")); }
        }
    }
}

// ── 모드 dll 의 GetModuleHandleW/A(NULL) 만 게임 주소로 ──
// 후킹형 모드는 exe 기준 주소를 GetModuleHandle(NULL) 로 얻는다. 여기서 NULL 은 계산기 exe 이므로 그 dll 의 IAT 만 바꿔 게임 이미지를 돌려준다(다른 모듈 영향 없음).
static GMH_W: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
static GMH_A: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
unsafe extern "system" fn gmh_w(name: usize) -> usize {
    if name == 0 { return GAME_BASE.load(std::sync::atomic::Ordering::Relaxed); }
    let f: unsafe extern "system" fn(usize) -> usize = std::mem::transmute(GMH_W.load(std::sync::atomic::Ordering::Relaxed)); f(name)
}
unsafe extern "system" fn gmh_a(name: usize) -> usize {
    if name == 0 { return GAME_BASE.load(std::sync::atomic::Ordering::Relaxed); }
    let f: unsafe extern "system" fn(usize) -> usize = std::mem::transmute(GMH_A.load(std::sync::atomic::Ordering::Relaxed)); f(name)
}
// GetModuleFileNameW(NULL 또는 게임 이미지) → 가짜 exe 경로(LAB_FAKE_EXE). 모드가 「exe 옆 mods/<id>」로 로그·설정을 찾을 때 게임 폴더 대신 사본 폴더를 쓰게 한다(2026-11-25 level_cap 이 게임 폴더 로그에 쓴 일 방지).
static GMF_W: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
unsafe extern "system" fn gmf_w(m: usize, buf: *mut u16, n: u32) -> u32 {
    let gb = GAME_BASE.load(std::sync::atomic::Ordering::Relaxed);
    if m == 0 || m == gb {
        let fake = std::env::var("LAB_FAKE_EXE").unwrap_or_else(|_| std::env::current_exe().map(|p| p.to_string_lossy().into_owned()).unwrap_or_default());
        let w: Vec<u16> = fake.replace('/', &char::from(92).to_string()).encode_utf16().collect();
        let k = w.len().min((n as usize).saturating_sub(1));
        std::ptr::copy_nonoverlapping(w.as_ptr(), buf, k);
        if n > 0 { *buf.add(k) = 0; }
        return k as u32;
    }
    let f: unsafe extern "system" fn(usize, *mut u16, u32) -> u32 = std::mem::transmute(GMF_W.load(std::sync::atomic::Ordering::Relaxed)); f(m, buf, n)
}
unsafe fn redirect_gmh(h: usize) -> usize {
    let pe = h + rd::<u32>(h + 0x3c) as usize;
    let imp: u32 = rd(pe + 24 + 112 + 8);
    if imp == 0 { return 0; }
    let mut d = h + imp as usize;
    let mut n = 0;
    while rd::<u32>(d + 12) != 0 {
        let (oft, ft): (u32, u32) = (rd(d), rd(d + 16));
        let mut i = 0usize;
        loop {
            let t: usize = rd(h + (if oft != 0 { oft } else { ft }) as usize + i * 8);
            if t == 0 { break; }
            if t >> 63 == 0 {
                let nm = std::ffi::CStr::from_ptr((h + t + 2) as *const _).to_string_lossy();
                let hook = match nm.as_ref() { "GetModuleHandleW" => Some((gmh_w as usize, &GMH_W)), "GetModuleHandleA" => Some((gmh_a as usize, &GMH_A)), "GetModuleFileNameW" => Some((gmf_w as usize, &GMF_W)), _ => None };
                if let Some((f, save)) = hook {
                    let slot = h + ft as usize + i * 8;
                    let cur: usize = rd(slot);
                    if cur != f { save.store(cur, std::sync::atomic::Ordering::Relaxed); let mut old = 0u32; VirtualProtect(slot, 8, 4, &mut old); *(slot as *mut usize) = f; VirtualProtect(slot, 8, old, &mut old); n += 1; }
                }
            }
            i += 1;
        }
        d += 20;
    }
    n
}

fn main() {
    // 죽어도 윈도우 「작동이 중지되었습니다」 창을 띄우지 않는다 — 앱이 종료 코드를 받아 화면에 이유를 쓴다
    unsafe { SetErrorMode(0x0001 | 0x0002 | 0x8000) };
    unsafe { AddVectoredExceptionHandler(1, veh) };
    unsafe { AddVectoredExceptionHandler(1, veh_bp) };
    let save = std::env::args().nth(1);
    let exe = std::env::args().nth(2).unwrap_or_else(|| DEFAULT_EXE.to_string());
    // 세이브 로더 프레임 합계 ≈400KB+ → 큰 스택 스레드에서 돌린다(RE 권장 8MB+).
    let h = std::thread::Builder::new().stack_size(256 << 20).spawn(move || {
        let ok = run(save, exe);
        unsafe { detach_game_tls() };
        ok
    }).unwrap();
    {
        use std::os::windows::io::AsRawHandle;
        let raw = h.as_raw_handle() as usize;
        std::thread::spawn(move || {
            let ms: u64 = std::env::var("LAB_WD_MS").ok().and_then(|v| v.parse().ok()).unwrap_or(15000);
            watchdog(raw, (1200_000 / ms) as u32, ms);
            println!("[watchdog] 1200초 안에 끝나지 않아 중단");
            std::process::exit(3);
        });
    }
    let ok = h.join().unwrap_or(false);
    println!("{}", if ok { "RESULT PASS" } else { "RESULT FAIL" });
    std::process::exit(if ok { 0 } else { 1 });
}
