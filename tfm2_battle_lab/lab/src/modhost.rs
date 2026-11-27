//! modhost — 어떤 모드든 「게임 안」처럼 보이게 하는 일반 환경 (모드별 처리 없음).
//! ①본체 바꾸기: PEB.ImageBaseAddress = 게임 → GetModuleHandle(NULL) 이 게임을 돌려준다. 게임 exe 를 사본 폴더에서 올리면 경로도 사본이 된다.
//! ②가짜 화면: stable API 확장(on_init/pre_update/post_update)을 빈 함수표 + 「경기 중」 장면으로 돌린다(빈 칸 = 기본값 규약).
//! ③자동 검출: 모드 로드 전후의 게임 코드·읽기 전용 자료를 비교해 바뀐 함수를 찾는다.
use crate::{rd, read_word};

/// PEB(+0x10 ImageBaseAddress) 를 게임 이미지로. 되돌릴 값은 반환.
pub unsafe fn become_game(base: usize, path: &str) -> usize {
    let peb: usize;
    std::arch::asm!("mov {}, gs:[0x60]", out(reg) peb);
    let old: usize = rd(peb + 0x10);
    *((peb + 0x10) as *mut usize) = base;
    // GetModuleFileName(NULL) = ProcessParameters(PEB+0x20) 의 ImagePathName(+0x60 UNICODE_STRING{len,max,ptr@8})
    let pp: usize = rd(peb + 0x20);
    if pp != 0 && !path.is_empty() {
        let mut w: Vec<u16> = path.replace('/', &char::from(92).to_string()).encode_utf16().collect();
        let n = (w.len() * 2) as u16; w.push(0);
        let p: &'static mut [u16] = Box::leak(w.into_boxed_slice());
        // 최신 윈도우는 로더 목록 첫 항목(본체)의 FullDllName(+0x48)·BaseDllName(+0x58) 을 쓴다
        let ldr: usize = rd(peb + 0x18);
        let first: usize = if ldr != 0 { rd(ldr + 0x10) } else { 0 };
        if first != 0 {
            *((first + 0x48) as *mut u16) = n; *((first + 0x4a) as *mut u16) = n + 2; *((first + 0x50) as *mut usize) = p.as_ptr() as usize;
            let cut = p.iter().rposition(|c| *c == 92).map_or(0, |i| i + 1);
            let bn = ((p.len() - 1 - cut) * 2) as u16;
            *((first + 0x58) as *mut u16) = bn; *((first + 0x5a) as *mut u16) = bn + 2; *((first + 0x60) as *mut usize) = p.as_ptr() as usize + cut * 2;
        }
        *((pp + 0x60) as *mut u16) = n; *((pp + 0x62) as *mut u16) = n + 2; *((pp + 0x68) as *mut usize) = p.as_ptr() as usize;
    }
    old
}

// ── 바뀐 곳 찾기 ──────────────────────────────────────
pub struct Snap { secs: Vec<(String, usize, Vec<u8>)>, funcs: Vec<(u32, u32)> }
unsafe fn sections(base: usize) -> Vec<(String, usize, usize)> {
    let pe = base + rd::<u32>(base + 0x3c) as usize;
    let n: u16 = rd(pe + 6);
    let opt: u16 = rd(pe + 20);
    (0..n as usize).map(|i| { let o = pe + 24 + opt as usize + 40 * i; let nm: [u8; 8] = rd(o); (String::from_utf8_lossy(&nm).trim_end_matches(char::from(0)).to_string(), rd::<u32>(o + 12) as usize, rd::<u32>(o + 8) as usize) }).collect()
}
pub unsafe fn snapshot(base: usize) -> Snap {
    let pe = base + rd::<u32>(base + 0x3c) as usize;
    let (pv, ps): (u32, u32) = (rd(pe + 24 + 112 + 24), rd(pe + 24 + 112 + 28));
    let funcs = (0..ps as usize / 12).map(|i| (rd(base + pv as usize + i * 12), rd(base + pv as usize + i * 12 + 4))).collect();
    let secs = sections(base).into_iter().filter(|s| s.0 == ".text" || s.0 == ".rdata").map(|(n, va, sz)| (n, va, std::slice::from_raw_parts((base + va) as *const u8, sz).to_vec())).collect();
    Snap { secs, funcs }
}
/// (구역, rva, 길이, 함수 시작 rva) 목록 — 가까운 변경(16B 이내)은 하나로 묶는다
pub unsafe fn diff(base: usize, s: &Snap) -> Vec<(String, usize, usize, usize)> {
    let mut out: Vec<(String, usize, usize, usize)> = vec![];
    for (n, va, old) in &s.secs {
        let cur = std::slice::from_raw_parts((base + va) as *const u8, old.len());
        let mut i = 0;
        while i < old.len() {
            // 4KB 단위로 먼저 비교
            let e = (i + 4096).min(old.len());
            if cur[i..e] == old[i..e] { i = e; continue; }
            for k in i..e {
                if cur[k] != old[k] {
                    let rva = va + k;
                    if let Some(l) = out.last_mut() { if &l.0 == n && rva <= l.1 + l.2 + 16 { l.2 = rva + 1 - l.1; continue; } }
                    let f = match s.funcs.binary_search_by(|p| p.0.cmp(&(rva as u32))) { Ok(j) => s.funcs[j].0, Err(j) if j > 0 && s.funcs[j - 1].1 as usize > rva => s.funcs[j - 1].0, _ => 0 };
                    out.push((n.clone(), rva, 1, f as usize));
                }
            }
            i = e;
        }
    }
    out
}

