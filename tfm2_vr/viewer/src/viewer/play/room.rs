//! 나눈 창을 방 안에 띄운다: 둥근 벽을 따라 놓인 카드, 튀어나온 버튼, 가리키면 떠오르는 줄, 앞으로 나오는 팝업.
use super::{shelf::{self, Rect, Shelf, Win}, show::Stage};
use crate::{geometry::{self, Instance, Vertex}, gpu::Gpu, live::{frame::Cmd, Live}};
use anyhow::Result;
use glam::{Mat4, Quat, Vec2, Vec3};
use std::collections::HashMap;

const UI: [f32; 2] = [1920.0, 1080.0];
pub type Key = (i32, i32, i32, i32, u32);
pub fn key(w: &Win) -> Key { ((w.rect[0] / 4.0) as i32, (w.rect[1] / 4.0) as i32, (w.rect[2] / 4.0) as i32, (w.rect[3] / 4.0) as i32, w.layer) }
/// 창들이 놓이는 벽: UI 픽셀 → 방 안 자리.
#[derive(Clone, Copy)]
pub struct Wall { pub ox: f32, pub oy: f32, pub radius: f32, pub scale: f32, pub middle: f32 }
impl Wall {
    /// UI 점이 거리 rad 에서 놓일 자리와 방향. k = 벽 배치를 따르는 정도(0 이면 정면 가운데).
    pub fn at(&self, place: &Place, x: f32, y: f32, rad: f32, k: f32) -> (Vec3, Quat) {
        let across = (x - self.ox) * self.scale / self.radius * k;
        let high = self.middle + (self.oy - y) * self.scale * rad / self.radius * k;
        let high = if k < 1.0 { place.head.y - 0.05 + (high - place.head.y + 0.05) * k } else { high };
        place.spot(across, rad, high, 0.0)
    }
}
#[derive(Clone, Copy)]
pub struct Frame { pub center: Vec3, pub rotation: Quat, pub at: [f32; 2], pub scale: f32 }
impl Frame {
    /// UI 픽셀 점을 방 안 점으로(앞으로 lift m).
    pub fn point(&self, x: f32, y: f32, lift: f32) -> Vec3 { self.center + self.rotation * Vec3::new((x - self.at[0]) * self.scale, (self.at[1] - y) * self.scale, lift) }
    pub fn quad(&self, r: Rect, lift: f32, grow: f32) -> Mat4 { Mat4::from_scale_rotation_translation(Vec3::new(r[2] * self.scale * grow, r[3] * self.scale * grow, 1.0), self.rotation, self.point(r[0] + r[2] * 0.5, r[1] + r[3] * 0.5, lift)) }
}
struct Anim { age: f32, delay: f32, lift: f32, back: f32, seen: u64, born: u64, shown: Vec<Frame> }
struct Pop { value: f32, seen: u64, radius: f32 }
pub struct Face { frame: Frame, rect: Rect, key: Key, lift: f32 }
#[derive(Clone, Copy)]
pub struct Place { pub head: Vec3, pub yaw: f32, pub floor: f32, pub table: Option<f32>, pub flat: bool }
impl Place {
    /// 가로 across(라디안 기준, 오른쪽 +)·거리 rad 의 자리. 곡면이면 나를 둘러싸고 평면이면 한 벽에 나란히.
    pub fn spot(&self, across: f32, rad: f32, height: f32, tilt: f32) -> (Vec3, Quat) {
        let a = self.yaw;
        if self.flat { let (f, r) = (Vec3::new(-a.sin(), 0.0, -a.cos()), Vec3::new(a.cos(), 0.0, -a.sin())); let p = self.head + f * rad + r * (across * rad); return (Vec3::new(p.x, height, p.z), Quat::from_rotation_y(a) * Quat::from_rotation_x(tilt)); }
        let a = a - across;
        (Vec3::new(self.head.x - a.sin() * rad, height, self.head.z - a.cos() * rad), Quat::from_rotation_y(a) * Quat::from_rotation_x(tilt))
    }
}
pub struct Hit { pub key: Key, pub x: f32, pub y: f32, pub distance: f32, pub zone: Option<Rect> }
/// 지금 가리키는 자리. since·pressed = 시작·마지막으로 누른 장 번호, lift = 그 자리가 튀어나온 만큼.
struct Hover { key: Key, rect: Rect, zone: Option<Rect>, since: u64, pressed: u64, lift: f32 }
pub struct Room { faces: Vec<Face>, anims: HashMap<Key, Anim>, pops: HashMap<(Key, [i32; 4]), Pop>, tick: u64, face_id: usize, glow_id: usize, cut_id: usize, dome_id: usize, floor_id: usize, arena: Vec<Cmd>, arena_light: f32, time: f32, show: Stage, hover: Option<Hover>, zones: HashMap<Key, Vec<Rect>>, extent: Option<[f32; 4]>, pub note: Option<String>, pub solo: bool, pub count: usize }

