//! 프레임 공급: 게임(공유 메모리) 또는 저장 폴더. 텍스처 파일은 별도 스레드가 읽는다.
use super::frame::{parse, Frame};
use anyhow::{bail, Context, Result};
use image::RgbaImage;
use std::{collections::HashMap, path::{Path, PathBuf}, sync::{atomic::{AtomicU64, Ordering}, mpsc, Arc}, time::{Duration, Instant}};
use windows::{core::w, Win32::{Foundation::*, System::Memory::*}};

const SLOT: usize = 8 << 20;
const SLOTS: usize = 3;
const MAP: usize = 4096 + SLOT * SLOTS;

pub struct Texture { pub key: String, pub unique: u32, pub image: Arc<RgbaImage> }
enum Source { Game { handle: HANDLE, view: usize, last: u64 }, Folder { files: Vec<PathBuf>, at: usize, next: Instant, cache: HashMap<usize, Frame> } }
pub struct Feed { source: Source, textures: mpsc::Receiver<(u32, Texture)>, scratch: Vec<u8>, pub last_frame: Option<Instant> }

fn decode(path: &Path) -> Result<(u32, Texture)> {
    let b = std::fs::read(path)?;
    if b.len() < 36 || &b[..4] != b"TVTX" { bail!("텍스처 형식 오류"); }
    let v = |i: usize| u32::from_le_bytes(b[4 + i * 4..8 + i * 4].try_into().unwrap());
    let (id, unique, w, h, kind) = (v(1), v(2), v(3), v(4), v(5));
    let n = u32::from_le_bytes(b[32..36].try_into().unwrap()) as usize;
    let key = String::from_utf8_lossy(b.get(36..36 + n).context("키")?).into_owned();
    let body = &b[36 + n..];
    let image = if kind == 0 { image::load_from_memory(body)?.to_rgba8() } else {
        // 프리멀티 BGRA 를 일반 RGBA 로 되돌린다.
        let mut im = RgbaImage::from_raw(w, h, body.to_vec()).context("픽셀 크기")?;
        for p in im.pixels_mut() { if kind == 6 { p.0.swap(0, 2); } let a = p[3] as u32; if a > 0 && a < 255 { for k in 0..3 { p[k] = (p[k] as u32 * 255 / a).min(255) as u8; } } }
        im
    };
    Ok((id, Texture { key, unique, image: Arc::new(image) }))
}
fn watch(folder: PathBuf) -> mpsc::Receiver<(u32, Texture)> {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let mut seen: HashMap<PathBuf, std::time::SystemTime> = HashMap::new();
        loop {
            if let Ok(dir) = std::fs::read_dir(&folder) {
                for e in dir.flatten() {
                    let p = e.path();
                    if p.extension().is_none_or(|x| x != "bin") || !e.file_name().to_string_lossy().starts_with("tex_") { continue; }
                    let Ok(t) = e.metadata().and_then(|m| m.modified()) else { continue };
                    if seen.get(&p) == Some(&t) { continue; }
                    match decode(&p) { Ok(v) => { seen.insert(p, t); if tx.send(v).is_err() { return; } } Err(_) => {} }
                }
            }
            std::thread::sleep(Duration::from_millis(30));
        }
    });
    rx
}
pub fn live_folder() -> PathBuf { std::env::var_os("LOCALAPPDATA").map(PathBuf::from).unwrap_or_default().join("tfm2_vr").join("live") }

impl Feed {
    pub fn game() -> Result<Self> {
        unsafe {
            let handle = CreateFileMappingW(INVALID_HANDLE_VALUE, None, PAGE_READWRITE, 0, MAP as u32, w!("Local\\tfm2_vr_frame_v2"))?;
            let view = MapViewOfFile(handle, FILE_MAP_ALL_ACCESS, 0, 0, MAP);
            if view.Value.is_null() { bail!("공유 메모리 연결 실패"); }
            let view = view.Value as usize;
            let last = (*((view + 8) as *const AtomicU64)).load(Ordering::Acquire);
            Ok(Self { source: Source::Game { handle, view, last }, textures: watch(live_folder()), scratch: Vec::new(), last_frame: None })
        }
    }
    pub fn folder(path: &Path) -> Result<Self> {
        let mut files: Vec<_> = std::fs::read_dir(path)?.flatten().map(|e| e.path()).filter(|p| p.file_name().is_some_and(|n| n.to_string_lossy().starts_with("frame_"))).collect();
        files.sort();
        if files.is_empty() { bail!("프레임 파일 없음: {}", path.display()); }
        Ok(Self { source: Source::Folder { files, at: 0, next: Instant::now(), cache: HashMap::new() }, textures: watch(path.to_path_buf()), scratch: Vec::new(), last_frame: None })
    }
    pub fn poll_textures(&mut self) -> Vec<(u32, Texture)> { self.textures.try_iter().take(8).collect() }
    /// 새 프레임이 있으면 하나. 게임이 쓰는 도중이면 다음에 다시 본다.
    pub fn poll(&mut self) -> Option<Frame> {
        let frame = match &mut self.source {
            Source::Game { view, last, .. } => unsafe {
                let seq = (*((*view + 8) as *const AtomicU64)).load(Ordering::Acquire);
                if seq == *last || seq == 0 { return None; }
                let slot = *view + 4096 + (seq as usize % SLOTS) * SLOT;
                let mark = &*(slot as *const AtomicU64);
                if mark.load(Ordering::Acquire) != seq { return None; }
                let n = *((slot + 8) as *const u64) as usize;
                if n > SLOT - 64 { return None; }
                self.scratch.clear();
                self.scratch.extend_from_slice(std::slice::from_raw_parts((slot + 64) as *const u8, n));
                if mark.load(Ordering::Acquire) != seq { return None; }
                *last = seq;
                parse(&self.scratch).ok()
            },
            Source::Folder { files, at, next, cache } => {
                if Instant::now() < *next { return None; }
                *next = Instant::now() + Duration::from_micros(16_667);
                let i = *at % files.len();
                *at += 1;
                if !cache.contains_key(&i) { let f = parse(&std::fs::read(&files[i]).ok()?).ok()?; cache.insert(i, f); }
                cache.get(&i).cloned()
            }
        };
        if frame.is_some() { self.last_frame = Some(Instant::now()); }
        frame
    }
}
impl Drop for Feed { fn drop(&mut self) { if let Source::Game { handle, view, .. } = &self.source { unsafe { let _ = UnmapViewOfFile(MEMORY_MAPPED_VIEW_ADDRESS { Value: *view as *mut _ }); let _ = CloseHandle(*handle); } } } }
