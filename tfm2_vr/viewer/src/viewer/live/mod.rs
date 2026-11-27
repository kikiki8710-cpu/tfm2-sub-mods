//! 실시간 경기 장면. 게임이 만든 2D 그리기 명령을 테이블 위 3D 배치로 옮긴다.
//! 규칙: 층<0 = 바닥에 눕힘 · 0~2 = 세움(발밑 그림자 중심이 바닥 위치) · 3 이상 = 머리 위(같은 기준점).
pub mod cells;
pub mod feed;
pub mod frame;
pub mod terrain;
pub mod text;

use crate::{geometry::{self, rgb, Instance, Vertex}, gpu::Gpu, log::Log, scene::{PIXEL, TABLE}};
use anyhow::Result;
use cells::{CellKey, Cells, Region};
use feed::{Feed, Texture};
use frame::{Cmd, Frame, Kind};
use glam::{Mat4, Quat, Vec3};
use image::RgbaImage;
use std::{collections::{HashMap, HashSet}, sync::{mpsc, Arc}};

const CAPACITY: usize = 16384;
const FLAT: f32 = -std::f32::consts::FRAC_PI_2;
#[derive(Clone, Copy)]
struct Anchor { x: f32, y: f32, top: f32 }
#[derive(Clone)]
struct MapTexture(CellKey, Arc<RgbaImage>);
struct Ids { units: usize, ground: usize, over: usize, tri_ground: usize, tri_over: usize }
struct TextCell { image: Arc<RgbaImage>, id: u32 }
#[derive(Default)]
struct Batch { units: Vec<(f32, Instance)>, ground: Vec<(i32, Instance)>, over: Vec<(i32, Instance)>, tri_ground: Vec<Instance>, tri_over: Vec<Instance> }
pub struct Live {
    feed: Feed, cells: Cells, textures: HashMap<u32, Texture>, texts: HashMap<(String, bool), TextCell>, variants: HashMap<(CellKey, u64), (Arc<RgbaImage>, u32)>, averages: HashMap<CellKey, ([f32; 3], f32)>, frame: Option<Frame>, ids: Ids, batch: Batch, log: Log,
    pub hold: bool, map_key: Vec<(String, u32)>, terrain_rx: Option<mpsc::Receiver<(Vec<MapMesh>, Vec<(String, u32)>)>>, fog: Option<(Vec<u8>, Arc<RgbaImage>, u32)>, fog_unique: u32, map_ids: Vec<usize>, map_pending: Vec<MapMesh>, map_uses: Vec<(CellKey, Arc<RgbaImage>)>, unknown: HashSet<String>,
    pub stats: Stats,
}
#[derive(Clone, Copy, Default, Debug)]
pub struct Stats { pub frames: u64, pub commands: usize, pub sprites: usize, pub missing: usize, pub anchored: usize, pub free: usize, pub build_ms: f64, pub parse_ms: f64, pub upload_ms: f64, pub map_ms: f64 }
fn plane() -> Vec<Vertex> {
    let w = |x: f32, y: f32| [(x - 640.0) * PIXEL, TABLE, (y - 640.0) * PIXEL];
    let p = [(w(0.0, 96.0), [0.0, 0.075]), (w(0.0, 1184.0), [0.0, 0.925]), (w(1280.0, 1184.0), [1.0, 0.925]), (w(1280.0, 96.0), [1.0, 0.075])];
    [0, 1, 2, 0, 2, 3].iter().map(|&i| Vertex { position: p[i].0, uv: p[i].1, color: [1.0; 4] }).collect()
}
fn triangle(v: [Vec3; 3], col: [f32; 4]) -> Instance {
    let (a, b) = (v[1] - v[0], v[2] - v[0]);
    let n = a.cross(b).normalize_or_zero();
    Instance::new(Mat4::from_cols(a.extend(0.0), b.extend(0.0), n.extend(0.0), v[0].extend(1.0)), [0.0; 4], col, [0.0; 4])
}
struct MapMesh { name: &'static str, vertices: Vec<Vertex>, texture: Option<MapTexture>, height: f32, blend: bool, opacity: f32 }

fn ground(x: f32, y: f32, h: f32) -> Vec3 { Vec3::new((x - 480.0) * PIXEL, TABLE + h, (y - 480.0) * PIXEL) }
struct Eye { head: Vec3, rotation: Quat, pitch_factor: f32, pitch_max: f32 }
impl Eye {
    /// 머리 위 요소의 배치: 개체 머리 위 한 점을 화면과 나란한 평면으로 펼친다. 발밑에 그려진 것은 머리 위로 올린다.
    fn over(&self, a: Anchor, x: f32, y: f32, front: f32) -> Mat4 {
        let foot = ground(a.x, a.y, 0.003);
        let (yaw, lean, side) = self.facing(foot);
        let up = a.y - y;
        let up = if up < 4.0 { up + a.top + 18.0 } else { up };
        let post = foot + (yaw * lean) * Vec3::new(0.0, (a.top + 8.0) * PIXEL, 0.0);
        let p = post + self.rotation * Vec3::new((x - a.x) * side * PIXEL, (up - a.top - 8.0) * PIXEL, front);
        Mat4::from_rotation_translation(self.rotation, p)
    }
    /// 기준점에서 보는 이를 향한 (회전, 좌우 부호). 북쪽에서 보면 좌우를 뒤집는다.
    fn facing(&self, foot: Vec3) -> (Quat, Quat, f32) {
        let d = self.head - foot;
        let flat = d.x.hypot(d.z);
        let yaw = if flat > 0.0001 { d.x.atan2(d.z) } else { 0.0 };
        let pitch = (self.pitch_factor * d.y.max(0.0).atan2(flat)).min(self.pitch_max);
        (Quat::from_rotation_y(yaw), Quat::from_rotation_x(-pitch), if yaw.cos() < 0.0 { -1.0 } else { 1.0 })
    }
}
fn color(c: [f32; 4]) -> [f32; 4] { rgb([c[0], c[1], c[2]], c[3]) }
fn big(key: &str, w: f32) -> bool { w >= 900.0 && ["background", "wall", "bush"].iter().any(|n| key.contains(n)) }

impl Live {
    pub fn new(gpu: &mut Gpu, feed: Feed, log: &Log) -> Result<Self> {
        let cells = Cells::new(gpu)?;
        let mut add = |name: &str, vertices: Vec<Vertex>, blend: bool, write: bool| -> Result<usize> { let mut m = geometry::mesh(name, vertices, vec![], blend, write); m.dynamic = true; gpu.add_mesh(&m, CAPACITY) };
        let triangle = || [([0.0, 0.0, 0.0], [0.0, 0.0]), ([1.0, 0.0, 0.0], [1.0, 0.0]), ([0.0, 1.0, 0.0], [0.0, 1.0])].iter().map(|&(p, uv)| Vertex { position: p, uv, color: [1.0; 4] }).collect::<Vec<_>>();
        let ids = Ids { units: add("live0units", geometry::quad(), false, true)?, ground: add("live3ground", geometry::quad(), true, false)?, tri_ground: add("live4triGround", triangle(), true, false)?, over: add("live8over", geometry::quad(), true, false)?, tri_over: add("live7triOver", triangle(), true, false)? };
        Ok(Self { feed, cells, textures: HashMap::new(), texts: HashMap::new(), variants: HashMap::new(), averages: HashMap::new(), frame: None, ids, batch: Batch::default(), log: log.clone(), hold: false, map_key: Vec::new(), terrain_rx: None, fog: None, fog_unique: 0, map_ids: Vec::new(), map_pending: Vec::new(), map_uses: Vec::new(), unknown: HashSet::new(), stats: Stats::default() })
    }
    /// 마지막으로 받은 장면이 경기 화면인가. 게임이 불러오느라 멈춘 동안에도 그대로 둔다(배치가 왔다 갔다 하지 않게).
    pub fn in_match(&self) -> bool { self.frame.as_ref().is_some_and(|f| f.list("Game").iter().any(|c| c.kind == Kind::Sprite && c.key.contains("background"))) }
    pub fn take(&mut self) -> Option<Frame> { self.frame.take() }
    pub fn put(&mut self, frame: Frame) { self.frame = Some(frame); }
    pub fn sprite_size(&self, c: &Cmd) -> Option<[f32; 2]> { self.rect(c).map(|(_, r)| [r[2] as f32, r[3] as f32]) }
    pub fn text(&mut self, text: &str) -> Option<(Region, (f32, f32))> { self.text_cell(text, true) }
    /// UI 그림 명령 하나의 칸과 색.
    pub fn cell(&mut self, c: &Cmd) -> Option<(Region, [f32; 4])> { let (t, _) = Self::tint(c)?; let r = self.region(c)?; Some((r, color(t))) }
    pub fn current(&self) -> Option<&Frame> { self.frame.as_ref() }
    pub fn pending(&self) -> usize { self.cells.pending }
    /// 장면이 끊긴 지 몇 초인가.
    pub fn silence(&self) -> f32 { self.feed.last_frame.map(|t| t.elapsed().as_secs_f32()).unwrap_or(f32::MAX) }
    pub fn alive(&self) -> bool { self.feed.last_frame.is_some_and(|t| t.elapsed().as_secs_f32() < 1.0) }

