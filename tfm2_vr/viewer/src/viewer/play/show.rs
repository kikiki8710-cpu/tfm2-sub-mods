//! 밴·픽 연출: 게임이 그리는 카드(변환 + 카드 명령)를 눈앞에 떠오르는 입체 카드로 다시 만든다.
//! 픽 = 돌면서 튀어나와 자리로 날아감, 밴 = 흑백 카드가 베여 두 동강. 자리·크기·투명도는 게임 값을 그대로 따른다.
use super::{room::{Frame, Place, Wall}, shelf::{Rect, Shelf}};
use crate::{geometry::{self, Instance}, gpu::Gpu, live::{frame::{Cmd, Frame as Commands, Kind}, Live}};
use anyhow::Result;
use glam::{Mat4, Quat, Vec3};

const PX: f32 = 0.00165; // 눈앞에서 카드 1픽셀(m)
const NEAR: f32 = 1.18; // 눈앞 거리(m)
fn lin(v: f32) -> f32 { v.clamp(0.0, 1.0).powf(2.2) }
fn lin3(c: [f32; 4]) -> [f32; 3] { [lin(c[0]), lin(c[1]), lin(c[2])] }
fn ease_out(t: f32) -> f32 { let t = t.clamp(0.0, 1.0); 1.0 - (1.0 - t).powi(3) }
fn back_out(t: f32) -> f32 { let t = t.clamp(0.0, 1.0) - 1.0; 1.0 + 2.2 * t * t * t + 1.2 * t * t }
fn noise(n: u32) -> f32 { let mut x = n.wrapping_mul(0x9E37_79B9) ^ 0x5bd1_e995; x ^= x >> 15; x = x.wrapping_mul(0x85eb_ca6b); x ^= x >> 13; (x & 0xffff) as f32 / 65535.0 }
/// 눈앞에 떠 있는 카드 하나. 내용은 게임에서, 움직임은 여기서(게임 연출은 0.6초라 VR 에선 너무 짧다).
struct Card { name: String, ban: bool, age: f32, color: [f32; 3], body: Vec<Cmd>, origin: [f32; 2], hero: Option<Cmd>, cut: Option<([Cut; 2], [f32; 4])>, spawn: Option<(Vec3, Quat, f32)>, hold: f32, release: Option<f32>, target: [f32; 2], wait: f32, half: [f32; 2] }
struct Tail { age: f32, at: Vec3, rotation: Quat, color: [f32; 3], size: f32, ban: bool }
pub struct Stage { id: usize, cards: Vec<Card>, tracking: bool, tails: Vec<Tail>, pub active: bool }
const INTRO: f32 = 0.40;
const FLY: f32 = 0.50;
const SLASH: f32 = 0.42; // 밴: 베이는 순간
/// 자르는 직선: nx·x + ny·y + c > 0 인 쪽만 남긴다(카드 좌표 px).
type Cut = [f32; 3];

