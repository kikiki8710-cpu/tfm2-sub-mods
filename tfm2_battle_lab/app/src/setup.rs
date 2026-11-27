//! 첫 실행 준비: 게임 폴더 찾기 · 게임 번들에서 화면용 그림/설정 꺼내기(이 프로그램은 게임 파일을 담고 있지 않다) · 옛 기록 정리.
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

pub const GAME_VER: &str = "0.7.4";
pub const EXE_SIZE: u64 = 94_511_104; // 0.7.4 — lab/src/main.rs EXPECT_SIZE 와 같이 바꿀 것
const EXE: &str = "TeamfightManager2.exe";
const DIR_NAME: &str = "Teamfight Manager2";
const NOTE: &str = "game_path.txt";

fn has_game(p: &Path) -> bool { p.join(EXE).is_file() }

#[link(name = "advapi32")]
extern "system" { fn RegGetValueW(h: isize, sub: *const u16, val: *const u16, flags: u32, ty: *mut u32, data: *mut u16, len: *mut u32) -> i32; }
#[link(name = "kernel32")]
extern "system" { fn GetLogicalDrives() -> u32; fn GetDriveTypeW(p: *const u16) -> u32; }
fn wide(s: &str) -> Vec<u16> { s.encode_utf16().chain([0]).collect() }
/// 레지스트리 글자 값(유니코드 경로 그대로). 없으면 None.
fn reg(hive: isize, key: &str, name: &str) -> Option<String> {
    let (k, n) = (wide(&key.replace('/', &char::from(92).to_string())), wide(name));
    let mut buf = [0u16; 1024]; let mut len = (buf.len() * 2) as u32;
    let r = unsafe { RegGetValueW(hive, k.as_ptr(), n.as_ptr(), 0x2 /* RRF_RT_REG_SZ */, std::ptr::null_mut(), buf.as_mut_ptr(), &mut len) };
    if r != 0 { return None; }
    let e = buf.iter().position(|c| *c == 0).unwrap_or(buf.len());
    Some(String::from_utf16_lossy(&buf[..e])).filter(|x| !x.is_empty())
}
/// 스팀 라이브러리 목록(libraryfolders.vdf 의 "path" 줄 — 모든 드라이브)
fn libraries(steam: &Path) -> Vec<PathBuf> {
    let mut v = vec![steam.to_path_buf()];
    for f in ["steamapps/libraryfolders.vdf", "config/libraryfolders.vdf"] {
        if let Ok(b) = std::fs::read(steam.join(f)) {
            for l in String::from_utf8_lossy(&b).lines() {
                let p: Vec<&str> = l.split('"').collect();
                if p.len() >= 5 && p[1].eq_ignore_ascii_case("path") { let x = PathBuf::from(p[3].replace(&char::from(92).to_string().repeat(2), "/")); if !v.contains(&x) { v.push(x); } }
            }
        }
    }
    v
}
/// 라이브러리 하나 안의 게임 폴더(설치 정보 파일의 installdir 우선)
fn in_library(lib: &Path) -> PathBuf {
    let sa = lib.join("steamapps");
    let dir = std::fs::read(sa.join("appmanifest_3009300.acf")).ok().and_then(|b| String::from_utf8_lossy(&b).lines().find_map(|l| { let p: Vec<&str> = l.split('"').collect(); (p.len() >= 5 && p[1].eq_ignore_ascii_case("installdir")).then(|| p[3].to_string()) }));
    sa.join("common").join(dir.filter(|d| !d.is_empty() && !d.contains("..")).unwrap_or_else(|| DIR_NAME.into()))
}
/// 스팀 설치 위치 후보: 시험용 BL_STEAM, 레지스트리
fn steam_roots() -> Vec<PathBuf> {
    if let Some(p) = std::env::var_os("BL_STEAM") { return vec![PathBuf::from(p)]; }
    const HKCU: isize = 0x80000001u32 as i32 as isize; const HKLM: isize = 0x80000002u32 as i32 as isize;
    [(HKCU, "Software/Valve/Steam", "SteamPath"), (HKLM, "SOFTWARE/WOW6432Node/Valve/Steam", "InstallPath"), (HKLM, "SOFTWARE/Valve/Steam", "InstallPath")].iter().filter_map(|(h, k, n)| reg(*h, k, n)).map(PathBuf::from).collect()
}
/// 레지스트리로 못 찾을 때: 고정 드라이브마다 흔한 스팀 폴더를 본다
fn drive_guess() -> Vec<PathBuf> {
    let mut v = vec![]; let m = unsafe { GetLogicalDrives() };
    for k in 0..26u8 { if m >> k & 1 == 0 { continue; } let r = format!("{}:", (b'A' + k) as char);
        if unsafe { GetDriveTypeW(wide(&format!("{r}{}", char::from(92))).as_ptr()) } != 3 { continue; } // 고정 드라이브만(네트워크·CD 는 오래 걸릴 수 있음)
        for d in ["SteamLibrary", "Steam", "Program Files (x86)/Steam", "Program Files/Steam", "Games/Steam", "Games/SteamLibrary", "Game/Steam", "Game/SteamLibrary"] { v.push(PathBuf::from(format!("{r}/{d}"))); } }
    v
}