// ── 가짜 화면 ─────────────────────────────────────────
#[repr(C)]
struct ClientCtx { size: usize, ui: *const usize, scene: *const usize, asset: *const usize, save: *const usize, net: *const usize, state: *mut u8, data: *const usize, draw: *const usize }
static SCENE: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
unsafe extern "C" fn scene_kind(_s: *const u8) -> u32 { SCENE.load(std::sync::atomic::Ordering::Relaxed) }
pub fn set_scene(k: u32) { SCENE.store(k, std::sync::atomic::Ordering::Relaxed); }

pub struct Ext { pub name: String, ud: usize, vt: usize, pub server: bool }
/// 로더가 돌려준 Arc(ArcInner{strong,weak,data=안쪽}) 에서 모드의 {userdata, vtable} 을 찾는다. vtable 첫 워드(size)가 맞는 것만.
pub unsafe fn find_ext(name: &str, arc: usize, server: bool) -> Option<Ext> {
    if arc == 0 { return None; }
    let want = if server { 48usize } else { 64 };
    let check = |ud: usize, vt: usize| -> bool { vt > 0x10000 && matches!(read_word(vt), Some(s) if s >= want - 16 && s <= want + 64 && s % 8 == 0) && ud > 0x10000 && read_word(vt + 16).map_or(false, |f| f > 0x10000 || f == 0) };
    // 화면 확장 어댑터 = {id String, userdata, vtable}(+0x28/+0x30), 서버 = {userdata, vtable}(+0x10/+0x18) — 실측 2026-11-25. 배치가 바뀌어도 되게 훑는다.
    for o in (0x10..0x40).step_by(8) {
        let (a, b) = (read_word(arc + o).unwrap_or(0), read_word(arc + o + 8).unwrap_or(0));
        if check(a, b) { return Some(Ext { name: name.into(), ud: a, vt: b, server }); }
    }
    None
}
struct Tables { scene: Vec<usize>, empty: Vec<usize> }
fn tables() -> &'static Tables {
    static T: std::sync::OnceLock<Tables> = std::sync::OnceLock::new();
    T.get_or_init(|| {
        let mut scene = vec![0usize; 16]; scene[0] = 80; scene[1] = scene_kind as usize;
        let mut empty = vec![0usize; 64]; empty[0] = 8; // size=8 → 모든 칸이 「없음」
        Tables { scene, empty }
    })
}
/// 콜백 1회. slot = vtable 칸(8바이트 단위). 화면 확장: 2 on_init · 3 pre_update · 4 post_update · 서버 확장: 2 on_server_start
pub unsafe fn call(e: &Ext, slot: usize, dt: u64) {
    let size = read_word(e.vt).unwrap_or(0);
    if slot * 8 + 8 > size { return; }
    let f = read_word(e.vt + slot * 8).unwrap_or(0);
    if f == 0 { return; }
    let t = tables();
    if e.server {
        let mut ctx: [usize; 3] = [24, t.empty.as_ptr() as usize, 0];
        let f: unsafe extern "C" fn(usize, *mut usize) = std::mem::transmute(f);
        f(e.ud, ctx.as_mut_ptr());
    } else {
        let em = t.empty.as_ptr();
        let mut ctx = ClientCtx { size: std::mem::size_of::<ClientCtx>(), ui: em, scene: t.scene.as_ptr(), asset: em, save: em, net: em, state: 0x1000 as *mut u8, data: em, draw: em };
        let f: unsafe extern "C" fn(usize, *mut ClientCtx, u64) = std::mem::transmute(f);
        f(e.ud, &mut ctx, dt);
    }
}
