//! VR 안 안내 글자와 검은 창 안내의 언어(한국어·영어).
//! 정하는 순서: 실행 인자 `--lang ko|en` → viewer.toml 의 `lang = ko|en` → 게임 설정 `config/game/base.json` 의 lang → 스팀 언어 → 윈도우 표시 언어.
//! 한국어가 아니면 전부 영어로 보인다. 게임 화면 글자는 게임이 그리므로 여기와 상관없다.
use std::{path::PathBuf, sync::atomic::{AtomicBool, Ordering::Relaxed}};
use windows::{core::PCWSTR, Win32::{Globalization::GetUserDefaultUILanguage, System::Registry::*}};

static EN: AtomicBool = AtomicBool::new(false);
pub fn en() -> bool { EN.load(Relaxed) }
/// 한국어 원문을 주면 지금 언어의 글자를 돌려준다. 영어가 없는 글자는 원문 그대로.
pub fn t(ko: &'static str) -> &'static str { if en() { english(ko).unwrap_or(ko) } else { ko } }
fn english(ko: &str) -> Option<&'static str> {
    Some(match ko {
        "팀파이트 매니저 2" => "Teamfight Manager 2",
        "게임을 찾는 중" => "Looking for the game",
        "게임 화면 연결 중" => "Connecting to the game screen",
        "게임이 켜지기를 기다리는 중" => "Waiting for the game to start",
        "A / X 로 다시 가운데에 놓기" => "Press A / X to re-center",
        "불러오는 중" => "Loading",
        "휜 화면으로" => "Curved",
        "평평하게" => "Flat",
        "가운데로" => "Re-center",
        "자판" => "Keyboard",
        "지우기" => "Back",
        "한/영" => "IME",
        "띄어쓰기" => "Space",
        "확인" => "Enter",
        "닫기" => "Close",
        "VR 조작" => "VR controls",
        "누르기·잡기" => "Press / grab",
        "테이블 잡기" => "Grab table",
        "처음 배치" => "Re-center",
        "스크롤·테이블 조절" => "Scroll / adjust table",
        "손 위치" => "Hand pose",
        "가리키기" => "Aim",
        "진동" => "Haptics",
        "게임을 켭니다" => "Starting the game",
        "게임이 꺼져 VR 도 끝냅니다" => "The game closed, so VR is closing too",
        "VR 연결이 끊겨 다시 준비합니다" => "VR connection lost, reconnecting",
        "VR 정상 종료" => "VR closed",
        "openxr_loader.dll 이 없습니다(프로그램 폴더 또는 SteamVR)" => "openxr_loader.dll was not found (program folder or SteamVR)",
        "지금 켜진 VR 런타임에 D3D11 지원이 없습니다" => "The active OpenXR runtime does not support D3D11",
        "TFM2 VR 이 실행 중입니다.\n이 창이나 게임을 닫으면 VR 도 끝납니다." => "TFM2 VR is running.\nClose this window or the game to quit VR.",
        _ => return None,
    })
}