/// 사람이 적은 경로(따옴표·exe 경로도 허용) → 게임 폴더
pub fn clean_path(t: &str) -> Option<PathBuf> {
    let l = t.trim_start_matches('\u{feff}').lines().map(|x| x.trim().trim_matches('"').trim()).find(|x| !x.is_empty() && !x.starts_with('#'))?;
    let p = PathBuf::from(l);
    Some(if p.file_name().map_or(false, |n| n.eq_ignore_ascii_case(EXE)) { p.parent().map(|x| x.to_path_buf()).unwrap_or(p) } else { p })
}
pub fn save_game(root: &Path, p: &Path) -> Result<(), String> {
    if !p.is_absolute() || !has_game(p) { return Err(format!("그 폴더에 {EXE} 가 없습니다")); }
    std::fs::write(root.join(NOTE), p.to_string_lossy().as_bytes()).map_err(|e| format!("저장 실패: {e}"))
}
/// 게임 폴더: ①BL_GAME ②game_path.txt ③스팀 레지스트리 + 모든 라이브러리 ④드라이브별 흔한 폴더
pub fn find_game(root: &Path) -> PathBuf {
    if let Some(p) = std::env::var_os("BL_GAME") { return PathBuf::from(p); }
    if let Some(p) = std::fs::read_to_string(root.join(NOTE)).ok().and_then(|t| clean_path(&t)) { return p; }
    let libs: Vec<PathBuf> = steam_roots().iter().flat_map(|s| libraries(s)).collect();
    if let Some(p) = libs.iter().map(|l| in_library(l)).find(|p| has_game(p)) { return p; }
    if std::env::var_os("BL_STEAM").is_none() { if let Some(p) = drive_guess().iter().flat_map(|l| libraries(l)).map(|l| in_library(&l)).find(|p| has_game(p)) { return p; } }
    libs.first().map(|l| in_library(l)).unwrap_or_else(|| PathBuf::from("C:/Program Files (x86)/Steam/steamapps/common").join(DIR_NAME))
}

/// 지금 쓸 수 있는 상태인가(문제 목록 — 비면 정상)
pub fn problems(root: &Path, game: &Path, worker: &Path) -> Vec<String> {
    let mut v = Vec::new();
    if !has_game(game) {
        v.push(format!("게임을 찾지 못했습니다({}). 아래 칸에 게임 폴더 경로를 적어 주세요(또는 이 프로그램 폴더의 「{NOTE}」에 한 줄로).", game.display()));
    } else {
        let n = std::fs::metadata(game.join(EXE)).map(|m| m.len()).unwrap_or(0);
        if n != EXE_SIZE { v.push(format!("이 프로그램은 게임 {GAME_VER} 전용입니다. 설치된 게임의 버전이 다릅니다(exe 크기 {n}, 필요한 크기 {EXE_SIZE}).")); }
    }
    if !worker.is_file() { v.push("계산기(lab_worker.exe)가 없습니다. 백신이 지웠는지 확인하고 압축을 다시 풀어 주세요.".into()); }
    if !root.join("assets").is_dir() { v.push("그림 준비가 안 됐습니다(프로그램 폴더에 쓸 수 있는지 확인해 주세요).".into()); }
    v
}

fn u32le(f: &mut File) -> std::io::Result<usize> { let mut b = [0u8; 4]; f.read_exact(&mut b)?; Ok(u32::from_le_bytes(b) as usize) }
fn text(f: &mut File) -> std::io::Result<String> { let n = u32le(f)?; if n > 4096 { return Err(std::io::Error::other("번들 형식이 다릅니다")); } let mut b = vec![0u8; n]; f.read_exact(&mut b)?; Ok(String::from_utf8_lossy(&b).into_owned()) }

