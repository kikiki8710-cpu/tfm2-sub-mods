//! 브리지가 보낸 그리기 명령(TVF2) 해석. 208바이트 원본과 문자열·배열 부속을 그대로 들고 온다.
use anyhow::{bail, Result};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Kind { Sprite, Patch, Text, Line, Points, Box, Mask, Circle, Ring, Path, Svg, Other(u8) }
#[derive(Clone)]
pub struct Cmd { pub kind: Kind, pub raw: [u8; 208], pub key: String, pub tex: u32, pub shader: String, pub style: Vec<(String, Vec<u8>)>, pub text: String, pub data: Vec<u8> }
#[derive(Clone, Default)]
pub struct Frame { pub number: u64, pub micros: u64, pub lists: Vec<(String, Vec<Cmd>)> }

struct Reader<'a>(&'a [u8], usize);
impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8]> { let Some(v) = self.0.get(self.1..self.1 + n) else { bail!("프레임 자료가 짧음") }; self.1 += n; Ok(v) }
    fn u32(&mut self) -> Result<u32> { Ok(u32::from_le_bytes(self.take(4)?.try_into().unwrap())) }
    fn u64(&mut self) -> Result<u64> { Ok(u64::from_le_bytes(self.take(8)?.try_into().unwrap())) }
    fn blob(&mut self) -> Result<&'a [u8]> { let n = self.u32()? as usize; self.take(n) }
    fn string(&mut self) -> Result<String> { Ok(String::from_utf8_lossy(self.blob()?).into_owned()) }
}
impl Cmd {
    /// 늘린 그림(Patch)의 놓일 자리. 그림 칸이 원본 밖으로 나간 만큼(여백)은 뺀다.
    pub fn patch_rect(&self) -> [f32; 4] {
        let (u, v, uw, vh) = (self.f(104), self.f(108), self.f(112), self.f(116));
        let (x, y, w, h) = (self.f(120), self.f(124), self.f(128), self.f(132));
        let cut = |a: f32, n: f32| if n > 1e-6 { (((-a).max(0.0) / n).min(1.0), ((a + n - 1.0).max(0.0) / n).min(1.0)) } else { (0.0, 0.0) };
        let ((l, r), (t, b)) = (cut(u, uw), cut(v, vh));
        [x + w * l, y + h * t, w * (1.0 - l - r).max(0.0), h * (1.0 - t - b).max(0.0)]
    }
    pub fn f(&self, at: usize) -> f32 { f32::from_le_bytes(self.raw[at..at + 4].try_into().unwrap()) }
    pub fn i(&self, at: usize) -> i32 { i32::from_le_bytes(self.raw[at..at + 4].try_into().unwrap()) }
    pub fn rgba(&self, at: usize) -> [f32; 4] { [self.f(at), self.f(at + 4), self.f(at + 8), self.f(at + 12)] }
    /// 상자(Box)의 채움색과 테두리색.
    pub fn box_fill(&self) -> ([f32; 4], Option<[f32; 4]>) { if self.raw[28] != 0 { (self.rgba(36), Some(self.rgba(112))) } else { (self.rgba(112), None) } }
    pub fn style_f(&self, name: &str) -> Option<Vec<f32>> { self.style.iter().find(|(k, _)| k == name).map(|(_, v)| v.chunks_exact(4).map(|c| f32::from_le_bytes(c.try_into().unwrap())).collect()) }
    /// 화면 층(작을수록 먼저). 종류마다 자리가 다르다.
    pub fn layer(&self) -> i32 {
        self.i(match self.kind { Kind::Sprite => 0x80, Kind::Patch | Kind::Svg => 0x88, Kind::Text => 0xb8, Kind::Line => 60, Kind::Points | Kind::Ring => 48, Kind::Box => 104, Kind::Mask => 68, Kind::Circle => 108, Kind::Path => 100, Kind::Other(_) => return 0 })
    }
    pub fn points(&self, stride: usize) -> Vec<[f32; 2]> { self.data.chunks_exact(stride).map(|c| [f32::from_le_bytes(c[..4].try_into().unwrap()), f32::from_le_bytes(c[4..8].try_into().unwrap())]).collect() }
}
pub fn parse(bytes: &[u8]) -> Result<Frame> {
    let mut r = Reader(bytes, 0);
    if r.take(4)? != b"TVF2" { bail!("프레임 형식 오류"); }
    r.u32()?;
    let mut out = Frame { number: r.u64()?, micros: r.u64()?, lists: Vec::new() };
    for _ in 0..r.u32()? { r.take(112)?; r.blob()?; }
    for _ in 0..r.u32()? { r.blob()?; r.take(8)?; }
    for _ in 0..r.u32()? {
        let name = r.string()?;
        let count = r.u32()? as usize;
        let mut list = Vec::with_capacity(count);
        for _ in 0..count {
            let raw: [u8; 208] = r.take(208)?.try_into().unwrap();
            let d = u64::from_le_bytes(raw[..8].try_into().unwrap());
            let tag = if d >> 63 == 1 { (d & 0xff) as u8 } else { 7 };
            let mut c = Cmd { kind: Kind::Other(tag), raw, key: String::new(), tex: 0, shader: String::new(), style: Vec::new(), text: String::new(), data: Vec::new() };
            match tag {
                2 => { c.kind = Kind::Svg; c.key = r.string()?; }
                3 | 4 => {
                    c.kind = if tag == 3 { Kind::Sprite } else { Kind::Patch };
                    c.key = r.string()?; c.tex = r.u32()?; c.shader = r.string()?;
                    for _ in 0..r.take(1)?[0] { let k = r.string()?; c.style.push((k, r.blob()?.to_vec())); }
                }
                5 | 6 => { c.key = r.string()?; c.tex = r.u32()?; c.data = r.blob()?.to_vec(); }
                7 => { c.kind = Kind::Text; c.text = r.string()?; c.key = r.string()?; }
                8 => c.kind = Kind::Line,
                9 => { c.kind = Kind::Points; c.data = r.blob()?.to_vec(); }
                10 => c.kind = Kind::Box,
                11 => { c.kind = Kind::Mask; c.data = r.blob()?.to_vec(); }
                12 => c.kind = Kind::Circle,
                13 => c.kind = Kind::Ring,
                14 => { c.kind = Kind::Path; c.data = r.blob()?.to_vec(); }
                _ => {}
            }
            list.push(c);
        }
        out.lists.push((name, list));
    }
    Ok(out)
}
impl Frame { pub fn list(&self, name: &str) -> &[Cmd] { self.lists.iter().find(|l| l.0 == name).map(|l| l.1.as_slice()).unwrap_or(&[]) } }
