// tfm2_vr_bridge — 게임이 만든 그리기 명령을 읽기만 한다(게임 0.7.4).
// 프레임 처리 0xfc7e0(rcx=renderer, rdx=assets, r8=프레임 입력) 진입에서 r8 을 읽고 원본을 그대로 부른다.
// r8: 층 Vec @0(원소 112B) · 이름→크기 표 ctrl@+72 mask@+80 items@+96 · 이름→명령 Vec 표 ctrl@+120 mask@+128 items@+144(원소 48B, ctrl 앞으로 쌓임)
// 뷰어가 만든 공유 메모리 Local\tfm2_vr_frame_v2 가 있을 때만 일한다. 텍스처는 %LOCALAPPDATA%\tfm2_vr\live\tex_*.bin.
// 시험: mods\tfm2_vr_bridge\probe\req.txt 에 "<프레임 수> <간격>" 을 쓰면 그만큼 probe\frame_*.bin 으로 쓴다.
use mod_api_stable::{declare_stable_mod, LogLevel, StableHost, StableMod};
use std::{collections::HashMap, fs, io::Write, path::PathBuf, sync::{Mutex, OnceLock, atomic::{AtomicU64, AtomicUsize, Ordering::{Acquire, Relaxed, Release}}}, time::Instant};

const RVA: usize = 0xfc7e0;
const PROLOGUE: [u8; 32] = [0x55,0x41,0x57,0x41,0x56,0x41,0x55,0x41,0x54,0x56,0x57,0x53,0x48,0x81,0xec,0xd8,0x03,0,0,0x48,0x8d,0xac,0x24,0x80,0,0,0,0x66,0x44,0x0f,0x7f,0xbd];
const RVA_LOOKUP: usize = 0xf4f20; // (렌더러, 자산, 키 ptr, 길이) -> SkImage*(참조 +1). 키가 # 로 시작하지 않으면 렌더러를 읽지 않는다
const RVA_UNREF: usize = 0x3f4bb30;
const LOOKUP_BYTES: [u8; 12] = [0x55,0x41,0x57,0x41,0x56,0x41,0x55,0x41,0x54,0x56,0x57,0x53];
const UNREF_BYTES: [u8; 10] = [0xb8,0xff,0xff,0xff,0xff,0xf0,0x0f,0xc1,0x41,0x08];
const VT_LAZY: usize = 0x4eb35f0;
const TEX_BUDGET: usize = 24 << 20;
static ORIGINAL: AtomicUsize = AtomicUsize::new(0);
static FRAMES: AtomicU64 = AtomicU64::new(0);
static COST: AtomicU64 = AtomicU64::new(0);
static COUNT: AtomicU64 = AtomicU64::new(0);
static PROBE: Mutex<(u32, u32)> = Mutex::new((0, 1));
static DIR: OnceLock<PathBuf> = OnceLock::new();

#[repr(C)] struct Mbi { base: usize, alloc: usize, ap: u32, p0: u32, size: usize, state: u32, protect: u32, typ: u32, p1: u32 }
#[link(name = "kernel32")]
extern "system" {
    fn GetModuleHandleW(n: *const u16) -> usize;
    fn GetModuleHandleExW(f: u32, a: *const u16, h: *mut usize) -> i32;
    fn GetModuleFileNameW(h: usize, b: *mut u16, n: u32) -> u32;
    fn VirtualProtect(a: usize, s: usize, p: u32, o: *mut u32) -> i32;
    fn VirtualAlloc(a: usize, s: usize, t: u32, p: u32) -> usize;
    fn VirtualQuery(a: usize, b: *mut Mbi, n: usize) -> usize;
    fn OpenFileMappingW(a: u32, i: i32, n: *const u16) -> usize;
    fn MapViewOfFile(h: usize, a: u32, hi: u32, lo: u32, n: usize) -> usize;
    fn FlushInstructionCache(p: usize, a: usize, s: usize) -> i32;
    fn GetCurrentProcess() -> usize;
}
unsafe fn readable(a: usize, n: usize) -> bool {
    if a < 0x10000 || a >= 1 << 47 || n == 0 { return false; }
    let mut at = a; let end = a.wrapping_add(n);
    while at < end {
        let mut m: Mbi = std::mem::zeroed();
        if VirtualQuery(at, &mut m, std::mem::size_of::<Mbi>()) == 0 || m.state != 0x1000 || m.protect & 0x101 != 0 { return false; }
        at = m.base + m.size;
    }
    true
}
unsafe fn u(a: usize) -> usize { (a as *const usize).read_unaligned() }
fn dir() -> Option<&'static PathBuf> {
    if let Some(d) = DIR.get() { return Some(d); }
    unsafe {
        let mut h = 0usize;
        if GetModuleHandleExW(6, dir as *const () as *const u16, &mut h) == 0 { return None; }
        let mut b = [0u16; 2048]; let n = GetModuleFileNameW(h, b.as_mut_ptr(), 2048);
        let mut p = PathBuf::from(String::from_utf16_lossy(&b[..n as usize])); p.pop();
        let _ = DIR.set(p); DIR.get()
    }
}
fn log(s: &str) { if let Some(d) = dir() { if let Ok(mut f) = fs::OpenOptions::new().create(true).append(true).open(d.join("bridge_log.txt")) { let _ = writeln!(f, "{s}"); } } }