/// 번들 항목 → 우리 자산 폴더 안의 상대 경로(필요 없는 것은 None)
fn target(kind: &str, key: &str) -> Option<String> {
    let k = key.strip_prefix("asset/base/")?;
    if k.contains("..") || k.contains(':') || k.contains(char::from(92)) { return None; }
    if let Some(s) = k.strip_prefix("setting/") {
        let ok = matches!((kind, s), ("game_setting", "game_setting") | ("item_setting", "item_setting") | ("champion_info_sheet", "champion_info")) || kind == "data_champion" && !s.contains('/');
        return ok.then(|| format!("setting_src/setting__{s}.{kind}"));
    }
    if kind != "png" && kind != "fanim" { return None; }
    let r = k.strip_prefix("aseprite_resources/").unwrap_or(k);
    let top = r.split('/').next().unwrap_or("");
    let ok = matches!(top, "champions" | "skill_effect" | "ingame") || (top == "sprite" && r.matches('/').count() == 1) || r.starts_with("UI_aseprite/minion");
    ok.then(|| format!("{r}.{kind}"))
}

/// 게임 번들에서 화면에 쓰는 것만 꺼낸다. 이미 같은 번들로 꺼냈으면 건너뛴다.
pub fn prepare(root: &Path, game: &Path, tables: &[(&str, &[u8])]) -> Result<Option<usize>, String> {
    let a = root.join("assets");
    let stamp = a.join(".bundle");
    if a.join("art.json").exists() && !stamp.exists() { return Ok(None); } // 개발 폴더(직접 만든 자산)
    let bp = game.join("bundle.game_data");
    let m = std::fs::metadata(&bp).map_err(|_| format!("게임 자료 파일이 없습니다: {}", bp.display()))?;
    let t = m.modified().ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_secs()).unwrap_or(0);
    let want = format!("{} {} {}", m.len(), t, env!("CARGO_PKG_VERSION"));
    if std::fs::read_to_string(&stamp).map_or(false, |s| s == want) { return Ok(None); }
    println!("처음 준비 중: 설치된 게임에서 화면용 그림을 꺼냅니다… / First run: extracting graphics from the installed game…");
    let e = |x: std::io::Error| format!("그림 준비 실패: {x}");
    let mut f = File::open(&bp).map_err(e)?;
    let n = u32le(&mut f).map_err(e)?;
    if n > 1_000_000 { return Err("번들 형식이 다릅니다".into()); }
    let mut done = 0usize;
    for _ in 0..n {
        let (kind, key) = (text(&mut f).map_err(e)?, text(&mut f).map_err(e)?);
        let len = u32le(&mut f).map_err(e)?;
        match target(&kind, &key) {
            Some(rel) if len < 64_000_000 => {
                let mut b = vec![0u8; len];
                f.read_exact(&mut b).map_err(e)?;
                let p = a.join(rel);
                if let Some(d) = p.parent() { std::fs::create_dir_all(d).map_err(e)?; }
                std::fs::write(&p, &b).map_err(e)?;
                done += 1;
            }
            _ => { f.seek(SeekFrom::Current(len as i64)).map_err(e)?; }
        }
    }
    if done == 0 { return Err("번들에서 그림을 찾지 못했습니다".into()); }
    std::fs::create_dir_all(&a).map_err(e)?;
    for (n, b) in tables { std::fs::write(a.join(n), b).map_err(e)?; }
    std::fs::write(&stamp, want).map_err(e)?;
    Ok(Some(done))
}

/// 경기 기록(runs/<16진수>)을 최근 keep 개만 남긴다. 이 프로그램이 만든 파일 이름만 지운다.
pub fn prune_runs(root: &Path, keep: usize) {
    let Ok(rd) = std::fs::read_dir(root.join("runs")) else { return };
    let mut v: Vec<PathBuf> = rd.flatten().filter(|e| e.file_type().map_or(false, |t| t.is_dir())).map(|e| e.path())
        .filter(|p| p.file_name().and_then(|n| n.to_str()).map_or(false, |n| n.len() >= 10 && n.chars().all(|c| c.is_ascii_hexdigit()))).collect();
    v.sort();
    let cut = v.len().saturating_sub(keep);
    for d in &v[..cut] {
        for n in ["run.ndjson", "frames.bin", "scenario.txt", "progress.txt"] { let _ = std::fs::remove_file(d.join(n)); }
        let _ = std::fs::remove_dir(d); // 다른 파일이 있으면 남는다
    }
}