    pub fn update(&mut self, gpu: &mut Gpu, head: Vec3, rotation: Quat, pitch_factor: f32, pitch_max: f32) -> Result<()> {
        let start = std::time::Instant::now();
        for (id, t) in self.feed.poll_textures() { self.textures.insert(id, t); }
        let t = std::time::Instant::now();
        if !self.hold || self.stats.frames == 0 { while let Some(f) = self.feed.poll() { self.frame = Some(f); self.stats.frames += 1; if matches!(self.stats.frames, 1) { self.log.line("실시간 첫 프레임 수신"); } break; } }
        self.stats.parse_ms = t.elapsed().as_secs_f64() * 1000.0;
        let t = std::time::Instant::now();
        self.cells.pump(gpu);
        self.stats.upload_ms = t.elapsed().as_secs_f64() * 1000.0;
        let t = std::time::Instant::now();
        self.install_map(gpu)?;
        self.stats.map_ms = t.elapsed().as_secs_f64() * 1000.0;
        let Some(frame) = self.frame.take() else { return Ok(()) };
        self.build(gpu, &frame, &Eye { head, rotation, pitch_factor, pitch_max })?;
        self.frame = Some(frame);
        self.stats.build_ms = start.elapsed().as_secs_f64() * 1000.0;
        Ok(())
    }

