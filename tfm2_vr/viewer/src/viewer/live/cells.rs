//! 동적 텍스처 칸: (텍스처, 잘라낼 영역) 하나가 슬라이스 하나. 밉맵은 작업 스레드가 만든다.
use crate::{gpu::Gpu, mips};
use anyhow::Result;
use image::{GenericImageView, RgbaImage};
use std::{collections::HashMap, sync::{mpsc, Arc, Mutex}};

pub const SLICES: [u32; 3] = [1024, 384, 16];
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct CellKey { pub tex: u32, pub unique: u32, pub rect: [u32; 4] }
#[derive(Clone, Copy, Debug)]
pub struct Region { pub uv: [f32; 4], pub page: u32, pub w: u32, pub h: u32 }
struct Slot { region: Region, ready: bool, used: u64 }
type Job = (CellKey, Arc<RgbaImage>, usize);
pub struct Cells { map: HashMap<CellKey, Slot>, free: [Vec<u32>; 3], tx: mpsc::Sender<Job>, rx: mpsc::Receiver<(CellKey, usize, Vec<RgbaImage>)>, tick: u64, pub pending: usize, pub evicted: u64 }

fn tile(source: &RgbaImage, rect: [u32; 4], size: u32) -> RgbaImage {
    let [x, y, w, h] = rect;
    let part = source.view(x, y, w, h);
    let mut tile = RgbaImage::new(size, size);
    for ty in 0..(h + 2).min(size) { for tx in 0..(w + 2).min(size) { tile.put_pixel(tx, ty, part.get_pixel(tx.saturating_sub(1).min(w - 1), ty.saturating_sub(1).min(h - 1))); } }
    tile
}
impl Cells {
    pub fn new(gpu: &mut Gpu) -> Result<Self> {
        gpu.live_init(SLICES)?;
        let (tx, jobs) = mpsc::channel::<Job>();
        let (done, rx) = mpsc::channel();
        let jobs = Arc::new(Mutex::new(jobs));
        for _ in 0..3 {
            let (jobs, done) = (jobs.clone(), done.clone());
            std::thread::spawn(move || loop {
                let Ok((key, image, group)) = jobs.lock().unwrap().recv() else { return };
                let chain = mips::chain(&tile(&image, key.rect, mips::BUCKETS[group]));
                if done.send((key, group, chain)).is_err() { return; }
            });
        }
        Ok(Self { map: HashMap::new(), free: std::array::from_fn(|g| (0..SLICES[g]).rev().collect()), tx, rx, tick: 0, pending: 0, evicted: 0 })
    }
    /// 준비된 칸을 올린다. 프레임마다 한 번.
    pub fn pump(&mut self, gpu: &Gpu) {
        self.tick += 1;
        let mut bytes = 0usize;
        while bytes < 12 << 20 {
            let Ok((key, group, chain)) = self.rx.try_recv() else { break };
            self.pending -= 1;
            if let Some(s) = self.map.get_mut(&key) {
                bytes += chain[0].len();
                gpu.live_upload(group, s.region.page - group as u32 * mips::PAGE_STRIDE, &chain);
                s.ready = true;
            }
        }
    }
    /// 준비된 칸이면 돌려준다. 없으면 만들기를 요청하고 None.
    pub fn get(&mut self, key: CellKey, image: &Arc<RgbaImage>) -> Option<Region> {
        let tick = self.tick;
        if let Some(s) = self.map.get_mut(&key) { s.used = tick; return s.ready.then_some(s.region); }
        let [x, y, w, h] = key.rect;
        if w == 0 || h == 0 || x + w > image.width() || y + h > image.height() { return None; }
        let group = mips::BUCKETS.iter().position(|&s| w + 2 <= s && h + 2 <= s)?;
        if self.free[group].is_empty() {
            // 오래 안 쓴 칸 절반을 비운다.
            let mut old: Vec<_> = self.map.iter().filter(|(_, s)| s.ready && s.region.page / mips::PAGE_STRIDE == group as u32 && tick.saturating_sub(s.used) > 120).map(|(k, s)| (s.used, *k)).collect();
            old.sort_unstable_by_key(|v| v.0);
            let n = (old.len() / 2).max(1);
            for (_, k) in old.into_iter().take(n) { if let Some(s) = self.map.remove(&k) { self.free[group].push(s.region.page % mips::PAGE_STRIDE); self.evicted += 1; } }
        }
        let slice = self.free[group].pop()?;
        let size = mips::BUCKETS[group] as f32;
        let region = Region { uv: [1.0 / size, 1.0 / size, w as f32 / size, h as f32 / size], page: group as u32 * mips::PAGE_STRIDE + slice, w, h };
        self.map.insert(key, Slot { region, ready: false, used: tick });
        self.pending += 1;
        let _ = self.tx.send((key, image.clone(), group));
        None
    }
}