fn smooth(t: f32) -> f32 { let t = t.clamp(0.0, 1.0); 1.0 - (1.0 - t).powi(3) }
fn dome() -> Vec<Vertex> {
    let mut out = Vec::new();
    let (rings, seg) = (14, 32);
    let color = |t: f32| -> [f32; 4] { // t: -1 바닥 … 1 꼭대기
        let (low, mid, high) = ([0.012f32, 0.017, 0.022], [0.030, 0.048, 0.060], [0.006, 0.009, 0.016]);
        let k = if t < 0.0 { (low, mid, (t + 1.0).powf(3.0)) } else { (mid, high, t.powf(0.55)) };
        let v: [f32; 3] = std::array::from_fn(|i| k.0[i] + (k.1[i] - k.0[i]) * k.2);
        [v[0], v[1], v[2], 1.0]
    };
    let p = |i: usize, j: usize| { let a = std::f32::consts::PI * (i as f32 / rings as f32 - 0.5); let b = std::f32::consts::TAU * j as f32 / seg as f32; (Vec3::new(a.cos() * b.cos(), a.sin(), a.cos() * b.sin()) * 14.0, color(a.sin())) };
    for i in 0..rings { for j in 0..seg { let q = [p(i, j), p(i + 1, j), p(i + 1, j + 1), p(i, j + 1)]; for k in [0, 1, 2, 0, 2, 3] { out.push(Vertex { position: q[k].0.to_array(), uv: [0.0; 2], color: q[k].1 }); } } }
    out
}
impl Room {
    pub fn new(gpu: &mut Gpu) -> Result<Self> {
        fn add(gpu: &mut Gpu, name: &str, v: Vec<Vertex>, blend: bool, write: bool, cap: usize) -> Result<usize> { let mut m = geometry::mesh(name, v, vec![], blend, write); m.dynamic = true; gpu.add_mesh(&m, cap) }
        Ok(Self { dome_id: add(gpu, "live0roomDome", dome(), false, false, 1)?, face_id: add(gpu, "live0roomFace", geometry::quad(), false, true, 2048)?, floor_id: add(gpu, "live2roomFloor", geometry::quad(), true, false, 16)?, glow_id: add(gpu, "live8roomGlow", geometry::quad(), true, false, 4096)?, cut_id: add(gpu, "live9roomCut", geometry::quad(), true, false, 1024)?, arena: Vec::new(), arena_light: 0.0, time: 0.0, show: Stage::new(gpu)?, hover: None, zones: HashMap::new(), extent: None, note: None, faces: Vec::new(), anims: HashMap::new(), pops: HashMap::new(), tick: 0, solo: false, count: 0 })
    }
    /// 시험용: 지금 놓인 창들의 (UI 사각형, 가운데 자리).
    pub fn placed(&self) -> Vec<(Rect, Vec3)> { self.faces.iter().map(|f| (f.rect, f.frame.point(f.rect[0] + f.rect[2] * 0.5, f.rect[1] + f.rect[3] * 0.5, f.lift))).collect() }
    /// 레이저가 닿은 자리. 판정은 **튀어나오기 전 제자리 면**에서 하므로 튀어나와도 결과가 바뀌지 않는다.
    /// 이미 가리키던 자리는 조금 벗어나도, 튀어나온 면에서 보아 아직 안이어도 그대로 잡고 있는다.
    pub fn hit(&self, origin: Vec3, direction: Vec3) -> Option<Hit> {
        let cross = |f: &Face, lift: f32| -> Option<(f32, f32, f32)> {
            let n = f.frame.rotation * Vec3::Z;
            let k = direction.dot(n);
            if k.abs() < 1e-4 { return None; }
            let t = (f.frame.point(f.frame.at[0], f.frame.at[1], lift) - origin).dot(n) / k;
            if t < 0.02 { return None; }
            let l = f.frame.rotation.inverse() * (origin + direction * t - f.frame.center);
            Some((f.frame.at[0] + l.x / f.frame.scale, f.frame.at[1] - l.y / f.frame.scale, t))
        };
        let wide = |r: Rect, m: f32| [r[0] - m, r[1] - m, r[2] + m * 2.0, r[3] + m * 2.0];
        let mut best: Option<(Hit, &Face)> = None;
        for f in &self.faces {
            let Some((x, y, t)) = cross(f, 0.0) else { continue };
            if best.as_ref().is_some_and(|b| b.0.distance < t) || !shelf::has(f.rect, x, y) { continue; }
            best = Some((Hit { key: f.key, x, y, distance: t, zone: self.zone(&f.key, x, y) }, f));
        }
        if let Some(h) = &self.hover {
            // 잡고 있는 자리 안인가(제자리 면은 여유 10픽셀, 튀어나온 면은 그대로).
            let area = h.zone.unwrap_or(h.rect);
            let kept = self.faces.iter().filter(|f| f.key == h.key).find_map(|f| {
                let rest = cross(f, 0.0)?;
                let inside = shelf::has(wide(shelf::cut(area, wide(f.rect, 1.0)), 10.0), rest.0, rest.1);
                let out = cross(f, h.lift).filter(|p| shelf::has(wide(shelf::cut(area, wide(f.rect, 1.0)), 1.0), p.0, p.1));
                // 튀어나온 면 때문에 옆 줄을 너무 오래 붙잡지 않게 제자리 면에서도 가까워야 한다.
                let close = shelf::has(wide(area, 14.0), rest.0, rest.1);
                (inside || (close && out.is_some())).then(|| (rest, out, f))
            });
            if let Some((rest, _, f)) = kept {
                let steal = best.as_ref().is_some_and(|(b, _)| {
                    if b.key == h.key {
                        // 같은 창: 지금 자리 안쪽의 더 작은 자리로만 옮겨 간다. 자리 없이 창만 가리키던 중이면 어느 자리든.
                        match (h.zone, b.zone) { (None, Some(_)) => true, (Some(z), Some(q)) => q != z && shelf::inside(q, z, 1.0) && shelf::area(q) < shelf::area(z) * 0.95, _ => false }
                    } else {
                        // 다른 창: 더 앞에 있고, 가리키기 시작했을 때 이미 있던 창(또는 방금 눌러서 나온 창)일 때만. 가리켜서 생긴 설명 창은 빼앗지 않는다.
                        let born = self.anims.get(&b.key).map(|a| a.born).unwrap_or(0);
                        b.distance + 0.004 < rest.2 && (born < h.since || born >= h.pressed && h.pressed > 0 && born < h.pressed + 180)
                    }
                });
                if !steal {
                    let (x, y) = (rest.0.clamp(area[0] + 1.5, area[0] + (area[2] - 1.5).max(1.5)), rest.1.clamp(area[1] + 1.5, area[1] + (area[3] - 1.5).max(1.5)));
                    let distance = cross(f, h.lift).filter(|p| shelf::has(wide(area, 4.0), p.0, p.1)).map(|p| p.2).unwrap_or(rest.2);
                    return Some(Hit { key: h.key, x, y, distance, zone: h.zone });
                }
            }
        }
        best.map(|(h, f)| Hit { distance: cross(f, f.lift.max(0.0)).map(|p| p.2).unwrap_or(h.distance).min(h.distance), ..h })
    }
    fn zone(&self, key: &Key, x: f32, y: f32) -> Option<Rect> { self.zones.get(key)?.iter().filter(|z| shelf::has(**z, x, y)).min_by(|a, b| shelf::area(**a).total_cmp(&shelf::area(**b))).copied() }
    fn wall(shelf: &Shelf, place: &Place, hole: Option<Rect>, held: Option<[f32; 4]>) -> (Wall, f32, f32, f32, f32) {
        let in_hole = |r: Rect| hole.is_some_and(|h| shelf::area(shelf::cut(r, h)) > 0.5 * shelf::area(r));
        let normal = |w: &Win| !(w.rect[0] <= 8.0 && w.rect[3] >= 600.0 && w.rect[2] <= 420.0) && !(w.rect[1] + w.rect[3] * 0.5 >= 1000.0 && w.rect[3] <= 90.0);
        let (mut lo, mut hi, mut top, mut bottom) = (f32::INFINITY, f32::NEG_INFINITY, f32::INFINITY, f32::NEG_INFINITY);
        // 다른 창 위에 겹쳐 뜬 것(설명 창 등)은 벽 크기를 바꾸지 않는다.
        for w in shelf.wins.iter().filter(|w| !w.ghost && w.level == 0 && normal(w) && w.layer == shelf.wins[0].layer && !in_hole(w.rect) && !(w.rect[1] <= 8.0 && w.rect[3] <= 90.0)) { lo = lo.min(w.rect[0]); hi = hi.max(w.rect[0] + w.rect[2]); top = top.min(w.rect[1]); bottom = bottom.max(w.rect[1] + w.rect[3]); }
        if !lo.is_finite() { (lo, hi, top, bottom) = (0.0, UI[0], 0.0, UI[1]); }
        if let Some(e) = held { (lo, hi, top, bottom) = (e[0], e[1], e[2], e[3]); }
        // 관리 화면은 내용이 짧아도 벽 높이를 지킨다(메뉴·아래 조작대가 오르내리지 않게).
        if shelf.wins.iter().any(|w| w.layer == 0 && w.rect[0] <= 8.0 && w.rect[3] >= 600.0 && w.rect[2] <= 420.0) && place.table.is_none() { (top, bottom) = (top.min(60.0), bottom.max(980.0)); (lo, hi) = (lo.min(296.0), hi.max(1896.0)); }
        let ox = (lo + hi) * 0.5;
        // 경기: 테이블 뒤 먼 벽. 그 밖: 눈앞 둥근 벽.
        let (radius, scale, middle) = match place.table { Some(t) => (2.8f32, 0.00168f32, t + 0.12 + (bottom - top) * 0.5 * 0.00168), None => (2.0, 0.00146, place.head.y - 0.02) };
        let oy = (top + bottom) * 0.5;
        (Wall { ox, oy, radius, scale, middle }, lo, hi, top, bottom)
    }
    /// 창마다 놓일 틀. 넓은 창은 몇 조각으로 나눠 둥근 벽을 따른다.
    fn frames(shelf: &Shelf, place: &Place, hole: Option<Rect>, held: Option<[f32; 4]>) -> Vec<Vec<(Frame, Rect)>> {
        let bar = |r: Rect| r[1] <= 8.0 && r[3] <= 90.0 && r[2] >= 1500.0;
        let (Wall { ox, oy, radius, scale, middle }, lo, hi, top, bottom) = Self::wall(shelf, place, hole, held);
        let half = (hi - lo) * 0.5 * scale * 1.03 / radius;
        if let (Some(table), Some(h)) = (place.table, hole) { return Self::match_frames(shelf, place, table, h); }
        shelf.wins.iter().map(|w| {
            let r = w.rect;
            let layer = w.layer as f32;
            let put = |angle: f32, rad: f32, height: f32, tilt: f32, at: [f32; 2], scale: f32| { let (center, rotation) = place.spot(-angle, rad, height, tilt); Frame { center, rotation, at, scale } };
            let (cx, cy) = (r[0] + r[2] * 0.5, r[1] + r[3] * 0.5);
            if layer == 0.0 && place.table.is_none() && r[0] <= 8.0 && r[3] >= 600.0 && r[2] <= 420.0 {
                // 왼쪽 메뉴 기둥: 벽 끝에서 더 가깝게, 나를 보게.
                let (rad, s) = if place.flat { (radius, scale) } else { (1.62, scale * 0.86) };
                let a = half * 1.02 + r[2] * 0.5 * s / rad + 0.03;
                return vec![(put(a, rad, middle + (oy - cy) * s, 0.0, [cx, cy], s), r)];
            }
            if layer == 0.0 && cy >= 1000.0 && r[3] <= 90.0 {
                // 아래 조작대: 가깝고 낮게, 위로 기울여.
                let (rad, s, tilt, drop) = if place.table.is_some() { (radius - 0.06, scale, 0.0, 0.03) } else { (1.5, scale * if r[2] > 500.0 { 0.72 } else { 0.92 }, -0.42, 0.0) };
                let h = if place.table.is_some() { middle + (oy - bottom) * scale - drop - r[3] * 0.5 * s } else { middle + (oy - bottom) * scale - 0.13 };
                return vec![(put(-(cx - ox) * s / rad, rad, h, tilt, [cx, cy], s), r)];
            }
            let rad = radius - 0.085 * w.level as f32 - 0.42 * layer;
            let s = scale * rad / radius;
            if !w.scene.is_empty() { let s = scale * 1.12; return vec![(put(-(cx - ox) * scale / radius, radius + 0.35, middle + (oy - cy) * scale, 0.0, [cx, cy], s), r)]; }
            if bar(r) && place.table.is_some() {
                // 경기 위 띠: 벽 묶음 위 가운데에 조금 작게.
                let s = scale * 0.78;
                let n = 6;
                let h = middle + (oy - top) * scale * 1.02 + 0.05 + r[3] * 0.5 * s;
                return (0..n).map(|i| { let part = [r[0] + r[2] * i as f32 / n as f32, r[1], r[2] / n as f32, r[3]]; let px = part[0] + part[2] * 0.5; (put(-(px - cx) * s / rad, rad, h, 0.0, [px, cy], s), part) }).collect();
            }
            let n = if r[2] > 1000.0 && !place.flat { (r[2] / 380.0).ceil() as usize } else { 1 };
            (0..n).map(|i| {
                let part = [r[0] + r[2] * i as f32 / n as f32, r[1], r[2] / n as f32, r[3]];
                let px = part[0] + part[2] * 0.5;
                let spread = if n == 1 { 1.03 } else { 1.0 };
                // 조각 이음새가 벌어지지 않게 호가 아니라 현 길이로 맞춘다.
                let half = part[2] * scale * 0.5 / radius;
                let s = if n == 1 { s } else { s * half.tan() / half * 1.0008 };
                (put(-(px - ox) * scale * spread / radius, rad, middle + (oy - cy) * scale * 1.02, 0.0, [px, cy], s), part)
            }).collect()
        }).collect()
    }
    /// 경기: 테이블 뒤 벽. 선수 표·체력 묶음은 오른쪽, 미니맵·단추 묶음은 왼쪽, 조작 띠는 아래 한 줄, 점수 띠는 위.
    fn match_frames(shelf: &Shelf, place: &Place, table: f32, h: Rect) -> Vec<Vec<(Frame, Rect)>> {
        let (radius, scale) = (2.75f32, 0.00196f32);
        let split = h[0] + h[2]; // 게임 그림 오른쪽 끝
        let right: Vec<&Win> = shelf.wins.iter().filter(|w| !w.ghost && w.level == 0 && (!w.bare || w.rect[3] >= 300.0) && w.rect[0] >= split - 10.0 && w.rect[1] + w.rect[3] * 0.5 < 1000.0 && w.rect[3] > 24.0).collect();
        let column = |lower: bool| right.iter().filter(|w| (w.rect[1] + w.rect[3] * 0.5 >= 700.0) == lower).fold(None::<Rect>, |a, w| Some(match a { None => w.rect, Some(b) => { let (x, y) = (b[0].min(w.rect[0]), b[1].min(w.rect[1])); [x, y, (b[0] + b[2]).max(w.rect[0] + w.rect[2]) - x, (b[1] + b[3]).max(w.rect[1] + w.rect[3]) - y] } }));
        let (mut upper, mut lower) = (column(false), column(true));
        // 타이틀: 오른쪽을 채운 키 큰 그림이 있거나(모드 타이틀), 오른쪽이 같은 폭의 단추 한 줄뿐이면(기본 타이틀) 나누지 않는다.
        let pillar = right.iter().any(|w| w.rect[3] >= 800.0);
        let list: Vec<&&Win> = right.iter().filter(|w| w.layer == 0 && !w.bare).collect();
        let row = list.len() >= 4 && list.iter().all(|w| (w.rect[0] + w.rect[2] * 0.5 - list[0].rect[0] - list[0].rect[2] * 0.5).abs() < 12.0 && (w.rect[2] - list[0].rect[2]).abs() < 40.0) && [false, true].iter().all(|low| list.iter().any(|w| (w.rect[1] + w.rect[3] * 0.5 >= 700.0) == *low));
        let tall = pillar || row;
        if tall { if let (Some(a), Some(b)) = (upper, lower) { let (x, y) = (a[0].min(b[0]), a[1].min(b[1])); upper = Some([x, y, (a[0] + a[2]).max(b[0] + b[2]) - x, (a[1] + a[3]).max(b[1] + b[3]) - y]); } lower = None; }
        let two = upper.is_some() && lower.is_some();
        // 타이틀처럼 키 큰 화면은 고개를 들지 않게 낮추고 조금 줄인다.
        let floor = table + if pillar { 0.05 } else if tall { 0.16 } else { 0.30 }; // 벽 묶음의 아래 끝
        let scale = if tall { 0.00172 } else { scale };
        let put = |x: f32, height: f32, rad: f32, s: f32, at: [f32; 2]| { let (center, rotation) = place.spot(x / rad, rad, height, 0.0); (Frame { center, rotation, at, scale: s }, ()) };
        let top = floor + upper.or(lower).map(|u| u[3]).unwrap_or(600.0) * scale;
        shelf.wins.iter().map(|w| {
            let r = w.rect;
            let (cx, cy) = (r[0] + r[2] * 0.5, r[1] + r[3] * 0.5);
            let rad = radius - 0.07 * w.level as f32 - 0.4 * w.layer as f32;
            if w.layer > 0 { let s = scale * 0.9; return vec![(put((cx - 960.0) * s, floor + 0.55 + (540.0 - cy) * s * 0.8, rad, s, [cx, cy]).0, r)]; }
            if r[1] <= 8.0 && r[3] <= 90.0 && r[2] >= 1500.0 {
                let s = scale * 0.74;
                return (0..6).map(|i| { let part = [r[0] + r[2] * i as f32 / 6.0, r[1], r[2] / 6.0, r[3]]; let px = part[0] + part[2] * 0.5; let half = part[2] * s * 0.5 / radius; (put((px - cx) * s, top + 0.06 + r[3] * 0.5 * s, radius, s * half.tan() / half * 1.0008, [px, cy]).0, part) }).collect();
            }
            if r[1] <= 100.0 && r[3] <= 90.0 && r[0] + r[2] > 880.0 && r[0] < 1040.0 { let s = scale * 0.74; return vec![(put((cx - 960.0) * s, top + 0.02, radius - 0.03, s, [cx, cy]).0, r)]; }
            // 아래 조작 띠와 왼쪽 위 재생 단추는 한 줄로.
            let controls = cy < h[1] + 56.0 && cx < split;
            if cy >= 1000.0 || controls {
                let s = scale * 0.92;
                let x = if controls { cx - 9.0 + 1010.0 } else { cx };
                return vec![(put((x - 660.0) * s, table + 0.13 + (1053.0 - cy.max(1000.0)) * 0.0, radius - 0.12, s, [cx, cy]).0, r)];
            }
            if cx < split { let s = scale * 0.9; return vec![(put((cx - split) * s - 0.55, floor + 0.9 - (cy - 208.0) * s * 0.6, radius - 0.35, s, [cx, cy]).0, r)]; }
            let group = if cy >= 700.0 && two { lower } else { upper }.unwrap_or(r);
            let side = if !two { 0.0 } else if cy >= 700.0 { -1.0 } else { 1.0 };
            let half = group[2] * 0.5 + 18.0;
            let x = (cx - group[0] - group[2] * 0.5 + side * half) * scale;
            let height = floor + (group[1] + group[3] - cy) * scale + if side < 0.0 { 0.12 } else { 0.0 };
            vec![(put(x, height, rad, scale, [cx, cy]).0, r)]
        }).collect()
    }
    #[allow(clippy::too_many_arguments)]
    pub fn update(&mut self, gpu: &mut Gpu, live: &mut Live, commands: &crate::live::frame::Frame, shelf: &Shelf, place: &Place, local: Mat4, crop: [f32; 4], pointer: Option<(Key, Option<Rect>, bool)>, dt: f32, hole: Option<Rect>) -> Result<()> {
        let list = commands.list("UI");
        self.tick += 1;
        let tick = self.tick;
        self.faces.clear();
        // 가리킴 갱신: 같은 자리를 계속 가리키면 시작 시각을 지킨다.
        let same = |a: Rect, b: Rect| { let c = shelf::area(shelf::cut(a, b)); c > 0.6 * shelf::area(a).max(shelf::area(b)) };
        self.hover = match (pointer, self.hover.take()) {
            (Some((key, zone, down)), old) => {
                let kept = old.filter(|o| o.key == key && match (o.zone, zone) { (Some(a), Some(b)) => same(a, b), (None, None) => true, _ => false });
                let mut h = kept.unwrap_or(Hover { key, rect: [0.0; 4], zone, since: tick, pressed: 0, lift: 0.0 });
                h.zone = zone;
                if down { h.pressed = tick; }
                Some(h)
            }
            _ => None,
        };
        self.zones.clear();
        let (mut faces, mut glow, mut cuts): (Vec<Instance>, Vec<Instance>, Vec<Instance>) = (Vec::new(), Vec::new(), Vec::new());
        let uv = |r: Rect| [crop[0] + crop[2] * r[0] / UI[0], crop[1] + crop[3] * r[1] / UI[1], crop[2] * r[2] / UI[0], crop[3] * r[3] / UI[1]];
        // 둥근 사각형: params = (종류, 반지름/높이, 가로세로비, 값)
        let panel = |m: Mat4, r: Rect, radius: f32, tint: [f32; 4]| Instance::new(local * m, uv(r), tint, [13.0, (radius / r[3].max(1.0)).min(0.5), r[2] / r[3].max(1.0), 0.0]);
        let solid = |m: Mat4, r: Rect, radius: f32, color: [f32; 4], soft: f32| Instance::new(local * m, [0.0; 4], color, [14.0, (radius / r[3].max(1.0)).min(0.5), r[2] / r[3].max(1.0), soft]);
        let ring = |m: Mat4, r: Rect, radius: f32, color: [f32; 4], width: f32| Instance::new(local * m, [0.0; 4], color, [15.0, (radius / r[3].max(1.0)).min(0.5), r[2] / r[3].max(1.0), width / r[3].max(1.0)]);
        let fresh = shelf.wins.iter().filter(|w| !self.anims.contains_key(&key(w))).count();
        let mut order = 0.0f32;
        self.count = shelf.wins.len();
        let mut spots: Vec<(Rect, Frame)> = Vec::new();
        let keys: std::collections::HashSet<Key> = shelf.wins.iter().map(key).collect();
        // 가리켜서 작은 창이 생기거나 사라져도 벽 전체가 움직이지 않게, 가리키는 동안은 벽 크기를 지킨다(눌렀거나 화면이 크게 바뀌면 새로 잰다).
        let held = {
            let stay = self.anims.keys().filter(|k| keys.contains(*k)).count() * 10 >= self.anims.len().max(keys.len()) * 7;
            let quiet = self.hover.as_ref().is_some_and(|h| tick > h.since + 12 && (h.pressed == 0 || tick > h.pressed + 120));
            let old = self.extent.filter(|_| stay && quiet && hole.is_none());
            let (_, lo, hi, top, bottom) = if shelf.wins.is_empty() { (Wall { ox: 0.0, oy: 0.0, radius: 1.0, scale: 1.0, middle: 0.0 }, 0.0, 0.0, 0.0, 0.0) } else { Self::wall(shelf, place, hole, old) };
            self.extent = Some([lo, hi, top, bottom]);
            old
        };
        if !shelf.wins.is_empty() {
            let frames = Self::frames(shelf, place, hole, held);
            for (index, (w, parts)) in shelf.wins.iter().zip(&frames).enumerate() {
                let k = key(w);
                // 조금 움직이거나 크기가 바뀐 같은 창은 이어받는다(다시 나타나는 연출이 되풀이되지 않게).
                if !self.anims.contains_key(&k) {
                    let near = |a: &Key| a.4 == k.4 && (a.0 - k.0).abs() <= 8 && (a.1 - k.1).abs() <= 8 && (a.2 - k.2).abs() <= 10 && (a.3 - k.3).abs() <= 10;
                    let old = self.anims.iter().filter(|(a, v)| v.seen + 1 == tick && near(a) && !keys.contains(a)).min_by_key(|(a, _)| (a.0 - k.0).abs() + (a.1 - k.1).abs() + (a.2 - k.2).abs() + (a.3 - k.3).abs()).map(|(a, _)| *a);
                    if let Some(v) = old.and_then(|o| self.anims.remove(&o)) { self.anims.insert(k, v); }
                }
                // 게임이 다시 그리며 크기가 조금 바뀐 창·자리는 같은 것으로 따라간다.
                if let Some(h) = &mut self.hover {
                    let near = |a: &Key| a.4 == k.4 && (a.0 - k.0).abs() <= 8 && (a.1 - k.1).abs() <= 8 && (a.2 - k.2).abs() <= 10 && (a.3 - k.3).abs() <= 10;
                    if h.key != k && !keys.contains(&h.key) && near(&h.key) { h.key = k; }
                    if h.key == k {
                        h.rect = w.rect;
                        if let Some(z) = h.zone { h.zone = w.zones.iter().map(|q| q.0).find(|q| *q == z).or_else(|| w.zones.iter().map(|q| q.0).filter(|q| same(*q, z)).min_by(|a, b| { let d = |q: &Rect| (0..4).map(|i| (q[i] - z[i]).abs()).sum::<f32>(); d(a).total_cmp(&d(b)) })); }
                    }
                }
                self.zones.insert(k, w.zones.iter().map(|z| z.0).collect());
                let hovered = self.hover.as_ref().is_some_and(|h| h.key == k);
                let anim = self.anims.entry(k).or_insert_with(|| { order += 1.0; Anim { age: 0.0, delay: if fresh > 3 { (order * 0.022).min(0.35) } else { 0.0 }, lift: 0.0, back: 0.0, seen: tick, born: tick, shown: Vec::new() } });
                anim.seen = tick;
                anim.age += dt;
                // E. 자리가 바뀌면 튀지 않고 미끄러져 간다.
                let smooth_to = 1.0 - (-dt * 14.0).exp();
                if anim.shown.len() != parts.len() || anim.age <= dt { anim.shown = parts.iter().map(|p| p.0).collect(); }
                for (shown, (goal, _)) in anim.shown.iter_mut().zip(parts) {
                    let far = (goal.center - shown.center).length() > 1.2;
                    if far { *shown = *goal; } else { shown.center = shown.center.lerp(goal.center, smooth_to); shown.rotation = shown.rotation.slerp(goal.rotation, smooth_to); shown.scale += (goal.scale - shown.scale) * smooth_to; shown.at = goal.at; }
                }
                let settled: Vec<Frame> = parts.iter().map(|p| p.0).collect();
                let parts: Vec<(Frame, Rect)> = anim.shown.iter().zip(parts).map(|(f, p)| (*f, p.1)).collect();
                let parts = &parts;
                let goal = if hovered && !w.bare { 0.022 } else { 0.0 };
                anim.lift += (goal - anim.lift) * (1.0 - (-dt * 10.0).exp());
                let e = smooth((anim.age - anim.delay) / 0.32);
                if e <= 0.001 { continue; }
                let behind = w.layer < shelf.top_layer;
                anim.back += (if behind || (self.show.active && w.layer == 0) { 1.0 } else { 0.0 } - anim.back) * (1.0 - (-dt * 9.0).exp());
                let dim = 1.0 - 0.5 * anim.back;
                let tint = [dim, dim, dim, e];
                let base = anim.lift - 0.30 * (1.0 - e) - 0.12 * anim.back;
                if w.ghost { continue; }
                for (f, part) in parts { spots.push((*part, *f)); }
                // 여러 조각 창은 뒤로 밀린 만큼 넓혀 이음새를 막는다.
                let around = |f: &Frame, lift: f32| { let d = Vec3::new(f.center.x - place.head.x, 0.0, f.center.z - place.head.z).length().max(0.5); 1.0 - lift / d };
                let grow = if parts.len() > 1 { around(&parts[0].0, base) } else { 0.94 + 0.06 * e };
                for (i, (f, part)) in parts.iter().enumerate() {
                    let radius = if parts.len() == 1 { w.radius } else { 0.0 };
                    if !w.scene.is_empty() { faces.push(solid(f.quad(*part, base - 0.66, 1.25), *part, 0.0, [0.004, 0.006, 0.009, e], 0.0)); }
                    if !w.scene.is_empty() { } else if w.bare && w.layer == 0 { faces.push(Instance::new(local * f.quad(*part, base + 0.004, grow), uv(*part), tint, [16.0, 0.0, 0.0, 0.0])); } else { faces.push(panel(f.quad(*part, base, grow), *part, radius, tint)); }
                    if !behind { self.faces.push(Face { frame: settled.get(i).copied().unwrap_or(*f), rect: *part, key: k, lift: base }); }
                    if !w.bare && w.scene.is_empty() {
                        // 두께: 뒤로 몇 겹.
                        for (n, shade) in [(0.004f32, 0.055f32), (0.008, 0.035), (0.012, 0.02)] { faces.push(solid(f.quad(*part, base - n, grow), *part, radius, [shade, shade * 1.1, shade * 1.25, e], 0.0)); }
                        if parts.len() == 1 { glow.push(ring(f.quad(*part, base + 0.0006, grow), *part, radius, if hovered { [0.20, 0.75, 0.66, 0.55 * e] } else { [0.16, 0.24, 0.30, 0.35 * e * dim] }, 1.6)); }
                        let wide = [part[0] - 26.0, part[1] - 18.0, part[2] + 52.0, part[3] + 52.0];
                        glow.push(solid(f.quad(wide, base - 0.02, grow), wide, radius + 26.0, [0.0, 0.0, 0.0, 0.45 * e], 26.0 / wide[3]));
                    }
                }
                // 앞에 나온 창이 겹치는 자리는 덮는다(같은 화면 그림이라 잔상이 남는다).
                for v in shelf.wins[index + 1..].iter().filter(|v| v.layer > w.layer || v.level > w.level) {
                    let q = shelf::cut(v.rect, w.rect);
                    if q[2] < 4.0 || q[3] < 4.0 || (w.bare && w.layer == 0) { continue; }
                    let c = if w.fill[3] > 0.5 { [w.fill[0] * w.fill[0] * dim, w.fill[1] * w.fill[1] * dim, w.fill[2] * w.fill[2] * dim, e] } else { [0.006, 0.007, 0.009, e] };
                    for (f, part) in parts { let p = shelf::cut(q, *part); if p[2] > 1.0 && p[3] > 1.0 { faces.push(solid(f.quad(p, base + 0.0018, grow), p, 0.0, c, 0.0)); } }
                }
                if !w.scene.is_empty() { self.scene(&mut cuts, &mut glow, live, list, w, parts[0].0, local, base, e, dim); }
                if hovered { if let Some(h) = &mut self.hover { h.lift = base.max(0.0); } }
                if e < 0.98 || w.plates.len() > 220 { continue; }
                let frame_at = |x: f32| parts.iter().min_by(|a, b| (a.0.at[0] - x).abs().total_cmp(&(b.0.at[0] - x).abs())).map(|p| p.0).unwrap();
                let whole = shelf::area(w.rect);
                let lift_of = |depth: u32, button: bool| base + depth as f32 * if button { 0.0075 } else { 0.0035 };
                for p in &w.plates {
                    if shelf::area(p.rect) > whole * 0.55 || (parts.len() > 1 && p.rect[2] > 420.0) { continue; }
                    let f = frame_at(p.rect[0] + p.rect[2] * 0.5);
                    let lift = lift_of(p.depth, p.button);
                    let shadow = [p.rect[0] - 3.0, p.rect[1] - 1.0, p.rect[2] + 6.0, p.rect[3] + 6.0];
                    glow.push(solid(f.quad(shadow, lift_of(p.depth - 1, p.button) + 0.0012, 1.0), shadow, p.radius + 3.0, [0.0, 0.0, 0.0, 0.55 * dim], 5.0 / shadow[3]));
                    faces.push(panel(f.quad(p.rect, lift, 1.0), p.rect, p.radius, tint));
                    if p.button { for n in [0.0016f32, 0.0032] { faces.push(solid(f.quad(p.rect, lift - n, 1.0), p.rect, p.radius, [p.fill[0] * p.fill[0] * 0.35 + 0.004, p.fill[1] * p.fill[1] * 0.35 + 0.004, p.fill[2] * p.fill[2] * 0.35 + 0.005, e], 0.0)); } }
                }
                for c in &w.cuts {
                    let Some(cmd) = list.get(c.index) else { continue };
                    let Some((cell, color)) = live.cell(cmd) else { continue };
                    let f = frame_at(c.rect[0] + c.rect[2] * 0.5);
                    let at = lift_of(c.depth, false) + 0.0016;
                    if let Some(u) = c.under { let m = f.quad(c.rect, at, 1.0); faces.push(solid(m, c.rect, 0.0, [u[0] * u[0], u[1] * u[1], u[2] * u[2], dim], 0.0)); }
                    let lift = at + if c.big { 0.030 } else { 0.010 };
                    let grow = if c.big { 1.06 } else { 1.0 };
                    if c.big { let s = [c.rect[0], c.rect[1] + c.rect[3] * 0.86, c.rect[2], c.rect[3] * 0.16]; glow.push(solid(f.quad(s, at + 0.001, 1.0), s, s[3] * 0.5, [0.0, 0.0, 0.0, 0.5], 0.5)); }
                    let (fx, fy) = if cmd.kind == crate::live::frame::Kind::Sprite { (if cmd.raw[0x90] != 0 { -1.0 } else { 1.0 }, if cmd.raw[0x91] != 0 { -1.0 } else { 1.0 }) } else { (1.0, 1.0) };
                    cuts.push(Instance::new(local * f.quad(c.rect, lift, grow) * Mat4::from_scale(Vec3::new(fx, fy, 1.0)), cell.uv, [color[0] * dim, color[1] * dim, color[2] * dim, color[3] * e], [11.0, 0.15, 0.0, cell.page as f32]));
                }
                // 가리킨 자리 띄우기
                let active = self.hover.as_ref().filter(|h| h.key == k).and_then(|h| h.zone).and_then(|q| w.zones.iter().find(|z| z.0 == q).copied()).map(|z| (z, pointer.is_some_and(|p| p.2)));
                if let Some((z, pressed)) = active {
                    if shelf::area(z.0) < whole * 0.7 {
                        let pop = self.pops.entry((k, z.0.map(|v| v as i32))).or_insert(Pop { value: 0.0, seen: tick, radius: z.1 });
                        pop.seen = tick;
                        let goal = if pressed { 0.35 } else { 1.0 };
                        pop.value += (goal - pop.value) * (1.0 - (-dt * 18.0).exp());
                    }
                }
                for ((pk, pr), pop) in self.pops.iter_mut().filter(|p| p.0 .0 == k) {
                    let _ = pk;
                    if pop.seen != tick { pop.value *= (-dt * 12.0).exp(); }
                    let v = pop.value;
                    if v < 0.02 { continue; }
                    let r = pr.map(|v| v as f32);
                    let f = frame_at(r[0] + r[2] * 0.5);
                    let depth = w.plates.iter().find(|p| p.rect.map(|v| v as i32) == *pr).map(|p| lift_of(p.depth, p.button)).unwrap_or(base + 0.001);
                    let lift = depth + 0.001 + 0.017 * v;
                    if let Some(h) = &mut self.hover { if h.key == k && h.zone.is_some_and(|z| z.map(|v| v as i32) == *pr) { h.lift = lift.max(0.0); } }
                    let radius = pop.radius.max(6.0);
                    let shadow = [r[0] - 8.0, r[1] - 4.0, r[2] + 16.0, r[3] + 18.0];
                    if !(parts.len() > 1 && r[2] > 420.0) {
                    glow.push(solid(f.quad(shadow, depth + 0.002, 1.0), shadow, radius + 8.0, [0.0, 0.0, 0.0, 0.6 * v], 12.0 / shadow[3]));
                    }
                    let bright = [1.0 + 0.25 * v, 1.0 + 0.25 * v, 1.0 + 0.25 * v, 1.0];
                    if parts.len() > 1 && r[2] > 420.0 {
                        // 굽은 창의 긴 줄: 벽을 따라 조각으로.
                        for (f, part) in parts {
                            let q = shelf::cut(r, *part);
                            if q[2] <= 1.0 { continue; }
                            faces.push(panel(f.quad(q, lift, around(f, lift)), q, 0.0, bright));
                            for y in [q[1], q[1] + q[3] - 2.0] { let e = [q[0], y, q[2], 2.0]; glow.push(solid(f.quad(e, lift + 0.0012, around(f, lift)), e, 0.0, [0.22, 0.85, 0.72, 0.9 * v], 0.0)); }
                        }
                        continue;
                    }
                    faces.push(panel(f.quad(r, lift, 1.0 + 0.02 * v), r, radius, bright));
                    glow.push(ring(f.quad(r, lift + 0.0012, 1.0 + 0.02 * v), r, radius, [0.22, 0.85, 0.72, 0.9 * v], 2.4));
                }
            }
        }
        // 무대: 지금 나오면 그대로, 지나갔으면 관리 화면(왼쪽 메뉴)이 나올 때까지 어둡게 배경으로 남긴다.
        self.time += dt;
        if !shelf.stage.is_empty() {
            let pictures: Vec<Cmd> = shelf.stage.iter().filter_map(|s| list.get(s.1).cloned()).collect();
            if pictures.len() >= self.arena.len() || shelf.fade < 0.1 { if shelf.fade < 0.5 { self.arena = pictures.clone(); } }
            self.arena_light = (1.0 - shelf.fade).clamp(0.0, 1.0);
            Self::stage(&mut cuts, live, &pictures, self.arena_light, place, local, self.time);
        } else if !self.arena.is_empty() {
            let menu = shelf.wins.iter().any(|w| w.rect[0] <= 8.0 && w.rect[3] >= 600.0 && w.rect[2] <= 420.0 && w.layer == 0);
            if menu { self.arena.clear(); self.arena_light = 0.0; } else {
                self.arena_light += (0.11 - self.arena_light) * (1.0 - (-dt * 2.0).exp());
                let arena = std::mem::take(&mut self.arena);
                let mut back = Vec::new();
                Self::stage(&mut back, live, &arena, self.arena_light, place, local, self.time);
                self.arena = arena;
                back.extend(cuts.drain(..));
                cuts = back;
            }
        }
        self.anims.retain(|_, a| a.seen == tick);
        self.pops.retain(|_, p| p.value > 0.02 || p.seen == tick);
        faces.truncate(2048);
        glow.truncate(4096);
        cuts.truncate(1020);
        if let Some((cell, size)) = self.note.as_ref().and_then(|n| live.text(n)) {
            let (a, rad, h) = (place.yaw, 1.45f32, 0.062);
            let at = Vec3::new(place.head.x - a.sin() * rad, place.head.y - 0.36, place.head.z - a.cos() * rad);
            cuts.push(Instance::new(local * Mat4::from_scale_rotation_translation(Vec3::new(h * size.0 / size.1, h, 1.0), Quat::from_rotation_y(a), at), cell.uv, { let p = 0.65 + 0.35 * (self.time * 4.0).sin(); [0.55 * p, 0.95 * p, 0.85 * p, 1.0] }, [11.0, 0.15, 0.0, cell.page as f32]));
        }
        gpu.update(self.face_id, &faces)?;
        gpu.update(self.glow_id, &glow)?;
        gpu.update(self.cut_id, &cuts)?;
        let wall = Self::wall(shelf, place, hole, held).0;
        self.show.update(gpu, live, commands, shelf, place, &wall, &spots, local, dt)?;
        self.ambient(gpu, place, local)
    }
    fn ambient(&mut self, gpu: &mut Gpu, place: &Place, local: Mat4) -> Result<()> {
        let center = Vec3::new(place.head.x, place.floor, place.head.z);
        gpu.update(self.dome_id, &[Instance::new(local * Mat4::from_translation(Vec3::new(center.x, place.floor + 1.2, center.z)), [0.0; 4], [1.0; 4], [0.0; 4])])?;
        let disc = |radius: f32, h: f32, color: [f32; 4], mode: f32| Instance::new(local * Mat4::from_scale_rotation_translation(Vec3::new(radius * 2.0, radius * 2.0, 1.0), Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2), center + Vec3::new(0.0, h, 0.0)), [0.0; 4], color, [mode, 0.0, 0.0, 0.0]);
        gpu.update(self.floor_id, &[disc(5.5, 0.0, [0.05, 0.09, 0.11, 0.9], 7.0), disc(2.2, 0.002, [0.10, 0.32, 0.30, 0.16], 5.0), disc(1.1, 0.002, [0.10, 0.32, 0.30, 0.22], 5.0)])?;
        Ok(())
    }
    /// 둘러싸는 무대: 층마다 반지름을 달리해 세운다. 처음 자리에서 보면 원래 그림과 같은 구도다.
    fn stage(cuts: &mut Vec<Instance>, live: &mut Live, list: &[Cmd], light: f32, place: &Place, local: Mat4, time: f32) {
        if light < 0.02 { return; }
        let class = |c: &Cmd| c.key.rsplit('/').next().unwrap_or("").split('#').next().unwrap_or("").to_string();
        let group = |n: &str| if n.contains("player") || n.contains("coach") { 40 } else if n.contains("audience") { 90 } else if n.contains("chair") { 20 } else if n.contains("desk") { 60 } else if n.contains("logo") { 10 } else if n.contains("front") { 80 } else if n.contains("top") { 30 } else if n.contains("floor") { 50 } else { 0 };
        let k = 0.00112f32; // UI 픽셀당 각도(rad): 화면 전체가 약 123도
        let mut rows: Vec<(i32, usize)> = list.iter().enumerate().map(|(i, c)| (group(&class(c)), i)).collect();
        rows.sort_by_key(|r| r.0);
        let mut step = 0.0f32;
        for (depth, index) in rows {
            let c = &list[index];
            let Some((cell, color)) = live.cell(c) else { continue };
            step += 0.0006;
            let rad = 6.4 - 0.042 * depth as f32 - step;
            let mut r = [c.f(120), c.f(124), c.f(128), c.f(132)];
            // 관중은 저마다 조금씩 들썩인다.
            if depth == 90 { r[1] -= ((time * 5.0 + r[0] * 0.013 + r[1] * 0.02).sin().max(0.0)) * 7.0; }
            let flip = c.raw[173] != 0;
            let n = ((r[2] * k / 0.09).ceil() as usize).clamp(1, 40);
            for i in 0..n {
                let (x, w) = (r[0] + r[2] * i as f32 / n as f32, r[2] / n as f32);
                let a = place.yaw - (x + w * 0.5 - 960.0) * k;
                let chord = 2.0 * rad * (w * k * 0.5).tan();
                let m = Mat4::from_scale_rotation_translation(Vec3::new(chord * 1.002, r[3] * k * rad, 1.0), Quat::from_rotation_y(a), Vec3::new(place.head.x - a.sin() * rad, place.head.y - 0.05 + (540.0 - r[1] - r[3] * 0.5) * k * rad, place.head.z - a.cos() * rad));
                let u = if flip { 1.0 - (i as f32 + 1.0) / n as f32 } else { i as f32 / n as f32 };
                let uv = [cell.uv[0] + cell.uv[2] * u, cell.uv[1], cell.uv[2] / n as f32, cell.uv[3]];
                cuts.push(Instance::new(local * m * Mat4::from_scale(Vec3::new(if flip { -1.0 } else { 1.0 }, 1.0, 1.0)), uv, [color[0] * light, color[1] * light, color[2] * light, color[3]], [11.0, 0.02, 0.0, cell.page as f32]));
            }
        }
    }
    /// 배경 그림들은 뒤로 깊게, 가구·사람은 앞으로: 상자 안을 들여다보는 장면.
    #[allow(clippy::too_many_arguments)]
    fn scene(&self, cuts: &mut Vec<Instance>, glow: &mut Vec<Instance>, live: &mut Live, list: &[Cmd], w: &Win, f: Frame, local: Mat4, base: f32, e: f32, dim: f32) {
        use crate::live::frame::Kind;
        let win = w.rect;
        let (mut depth, mut covers, mut last, mut step) = (-0.62f32, 0, String::new(), 0.0f32);
        for &index in &w.scene {
            let Some(c) = list.get(index) else { continue };
            if c.kind == Kind::Text {
                let (tx, ty) = (c.f(152) + c.f(160) * 0.5, c.f(156) + c.f(164) * 0.5);
                if w.plates.iter().any(|p| shelf::has(p.rect, tx, ty)) { continue; }
                if let Some((cell, size)) = live.text(&c.text) { let (h, width) = (c.f(164).min(c.f(192) * 1.5), 0.0f32); let _ = width; let scale = h.max(12.0) / size.1; let r = [c.f(152) + c.f(160) * 0.5 - size.0 * scale * 0.5, c.f(156) + c.f(164) * 0.5 - size.1 * scale * 0.5, size.0 * scale, size.1 * scale]; cuts.push(Instance::new(local * f.quad(r, base + 0.02, 1.0), cell.uv, [dim, dim, dim, e], [11.0, 0.15, 0.0, cell.page as f32])); }
                continue;
            }
            let (r, fx) = match c.kind { Kind::Patch => ([c.f(120), c.f(124), c.f(128), c.f(132)], c.raw[173] != 0), _ => continue };
            let Some((cell, color)) = live.cell(c) else { continue };
            let class = c.key.rsplit('/').next().unwrap_or("").to_string();
            let cover = shelf::area(r) > 0.6 * shelf::area(win);
            if cover { if class != last { covers += 1; } depth = [-0.62, -0.50, -0.30, -0.16, -0.12][(covers as usize).min(4)]; step = 0.0; }
            else if class != last { depth = if class.contains("wallpaper") { depth - 0.02 } else if class.contains("window") { depth + 0.006 } else if class.contains("furniture") { -0.075 } else if class.contains("wall") { -0.085 } else if class.contains("player") || class.contains("coach") { -0.03 } else { depth + 0.01 }; step = 0.0; }
            step += 0.00025;
            last = class;
            // 창 밖으로 나간 부분은 잘라 낸다(멀리 있는 배경은 시차만큼 넉넉히).
            let q = shelf::cut(r, win);
            if q[2] < 0.5 || q[3] < 0.5 { continue; }
            let (a, b) = ((q[0] - r[0]) / r[2], (q[1] - r[1]) / r[3]);
            let (sw, sh) = (q[2] / r[2], q[3] / r[3]);
            let u0 = if fx { 1.0 - a - sw } else { a };
            let uv = [cell.uv[0] + cell.uv[2] * u0, cell.uv[1] + cell.uv[3] * b, cell.uv[2] * sw, cell.uv[3] * sh];
            let grow = if cover { 1.0 - depth * 0.30 } else { 1.0 };
            let m = f.quad(q, base + depth + step, grow) * Mat4::from_scale(Vec3::new(if fx { -1.0 } else { 1.0 }, 1.0, 1.0));
            cuts.push(Instance::new(local * m, uv, [color[0] * dim, color[1] * dim, color[2] * dim, color[3] * e], [11.0, 0.02, 0.0, cell.page as f32]));
        }
        // 액자.
        for (n, width, c) in [(0.0f32, 9.0f32, [0.03, 0.045, 0.06, e]), (0.012, 3.0, [0.10, 0.20, 0.24, 0.8 * e])] { glow.push(Instance::new(local * f.quad([win[0] - 8.0, win[1] - 8.0, win[2] + 16.0, win[3] + 16.0], base + n, 1.0), [0.0; 4], c, [15.0, 14.0 / win[3], win[2] / win[3], width / win[3]])); }
    }
    /// 명령이 없을 때(브리지 꺼짐): 게임 화면 전체를 둥근 화면 하나로.
    pub fn screen(&mut self, gpu: &mut Gpu, place: &Place, local: Mat4, crop: [f32; 4]) -> Result<()> {
        self.faces.clear();
        self.tick += 1;
        let (n, radius, scale) = (24usize, 1.9f32, 0.00142f32);
        let mut out = Vec::new();
        for i in 0..n {
            let rect = [UI[0] * i as f32 / n as f32, 0.0, UI[0] / n as f32, UI[1]];
            let px = rect[0] + rect[2] * 0.5;
            let a = place.yaw - (px - 960.0) * scale / radius;
            let f = Frame { center: Vec3::new(place.head.x - a.sin() * radius, place.head.y - 0.03, place.head.z - a.cos() * radius), rotation: Quat::from_rotation_y(a), at: [px, 540.0], scale };
            let chord = 2.0 * radius * (rect[2] * scale / radius * 0.5).tan() / (rect[2] * scale);
            let m = Mat4::from_scale_rotation_translation(Vec3::new(rect[2] * scale * chord, rect[3] * scale, 1.0), f.rotation, f.center);
            out.push(Instance::new(local * m, [crop[0] + crop[2] * rect[0] / UI[0], crop[1], crop[2] * rect[2] / UI[0], crop[3]], [1.0; 4], [12.0, 0.0, 0.0, 0.0]));
            self.faces.push(Face { frame: f, rect, key: (0, 0, 0, 0, 99), lift: 0.0 });
        }
        gpu.update(self.face_id, &out)?;
        gpu.update(self.glow_id, &[])?;
        gpu.update(self.cut_id, &[])?;
        Ok(())
    }
    /// 기다리는 동안의 안내 글.
    pub fn message(&mut self, gpu: &mut Gpu, live: &mut Live, place: &Place, local: Mat4, lines: &[&str]) -> Result<()> {
        self.clear(gpu)?;
        let mut out = Vec::new();
        let (a, rad) = (place.yaw, 1.9f32);
        let center = Vec3::new(place.head.x - a.sin() * rad, place.head.y, place.head.z - a.cos() * rad);
        for (i, line) in lines.iter().enumerate() {
            let Some((cell, size)) = live.text(line) else { continue };
            let h = [0.17f32, 0.075, 0.05][i.min(2)];
            let y = [0.18f32, 0.0, -0.16][i.min(2)];
            let color = [[0.75, 1.0, 0.94, 1.0], [0.85, 0.92, 0.95, 1.0], [0.35, 0.45, 0.5, 1.0]][i.min(2)];
            out.push(Instance::new(local * Mat4::from_scale_rotation_translation(Vec3::new(h * size.0 / size.1, h, 1.0), Quat::from_rotation_y(a), center + Vec3::Y * y), cell.uv, color, [11.0, 0.15, 0.0, cell.page as f32]));
        }
        gpu.update(self.cut_id, &out)?;
        self.ambient(gpu, place, local)
    }
    pub fn clear(&mut self, gpu: &mut Gpu) -> Result<()> { self.hover = None; self.faces.clear(); for id in [self.face_id, self.glow_id, self.cut_id] { gpu.update(id, &[])?; } Ok(()) }
    pub fn faces(&self) -> Vec2 { Vec2::new(self.faces.len() as f32, self.pops.len() as f32) }
}
