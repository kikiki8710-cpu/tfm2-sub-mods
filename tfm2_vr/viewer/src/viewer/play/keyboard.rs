//! 손 앞에 뜨는 자판. 글자 입력이 필요한 곳(검색·이름)에서 B/Y 로 켠다. 한글 조합은 윈도우 입력기가 한다(한/영 키 포함).
use crate::{geometry::{self, Instance}, gpu::Gpu, live::Live};
use anyhow::Result;
use glam::{Mat4, Quat, Vec3};

#[derive(Clone, Copy)]
struct Key { label: &'static str, vk: u16, x: f32, y: f32, w: f32 }
pub struct Keyboard { pub open: bool, center: Vec3, rotation: Quat, keys: Vec<Key>, hover: Option<usize>, press: Vec<f32>, down: bool, back_id: usize, text_id: usize }
const U: f32 = 0.046;
fn layout() -> Vec<Key> {
    let mut out = Vec::new();
    let rows: [(&str, f32); 4] = [("1234567890", 0.0), ("QWERTYUIOP", 0.0), ("ASDFGHJKL", 0.35), ("ZXCVBNM", 0.8)];
    const NAMES: [&str; 36] = ["0", "1", "2", "3", "4", "5", "6", "7", "8", "9", "A", "B", "C", "D", "E", "F", "G", "H", "I", "J", "K", "L", "M", "N", "O", "P", "Q", "R", "S", "T", "U", "V", "W", "X", "Y", "Z"];
    for (y, (row, shift)) in rows.iter().enumerate() {
        for (x, c) in row.bytes().enumerate() {
            let index = if c.is_ascii_digit() { (c - b'0') as usize } else { (c - b'A') as usize + 10 };
            out.push(Key { label: NAMES[index], vk: c as u16, x: shift + x as f32, y: y as f32, w: 1.0 });
        }
    }
    out.push(Key { label: crate::lang::t("지우기"), vk: 0x08, x: 7.8 + 0.2, y: 3.0, w: 2.0 });
    out.push(Key { label: crate::lang::t("한/영"), vk: 0x15, x: 0.0, y: 4.0, w: 1.6 });
    out.push(Key { label: crate::lang::t("띄어쓰기"), vk: 0x20, x: 1.6, y: 4.0, w: 4.4 });
    out.push(Key { label: crate::lang::t("확인"), vk: 0x0d, x: 6.0, y: 4.0, w: 2.0 });
    out.push(Key { label: crate::lang::t("닫기"), vk: 0, x: 8.0, y: 4.0, w: 2.0 });
    out
}
impl Keyboard {
    pub fn new(gpu: &mut Gpu) -> Result<Self> {
        let mut add = |name: &str| -> Result<usize> { let mut m = geometry::mesh(name, geometry::quad(), vec![], true, false); m.dynamic = true; gpu.add_mesh(&m, 256) };
        let keys = layout();
        Ok(Self { open: false, center: Vec3::ZERO, rotation: Quat::IDENTITY, press: vec![0.0; keys.len()], keys, hover: None, down: false, back_id: add("live9keyBack")?, text_id: add("live9keyText")? })
    }
    pub fn toggle(&mut self, head: Vec3, rotation: Quat) {
        self.open = !self.open;
        let mut d = rotation * Vec3::NEG_Z;
        d.y = 0.0;
        let d = if d.length_squared() < 1e-4 { Vec3::NEG_Z } else { d.normalize() };
        let yaw = (-d.x).atan2(-d.z);
        self.center = head + d * 0.62 - Vec3::Y * 0.42;
        self.rotation = Quat::from_rotation_y(yaw) * Quat::from_rotation_x(-0.75);
        self.hover = None;
    }
    fn at(&self, k: &Key, lift: f32) -> Vec3 { self.center + self.rotation * Vec3::new((k.x + k.w * 0.5 - 5.0) * U, (2.0 - k.y) * U, lift) }
    /// 레이저가 자판에 닿은 거리.
    pub fn hit(&mut self, origin: Vec3, direction: Vec3) -> Option<f32> {
        if !self.open { return None; }
        let n = self.rotation * Vec3::Z;
        let k = direction.dot(n);
        if k.abs() < 1e-4 { return None; }
        let t = (self.center - origin).dot(n) / k;
        if !(0.02..3.0).contains(&t) { return None; }
        let l = self.rotation.inverse() * (origin + direction * t - self.center);
        let (x, y) = (l.x / U + 5.0, 2.5 - l.y / U);
        if !(-0.3..10.4).contains(&x) || !(-0.3..5.3).contains(&y) { return None; }
        self.hover = self.keys.iter().position(|k| x >= k.x && x < k.x + k.w && y >= k.y && y < k.y + 1.0);
        Some(t)
    }
    /// 누르는 순간 보낼 키. 닫기는 0.
    pub fn step(&mut self, pointing: bool, down: bool, dt: f32) -> Option<u16> {
        let mut out = None;
        if !pointing { self.hover = None; }
        if pointing && down && !self.down { if let Some(h) = self.hover { self.press[h] = 1.0; out = Some(self.keys[h].vk); if self.keys[h].vk == 0 { self.open = false; } } }
        self.down = down;
        for (i, p) in self.press.iter_mut().enumerate() { let goal = if Some(i) == self.hover { 0.45 } else { 0.0 }; *p += (goal - *p) * (1.0 - (-dt * 14.0).exp()); }
        out
    }
    pub fn draw(&mut self, gpu: &mut Gpu, live: &mut Live, local: Mat4) -> Result<()> {
        let (mut back, mut text) = (Vec::new(), Vec::new());
        if self.open {
            let plate = |m: Mat4, w: f32, h: f32, color: [f32; 4], soft: f32| Instance::new(local * m, [0.0; 4], color, [14.0, 0.18 * (U / h).min(1.0), w / h, soft]);
            let (bw, bh) = (10.6 * U, 5.6 * U);
            back.push(plate(Mat4::from_scale_rotation_translation(Vec3::new(bw, bh, 1.0), self.rotation, self.center + self.rotation * Vec3::new(0.0, 0.0, -0.006)), bw, bh, [0.012, 0.018, 0.024, 0.94], 0.0));
            for (i, k) in self.keys.iter().enumerate() {
                let p = self.press[i];
                let (w, h) = (k.w * U * 0.94, U * 0.92);
                let c = 0.035 + 0.10 * p;
                back.push(plate(Mat4::from_scale_rotation_translation(Vec3::new(w, h, 1.0), self.rotation, self.at(k, 0.004 * p)), w, h, [c * 0.8, c * 1.5, c * 1.6, 0.98], 0.0));
                if let Some((cell, size)) = live.text(k.label) { let th = U * 0.62; let tw = (th * size.0 / size.1).min(w * 0.9); text.push(Instance::new(local * Mat4::from_scale_rotation_translation(Vec3::new(tw, th, 1.0), self.rotation, self.at(k, 0.004 * p + 0.0015)), cell.uv, [0.9, 1.0, 0.98, 1.0], [11.0, 0.15, 0.0, cell.page as f32])); }
            }
        }
        gpu.update(self.back_id, &back)?;
        gpu.update(self.text_id, &text)?;
        Ok(())
    }
}
#[cfg(test)]
mod tests { #[test] fn 자판은_겹치지_않는다() { let k = super::layout(); for a in 0..k.len() { for b in a + 1..k.len() { assert!(k[a].y != k[b].y || k[a].x + k[a].w <= k[b].x + 1e-4 || k[b].x + k[b].w <= k[a].x + 1e-4, "{} {}", k[a].label, k[b].label); } } } }