unsafe fn raw<'a>(p: usize, n: usize) -> &'a [u8] { std::slice::from_raw_parts(p as *const u8, n) }
fn put_bytes(out: &mut Vec<u8>, b: &[u8]) { out.extend_from_slice(&(b.len() as u32).to_le_bytes()); out.extend_from_slice(b); }
/// 명령 안의 문자열·배열은 바로 뒤에서 게임이 그대로 읽는 값이라 길이 상한만 확인한다(버전은 설치 때 명령 바이트로 확인).
unsafe fn put_ptr(out: &mut Vec<u8>, p: usize, n: usize, max: usize) { if n == 0 || n > max || p < 0x10000 || p >> 47 != 0 { put_bytes(out, b""); } else { put_bytes(out, raw(p, n)); } }
/// hashbrown 표를 돈다: ctrl 앞으로 원소가 쌓이고 ctrl 바이트 최상위 비트가 0 이면 찬 칸.
unsafe fn each(ctrl: usize, mask: usize, stride: usize, mut f: impl FnMut(usize)) {
    if mask > 1 << 20 || ctrl < 0x10000 || ctrl >> 47 != 0 { return; }
    for i in 0..=mask { if *((ctrl + i) as *const u8) & 0x80 == 0 { f(ctrl - (i + 1) * stride); } }
}

