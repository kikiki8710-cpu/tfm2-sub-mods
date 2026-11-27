//! 게임이 쓰는 벽·수풀 그림에서 입체 지형을 만든다(맵 모드도 같은 방식). 좌표는 1280 그림 픽셀.
use crate::{geometry::Vertex, scene::{PIXEL, TABLE}};
use image::RgbaImage;
use std::collections::HashMap;

pub const SIZE: u32 = 1280;
const FOLD: f32 = 18.0;
pub struct Terrain { pub top: Vec<Vertex>, pub side: Vec<Vertex> }

/// 바깥과 이어지지 않은 투명 구멍을 메운 마스크. 좌우 장식 띠(플레이 영역 밖)는 세우지 않는다.
fn mask(images: &[&RgbaImage], border: bool) -> Vec<bool> {
    let n = SIZE as usize;
    let mut m = vec![false; n * n];
    for im in images { for (x, y, p) in im.enumerate_pixels() { if p[3] > 32 && x < SIZE && y < SIZE { m[y as usize * n + x as usize] = true; } } }
    if border { for y in 0..n { for x in (0..160).chain(1120..n) { m[y * n + x] = false; } } }
    let mut outside = vec![false; n * n];
    let mut stack: Vec<usize> = (0..n).flat_map(|i| [i, (n - 1) * n + i, i * n, i * n + n - 1]).filter(|&i| !m[i]).collect();
    while let Some(i) = stack.pop() {
        if outside[i] || m[i] { continue; }
        outside[i] = true;
        let (x, y) = (i % n, i / n);
        if x > 0 { stack.push(i - 1); } if x + 1 < n { stack.push(i + 1); } if y > 0 { stack.push(i - n); } if y + 1 < n { stack.push(i + n); }
    }
    outside.into_iter().map(|o| !o).collect()
}
fn world(x: f32, y: f32, h: f32) -> [f32; 3] { [(x - 640.0) * PIXEL, TABLE + 0.001 + h, (y - 640.0) * PIXEL] }
fn push(out: &mut Vec<Vertex>, p: [[f32; 3]; 4], uv: [[f32; 2]; 4], color: [[f32; 3]; 4]) { for i in [0, 1, 2, 0, 2, 3] { let c = color[i]; out.push(Vertex { position: p[i], uv: uv[i], color: [c[0], c[1], c[2], 1.0] }); } }

/// height 는 m. 윗면은 원본 그림 그대로, 옆면은 가장자리 안쪽 띠를 접어 내린 그림에 방향 음영.
pub fn build(images: &[&RgbaImage], height: f32, border: bool) -> Terrain {
    let n = SIZE as usize;
    let m = mask(images, border);
    let mut out = Terrain { top: Vec::new(), side: Vec::new() };
    let mut seen = vec![false; n * n];
    let s = SIZE as f32;
    for y in 0..n {
        let mut x = 0;
        while x < n {
            let i = y * n + x;
            if !m[i] || seen[i] { x += 1; continue; }
            let mut w = 1;
            while x + w < n && m[i + w] && !seen[i + w] { w += 1; }
            let mut h = 1;
            while y + h < n && (x..x + w).all(|k| m[(y + h) * n + k] && !seen[(y + h) * n + k]) { h += 1; }
            for yy in y..y + h { seen[yy * n + x..yy * n + x + w].fill(true); }
            let (x0, y0, x1, y1) = (x as f32, y as f32, (x + w) as f32, (y + h) as f32);
            push(&mut out.top, [world(x0, y0, height), world(x0, y1, height), world(x1, y1, height), world(x1, y0, height)], [[x0 / s, y0 / s], [x0 / s, y1 / s], [x1 / s, y1 / s], [x1 / s, y0 / s]], [[1.0; 3]; 4]);
            x += w;
        }
    }
    let mut edges: HashMap<(i32, i32, i32), Vec<i32>> = HashMap::new();
    for y in 0..n as i32 { for x in 0..n as i32 {
        if !m[(y as usize) * n + x as usize] { continue; }
        for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
            let (nx, ny) = (x + dx, y + dy);
            if nx >= 0 && ny >= 0 && nx < n as i32 && ny < n as i32 && m[ny as usize * n + nx as usize] { continue; }
            edges.entry((dx, dy, if dx != 0 { x + (dx > 0) as i32 } else { y + (dy > 0) as i32 })).or_default().push(if dx != 0 { y } else { x });
        }
    } }
    let mut keys: Vec<_> = edges.into_iter().collect();
    keys.sort_by_key(|k| k.0);
    for ((dx, dy, fixed), mut list) in keys {
        list.sort_unstable();
        let mut i = 0;
        while i < list.len() {
            let start = list[i];
            let mut end = start + 1;
            while i + 1 < list.len() && list[i + 1] == end && end - start < 64 { end += 1; i += 1; }
            i += 1;
            let shade = if dx < 0 { 0.66 } else if dx > 0 { 0.74 } else if dy < 0 { 0.6 } else { 0.92 };
            let (a, b, f) = (start as f32, end as f32, fixed as f32);
            let p = if dx != 0 { [world(f, a, 0.0), world(f, a, height), world(f, b, height), world(f, b, 0.0)] } else { [world(a, f, 0.0), world(a, f, height), world(b, f, height), world(b, f, 0.0)] };
            // 가장자리 안쪽 띠를 접어 내린다: 아래 = 가장자리, 위 = 안쪽 FOLD 픽셀.
            let inside = |v: f32, d: i32| v - d as f32 * FOLD;
            let uv = if dx != 0 { [[f, a], [inside(f, dx), a], [inside(f, dx), b], [f, b]] } else { [[a, f], [a, inside(f, dy)], [b, inside(f, dy)], [b, f]] };
            push(&mut out.side, p, uv.map(|v| [v[0] / s, v[1] / s]), [[shade * 0.7; 3], [shade; 3], [shade; 3], [shade * 0.7; 3]]);
        }
    }
    out
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn 벽_한_칸은_윗면_하나와_옆면_넷() {
        let mut im = RgbaImage::new(SIZE, SIZE);
        for y in 600..604 { for x in 600..604 { im.put_pixel(x, y, image::Rgba([90, 100, 110, 255])); } }
        im.put_pixel(601, 601, image::Rgba([0, 0, 0, 0])); // 안쪽 구멍은 메운다
        let t = build(&[&im], 0.06, true);
        assert_eq!(t.top.len(), 6);
        assert_eq!(t.side.len(), 4 * 6);
    }
    #[test] fn 좌우_장식은_세우지_않는다() {
        let mut im = RgbaImage::new(SIZE, SIZE);
        im.put_pixel(50, 600, image::Rgba([1, 1, 1, 255]));
        assert!(build(&[&im], 0.06, true).top.is_empty());
    }
}