fn rect_of(live: &Live, c: &Cmd) -> Option<Rect> {
    Some(match c.kind {
        Kind::Box => [c.f(88), c.f(92), c.f(96), c.f(100)],
        Kind::Patch => c.patch_rect(),
        Kind::Text => [c.f(152), c.f(156), c.f(160), c.f(164)],
        Kind::Sprite => { let s = live.sprite_size(c)?; [c.f(0x78) - c.f(0x88) * s[0], c.f(0x7c) - c.f(0x8c) * s[1], s[0], s[1]] }
        _ => return None,
    })
}
struct Pen<'a> { out: &'a mut Vec<Instance>, local: Mat4 }
impl Pen<'_> {
    fn solid(&mut self, m: Mat4, r: Rect, radius: f32, color: [f32; 3], a: f32, soft: f32) { if a > 0.004 { self.out.push(Instance::new(self.local * m, [0.0; 4], [color[0], color[1], color[2], a.min(1.0)], [14.0, (radius / r[3].max(1.0)).min(0.5), r[2] / r[3].max(1.0), soft])); } }
    fn ring(&mut self, m: Mat4, r: Rect, radius: f32, color: [f32; 3], a: f32, width: f32) { if a > 0.004 { self.out.push(Instance::new(self.local * m, [0.0; 4], [color[0], color[1], color[2], a.min(1.0)], [15.0, (radius / r[3].max(1.0)).min(0.5), r[2] / r[3].max(1.0), width / r[3].max(1.0)])); } }
    /// 더해지는 둥근 빛(가로세로를 달리하면 빛줄기).
    fn light(&mut self, m: Mat4, color: [f32; 3], a: f32, sharp: f32) { if a > 0.004 { self.out.push(Instance::new(self.local * m, [0.0; 4], [color[0], color[1], color[2], a], [17.0, 0.0, sharp, 0.0])); } }
    /// 카드 명령들을 층마다 앞뒤로 띄워 그린다. origin = 카드 가운데(명령 좌표).
    fn card(&mut self, live: &mut Live, cmds: &[Cmd], f: &Frame, origin: [f32; 2], alpha: f32, cut: Option<Cut>, glow: [f32; 3], swell: f32, out: f32) {
        let depth = f.scale / PX;
        let slab = cmds.iter().filter(|c| c.kind == Kind::Box && c.box_fill().0[3] > 0.5).max_by(|a, b| (a.f(96) * a.f(100)).total_cmp(&(b.f(96) * b.f(100)))).map(|c| [c.f(88) - origin[0], c.f(92) - origin[1], c.f(96), c.f(100)]);
        // 뒤쪽 후광과 카드 두께.
        if let Some(r) = slab {
            if cut.is_none() {
                let wide = [r[0] - 60.0, r[1] - 60.0, r[2] + 120.0, r[3] + 120.0];
                self.solid(f.quad(wide, -0.045 * depth, 1.0), wide, 60.0, glow, 0.30 * alpha * swell, 60.0 / wide[3]);
                for n in [3, 2, 1] { let k = n as f32; self.solid(f.quad(r, -0.007 * k * depth, 1.0), r, 18.0, glow.map(|v| 0.02 + v * 0.10 * (4.0 - k)), alpha, 0.0); }
            } else { for n in [3, 2, 1] { self.half(f.quad(r, -0.006 * n as f32 * depth, 1.0), r, [0.05, 0.05, 0.06], alpha, cut.unwrap()); } }
        }
        for (i, c) in cmds.iter().enumerate() {
            let Some(r) = rect_of(live, c) else { continue };
            let r = [r[0] - origin[0], r[1] - origin[1], r[2], r[3]];
            let lift = ((c.layer() - 1210) as f32 * 0.011).clamp(-0.02, 0.06) * depth;
            let m = f.quad(r, lift, 1.0);
            match c.kind {
                Kind::Box => {
                    let (fill, border) = c.box_fill();
                    let radius = c.f(12).clamp(0.0, 60.0);
                    if let Some(k) = cut { self.half(m, r, lin3(fill), fill[3] * alpha, k); continue; }
                    self.solid(m, r, radius, lin3(fill), fill[3], 0.0);
                    if let Some(b) = border { self.ring(f.quad(r, lift + 0.0006, 1.0), r, radius, lin3(b), b[3], 4.0); }
                }
                Kind::Patch | Kind::Sprite => {
                    let Some((cell, color)) = live.cell(c) else { continue };
                    let a = color[3] * if cut.is_some() { alpha } else { 1.0 };
                    // 기본 카드의 그림은 챔피언 자신: 카드 앞으로 걸어 나온다.
                    let walk = if cut.is_none() && c.key.contains("/champions/") { out } else { 0.0 };
                    let m = if walk > 0.0 { let q = [r[0] + 8.0, r[1] + 10.0, r[2], r[3]]; self.out.push(Instance::new(self.local * f.quad(q, lift + 0.03 * walk * depth, 1.0), cell.uv, [0.0, 0.0, 0.0, 0.5 * a * walk], [11.0, 0.02, 0.0, cell.page as f32])); f.quad(r, lift + 0.15 * walk * depth, 1.0 + 0.10 * walk) } else { m };
                    if let Some(k) = cut { let p = plane(k, r); self.out.push(Instance::new(self.local * m, cell.uv, [p[0], p[1], p[2], a], [18.0, 0.02, 1.0, cell.page as f32])); }
                    else { self.out.push(Instance::new(self.local * m, cell.uv, [color[0], color[1], color[2], a], [11.0, 0.02, 0.0, cell.page as f32])); }
                }
                Kind::Text => {
                    // 테두리는 같은 글자를 여러 번 그린 것: 마지막 것만.
                    if cmds.get(i + 1).is_some_and(|n| n.kind == Kind::Text && n.text == c.text) { continue; }
                    let Some((cell, size)) = live.text(&c.text) else { continue };
                    let h = c.f(192) * c.f(196).clamp(0.2, 4.0) * 1.4;
                    let w = (h * size.0 / size.1).min(r[2]);
                    let q = [r[0] + (r[2] - w) * 0.5, r[1] + (r[3] - h) * 0.5, w, h];
                    let m = f.quad(q, lift + 0.012 * depth, 1.0);
                    let col = c.rgba(168);
                    if let Some(k) = cut { let p = plane(k, q); self.out.push(Instance::new(self.local * m, cell.uv, [p[0], p[1], p[2], alpha], [18.0, 0.1, 0.0, cell.page as f32])); }
                    else { let l = lin3(col); self.out.push(Instance::new(self.local * m, cell.uv, [l[0], l[1], l[2], col[3]], [11.0, 0.1, 0.0, cell.page as f32])); }
                }
                _ => {}
            }
        }
    }
    fn half(&mut self, m: Mat4, r: Rect, color: [f32; 3], a: f32, k: Cut) { let p = plane(k, r); self.out.push(Instance::new(self.local * m, [color[0], color[1], color[2], a.min(1.0)], [p[0], p[1], p[2], 1.0], [19.0, 0.0, 0.0, 0.0])); }
}
/// 카드 좌표의 직선을 조각 안(0~1) 좌표로.
fn plane(k: Cut, r: Rect) -> [f32; 3] { [k[0] * r[2], k[1] * r[3], k[0] * r[0] + k[1] * r[1] + k[2]] }
/// 조각 메시(32B 점: RGBA·xy·uv)에서 비스듬한 변 = 자른 선.
fn cut_of(c: &Cmd) -> Option<(Cut, [f32; 4])> {
    let v: Vec<[f32; 8]> = c.data.chunks_exact(32).map(|p| std::array::from_fn(|k| f32::from_le_bytes(p[k * 4..k * 4 + 4].try_into().unwrap()))).collect();
    if v.len() < 3 { return None; }
    let mid = [v.iter().map(|p| p[4]).sum::<f32>() / v.len() as f32, v.iter().map(|p| p[5]).sum::<f32>() / v.len() as f32];
    for i in 0..v.len() {
        let (a, b) = (&v[i], &v[(i + 1) % v.len()]);
        let (dx, dy) = (b[4] - a[4], b[5] - a[5]);
        if dx.abs() < 2.0 || dy.abs() < 2.0 { continue; }
        let l = (dx * dx + dy * dy).sqrt();
        let (mut nx, mut ny) = (-dy / l, dx / l);
        let mut d = -(nx * a[4] + ny * a[5]);
        if nx * mid[0] + ny * mid[1] + d < 0.0 { nx = -nx; ny = -ny; d = -d; }
        return Some(([nx, ny, d], [a[4], a[5], b[4], b[5]]));
    }
    None
}
/// 벽에 놓인 창들의 (UI 사각형, 틀): 날아갈 자리를 찾는다.
pub type Spots = Vec<(Rect, Frame)>;
fn spot(spots: &Spots, wall: &Wall, place: &Place, x: f32, y: f32) -> (Vec3, Quat, f32) {
    let found = spots.iter().filter(|s| super::shelf::has(s.0, x, y)).min_by(|a, b| (a.0[2] * a.0[3]).total_cmp(&(b.0[2] * b.0[3])));
    match found { Some((_, f)) => (f.point(x, y, 0.05), f.rotation, f.scale), None => { let (p, r) = wall.at(place, x, y, wall.radius - 0.08, 1.0); (p, r, wall.scale) } }
}
impl Stage {
    pub fn new(gpu: &mut Gpu) -> Result<Self> {
        let mut m = geometry::mesh("live9roomShow", geometry::quad(), vec![], true, false);
        m.dynamic = true;
        Ok(Self { id: gpu.add_mesh(&m, 1024)?, cards: Vec::new(), tracking: false, tails: Vec::new(), active: false })
    }
    #[allow(clippy::too_many_arguments)]
    pub fn update(&mut self, gpu: &mut Gpu, live: &mut Live, frame: &Commands, shelf: &Shelf, place: &Place, wall: &Wall, spots: &Spots, local: Mat4, dt: f32) -> Result<()> {
        let mut out: Vec<Instance> = Vec::new();
        let list = frame.list("UI");
        self.active = false;
        let mut seen = false;
        for s in &shelf.shows {
            let Some(cmds) = list.get(s.start..=s.end) else { continue };
            // 카드가 아직 눈앞에 있으면 안내 글은 그 뒤에.
            if s.banner && self.cards.iter().any(|c| c.release.is_none()) { continue; }
            if s.banner { banner(&mut Pen { out: &mut out, local }, live, cmds, s.t, place, wall); self.active = true; continue; }
            if seen { continue; }
            seen = true;
            self.read(live, frame, list, cmds, s.t, place, wall, spots);
        }
        if !seen { self.tracking = false; }
        let mut pen = Pen { out: &mut out, local };
        for c in &mut self.cards {
            c.age += dt;
            // 그림이 아직 준비되지 않았으면 뒷면을 보인 채 잠깐 기다린다.
            if c.wait < 0.7 && !c.ready(live) { c.wait += dt; }
            let landed = c.draw(&mut pen, live, place, wall, spots);
            if let Some(t) = landed { self.tails.push(t); }
        }
        self.cards.retain(|c| c.release.is_none_or(|r| c.age < r + FLY));
        if !self.cards.is_empty() { self.active = true; }
        for t in &mut self.tails {
            t.age += dt;
            let k = t.age / 0.55;
            let f = Frame { center: t.at, rotation: t.rotation, at: [0.0, 0.0], scale: t.size };
            let fade = (1.0 - k).max(0.0);
            let grow = 0.9 + 0.6 * ease_out(k);
            let r = [-140.0 * grow, -62.0 * grow, 280.0 * grow, 124.0 * grow];
            pen.ring(f.quad(r, 0.02, 1.0), r, 14.0, t.color, 0.8 * fade * fade, 6.0);
            let g = [-260.0, -190.0, 520.0, 380.0];
            pen.light(f.quad(g, 0.015, 1.0), t.color, if t.ban { 0.35 } else { 0.8 } * fade * fade, 4.0);
            for n in 0..18u32 { let a = noise(n * 7 + 1) * std::f32::consts::TAU; let d = (0.04 + 0.20 * noise(n * 7 + 2)) * ease_out(k); let p = t.at + t.rotation * Vec3::new(a.cos() * d * 1.3, a.sin() * d * 0.8, 0.02 + 0.12 * noise(n * 7 + 3) * k); pen.light(Mat4::from_scale_rotation_translation(Vec3::splat(0.022 * fade), t.rotation, p), t.color, 1.3 * fade, 3.0); }
        }
        self.tails.retain(|t| t.age < 0.55);
        out.truncate(1024);
        gpu.update(self.id, &out)
    }
    /// 게임이 지금 그리는 카드를 읽는다.
    #[allow(clippy::too_many_arguments)]
    fn read(&mut self, live: &mut Live, frame: &Commands, list: &[Cmd], cmds: &[Cmd], t: [f32; 4], place: &Place, wall: &Wall, spots: &Spots) {
        let pieces: Vec<&Cmd> = cmds.iter().filter(|c| c.kind == Kind::Other(5)).collect();
        let sliced = pieces.len() >= 2;
        let body: Vec<Cmd> = if sliced { frame.list(pieces[0].key.trim_start_matches('#')).to_vec() } else { cmds.iter().filter(|c| matches!(c.kind, Kind::Box | Kind::Patch | Kind::Sprite | Kind::Text)).cloned().collect() };
        let Some(outer) = body.iter().filter(|c| c.kind == Kind::Box).max_by(|a, b| (a.f(96) * a.f(100)).total_cmp(&(b.f(96) * b.f(100)))) else { return };
        let origin = if sliced { [outer.f(88) + outer.f(96) * 0.5, outer.f(92) + outer.f(100) * 0.5] } else { [0.0, 0.0] };
        let tint = outer.box_fill().1.unwrap_or(outer.box_fill().0);
        let ban = sliced || tint[..3].iter().cloned().fold(0.0f32, f32::max) - tint[..3].iter().cloned().fold(1.0f32, f32::min) < 0.2;
        let name = body.iter().rev().find(|c| c.kind == Kind::Text).map(|c| c.text.clone()).unwrap_or_default();
        let solid = body.iter().filter(|c| c.kind == Kind::Box).map(|c| c.box_fill().0[3]).fold(0.0f32, f32::max);
        if self.tracking && self.cards.last().is_some_and(|c| c.name != name) { self.tracking = false; }
        if !self.tracking {
            // 앞 카드는 바로 제 칸으로 보낸다.
            for c in &mut self.cards { if c.release.is_none() { c.release = Some(c.age); } }
            if self.cards.len() > 3 { self.cards.remove(0); }
            let art = body.iter().find(|c| matches!(c.kind, Kind::Patch | Kind::Sprite) && c.tex != 0).map(|c| c.key.clone()).unwrap_or_default();
            let who = art.rsplit('/').next().unwrap_or("").split('#').next().unwrap_or("").to_string();
            let tag = format!("/champions/{who}#");
            let hero = if who.is_empty() { None } else { {
                // 격자 안의 제 그림(가장 큰 것): 양옆 칸과 아래 띠의 작은 그림은 피한다.
                let inner = |c: &&Cmd| { let x = c.f(120) + c.f(128) * 0.5; let y = c.f(124) + c.f(132) * 0.5; (320.0..1600.0).contains(&x) && y < 900.0 };
                let all = || list.iter().filter(|c| c.kind == Kind::Patch && c.tex != 0 && c.key.contains(&tag));
                all().filter(inner).max_by(|a, b| (a.f(128) * a.f(132)).total_cmp(&(b.f(128) * b.f(132)))).or_else(|| all().next()).cloned()
            } };
            let spawn = hero.as_ref().map(|h| spot(spots, wall, place, h.f(120) + h.f(128) * 0.5, h.f(124) + h.f(132) * 0.5));
            let _ = live;
            let half = [outer.f(96) * 0.5, outer.f(100) * 0.5];
            self.cards.push(Card { name, ban, age: 0.0, color: if ban { [1.0, 0.16, 0.14] } else { lin3(tint) }, body: body.clone(), origin, hero: if art.contains("/champions/") { None } else { hero }, cut: None, spawn, hold: if ban { 1.25 } else { 1.15 }, release: None, target: if ban { [960.0, 1046.0] } else if tint[0] > tint[2] { [1693.0, 540.0] } else { [227.0, 540.0] }, wait: 0.0, half });
            self.tracking = true;
        }
        let Some(c) = self.cards.last_mut() else { return };
        // 흐려지기 전 내용만 갖는다(밴은 흑백이 된 뒤의 것이 남는다).
        if !sliced && solid > 0.9 { c.body = body.clone(); c.origin = origin; }
        if sliced && c.cut.is_none() {
            if let (Some(a), Some(b)) = (cut_of(pieces[0]), cut_of(pieces[1])) { c.ban = true; c.cut = Some(([a.0, b.0], a.1)); }
            if c.origin == [0.0, 0.0] && c.body.is_empty() { c.body = body; c.origin = origin; }
        }
        // 게임 카드는 가운데에서 제 칸까지 곧게, 크기(1 → 밴 0.15 · 픽 0.30)와 같은 박자로 간다. 중간 장을 놓쳐도 끝 자리를 셈한다.
        let k = (1.0 - t[3]) / if c.ban { 0.85 } else { 0.70 };
        let off = [t[0] - 960.0, t[1] - 540.0];
        if off[0].abs() + off[1].abs() > 12.0 && k > 0.03 { let k = k.min(1.0); c.target = [(960.0 + off[0] / k).clamp(0.0, 1919.0), (540.0 + off[1] / k).clamp(0.0, 1079.0)]; }
    }
}
impl Card {
    fn ready(&self, live: &mut Live) -> bool { self.age - self.wait > 0.12 || self.body.iter().filter(|c| matches!(c.kind, Kind::Patch | Kind::Sprite) && c.tex != 0).all(|c| live.cell(c).is_some()) }
    /// 그리고, 제 칸에 닿은 순간이면 도착 효과를 돌려준다.
    fn draw(&mut self, pen: &mut Pen, live: &mut Live, place: &Place, wall: &Wall, spots: &Spots) -> Option<Tail> {
        if self.release.is_none() && self.age - self.wait >= self.hold { self.release = Some(self.age); }
        let (color, ban, age) = (self.color, self.ban, self.age - self.wait);
        let [hw, hh] = self.half;
        let (center, facing) = place.spot(0.0, NEAR, place.head.y - 0.06, 0.0);
        let into = (self.age / INTRO).clamp(0.0, 1.0);
        let k = self.release.map(|r| ((self.age - r) / FLY).clamp(0.0, 1.0)).unwrap_or(0.0);
        let fly = k * k * (3.0 - 2.0 * k);
        let (goal, goal_rotation, goal_scale) = spot(spots, wall, place, self.target[0], self.target[1]);
        // 격자의 제 그림 자리에서 나와 눈앞으로, 끝나면 제 칸으로.
        let (from, from_scale) = self.spawn.map(|s| (s.0, s.2)).unwrap_or((center - facing * Vec3::Z * 0.7, wall.scale));
        let e = ease_out(into);
        let mut at = from.lerp(center, e);
        at += facing * Vec3::new(0.0, 0.10 * (std::f32::consts::PI * into).sin(), 0.10 * (std::f32::consts::PI * into).sin());
        at.y += 0.012 * (age * 2.1).sin() * (1.0 - fly);
        let to = goal - at;
        let arc = (std::f32::consts::PI * fly).sin();
        at += to * fly + facing * Vec3::new(0.0, 0.10 * arc, 0.12 * arc);
        let side = if facing.inverse().mul_vec3(to).x > 0.0 { 1.0 } else { -1.0 };
        let big = from_scale * 0.28 + (PX * (0.80 + 0.20 * back_out(self.age / (INTRO + 0.12))) - from_scale * 0.28) * e;
        let scale = big + (goal_scale * if ban { 0.15 } else { 0.30 } - big) * fly;
        let turn = facing.slerp(goal_rotation, fly);
        let spin = Quat::from_rotation_y((1.0 - ease_out(age / INTRO)) * -std::f32::consts::PI + 0.08 * (age * 1.7).sin() * (1.0 - fly) + side * 0.6 * arc) * Quat::from_rotation_x(0.05 * (age * 1.3).cos() * (1.0 - fly) - 0.35 * arc) * Quat::from_rotation_z(-side * 0.16 * arc);
        let f = Frame { center: at, rotation: turn * spin, at: [0.0, 0.0], scale };
        let depth = scale / PX;
        let alpha = if k > 0.85 { (1.0 - k) / 0.15 } else { 1.0 };
        let swell = 1.0 + 0.35 * (age * 5.0).sin().max(0.0);
        // 나는 동안 남는 꼬리.
        if k > 0.02 && k < 1.0 { for n in 1..9u32 { let b = (fly - 0.07 * n as f32).max(0.0); let arc = (std::f32::consts::PI * b).sin(); let p = center + (goal - center) * b + facing * Vec3::new(0.0, 0.10 * arc, 0.12 * arc - 0.03); pen.light(Mat4::from_scale_rotation_translation(Vec3::splat(0.26 * (1.0 - n as f32 / 10.0)), facing, p), color, 0.30 * (1.0 - n as f32 / 9.0), 4.0); } }
        let slash = age - SLASH;
        if ban && slash > 0.0 {
            let (lines, edge) = self.cut.unwrap_or_else(|| { let (ex, ey) = (hw, hw * 0.555); let l = ex.hypot(ey); ([[-ey / l, -ex / l, 0.0], [ey / l, ex / l, 0.0]], [ex, -ey, -ex, ey]) });
            let q = ease_out(slash / 0.28);
            for line in lines {
                let up = if line[1] < 0.0 { 1.0 } else { -1.0 };
                // 위 조각은 뒤로 젖혀 뜨고 아래 조각은 앞으로 숙여 떨어진다.
                let tip = Quat::from_rotation_z(-0.11 * up * q) * Quat::from_rotation_x(-0.34 * up * q);
                let push = Vec3::new(line[0] * 58.0 * q, -line[1] * 58.0 * q, 0.0) * scale + Vec3::new(0.0, -0.03 * (slash * slash).min(1.0) * (1.0 - up) * depth, 0.05 * up * q * depth);
                let piece = Frame { center: f.center + f.rotation * push, rotation: f.rotation * tip, at: [0.0, 0.0], scale };
                pen.card(live, &self.body, &piece, self.origin, alpha, Some(line), color, 1.0, 0.0);
            }
            let (a, b) = ([edge[0] * 1.35, edge[1] * 1.35], [edge[2] * 1.35, edge[3] * 1.35]);
            let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
            let len = (dx * dx + dy * dy).sqrt().max(1.0);
            let grow = ease_out(slash / 0.12);
            let angle = Quat::from_rotation_z((-dy).atan2(dx));
            let mid = f.point(a[0] + dx * 0.5 * grow, a[1] + dy * 0.5 * grow, 0.075 * depth);
            let life = (1.0 - (slash - 0.10).max(0.0) / 0.55).clamp(0.0, 1.0) * alpha;
            let m = |w: f32, h: f32| Mat4::from_scale_rotation_translation(Vec3::new(w * scale * grow, h * scale, 1.0), f.rotation * angle, mid);
            pen.light(m(len * 1.3, 170.0), color, 1.2 * life, 5.0);
            pen.light(m(len * 1.2, 46.0), [1.0, 0.72, 0.66], 1.8 * life, 6.0);
            pen.light(m(len * 1.1, 15.0), [1.0, 1.0, 1.0], 2.6 * life, 6.0);
            if slash < 0.16 { pen.light(Mat4::from_scale_rotation_translation(Vec3::splat(150.0 * scale), f.rotation, f.point(a[0] + dx * grow, a[1] + dy * grow, 0.085 * depth)), [1.0, 0.9, 0.85], 2.0, 5.0); }
            for n in 0..46u32 {
                let u = noise(n * 5 + 12);
                let t = slash - u * 0.10;
                if !(0.0..0.95).contains(&t) { continue; }
                let p = f.point(a[0] + dx * u, a[1] + dy * u, 0.07 * depth);
                let dir = Vec3::new((noise(n * 5 + 13) - 0.5) * 1.8, (noise(n * 5 + 14) - 0.25) * 1.5, 0.15 + 0.8 * noise(n * 5 + 15)) * 0.70;
                let p = p + facing * (dir * t) - Vec3::Y * 1.2 * t * t;
                let fade = (1.0 - t / 0.95) * alpha;
                if n % 3 == 0 { let r = Quat::from_rotation_z(t * 9.0 + n as f32) * Quat::from_rotation_x(t * 7.0); let s = (0.010 + 0.016 * noise(n + 400)) * (0.4 + 0.6 * fade); pen.solid(Mat4::from_scale_rotation_translation(Vec3::new(s, s * 0.7, 1.0), facing * r, p), [0.0, 0.0, 10.0, 7.0], 1.0, [0.09, 0.09, 0.11], fade, 0.0); }
                else { pen.light(Mat4::from_scale_rotation_translation(Vec3::splat(0.032 * fade), facing, p), [1.0, 0.50, 0.30], 2.0 * fade, 3.0); }
            }
        } else {
            let burst = (age - 0.18) / 0.65;
            if (0.0..1.0).contains(&burst) && !ban {
                let grow = 0.85 + 0.75 * ease_out(burst);
                let r = [-(hw + 10.0) * grow, -(hh + 10.0) * grow, (hw + 10.0) * 2.0 * grow, (hh + 10.0) * 2.0 * grow];
                let g = Frame { center: center - facing * Vec3::Z * 0.06, rotation: facing, at: [0.0, 0.0], scale: PX };
                pen.ring(g.quad(r, 0.0, 1.0), r, 40.0, color, 0.8 * (1.0 - burst).powi(2), 9.0);
            }
            let halo = [-hw * 2.0, -hh * 2.1, hw * 4.0, hh * 4.2];
            pen.light(f.quad(halo, -0.07 * depth, 1.0), color, (if ban { 0.10 } else { 0.34 }) * alpha * swell * e, 4.0);
            // 돌아서는 동안 뒷면: 무늬 없는 판.
            let front = (f.rotation * Vec3::Z).dot((place.head - f.center).normalize_or_zero()) > 0.0;
            if front { pen.card(live, &self.body, &f, self.origin, alpha, None, color, swell, if ban { 0.0 } else { ease_out((age - 0.30) / 0.4) }); }
            else { let r = [-hw, -hh, hw * 2.0, hh * 2.0]; pen.solid(f.quad(r, 0.0, 1.0), r, 18.0, color.map(|v| 0.004 + 0.022 * v), alpha, 0.0); pen.ring(f.quad(r, -0.001, 1.0), r, 18.0, color, alpha, 8.0); let i = [-hw + 24.0, -hh + 24.0, hw * 2.0 - 48.0, hh * 2.0 - 48.0]; pen.ring(f.quad(i, -0.002, 1.0), i, 12.0, color, 0.35 * alpha, 3.0); for (w, a) in [(150.0f32, 0.6f32), (96.0, 0.4), (34.0, 0.8)] { let q = [-w * 0.5, -w * 0.5, w, w]; pen.ring(f.quad(q, -0.003, 1.0), q, w * 0.5, color, a * alpha, 5.0); } }
            if !ban && front {
                // 주인공 그림이 카드 밖으로 걸어 나온다.
                if let Some(c) = &self.hero {
                    let (w, h) = (c.f(128).max(1.0), c.f(132).max(1.0));
                    if let Some((cell, _)) = live.cell(c) {
                        let step = ease_out((age - 0.30) / 0.4);
                        let (ch, cw) = (250.0, 250.0 * w / h);
                        let q = [hw - 110.0 - cw * 0.5 + 70.0 * step, hh + 34.0 - ch, cw, ch];
                        let lift = (0.07 + 0.11 * step) * depth;
                        let dark = [q[0] + 12.0, q[1] + 10.0, q[2], q[3]];
                        pen.out.push(Instance::new(pen.local * f.quad(dark, 0.6 * lift, 1.0), cell.uv, [0.0, 0.0, 0.0, 0.5 * alpha * step], [11.0, 0.02, 0.0, cell.page as f32]));
                        pen.out.push(Instance::new(pen.local * f.quad(q, lift, 1.0), cell.uv, [1.0, 1.0, 1.0, alpha * step], [11.0, 0.02, 0.0, cell.page as f32]));
                    }
                }
            }
            if !ban {
                for n in 0..34u32 {
                    let t = age - 0.16 - 0.08 * noise(n * 3 + 50);
                    let life = 0.8 + 0.5 * noise(n * 3 + 51);
                    if t < 0.0 || t > life { continue; }
                    let a = noise(n * 3 + 52) * std::f32::consts::TAU;
                    let d = 0.30 + 0.42 * ease_out(t / life) * (0.5 + noise(n * 3 + 53));
                    let p = center + facing * Vec3::new(a.cos() * d * 1.15, a.sin() * d * 0.9, -0.06 + 0.35 * ease_out(t / life) * noise(n + 900));
                    let fade = 1.0 - t / life;
                    pen.light(Mat4::from_scale_rotation_translation(Vec3::splat((0.016 + 0.024 * noise(n + 70)) * fade), facing, p), color, 1.6 * fade, 3.0);
                }
            }
        }
        (k >= 0.90 && self.hold < 1.0e6).then(|| { self.hold = f32::INFINITY; Tail { age: 0.0, at: goal, rotation: goal_rotation, color, size: goal_scale, ban } })
    }
}
/// 「금지 단계」 같은 큰 안내 글: 두께가 있는 글자가 눈앞으로 미끄러져 들어온다.
fn banner(pen: &mut Pen, live: &mut Live, cmds: &[Cmd], t: [f32; 4], place: &Place, wall: &Wall) {
    for c in cmds.iter().filter(|c| c.kind == Kind::Text) {
        let Some((cell, size)) = live.text(&c.text) else { continue };
        let r = [c.f(152), c.f(156), c.f(160), c.f(164)];
        let (x, y) = (t[0] + (r[0] + r[2] * 0.5) * t[3], t[1] + (r[1] + r[3] * 0.5) * t[3]);
        let (at, rotation) = wall.at(place, x, y, 1.32, 1.0);
        let h = c.f(192) * 1.5 * PX * t[3];
        let w = h * size.0 / size.1;
        let col = c.rgba(168);
        let a = col[3];
        let shade = [0.12f32, 0.45, 0.42];
        pen.light(Mat4::from_scale_rotation_translation(Vec3::new(w * 1.9, h * 3.2, 1.0), rotation, at - rotation * Vec3::Z * 0.08), shade, 0.35 * a, 4.0);
        for n in (1..=7).rev() { let k = n as f32; pen.out.push(Instance::new(pen.local * Mat4::from_scale_rotation_translation(Vec3::new(w, h, 1.0), rotation, at - rotation * Vec3::Z * 0.006 * k), cell.uv, [shade[0] * (1.0 - k / 9.0), shade[1] * (1.0 - k / 9.0), shade[2] * (1.0 - k / 9.0), a], [11.0, 0.12, 0.0, cell.page as f32])); }
        let l = lin3(col);
        pen.out.push(Instance::new(pen.local * Mat4::from_scale_rotation_translation(Vec3::new(w, h, 1.0), rotation, at), cell.uv, [l[0], l[1], l[2], a], [11.0, 0.12, 0.0, cell.page as f32]));
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn 자른_선은_남길_쪽이_양수() {
        let p = |x: f32, y: f32| { let mut v = Vec::new(); for f in [1.0f32, 1.0, 1.0, 1.0, x, y, 0.0, 0.0] { v.extend_from_slice(&f.to_le_bytes()); } v };
        let data: Vec<u8> = [p(-260.0, -204.0), p(260.0, -204.0), p(260.0, -144.0), p(-260.0, 144.0)].concat();
        let c = Cmd { kind: Kind::Other(5), raw: [0; 208], key: String::new(), tex: 0, shader: String::new(), style: vec![], text: String::new(), data };
        let (k, a) = cut_of(&c).unwrap();
        assert_eq!(a, [260.0, -144.0, -260.0, 144.0]);
        assert!(k[0] * -200.0 + k[1] * -150.0 + k[2] > 0.0 && k[0] * 200.0 + k[1] * 150.0 + k[2] < 0.0);
        let q = plane(k, [-260.0, -204.0, 520.0, 408.0]);
        assert!(q[2] > 0.0 && q[0] + q[1] + q[2] < 0.0);
    }
}