    fn rect(&self, c: &Cmd) -> Option<(&Texture, [u32; 4])> {
        let t = self.textures.get(&c.tex)?;
        let o = if c.kind == Kind::Sprite { 0x68 } else { 104 };
        let (w, h) = (t.image.width() as f32, t.image.height() as f32);
        let (x, y) = ((c.f(o) * w).round().max(0.0) as u32, (c.f(o + 4) * h).round().max(0.0) as u32);
        // 칸이 그림 왼쪽·위 밖에서 시작하면 안쪽만.
        let (cw, ch) = (((c.f(o + 8) + c.f(o).min(0.0)) * w).round().max(0.0) as u32, ((c.f(o + 12) + c.f(o + 4).min(0.0)) * h).round().max(0.0) as u32);
        let (x, y) = (x.min(t.image.width() - 1), y.min(t.image.height() - 1));
        Some((t, [x, y, cw.clamp(1, t.image.width() - x), ch.clamp(1, t.image.height() - y)]))
    }
    /// 그림 한 칸의 평균 색(선형)과 채워진 넓이(픽셀).
    fn average(&mut self, c: &Cmd) -> Option<([f32; 3], f32)> {
        let (t, rect) = self.rect(c)?;
        let (key, image) = (CellKey { tex: c.tex, unique: t.unique, rect }, t.image.clone());
        if let Some(v) = self.averages.get(&key) { return Some(*v); }
        let (mut sum, mut n) = ([0.0f32; 3], 0.0f32);
        for y in rect[1]..rect[1] + rect[3] { for x in rect[0]..rect[0] + rect[2] { let p = image.get_pixel(x, y); let a = p[3] as f32 / 255.0; n += a; for k in 0..3 { sum[k] += (p[k] as f32 / 255.0).powf(2.2) * a; } } }
        let v = (sum.map(|s| s / n.max(1.0)), n);
        if self.averages.len() > 4000 { self.averages.clear(); }
        self.averages.insert(key, v);
        Some(v)
    }
    fn region(&mut self, c: &Cmd) -> Option<Region> {
        let (t, rect) = self.rect(c)?;
        let t_unique = t.unique;
        let (key, image) = (CellKey { tex: c.tex, unique: t.unique, rect }, t.image.clone());
        if c.shader.ends_with("/color_replace") || c.shader.ends_with("/greyscale") {
            // 색 바꾸기(선수 머리·옷)·흑백은 잘라 낸 그림에 미리 적용해 둔다.
            use std::hash::{Hash, Hasher};
            let table = c.style.iter().find(|s| s.0 == "convert").map(|s| s.1.clone()).unwrap_or_default();
            let mut h = std::collections::hash_map::DefaultHasher::new();
            (c.shader.ends_with("/greyscale"), &table).hash(&mut h);
            let id = (key, h.finish());
            if !self.variants.contains_key(&id) {
                if self.variants.len() > 1500 { self.variants.clear(); }
                let mut im = image::imageops::crop_imm(&*image, rect[0], rect[1], rect[2], rect[3]).to_image();
                if c.shader.ends_with("/greyscale") { for p in im.pixels_mut() { let v = (p[0] as f32 * 0.299 + p[1] as f32 * 0.587 + p[2] as f32 * 0.114) as u8; p.0 = [v, v, v, p[3]]; } }
                else if table.len() > 16 {
                    let f: Vec<f32> = table[16..].chunks_exact(4).map(|v| f32::from_le_bytes(v.try_into().unwrap())).collect();
                    let map: Vec<([u8; 3], [u8; 3])> = f.chunks_exact(8).map(|v| (std::array::from_fn(|k| (v[k] * 255.0).round() as u8), std::array::from_fn(|k| (v[k + 4] * 255.0).round() as u8))).collect();
                    for p in im.pixels_mut() { if let Some(m) = map.iter().find(|m| m.0 == [p[0], p[1], p[2]]) { p.0 = [m.1[0], m.1[1], m.1[2], p[3]]; } }
                }
                let serial = 0x1000_0000 + self.variants.len() as u32 + (self.stats.frames as u32 % 4096) * 2048;
                self.variants.insert(id, (Arc::new(im), serial));
            }
            let (image, serial) = self.variants[&id].clone();
            return self.cells.get(CellKey { tex: serial, unique: t_unique, rect: [0, 0, image.width(), image.height()] }, &image);
        }
        self.cells.get(key, &image)
    }
    /// (색, 번쩍임). 빛무리(glow)는 같은 그림을 흐리게 겹치는 것이라 None = 그리지 않음.
    fn tint(c: &Cmd) -> Option<([f32; 4], f32)> {
        let mut tint = [1.0f32; 4];
        let mut flash = 0.0;
        if c.shader.ends_with("/glow") { return None; }
        if let Some(v) = c.style_f("color") { if v.len() == 4 { tint = [v[0], v[1], v[2], v[3]]; } }
        if let Some(v) = c.style_f("opacity") { if let Some(a) = v.first() { tint[3] *= a; } }
        if let Some(v) = c.style_f("flash") { flash = v.first().copied().unwrap_or(0.0).clamp(0.0, 1.0); }
        Some((tint, flash))
    }
    /// 명령의 화면 상자(가운데 x, y, 폭, 높이).
    fn bounds(&self, c: &Cmd) -> Option<[f32; 4]> {
        match c.kind {
            Kind::Sprite => {
                let (_, r) = self.rect(c)?;
                let (w, h) = (r[2] as f32, r[3] as f32);
                let (sx, sy) = (if c.raw[0x90] != 0 { -1.0 } else { 1.0 }, if c.raw[0x91] != 0 { -1.0 } else { 1.0 });
                let (ox, oy) = ((0.5 - c.f(0x88)) * w * sx, (0.5 - c.f(0x8c)) * h * sy);
                let (s, k) = c.f(0x84).sin_cos();
                Some([c.f(0x78) + ox * k - oy * s, c.f(0x7c) + ox * s + oy * k, w, h])
            }
            Kind::Patch => { let r = c.patch_rect(); Some([r[0] + r[2] * 0.5, r[1] + r[3] * 0.5, r[2], r[3]]) }
            Kind::Text => Some([c.f(152) + c.f(160) * 0.5, c.f(156) + c.f(164) * 0.5, c.f(160), c.f(164)]),
            Kind::Box => Some([c.f(88) + c.f(96) * 0.5, c.f(92) + c.f(100) * 0.5, c.f(96), c.f(100)]),
            Kind::Line => { let (a, b) = ([c.f(40), c.f(44)], [c.f(48), c.f(52)]); Some([(a[0] + b[0]) * 0.5, (a[1] + b[1]) * 0.5, (a[0] - b[0]).abs(), (a[1] - b[1]).abs()]) }
            Kind::Circle => Some([c.f(96), c.f(100), c.f(104) * 2.0, c.f(104) * 2.0]),
            _ => None,
        }
    }
    fn build(&mut self, gpu: &mut Gpu, frame: &Frame, eye: &Eye) -> Result<()> {
        let list = frame.list("Game");
        let mut b = std::mem::take(&mut self.batch);
        b.units.clear(); b.ground.clear(); b.over.clear(); b.tri_ground.clear(); b.tri_over.clear();
        self.stats.commands = list.len();
        (self.stats.sprites, self.stats.missing, self.stats.anchored, self.stats.free) = (0, 0, 0, 0);
        // 1) 바닥 기준점: 발밑 원 그림자 · 건물 그림자.
        let mut anchors: Vec<(usize, Anchor)> = Vec::new();
        for (i, c) in list.iter().enumerate() {
            let shadow = (c.kind == Kind::Patch && c.key.ends_with("sprite/circle") && c.layer() == -2) || (c.kind == Kind::Sprite && c.layer() < 0 && c.key.ends_with("_shadow") && !c.key.contains("wall") && !c.key.contains("bush"));
            if shadow { if let Some(r) = self.bounds(c) { anchors.push((i, Anchor { x: r[0], y: r[1], top: 24.0 })); } }
        }
        let mut map_now: Vec<(String, u32)> = Vec::new();
        let mut current: Option<Anchor> = None;
        // 이어지는 도형 조각을 한 덩어리로 묶는다(덩어리마다 기준점 하나).
        let mut runs: Vec<[f32; 4]> = Vec::new();
        let mut runs_of: Vec<usize> = vec![0; list.len()];
        let mut open = false;
        for (k, c) in list.iter().enumerate() {
            if c.kind != Kind::Path || c.layer() < 0 || c.data.len() < 24 { if c.kind != Kind::Path { open = false; } continue; }
            let mut q = [f32::INFINITY, f32::INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY];
            for v in c.data.chunks_exact(8) { let (x, y) = (f32::from_le_bytes(v[..4].try_into().unwrap()), f32::from_le_bytes(v[4..].try_into().unwrap())); q = [q[0].min(x), q[1].min(y), q[2].max(x), q[3].max(y)]; }
            let joined = open && runs.last().is_some_and(|r| q[0] < r[2] + 40.0 && q[2] > r[0] - 40.0 && q[1] < r[3] + 40.0 && q[3] > r[1] - 40.0 && r[2].max(q[2]) - r[0].min(q[0]) < 220.0);
            if joined { let r = runs.last_mut().unwrap(); *r = [r[0].min(q[0]), r[1].min(q[1]), r[2].max(q[2]), r[3].max(q[3])]; } else { runs.push(q); }
            open = true;
            runs_of[k] = runs.len() - 1;
        }
        let mut path_base: Option<(usize, Anchor, Mat4, f32)> = None;
        let mut i = 0;
        while i < list.len() {
            let c = &list[i];
            let layer = c.layer();
            i += 1;
            match c.kind {
                Kind::Sprite | Kind::Patch => {
                    if c.tex == 0 && c.key.is_empty() { continue; }
                    let Some(r) = self.bounds(c) else { self.stats.missing += 1; continue };
                    if big(&c.key, r[2]) { map_now.push((c.key.clone(), c.tex)); continue; }
                    if r[2] >= 900.0 { continue; }
                    // 빛 번짐(타워 구슬 등): 그림자처럼 번진 빛만 더해 그린다(원본도 그림 자체는 그리지 않는다).
                    let glow = c.shader.ends_with("/glow").then(|| { let f = |n: &str, d: f32| c.style_f(n).and_then(|v| v.first().copied()).filter(|v| v.is_finite()).unwrap_or(d); (f("sigma", 2.0).clamp(0.5, 80.0), f("intensity", 1.0).clamp(0.0, 8.0)) });
                    let Some((tint, flash)) = (if glow.is_some() { Some(([1.0; 4], 0.0)) } else { Self::tint(c) }) else { continue };
                    self.stats.sprites += 1;
                    let Some(cell) = self.region(c) else { self.stats.missing += 1; continue };
                    let params = |mode: f32| [mode, 0.15, flash, cell.page as f32];
                    let (fx, fy) = if c.kind == Kind::Sprite { (if c.raw[0x90] != 0 { -1.0 } else { 1.0 }, if c.raw[0x91] != 0 { -1.0 } else { 1.0 }) } else { (1.0, 1.0) };
                    let rot = if c.kind == Kind::Sprite { c.f(0x84) } else { 0.0 };
                    if layer < 0 {
                        let order = (layer.max(-9) + 10) * 10_000 + i as i32;
                        let m = Mat4::from_scale_rotation_translation(Vec3::new(r[2] * PIXEL * fx, r[3] * PIXEL * fy, 1.0), Quat::from_rotation_y(-rot) * Quat::from_rotation_x(FLAT), ground(r[0], r[1], 0.0024 + (layer + 10) as f32 * 0.00004));
                        b.ground.push((order, Instance::new(m, cell.uv, color(tint), params(11.0))));
                        continue;
                    }
                    let over = layer >= 3;
                    let found = if glow.is_some() && current.is_some_and(|a| (a.x - r[0]).abs() < 60.0 && (a.y - r[1]).abs() < 120.0) { current } else if over { self.near(current, &anchors, r) } else { Self::find(&anchors, i - 1, r) };
                    let (a, hover) = match found { Some(a) => { self.stats.anchored += 1; if !over && layer <= 1 { let top = if c.key.contains("/champions/") { 27.0 } else if c.key.contains("minion") { 14.0 } else { (a.y - r[1] + r[3] * 0.5).clamp(12.0, 130.0) }; if layer == 0 || current.is_none() { current = Some(Anchor { top, ..a }); } else if let Some(cur) = &mut current { if cur.x == a.x && cur.y == a.y && !c.key.contains("/champions/") { cur.top = cur.top.max(top); } } } (a, 0.0) } None => { self.stats.free += 1; if r[2].max(r[3]) <= 48.0 || rot != 0.0 { (Anchor { x: r[0], y: r[1], top: 24.0 }, 11.0) } else { (Anchor { x: r[0], y: r[1] + 11.5, top: 24.0 }, 0.0) } } };
                    let foot = ground(a.x, a.y, 0.003);
                    let (yaw, lean, side) = eye.facing(foot);
                    let (lx, ly) = ((r[0] - a.x) * side, a.y - r[1] + hover);
                    let depth = 0.0003 * layer.clamp(0, 9) as f32 + 0.000004 * (i % 400) as f32;
                    let base = Mat4::from_translation(foot) * Mat4::from_quat(yaw * lean);
                    let spin = if rot != 0.0 { Self::spin(rot, yaw, side) } else { 0.0 };
                    let m = base * Mat4::from_scale_rotation_translation(Vec3::new(r[2] * PIXEL * fx * if rot != 0.0 { 1.0 } else { side }, r[3] * PIXEL * fy, 1.0), Quat::from_rotation_z(spin), Vec3::new(lx * PIXEL, ly * PIXEL, depth + if over { 0.004 } else { 0.0 }));
                    if let Some((sigma, power)) = glow {
                        if let Some((avg, filled)) = self.average(c) {
                            let disc = (filled / std::f32::consts::PI).sqrt();
                            let peak = 0.42 * power * (1.0 - (-disc * disc / (2.0 * sigma * sigma)).exp());
                            let half = (r[2].max(r[3])) * 0.5 + 2.2 * sigma;
                            let spread = half * half / (2.0 * (sigma * sigma + disc * disc * 0.25));
                            let m = base * Mat4::from_scale_rotation_translation(Vec3::new(half * 2.0 * PIXEL, half * 2.0 * PIXEL, 1.0), Quat::IDENTITY, Vec3::new(lx * PIXEL, ly * PIXEL, depth + 0.004));
                            // 좁은 번짐일수록 밝게 타서 흰빛이 난다.
                            let white = (0.7 - sigma / 14.0).clamp(0.0, 0.5);
                            let glow = avg.map(|v| v + (1.0 - v) * white);
                            b.over.push((layer * 10_000 + i as i32, Instance::new(m, [0.0; 4], [glow[0], glow[1], glow[2], peak.min(1.2)], [17.0, 0.0, spread, 0.0])));
                        }
                        continue;
                    }
                    if over {
                        let m = eye.over(a, r[0], r[1], 0.004 + 0.0002 * (layer.clamp(0, 9) as f32) + 0.000004 * (i % 400) as f32) * Mat4::from_scale_rotation_translation(Vec3::new(r[2] * PIXEL * fx, r[3] * PIXEL * fy, 1.0), Quat::from_rotation_z(-rot), Vec3::ZERO);
                        b.over.push((layer * 10_000 + i as i32, Instance::new(m, cell.uv, color(tint), params(11.0))));
                        continue;
                    }
                    if tint[3] < 0.95 { b.over.push((layer * 10_000 + i as i32, Instance::new(m, cell.uv, color(tint), params(11.0)))); } else { b.units.push(((foot - eye.head).length(), Instance::new(m, cell.uv, color(tint), params(11.0)))); }
                    // 바닥선 아래는 눕혀서 보는 쪽으로 편다.
                    let low = r[3] * 0.5 - ly;
                    if !over && rot == 0.0 && fy > 0.0 && low > 1.5 && low < r[3] && c.kind == Kind::Sprite {
                        let d = low * PIXEL;
                        let cut = 1.0 - low / r[3];
                        let m = Mat4::from_translation(foot) * Mat4::from_quat(yaw) * Mat4::from_translation(Vec3::new(lx * PIXEL, -0.0002, d * 0.5)) * Mat4::from_quat(Quat::from_rotation_x(FLAT)) * Mat4::from_scale(Vec3::new(r[2] * PIXEL * fx * side, d, 1.0));
                        let inst = Instance::new(m, [cell.uv[0], cell.uv[1] + cell.uv[3] * cut, cell.uv[2], cell.uv[3] * (1.0 - cut)], color(tint), params(11.0));
                        b.ground.push((200_000 + i as i32, inst));
                    }
                }
                Kind::Text => {
                    let col = c.rgba(168);
                    let dark = col[0] + col[1] + col[2] < 0.15;
                    if dark && list[i..].iter().take(17).any(|n| n.kind == Kind::Text && n.text == c.text && n.f(168) + n.f(172) + n.f(176) >= 0.15) { continue; }
                    let outline = i >= 2 && list[i - 2].kind == Kind::Text && list[i - 2].text == c.text;
                    let Some(r) = self.bounds(c) else { continue };
                    let Some((cell, size)) = self.text_cell(&c.text, outline) else { continue };
                    let height = c.f(192) * c.f(196).clamp(0.1, 4.0) * 1.35;
                    let scale = height / size.1;
                    let a = self.near(current, &anchors, r).unwrap_or(Anchor { x: r[0], y: r[1] + 11.5, top: 24.0 });
                    let m = eye.over(a, r[0], r[1].min(a.y - 6.0), 0.014) * Mat4::from_scale(Vec3::new(size.0 * scale * PIXEL, size.1 * scale * PIXEL, 1.0));
                    b.over.push((900_000 + i as i32, Instance::new(m, cell.uv, color(col), [11.0, 0.15, 0.0, cell.page as f32])));
                }
                Kind::Line => {
                    let Some(r) = self.bounds(c) else { continue };
                    let (p0, p1) = ([c.f(40), c.f(44)], [c.f(48), c.f(52)]);
                    self.segment(&mut b, eye, current, &anchors, r, p0, p1, c.f(56), c.rgba(8), layer, i);
                }
                Kind::Points => {
                    // 점: x, y, RGBA (24바이트). 굵기 @44.
                    let pts: Vec<_> = c.data.chunks_exact(24).map(|v| { let f = |k: usize| f32::from_le_bytes(v[k * 4..k * 4 + 4].try_into().unwrap()); ([f(0), f(1)], [f(2), f(3), f(4), f(5)]) }).collect();
                    for w in pts.windows(2) { let r = [(w[0].0[0] + w[1].0[0]) * 0.5, (w[0].0[1] + w[1].0[1]) * 0.5, 0.0, 0.0]; self.segment(&mut b, eye, current, &anchors, r, w[0].0, w[1].0, c.f(44).clamp(0.5, 20.0), w[0].1, layer, i); }
                }
                Kind::Circle => {
                    let Some(r) = self.bounds(c) else { continue };
                    let m = Mat4::from_scale_rotation_translation(Vec3::new(r[2] * PIXEL, r[3] * PIXEL, 1.0), Quat::from_rotation_x(FLAT), ground(r[0], r[1], 0.0027));
                    b.ground.push((60_000 + i as i32, Instance::new(m, [0.0; 4], color(c.rgba(80)), [2.0, 0.0, 0.0, 0.0])));
                }
                Kind::Ring => {
                    let (x, y, outer) = (c.f(24), c.f(28), c.f(36).max(c.f(32)));
                    if !(0.5..2000.0).contains(&outer) { continue; }
                    let m = Mat4::from_scale_rotation_translation(Vec3::new(outer * 2.0 * PIXEL, outer * 2.0 * PIXEL, 1.0), Quat::from_rotation_x(FLAT), ground(x, y, 0.0028));
                    b.ground.push((61_000 + i as i32, Instance::new(m, [0.0; 4], color(c.rgba(8)), [5.0, 0.0, 0.0, 0.0])));
                }
                Kind::Path => {
                    let n = c.data.len() / 8;
                    if n < 3 { continue; }
                    let col = color(c.rgba(32));
                    let p = |k: usize| -> [f32; 2] { let v = &c.data[k * 8..k * 8 + 8]; [f32::from_le_bytes(v[..4].try_into().unwrap()), f32::from_le_bytes(v[4..].try_into().unwrap())] };
                    if layer < 0 {
                        for k in 1..n - 1 { let v = [p(0), p(k), p(k + 1)].map(|p| ground(p[0], p[1], 0.0026)); b.tri_ground.push(triangle(v, col)); }
                        continue;
                    }
                    if path_base.is_none_or(|v| v.0 != runs_of[i - 1]) {
                        let r = runs[runs_of[i - 1]];
                        let box_ = [(r[0] + r[2]) * 0.5, (r[1] + r[3]) * 0.5, r[2] - r[0], r[3] - r[1]];
                        let a = Self::find(&anchors, i - 1, box_).unwrap_or(Anchor { x: box_[0], y: r[3], top: 24.0 });
                        let foot = ground(a.x, a.y, 0.003);
                        let (yaw, lean, side) = eye.facing(foot);
                        path_base = Some((runs_of[i - 1], a, Mat4::from_translation(foot) * Mat4::from_quat(yaw * lean), side));
                    }
                    let Some((_, a, base, side)) = path_base else { continue };
                    for k in 1..n - 1 { let v = [p(0), p(k), p(k + 1)].map(|p| base.transform_point3(Vec3::new((p[0] - a.x) * side * PIXEL, (a.y - p[1]) * PIXEL, 0.001 + layer as f32 * 0.0002))); b.tri_over.push(triangle(v, col)); }
                }
                Kind::Mask => self.fog(&mut b, c),
                Kind::Box => {
                    if layer >= 100 { continue; }
                    let Some(r) = self.bounds(c) else { continue };
                    if r[2] >= 900.0 { continue; }
                    let a = self.near(current, &anchors, r).unwrap_or(Anchor { x: r[0], y: r[1] + 11.5, top: 24.0 });
                    let m = eye.over(a, r[0], r[1], 0.005) * Mat4::from_scale(Vec3::new(r[2] * PIXEL, r[3] * PIXEL, 1.0));
                    b.over.push((layer * 10_000 + i as i32, Instance::new(m, [0.0; 4], color(c.box_fill().0), [0.0, 0.0, 0.0, 0.0])));
                }
                Kind::Svg => { if self.unknown.insert(c.key.clone()) && self.unknown.len() < 30 { self.log.line(format!("경기 화면 SVG(미지원) {}", c.key)); } }
                Kind::Other(t) => { if matches!(t, 5 | 6) && self.unknown.insert(format!("{t}:{}", c.key)) && self.unknown.len() < 30 { self.log.line(format!("경기 화면 미지원 명령 {t} {}", c.key)); } }
            }
        }
        if !map_now.is_empty() && map_now != self.map_key && map_now.iter().all(|m| self.textures.contains_key(&m.1)) && self.terrain_rx.is_none() { self.make_map(map_now); }
        b.ground.sort_by_key(|v| v.0);
        b.over.sort_by_key(|v| v.0);
        b.units.sort_by(|a, c| c.0.total_cmp(&a.0));
        let cap = |v: Vec<Instance>| -> Vec<Instance> { let mut v = v; v.truncate(CAPACITY); v };
        gpu.update(self.ids.units, &cap(b.units.iter().map(|v| v.1).collect()))?;
        gpu.update(self.ids.ground, &cap(b.ground.iter().map(|v| v.1).collect()))?;
        gpu.update(self.ids.over, &cap(b.over.iter().map(|v| v.1).collect()))?;
        b.tri_ground.truncate(CAPACITY);
        b.tri_over.truncate(CAPACITY);
        gpu.update(self.ids.tri_ground, &b.tri_ground)?;
        gpu.update(self.ids.tri_over, &b.tri_over)?;
        self.batch = b;
        Ok(())
    }
    /// 그 그림이 서 있는 바닥. 바로 앞 그림자를 먼저, 없으면 가까운 그림자.
    fn find(anchors: &[(usize, Anchor)], index: usize, r: [f32; 4]) -> Option<Anchor> {
        let fits = |a: &Anchor| (r[0] - a.x).abs() <= (r[2] * 0.5).clamp(8.0, 40.0) && a.y >= r[1] - 6.0 && a.y <= r[1] + r[3] * 0.5 + 8.0;
        let at = anchors.partition_point(|v| v.0 < index);
        if at > 0 && index - anchors[at - 1].0 <= 6 && fits(&anchors[at - 1].1) { return Some(anchors[at - 1].1); }
        if at < anchors.len() && anchors[at].0 - index <= 8 && fits(&anchors[at].1) { return Some(anchors[at].1); }
        anchors.iter().filter(|v| (r[0] - v.1.x).abs() <= 7.0 && fits(&v.1)).min_by(|a, b| ((r[0] - a.1.x).abs() + (r[1] - a.1.y).abs()).total_cmp(&((r[0] - b.1.x).abs() + (r[1] - b.1.y).abs()))).map(|v| v.1)
    }
    /// 머리 위 요소의 기준점: 방금 그린 개체 → 가까운 그림자.
    fn near(&self, current: Option<Anchor>, anchors: &[(usize, Anchor)], r: [f32; 4]) -> Option<Anchor> {
        if let Some(a) = current { if (r[0] - a.x).abs() < 48.0 && (r[1] - a.y).abs() < 70.0 { return Some(a); } }
        anchors.iter().map(|v| v.1).filter(|a| (r[0] - a.x).abs() < 40.0 && a.y > r[1] - 40.0 && a.y < r[1] + 90.0).min_by(|a, b| ((r[0] - a.x).abs() + (r[1] - a.y).abs() * 0.5).total_cmp(&((r[0] - b.x).abs() + (r[1] - b.y).abs() * 0.5)))
    }
    /// 바닥 평면의 방향(rot)을 세운 그림의 화면 회전으로.
    fn spin(rot: f32, yaw: Quat, _side: f32) -> f32 {
        let d = Vec3::new(rot.cos(), 0.0, rot.sin());
        let right = yaw * Vec3::X;
        let up_on_ground = yaw * Vec3::NEG_Z;
        d.dot(up_on_ground).atan2(d.dot(right))
    }
    #[allow(clippy::too_many_arguments)]
    fn segment(&self, b: &mut Batch, eye: &Eye, current: Option<Anchor>, anchors: &[(usize, Anchor)], r: [f32; 4], p0: [f32; 2], p1: [f32; 2], width: f32, col: [f32; 4], layer: i32, i: usize) {
        let (dx, dy) = (p1[0] - p0[0], p1[1] - p0[1]);
        let len = dx.hypot(dy);
        if len < 0.01 || !len.is_finite() { return; }
        if layer >= 3 && len < 24.0 {
            let a = self.near(current, anchors, r).unwrap_or(Anchor { x: r[0], y: r[1] + 11.5, top: 24.0 });
            let m = eye.over(a, r[0], r[1], 0.0062) * Mat4::from_scale_rotation_translation(Vec3::new(len * PIXEL, width * PIXEL, 1.0), Quat::from_rotation_z((-dy).atan2(dx)), Vec3::ZERO);
            b.over.push((layer * 10_000 + i as i32, Instance::new(m, [0.0; 4], color(col), [0.0, 0.0, 0.0, 0.0])));
        } else {
            let m = Mat4::from_scale_rotation_translation(Vec3::new(len * PIXEL, width * PIXEL, 1.0), Quat::from_rotation_y(-dy.atan2(dx)) * Quat::from_rotation_x(FLAT), ground(r[0], r[1], 0.0029));
            b.ground.push((70_000 + i as i32, Instance::new(m, [0.0; 4], color(col), [0.0, 0.0, 0.0, 0.0])));
        }
    }
    fn text_cell(&mut self, text: &str, outline: bool) -> Option<(Region, (f32, f32))> {
        let key = (text.to_string(), outline);
        if !self.texts.contains_key(&key) {
            if self.texts.len() > 400 { self.texts.clear(); }
            let image = text::render(text, 40, outline)?;
            let id = 0x8000_0000 | (self.texts.len() as u32 + self.stats.frames as u32 * 512);
            self.texts.insert(key.clone(), TextCell { image: Arc::new(image), id });
        }
        let t = &self.texts[&key];
        let (image, id) = (t.image.clone(), t.id);
        let size = (image.width() as f32, image.height() as f32);
        let cell = self.cells.get(CellKey { tex: id, unique: 0, rect: [0, 0, image.width(), image.height()] }, &image)?;
        Some((cell, size))
    }
    /// 시야 안개: 30×30 값, 한 칸 32픽셀. 부드럽게 4배로 키워 바닥에 덮는다.
    fn fog(&mut self, b: &mut Batch, c: &Cmd) {
        let (w, h) = (c.i(48), c.i(52));
        if !(2..=64).contains(&w) || !(2..=64).contains(&h) || c.data.len() != (w * h) as usize { return; }
        let col = c.rgba(32);
        if self.fog.as_ref().is_none_or(|f| f.0 != c.data) {
            let mut im = RgbaImage::new((w * 4) as u32, (h * 4) as u32);
            let at = |x: i32, y: i32| c.data[(y.clamp(0, h - 1) * w + x.clamp(0, w - 1)) as usize] as f32;
            for y in 0..h * 4 { for x in 0..w * 4 {
                let (fx, fy) = ((x as f32 + 0.5) / 4.0 - 0.5, (y as f32 + 0.5) / 4.0 - 0.5);
                let (ix, iy) = (fx.floor() as i32, fy.floor() as i32);
                let (tx, ty) = (fx - ix as f32, fy - iy as f32);
                let v = at(ix, iy) * (1.0 - tx) * (1.0 - ty) + at(ix + 1, iy) * tx * (1.0 - ty) + at(ix, iy + 1) * (1.0 - tx) * ty + at(ix + 1, iy + 1) * tx * ty;
                im.put_pixel(x as u32, y as u32, image::Rgba([0, 0, 0, v.round() as u8]));
            } }
            self.fog_unique = self.fog_unique.wrapping_add(1);
            let previous = self.fog.as_ref().map(|f| f.2).unwrap_or(0);
            let image = Arc::new(im);
            let key = CellKey { tex: 0x4000_0000, unique: self.fog_unique, rect: [0, 0, image.width(), image.height()] };
            let _ = self.cells.get(key, &image);
            self.fog = Some((c.data.clone(), image, if previous == 0 { self.fog_unique } else { previous }));
        }
        let Some((_, image, shown)) = self.fog.clone() else { return };
        // 새 안개가 준비되면 바꾸고, 그 전에는 이전 것을 보인다.
        let full = [0, 0, image.width(), image.height()];
        let mut region = self.cells.get(CellKey { tex: 0x4000_0000, unique: self.fog_unique, rect: full }, &image);
        if region.is_some() { if let Some(f) = &mut self.fog { f.2 = self.fog_unique; } } else if shown != self.fog_unique { region = self.cells.get(CellKey { tex: 0x4000_0000, unique: shown, rect: full }, &image); }
        let Some(cell) = region else { return };
        let scale = c.f(64);
        let (mw, mh) = (w as f32 * scale, h as f32 * scale);
        let m = Mat4::from_scale_rotation_translation(Vec3::new(mw * PIXEL, mh * PIXEL, 1.0), Quat::from_rotation_x(FLAT), ground(c.f(56) + mw * 0.5, c.f(60) + mh * 0.5, 0.0028));
        b.ground.push((95_000, Instance::new(m, cell.uv, color(col), [11.0, 0.0, 0.0, cell.page as f32])));
    }
    /// 바닥·벽·수풀 큰 그림이 바뀌면 지형을 다시 만든다(작업 스레드).
    fn make_map(&mut self, now: Vec<(String, u32)>) {
        let images: Vec<(String, MapTexture)> = now.iter().filter_map(|(k, id)| self.textures.get(id).map(|t| (k.clone(), MapTexture(CellKey { tex: *id, unique: t.unique, rect: [0, 0, t.image.width(), t.image.height()] }, t.image.clone())))).collect();
        let serial = self.map_ids.len() as u32 + 1;
        self.log.line(format!("지형 만들기 {:?}", now.iter().map(|n| n.0.rsplit('/').next().unwrap_or("").to_string()).collect::<Vec<_>>()));
        let (tx, rx) = mpsc::channel();
        self.terrain_rx = Some(rx);
        std::thread::spawn(move || {
            let mut out = Vec::new();
            let name = |k: &str| k.rsplit('/').next().unwrap_or("").to_string();
            let sized = |im: &RgbaImage| im.width() == terrain::SIZE && im.height() == terrain::SIZE;
            for (k, tex) in &images {
                let im = &tex.1;
                let n = name(k);
                if n.contains("background") { out.push(MapMesh { name: "live0ground", vertices: plane(), texture: Some(tex.clone()), height: 0.0, blend: false, opacity: 1.0 }); }
                else if n.contains("shadow") { out.push(MapMesh { name: "live1shadow", vertices: plane(), texture: Some(tex.clone()), height: 0.0006, blend: true, opacity: 1.0 }); }
                else if n.contains("bush") && sized(im) {
                    let t = terrain::build(&[im], 0.035, false);
                    out.push(MapMesh { name: "live6bushSide", vertices: t.side, texture: Some(tex.clone()), height: 0.0, blend: true, opacity: 0.62 });
                    out.push(MapMesh { name: "live6bushTop", vertices: t.top, texture: Some(tex.clone()), height: 0.0, blend: true, opacity: 0.72 });
                }
                else if n.contains("wall") && sized(im) {
                    let front = n.contains("front");
                    if front { continue; } // 앞면 조각은 몸통과 합쳐 처리
                    let extra = images.iter().find(|o| name(&o.0).contains("front") && sized(&o.1.1)).map(|o| o.1.1.clone());
                    let mut whole = (**im).clone();
                    if let Some(e) = &extra { image::imageops::overlay(&mut whole, &**e, 0, 0); }
                    let t = terrain::build(&[&whole], 0.06, true);
                    let tex = MapTexture(CellKey { tex: 0x2000_0000 + serial, unique: 0, rect: [0, 0, whole.width(), whole.height()] }, Arc::new(whole));
                    out.push(MapMesh { name: "live0wallFlat", vertices: plane(), texture: Some(tex.clone()), height: 0.0012, blend: true, opacity: 1.0 });
                    out.push(MapMesh { name: "live0wallSide", vertices: t.side, texture: Some(tex.clone()), height: 0.0, blend: false, opacity: 1.0 });
                    out.push(MapMesh { name: "live0wallTop", vertices: t.top, texture: Some(tex), height: 0.0, blend: false, opacity: 1.0 });
                }
            }
            let _ = tx.send((out, now));
        });
    }
    fn install_map(&mut self, gpu: &mut Gpu) -> Result<()> {
        // 큰 그림 칸이 준비될 때까지 기다렸다가 한 번에 바꾼다.
        let Some(rx) = &self.terrain_rx else { return self.refresh_map(gpu) };
        let Ok((meshes, key)) = rx.try_recv() else { return Ok(()) };
        self.terrain_rx = None;
        self.map_key = key;
        for id in self.map_ids.drain(..) { gpu.update(id, &[])?; }
        self.map_pending = meshes;
        self.refresh_map(gpu)
    }
    fn refresh_map(&mut self, gpu: &mut Gpu) -> Result<()> {
        for (key, image) in &self.map_uses { let _ = self.cells.get(*key, image); }
        if self.map_pending.is_empty() { return Ok(()); }
        let mut ready = Vec::new();
        for m in &self.map_pending {
            let Some(MapTexture(key, image)) = m.texture.clone() else { ready.push(None); continue };
            match self.cells.get(key, &image) { Some(r) => ready.push(Some((r, key, image))), None => { if !self.map_uses.iter().any(|u| u.0 == key) { self.map_uses.push((key, image)); } return Ok(()); } }
        }
        self.map_uses.clear();
        for (m, r) in std::mem::take(&mut self.map_pending).into_iter().zip(ready) {
            if m.vertices.is_empty() { continue; }
            let (uv, params) = match &r { Some((r, _, _)) => (r.uv, [1.0, if m.blend { 0.02 } else { 0.5 }, 0.0, r.page as f32]), None => ([0.0; 4], [0.0; 4]) };
            let inst = Instance::new(Mat4::from_translation(Vec3::new(0.0, m.height, 0.0)), uv, [1.0, 1.0, 1.0, m.opacity], params);
            let id = gpu.add_mesh(&geometry::mesh(m.name, m.vertices, vec![inst], m.blend, !m.blend), 1)?;
            self.map_ids.push(id);
            if let Some((_, key, image)) = r { if !self.map_uses.iter().any(|u| u.0 == key) { self.map_uses.push((key, image)); } }
        }
        self.log.line(format!("지형 적용 {}개", self.map_ids.len()));
        Ok(())
    }
}