fn wide(s: &str) -> Vec<u16> { s.encode_utf16().chain([0]).collect() }
fn reg(root: HKEY, key: &str, value: &str) -> Option<String> {
    let (k, v) = (wide(key), wide(value));
    let mut buf = vec![0u16; 1024];
    let mut size = (buf.len() * 2) as u32;
    let r = unsafe { RegGetValueW(root, PCWSTR(k.as_ptr()), PCWSTR(v.as_ptr()), RRF_RT_REG_SZ, None, Some(buf.as_mut_ptr().cast()), Some(&mut size)) };
    if r.is_err() { return None; }
    let n = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    Some(String::from_utf16_lossy(&buf[..n])).filter(|s| !s.is_empty())
}
/// 게임 설치 폴더 후보. 창작마당 폴더(steamapps/workshop/...) 안에서 돌면 같은 steamapps 의 common 을 먼저 본다.
fn game_dirs() -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Ok(exe) = std::env::current_exe() { for a in exe.ancestors() { if a.file_name().is_some_and(|n| n.eq_ignore_ascii_case("steamapps")) { out.push(a.join("common/Teamfight Manager2")); } } }
    for k in ["SOFTWARE\\WOW6432Node\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\Steam App 3009300", "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\Steam App 3009300"] {
        if let Some(p) = reg(HKEY_LOCAL_MACHINE, k, "InstallLocation") { out.push(p.into()); }
    }
    if let Some(steam) = reg(HKEY_CURRENT_USER, "Software\\Valve\\Steam", "SteamPath") {
        let steam = PathBuf::from(steam);
        out.push(steam.join("steamapps/common/Teamfight Manager2"));
        if let Ok(vdf) = std::fs::read_to_string(steam.join("steamapps/libraryfolders.vdf")) {
            for line in vdf.lines() {
                let parts: Vec<&str> = line.split('"').collect();
                if parts.len() >= 4 && parts[1] == "path" { out.push(PathBuf::from(parts[3].replace("\\\\", "\\")).join("steamapps/common/Teamfight Manager2")); }
            }
        }
    }
    out.push("C:/Program Files (x86)/Steam/steamapps/common/Teamfight Manager2".into());
    out
}
fn game_lang() -> Option<(String, PathBuf)> {
    for d in game_dirs() {
        let p = d.join("config/game/base.json");
        if let Some(l) = std::fs::read_to_string(&p).ok().and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok()).and_then(|j| j["lang"].as_str().map(str::to_owned)) { return Some((l, p)); }
    }
    None
}
fn forced() -> Option<bool> {
    let args: Vec<String> = std::env::args().collect();
    if let Some(v) = args.iter().position(|a| a == "--lang").and_then(|i| args.get(i + 1)) { return match v.as_str() { "ko" => Some(false), "en" => Some(true), _ => None }; }
    let text = std::fs::read_to_string(crate::log::root().join("viewer.toml")).ok()?;
    for line in text.lines() {
        let line = line.split('#').next().unwrap_or("").trim();
        if let Some((k, v)) = line.split_once('=') { if k.trim() == "lang" { return match v.trim() { "ko" => Some(false), "en" => Some(true), _ => None }; } }
    }
    None
}
/// 실행 처음에 한 번 부른다. 고른 언어와 근거를 기록에 남긴다.
pub fn init(log: &crate::log::Log) {
    let (en, why) = if let Some(e) = forced() { (e, "--lang 또는 viewer.toml".to_string()) }
        else if let Some((l, p)) = game_lang() { (l != "ko", format!("게임 설정 lang={l} {}", p.display())) }
        else if let Some(l) = reg(HKEY_CURRENT_USER, "Software\\Valve\\Steam", "Language") { (l != "koreana", format!("스팀 언어 {l}")) }
        else { let id = unsafe { GetUserDefaultUILanguage() }; (id & 0x3ff != 0x12, format!("윈도우 표시 언어 0x{id:04x}")) };
    EN.store(en, Relaxed);
    log.line(format!("안내 언어 {} ({why})", if en { "en" } else { "ko" }));
}
#[cfg(test)] mod tests {
    use super::*;
    #[test] fn every_play_string_has_english_without_hangul() {
        for ko in ["팀파이트 매니저 2", "게임을 찾는 중", "게임 화면 연결 중", "게임이 켜지기를 기다리는 중", "A / X 로 다시 가운데에 놓기", "불러오는 중", "휜 화면으로", "평평하게", "가운데로", "자판", "지우기", "한/영", "띄어쓰기", "확인", "닫기", "VR 조작", "누르기·잡기", "테이블 잡기", "처음 배치", "스크롤·테이블 조절", "손 위치", "가리키기", "진동", "게임을 켭니다", "게임이 꺼져 VR 도 끝냅니다", "VR 연결이 끊겨 다시 준비합니다", "VR 정상 종료", "openxr_loader.dll 이 없습니다(프로그램 폴더 또는 SteamVR)", "지금 켜진 VR 런타임에 D3D11 지원이 없습니다", "TFM2 VR 이 실행 중입니다.\n이 창이나 게임을 닫으면 VR 도 끝납니다."] {
            let e = english(ko).unwrap_or_else(|| panic!("영어 없음: {ko}"));
            assert!(e.is_ascii(), "{e}");
        }
    }
}