// ── 텍스처 ──
struct Tex { id: u32, unique: u32, image: usize }
static TEX: Mutex<Option<HashMap<Vec<u8>, Tex>>> = Mutex::new(None);
static TEX_SEND: OnceLock<std::sync::mpsc::Sender<(PathBuf, Vec<u8>)>> = OnceLock::new();
static LOGGED: AtomicU64 = AtomicU64::new(0);
static BUDGET: AtomicUsize = AtomicUsize::new(0);
fn out_dir() -> &'static PathBuf {
    static D: OnceLock<PathBuf> = OnceLock::new();
    D.get_or_init(|| { let p = std::env::var_os("LOCALAPPDATA").map(PathBuf::from).unwrap_or_default().join("tfm2_vr").join("live"); let _ = fs::create_dir_all(&p); p })
}
/// SkImage(raster): 정보 @0x10(색공간 8·색종류 4·알파 4·폭·높이) 고유번호 @0x28, 픽셀 포인터·행바이트는 그 뒤에서 폭*4 로 검증해 찾는다.
unsafe fn pixels(img: usize) -> Option<(u32, u32, i32, i32, usize, usize)> {
    if !readable(img, 0xa0) { return None; }
    let (ct, at) = (*((img + 0x18) as *const i32), *((img + 0x1c) as *const i32)); let (w, h) = (*((img + 0x20) as *const i32), *((img + 0x24) as *const i32));
    if !(1..=16384).contains(&w) || !(1..=16384).contains(&h) { return None; }
    for o in (0x30..0x90).step_by(8) {
        let (p, rb) = (u(img + o), u(img + o + 8));
        if rb >= w as usize * 4 && rb <= w as usize * 4 + 256 && p > 0x10000 && readable(p, rb * (h as usize - 1) + w as usize * 4) { return Some((w as u32, h as u32, ct, at, p, rb)); }
    }
    None
}
/// SkImage_Lazy: 공유 생성기 @0x38 → 생성기 @8 → 코덱 @0x28 → 메모리 스트림 @0x30 → SkData @8(포인터 @24, 길이 @32) = 압축 원본(PNG 등).
unsafe fn encoded(img: usize) -> Option<(usize, usize)> {
    let mut at = img + 0x38;
    for o in [8usize, 0x28, 0x30, 8] { let v = u(at); if !readable(v, o + 8) { return None; } at = v + o; }
    let d = u(at); if !readable(d, 40) { return None; }
    let (p, n) = (u(d + 24), u(d + 32)); if !(16..64 << 20).contains(&n) || !readable(p, n) { return None; }
    let m = raw(p, 12);
    if m.starts_with(&[0x89, b'P', b'N', b'G']) || (m.starts_with(b"RIFF") && &m[8..12] == b"WEBP") || m.starts_with(&[0xff, 0xd8]) || m.starts_with(b"BM") { Some((p, n)) } else { None }
}
/// 압축 원본이 없으면 SkImage_Base::onReadPixels(가상 함수 15번, 캐시 안 함)으로 프리멀티 BGRA 를 받는다.
unsafe fn read_pixels(img: usize, w: i32, h: i32) -> Option<Vec<u8>> {
    let base = GetModuleHandleW(std::ptr::null()); let vt = u(img); if vt != base + VT_LAZY { return None; }
    let info: [u64; 3] = [0, 6 | (2u64 << 32), w as u32 as u64 | ((h as u32 as u64) << 32)];
    let mut out = vec![0u8; w as usize * h as usize * 4];
    let f: unsafe extern "system" fn(usize, usize, *const u64, *mut u8, usize, i32, i32, i32) -> bool = std::mem::transmute(u(vt + 15 * 8));
    if f(img, 0, info.as_ptr(), out.as_mut_ptr(), w as usize * 4, 0, 0, 1) { Some(out) } else { None }
}
unsafe fn explain(img: usize, key: &[u8]) {
    static N: AtomicU64 = AtomicU64::new(0); if N.fetch_add(1, Relaxed) >= 3 { return; }
    let base = GetModuleHandleW(std::ptr::null()); let mut t = format!("설명 {} base {base:x}", String::from_utf8_lossy(key));
    let shared = u(img + 0x38); let gen = if readable(shared, 16) { u(shared + 8) } else { 0 };
    for o in (0..0x80).step_by(8) { if !readable(gen + o, 8) { break; } let w = u(gen + o); t += &format!("
 g+{o:x} {w:x}");
        if readable(w, 0x60) && !(base..base + 0x6000000).contains(&w) { for q in (0..0x60).step_by(8) { let z = u(w + q); t += &format!(" [{q:x}]{z:x}"); if readable(z, 16) && !(base..base + 0x6000000).contains(&z) { t += &format!("<{:02x?}>", raw(z, 8)); } } } else if readable(w, 8) { t += &format!(" vt {:x}", w - base); } }
    log(&t);
}
unsafe fn texture(assets: usize, key: &[u8]) -> u32 {
    if key.is_empty() || key[0] == b'#' || assets == 0 { return 0; }
    let base = GetModuleHandleW(std::ptr::null());
    let find: unsafe extern "system" fn(usize, usize, *const u8, usize) -> usize = std::mem::transmute(base + RVA_LOOKUP);
    let unref: unsafe extern "system" fn(usize) = std::mem::transmute(base + RVA_UNREF);
    let img = find(0, assets, key.as_ptr(), key.len()); if img == 0 { return 0; }
    let unique = *((img + 0x28) as *const u32);
    let mut guard = TEX.lock().unwrap_or_else(|e| e.into_inner()); let map = guard.get_or_insert_with(HashMap::new);
    let next = map.len() as u32 + 1;
    let id = match map.get(key) { Some(t) if t.unique == unique && t.image == img => { let id = t.id; unref(img); return id; } Some(t) if t.id != 0 => t.id, _ => next };
    let (w, h) = (*((img + 0x20) as *const i32), *((img + 0x24) as *const i32));
    let left = BUDGET.load(Relaxed); let mut kind = 0u32; let mut data: Option<Vec<u8>> = None;
    if (1..=16384).contains(&w) && (1..=16384).contains(&h) {
        if let Some((p, n)) = encoded(img) {
            if left < n && left != TEX_BUDGET { unref(img); return 0; } // 다음 프레임에
            BUDGET.store(left.saturating_sub(n), Relaxed); data = Some(raw(p, n).to_vec());
        } else if let Some((_, _, ct, _, p, rb)) = pixels(img) {
            kind = ct as u32; let mut v = Vec::with_capacity(w as usize * h as usize * 4); for y in 0..h as usize { v.extend_from_slice(raw(p + y * rb, w as usize * 4)); } data = Some(v);
        } else {
            if left != TEX_BUDGET { unref(img); return 0; }
            explain(img, key); BUDGET.store(0, Relaxed); kind = 6; data = read_pixels(img, w, h);
        }
    }
    let ok = data.is_some();
    if let Some(d) = data {
        let mut out = Vec::with_capacity(d.len() + key.len() + 64);
        out.extend_from_slice(b"TVTX"); for v in [2u32, id, unique, w as u32, h as u32, kind, 0] { out.extend_from_slice(&v.to_le_bytes()); } put_bytes(&mut out, key); out.extend_from_slice(&d);
        if let Some(s) = TEX_SEND.get() { let _ = s.send((out_dir().join(format!("tex_{id:05}.bin")), out)); }
        if kind != 0 && LOGGED.fetch_add(1, Relaxed) < 40 { log(&format!("픽셀 직접 {kind} {}", String::from_utf8_lossy(key))); }
    } else if LOGGED.fetch_add(1, Relaxed) < 40 { log(&format!("텍스처 못 얻음 {}", String::from_utf8_lossy(key))); }
    map.insert(key.to_vec(), Tex { id: if ok { id } else { 0 }, unique, image: img });
    unref(img); if ok { id } else { 0 }
}

/// at: 셰이더 이름 String(cap -1 이면 없음) + 값 표 ctrl@+24 mask@+32 items@+48 (원소 40B: 이름 24·데이터 ptr·vtable)
unsafe fn style(out: &mut Vec<u8>, at: usize) {
    if u(at) == usize::MAX { put_bytes(out, b""); out.push(0); return; }
    put_ptr(out, u(at + 8), u(at + 16), 256);
    let count_at = out.len(); out.push(0); let mut n = 0u8;
    if u(at + 48) != 0 { each(u(at + 24), u(at + 32), 40, |e| {
        let (data, vt) = (u(e + 24), u(e + 32)); let (size, align) = (u(vt + 8), u(vt + 16));
        if size > 256 || align > 64 || n >= 32 { return; }
        n += 1; put_ptr(out, u(e + 8), u(e + 16), 64);
        let at = data + ((align.max(1) - 1) & !15) + 16;
        if size == 24 && align == 8 {
            // Vec 값(색 바꾸기 표 등): 길이 + 내용(원소 크기는 뷰어가 안다)
            let (cap, p, len) = (u(at), u(at + 8), u(at + 16));
            let n = len.min(16) * 32;
            let mut v = Vec::with_capacity(n + 16); v.extend_from_slice(&(len as u64).to_le_bytes()); v.extend_from_slice(&(cap as u64).to_le_bytes());
            if len > 0 && len <= cap && cap < 4096 && readable(p, n) { v.extend_from_slice(raw(p, n)); }
            put_bytes(out, &v);
        } else { put_bytes(out, raw(at, size)); }
    }); }
    out[count_at] = n;
}
unsafe fn keyed(out: &mut Vec<u8>, assets: usize, c: usize) {
    let (k, kl) = (u(c + 16), u(c + 24)); put_ptr(out, k, kl, 512);
    let t = if kl > 0 && kl <= 512 && k > 0x10000 { texture(assets, raw(k, kl)) } else { 0 }; out.extend_from_slice(&t.to_le_bytes());
}
unsafe fn capture(frame: u64, assets: usize, r8: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(1 << 20); BUDGET.store(TEX_BUDGET, Relaxed);
    out.extend_from_slice(b"TVF2"); out.extend_from_slice(&2u32.to_le_bytes()); out.extend_from_slice(&frame.to_le_bytes());
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_micros() as u64).unwrap_or(0); out.extend_from_slice(&now.to_le_bytes());
    let (lp, ln) = (u(r8 + 8), u(r8 + 16).min(4096)); out.extend_from_slice(&(ln as u32).to_le_bytes());
    for i in 0..ln { let e = lp + i * 112; out.extend_from_slice(raw(e, 112)); if u(e) == usize::MAX { put_ptr(&mut out, u(e + 16), u(e + 24), 512); } else { put_bytes(&mut out, b""); } }
    let mut part = Vec::new(); let mut n = 0u32;
    each(u(r8 + 72), u(r8 + 80), 32, |e| { n += 1; put_ptr(&mut part, u(e + 8), u(e + 16), 512); part.extend_from_slice(raw(e + 24, 8)); });
    out.extend_from_slice(&n.to_le_bytes()); out.extend_from_slice(&part);
    let at = out.len(); out.extend_from_slice(&[0; 4]); let mut n = 0u32;
    each(u(r8 + 120), u(r8 + 128), 48, |e| {
        n += 1; put_ptr(&mut out, u(e + 8), u(e + 16), 512);
        let (p, l) = (u(e + 32), u(e + 40).min(100_000)); out.extend_from_slice(&(l as u32).to_le_bytes());
        for i in 0..l {
            let c = p + i * 208; let d = u(c); let tag = if d >> 63 == 1 { d & 0xff } else { 7 };
            out.extend_from_slice(raw(c, 208));
            match tag {
                2 => put_ptr(&mut out, u(c + 16), u(c + 24), 512),
                3 | 4 => { keyed(&mut out, assets, c); style(&mut out, c + 32); }
                5 => { keyed(&mut out, assets, c); put_ptr(&mut out, u(c + 40), u(c + 48).saturating_mul(32), 1 << 20); }
                6 => { keyed(&mut out, assets, c); put_ptr(&mut out, u(c + 40), u(c + 48).saturating_mul(64), 1 << 20); }
                7 => { put_ptr(&mut out, u(c + 8), u(c + 16), 8192); put_ptr(&mut out, u(c + 32), u(c + 40), 256); }
                9 => put_ptr(&mut out, u(c + 16), u(c + 24).saturating_mul(24), 1 << 20),
                14 => put_ptr(&mut out, u(c + 16), u(c + 24).saturating_mul(8), 1 << 20),
                11 => put_ptr(&mut out, u(c + 16), u(c + 24), 1 << 22),
                _ => {}
            }
        }
    });
    out[at..at + 4].copy_from_slice(&n.to_le_bytes());
    out
}

