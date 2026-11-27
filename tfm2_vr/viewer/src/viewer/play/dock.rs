//! 왼쪽 아래에 떠 있는 작은 단추 판: 평면/곡면 바꾸기 · 가운데로 · 자판.
use crate::{geometry::{self, Instance}, gpu::Gpu, live::Live};
use anyhow::Result;
use glam::{Mat4, Quat, Vec3};

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Action { Flat, Center, Keyboard }
const BUTTONS: [Action; 3] = [Action::Flat, Action::Center, Action::Keyboard];
const W: f32 = 0.125;
const H: f32 = 0.043;
const STEP: f32 = 0.051;
pub struct Dock { center: Vec3, rotation: Quat, hover: Option<usize>, press: [f32; 3], down: bool, back_id: usize, text_id: usize }
impl Dock {
    pub fn new(gpu: &mut Gpu) -> Result<Self> {
        let mut add = |name: &str| -> Result<usize> { let mut m = geometry::mesh(name, geometry::quad(), vec![], true, false); m.dynamic = true; gpu.add_mesh(&m, 16) };
        Ok(Self { center: Vec3::ZERO, rotation: Quat::IDENTITY, hover: None, press: [0.0; 3], down: false, back_id: add("live9keyDockBack")?, text_id: add("live9keyDockText")? })
    }
    pub fn place(&mut self, head: Vec3, yaw: f32) {
        let a = yaw + 0.80;
        self.center = Vec3::new(head.x - a.sin() * 0.86, head.y - 0.44, head.z - a.cos() * 0.86);
        self.rotation = Quat::from_rotation_y(a) * Quat::from_rotation_x(-0.55);
    }
    fn at(&self, i: usize, lift: f32) -> Vec3 { self.center + self.rotation * Vec3::new(0.0, (1.0 - i as f32) * STEP, lift) }
    pub fn hit(&mut self, origin: Vec3, direction: Vec3) -> Option<f32> {
        let n = self.rotation * Vec3::Z;
        let k = direction.dot(n);
        if k.abs() < 1e-4 { return None; }
        let t = (self.center - origin).dot(n) / k;
        if !(0.02..3.0).contains(&t) { return None; }
        let l = self.rotation.inverse() * (origin + direction * t - self.center);
        if l.x.abs() > W * 0.5 + 0.012 || l.y.abs() > STEP * 1.5 + 0.008 { return None; }
        self.hover = (0..3).find(|&i| l.x.abs() <= W * 0.5 && (l.y - (1.0 - i as f32) * STEP).abs() <= H * 0.5);
        Some(t)
    }
    pub fn step(&mut self, pointing: bool, down: bool, dt: f32) -> Option<Action> {
        if !pointing { self.hover = None; }
        let mut out = None;
        if pointing && down && !self.down { if let Some(h) = self.hover { self.press[h] = 1.0; out = Some(BUTTONS[h]); } }
        self.down = down;
        for (i, p) in self.press.iter_mut().enumerate() { let goal = if Some(i) == self.hover { 0.5 } else { 0.0 }; *p += (goal - *p) * (1.0 - (-dt * 12.0).exp()); }
        out
    }
    pub fn draw(&mut self, gpu: &mut Gpu, live: &mut Live, local: Mat4, flat: bool) -> Result<()> {
        let (mut back, mut text) = (Vec::new(), Vec::new());
        let plate = |m: Mat4, w: f32, h: f32, color: [f32; 4]| Instance::new(local * m, [0.0; 4], color, [14.0, 0.22, w / h, 0.0]);
        let (bw, bh) = (W + 0.024, STEP * 3.0 + 0.014);
        back.push(plate(Mat4::from_scale_rotation_translation(Vec3::new(bw, bh, 1.0), self.rotation, self.center - self.rotation * Vec3::Z * 0.006), bw, bh, [0.010, 0.015, 0.020, 0.85]));
        for (i, b) in BUTTONS.iter().enumerate() {
            let p = self.press[i];
            let c = 0.030 + 0.11 * p;
            back.push(plate(Mat4::from_scale_rotation_translation(Vec3::new(W, H, 1.0), self.rotation, self.at(i, 0.008 * p)), W, H, [c * 0.8, c * 1.5, c * 1.6, 0.97]));
            let label = crate::lang::t(match b { Action::Flat => if flat { "휜 화면으로" } else { "평평하게" }, Action::Center => "가운데로", Action::Keyboard => "자판" });
            if let Some((cell, size)) = live.text(label) { let h = 0.022; let w = (h * size.0 / size.1).min(W * 0.92); text.push(Instance::new(local * Mat4::from_scale_rotation_translation(Vec3::new(w, h, 1.0), self.rotation, self.at(i, 0.008 * p + 0.0015)), cell.uv, [0.85, 1.0, 0.97, 1.0], [11.0, 0.15, 0.0, cell.page as f32])); }
        }
        gpu.update(self.back_id, &back)?;
        gpu.update(self.text_id, &text)
    }
}