// ── 공유 메모리(뷰어가 만든다): 머리 4096B(+8 최신 번호) + 칸 3개(번호 8B·길이 8B·여백·자료@64) ──
const SLOT: usize = 8 << 20; const SLOTS: usize = 3; const MAP: usize = 4096 + SLOT * SLOTS;
static VIEW: AtomicUsize = AtomicUsize::new(0);
unsafe fn view(frame: u64) -> usize {
    let v = VIEW.load(Relaxed); if v != 0 || frame % 60 != 0 { return v; }
    let name: Vec<u16> = "Local\\tfm2_vr_frame_v2\0".encode_utf16().collect();
    let h = OpenFileMappingW(6, 0, name.as_ptr()); if h == 0 { return 0; }
    let v = MapViewOfFile(h, 6, 0, 0, MAP); if v != 0 { VIEW.store(v, Relaxed); log("뷰어 연결"); }
    v
}
unsafe fn publish(v: usize, data: &[u8]) {
    if data.len() + 64 > SLOT { return; }
    let latest = &*((v + 8) as *const AtomicU64); let seq = latest.load(Acquire) + 1;
    let s = v + 4096 + (seq as usize % SLOTS) * SLOT; let mark = &*(s as *const AtomicU64);
    mark.store(0, Release);
    *((s + 8) as *mut u64) = data.len() as u64; std::ptr::copy_nonoverlapping(data.as_ptr(), (s + 64) as *mut u8, data.len());
    mark.store(seq, Release); latest.store(seq, Release);
}
fn on_frame(assets: usize, r8: usize) {
    let frame = FRAMES.fetch_add(1, Relaxed) + 1;
    let v = unsafe { view(frame) };
    let Some(d) = dir() else { return };
    let mut probe = PROBE.lock().unwrap_or_else(|e| e.into_inner());
    if probe.0 == 0 && frame % 30 == 0 {
        let req = d.join("probe").join("req.txt");
        if let Ok(text) = fs::read_to_string(&req) {
            let _ = fs::remove_file(&req); let mut it = text.split_whitespace().filter_map(|v| v.parse::<u32>().ok());
            *probe = (it.next().unwrap_or(1).min(2000), it.next().unwrap_or(1).max(1));
        }
    }
    let dump = probe.0 > 0 && frame % probe.1 as u64 == 0;
    if v == 0 && !dump { return; }
    let t = Instant::now(); let b = unsafe { capture(frame, assets, r8) }; let us = t.elapsed().as_micros() as u64;
    if v != 0 { unsafe { publish(v, &b) }; }
    COST.fetch_add(us, Relaxed); let n = COUNT.fetch_add(1, Relaxed) + 1;
    if n % 600 == 0 { log(&format!("수집 평균 {}us (600프레임) 크기 {}", COST.swap(0, Relaxed) / 600, b.len())); }
    if dump { probe.0 -= 1; let _ = fs::write(d.join("probe").join(format!("frame_{frame:08}.bin")), &b); log(&format!("프레임 {frame} {}바이트 수집 {us}us", b.len())); }
}
unsafe extern "system" fn detour(a: usize, b: usize, c: usize) -> usize {
    let _ = std::panic::catch_unwind(|| on_frame(b, c));
    let f: unsafe extern "system" fn(usize, usize, usize) -> usize = std::mem::transmute(ORIGINAL.load(Relaxed));
    f(a, b, c)
}
unsafe fn install() -> Result<String, String> {
    let base = GetModuleHandleW(std::ptr::null()); let site = base + RVA;
    if !readable(site, 32) || !readable(base + RVA_LOOKUP, 12) || !readable(base + RVA_UNREF, 10) { return Err("지점을 읽을 수 없음".into()); }
    let cur = raw(site, 32).to_vec();
    if cur != PROLOGUE { return Err(format!("명령 바이트 불일치(게임 버전 다름) {cur:02x?}")); }
    if raw(base + RVA_LOOKUP, 12) != LOOKUP_BYTES || raw(base + RVA_UNREF, 10) != UNREF_BYTES { return Err("보조 함수 바이트 불일치".into()); }
    let t = VirtualAlloc(0, 64, 0x3000, 0x40); if t == 0 { return Err("VirtualAlloc".into()); }
    let mut code = PROLOGUE[..12].to_vec(); code.extend_from_slice(&[0xff, 0x25, 0, 0, 0, 0]); code.extend_from_slice(&(site + 12).to_le_bytes());
    std::ptr::copy_nonoverlapping(code.as_ptr(), t as *mut u8, code.len()); ORIGINAL.store(t, Relaxed);
    let mut jump = vec![0x48u8, 0xb8]; jump.extend_from_slice(&(detour as *const () as usize).to_le_bytes()); jump.extend_from_slice(&[0xff, 0xe0]);
    let mut old = 0; if VirtualProtect(site, 12, 0x40, &mut old) == 0 { return Err("VirtualProtect".into()); }
    std::ptr::copy_nonoverlapping(jump.as_ptr(), site as *mut u8, 12); VirtualProtect(site, 12, old, &mut old); FlushInstructionCache(GetCurrentProcess(), site, 12);
    Ok(format!("훅 설치 {site:#x}"))
}
fn init(host: &StableHost) -> StableMod {
    if let Some(d) = dir() { let _ = fs::create_dir_all(d.join("probe")); }
    let (tx, rx) = std::sync::mpsc::channel::<(PathBuf, Vec<u8>)>(); let _ = TEX_SEND.set(tx);
    std::thread::spawn(move || while let Ok((p, b)) = rx.recv() { let t = p.with_extension("tmp"); if fs::write(&t, &b).is_ok() { let _ = fs::rename(&t, &p); } });
    if let Ok(rd) = fs::read_dir(out_dir()) { for e in rd.flatten() { if e.file_name().to_string_lossy().starts_with("tex_") { let _ = fs::remove_file(e.path()); } } }
    match unsafe { install() } {
        Ok(s) => { log(&s); host.log(LogLevel::Info, "tfm2_vr_bridge hook installed"); }
        Err(e) => { log(&format!("훅 실패: {e}")); host.log(LogLevel::Warn, "tfm2_vr_bridge disabled (version mismatch)"); }
    }
    StableMod::new("tfm2_vr_bridge")
}
declare_stable_mod!(init);
