//! 게임 UI 명령을 창·판(버튼)·가리킬 수 있는 자리로 나눈다. 좌표는 UI 픽셀(1920×1080).
use crate::live::frame::{Cmd, Kind};

pub type Rect = [f32; 4]; // x, y, w, h
#[derive(Clone, Debug)]
pub struct Plate { pub rect: Rect, pub radius: f32, pub depth: u32, pub fill: [f32; 4], pub under: [f32; 3], pub button: bool }
#[derive(Clone, Debug)]
pub struct Cut { pub index: usize, pub rect: Rect, pub depth: u32, pub under: Option<[f32; 3]>, pub big: bool }
#[derive(Clone, Debug)]
pub struct Win { pub rect: Rect, pub radius: f32, pub fill: [f32; 4], pub layer: u32, pub level: u32, pub bare: bool, pub plates: Vec<Plate>, pub zones: Vec<(Rect, f32)>, pub cuts: Vec<Cut>, pub order: usize, pub scene: Vec<usize>, pub ghost: bool }
#[derive(Clone, Debug, Default)]
/// stage: 화면 전체 그림이 여러 장 겹친 장면(경기장 입장 등)의 (층, 명령 번호). fade: 그 위를 덮는 어두움.
/// shows: 변환으로 그리는 연출(밴·픽 카드, 단계 안내 글).
pub struct Shelf { pub wins: Vec<Win>, pub top_layer: u32, pub stage: Vec<(i32, usize)>, pub fade: f32, pub shows: Vec<Show> }
/// list[start..=end] = 변환 시작(18)에서 짝 끝(19)까지. t = x, y, 회전, 크기.
#[derive(Clone, Debug)]
pub struct Show { pub start: usize, pub end: usize, pub t: [f32; 4], pub banner: bool }

pub fn area(r: Rect) -> f32 { r[2].max(0.0) * r[3].max(0.0) }
pub fn cut(a: Rect, b: Rect) -> Rect { let (x, y) = (a[0].max(b[0]), a[1].max(b[1])); [x, y, (a[0] + a[2]).min(b[0] + b[2]) - x, (a[1] + a[3]).min(b[1] + b[3]) - y] }
pub fn inside(a: Rect, b: Rect, t: f32) -> bool { a[0] >= b[0] - t && a[1] >= b[1] - t && a[0] + a[2] <= b[0] + b[2] + t && a[1] + a[3] <= b[1] + b[3] + t }
pub fn has(r: Rect, x: f32, y: f32) -> bool { x >= r[0] && y >= r[1] && x < r[0] + r[2] && y < r[1] + r[3] }
fn union(a: Rect, b: Rect) -> Rect { let (x, y) = (a[0].min(b[0]), a[1].min(b[1])); [x, y, (a[0] + a[2]).max(b[0] + b[2]) - x, (a[1] + a[3]).max(b[1] + b[3]) - y] }

struct Item { picture: bool, rect: Rect, boxed: bool, fill: [f32; 4], stroke: bool, radius: f32, layer: u32, index: usize, cutout: bool, big: bool }
const SCREEN: Rect = [0.0, 0.0, 1920.0, 1080.0];

fn bounds(c: &Cmd) -> Option<Rect> {
    let r = match c.kind {
        Kind::Box => [c.f(88), c.f(92), c.f(96), c.f(100)],
        Kind::Text => {
            // 글자 상자는 실제 글자보다 넓을 때가 많다: 글자 수로 폭을 어림하고 정렬(@200: 0 왼쪽·1 가운데·2 오른쪽)을 따른다.
            let (x, y, w, h) = (c.f(152), c.f(156), c.f(160), c.f(164));
            let size = c.f(192) * c.f(196).clamp(0.2, 4.0);
            let line = c.text.lines().map(|l| l.chars().map(|k| if k.is_ascii() { 0.58 } else { 0.95 }).sum::<f32>()).fold(0.0f32, f32::max) * size + 4.0;
            let lines = (line / w.max(1.0)).ceil().max(c.text.lines().count().max(1) as f32);
            let (nw, nh) = (line.min(w), (lines * size * 1.45).min(h.max(size)));
            let nx = match c.raw[200] { 1 => x + (w - nw) * 0.5, 2 => x + w - nw, _ => x };
            let ny = match c.raw[201] { 1 => y + (h - nh) * 0.5, 2 => y + h - nh, _ => y };
            [nx, ny, nw, nh]
        }
        Kind::Patch => c.patch_rect(),
        Kind::Svg => [c.f(104), c.f(108), c.f(112), c.f(116)],
        _ => return None,
    };
    r.iter().all(|v| v.is_finite()).then_some(r)
}
/// sprite_size: 그림 명령의 픽셀 크기(텍스처가 있어야 안다).
pub fn analyze(list: &[Cmd], hole: Option<Rect>, sprite_size: &dyn Fn(&Cmd) -> Option<[f32; 2]>) -> Shelf {
    // 화면 전체 그림이 두 장 이상이면 둘러싸는 무대로 본다.
    let full = |c: &Cmd| matches!(c.kind, Kind::Patch) && c.tex != 0 && !c.key.ends_with("sprite/white") && c.f(128) >= 1880.0 && c.f(132) >= 1040.0;
    let on_stage = hole.is_none() && list.iter().filter(|c| full(c)).count() >= 2;
    let (mut stage, mut fade) = (Vec::new(), 0.0f32);
    let mut items: Vec<Item> = Vec::with_capacity(list.len());
    let mut clip = SCREEN;
    let mut layer = 0u32;
    let (mut shows, mut ghosts, mut skip): (Vec<Show>, Vec<(Rect, u32, usize)>, usize) = (Vec::new(), Vec::new(), 0);
    let mut moves: Vec<[f32; 3]> = Vec::new();
    for (index, c) in list.iter().enumerate() {
        if index < skip { continue; }
        if c.kind == Kind::Other(19) { moves.pop(); continue; }
        if c.kind == Kind::Other(18) {
            // 변환 = 상자(x, y, 폭, 높이)의 기준점(@28, @32 · 비율)으로 옮긴 뒤 회전·확대. 커지는 단추는 안쪽 변환이 그만큼 되돌린다.
            let anchor = |at: usize, size: f32| { let v = c.f(at); if v.is_finite() && (0.0..=1.0).contains(&v) && size.is_finite() { v * size } else { 0.0 } };
            let (tx, ty, rot, s) = (c.f(8) + anchor(28, c.f(16)), c.f(12) + anchor(32, c.f(20)), c.f(24), c.f(36));
            if [tx, ty, rot, s].iter().all(|v| v.is_finite()) && moves.is_empty() {
                let mut depth = 0;
                let end = (index..list.len()).find(|&i| { match list[i].kind { Kind::Other(18) => depth += 1, Kind::Other(19) => depth -= 1, _ => {} } depth == 0 });
                if let Some(end) = end {
                    let inner = &list[index + 1..end];
                    let card = inner.iter().any(|n| matches!(n.kind, Kind::Other(5)) || (matches!(n.kind, Kind::Box | Kind::Patch) && n.layer() >= 1200)) && s > 0.02 && s < 3.0;
                    let banner = !card && inner.len() == 1 && inner[0].kind == Kind::Text && inner[0].f(192) >= 48.0;
                    if card {
                        // 카드 크기는 카드마다 다르다(기본 360×480, 모드 520×408): 가장 큰 상자나 조각의 점에서.
                        let mut half = [0.0f32; 2];
                        for n in inner { match n.kind {
                            Kind::Box => { half[0] = half[0].max(n.f(88).abs()).max((n.f(88) + n.f(96)).abs()); half[1] = half[1].max(n.f(92).abs()).max((n.f(92) + n.f(100)).abs()); }
                            Kind::Other(5) => for p in n.data.chunks_exact(32) { let f = |k: usize| f32::from_le_bytes(p[k..k + 4].try_into().unwrap()); half[0] = half[0].max(f(16).abs() + 28.0); half[1] = half[1].max(f(20).abs() + 34.0); },
                            _ => {} } }
                        let half = [half[0].clamp(60.0, 500.0) + 14.0, half[1].clamp(60.0, 400.0) + 18.0];
                        ghosts.push(([tx - half[0] * s, ty - half[1] * s, half[0] * 2.0 * s, half[1] * 2.0 * s], layer, index));
                    }
                    if banner { if let Some(r) = bounds(&inner[0]) { ghosts.push(([tx + r[0] * s - 8.0, ty + r[1] * s - 6.0, r[2] * s + 16.0, r[3] * s + 12.0], layer, index)); } }
                    if card || banner { shows.push(Show { start: index, end, t: [tx, ty, rot, s], banner }); skip = end + 1; continue; }
                }
            }
            let last = moves.last().copied().unwrap_or([0.0, 0.0, 1.0]);
            moves.push(if [tx, ty, s].iter().all(|v| v.is_finite()) { [last[0] + tx * last[2], last[1] + ty * last[2], last[2] * s] } else { last });
            continue;
        }
        if c.kind == Kind::Other(0) { let r = [c.f(8), c.f(12), c.f(16), c.f(20)]; if r.iter().all(|v| v.is_finite()) && r[2] > 0.0 { clip = r; } continue; }
        if on_stage {
            let big = c.kind == Kind::Box && c.f(96) >= 1880.0 && c.f(100) >= 1040.0;
            if big { let a = c.box_fill().0[3]; if a >= 0.97 && !stage.is_empty() && c.layer() >= 0 && index + 4 < list.len() && list[index..].iter().any(|n| full(n)) { stage.clear(); items.clear(); } else if !stage.is_empty() { fade = fade.max(a.clamp(0.0, 1.0)); } continue; }
            if matches!(c.kind, Kind::Patch) && c.tex != 0 && !c.key.ends_with("sprite/white") { stage.push((c.layer(), index)); continue; }
            // 머리 위 이름표 같은 작은 글자는 무대에 섞여 있으므로 큰 글자만 창으로 남긴다.
            if c.kind == Kind::Text && c.f(192) < 22.0 { continue; }
            if !matches!(c.kind, Kind::Text | Kind::Box) { continue; }
        }
        let rect = match c.kind {
            Kind::Sprite => { if c.key == "#Game" { continue; } let Some(s) = sprite_size(c) else { continue }; let (fx, fy) = (c.raw[0x90] != 0, c.raw[0x91] != 0); let (x, y) = (c.f(0x78), c.f(0x7c)); [if fx { x - s[0] } else { x }, if fy { y - s[1] } else { y }, s[0], s[1]] }
            Kind::Line => { let (x0, y0, x1, y1) = (c.f(40), c.f(44), c.f(48), c.f(52)); [x0.min(x1), y0.min(y1), (x0 - x1).abs().max(1.0), (y0 - y1).abs().max(1.0)] }
            Kind::Points => { let p = c.points(24); if p.is_empty() { continue; } let mut r = [p[0][0], p[0][1], 0.0, 0.0]; for q in &p { r = union(r, [q[0], q[1], 0.0, 0.0]); } r }
            Kind::Path => { let p = c.points(8); if p.is_empty() { continue; } let mut r = [p[0][0], p[0][1], 0.0, 0.0]; for q in &p { r = union(r, [q[0], q[1], 0.0, 0.0]); } r }
            _ => match bounds(c) { Some(r) => r, None => continue },
        };
        let rect = match moves.last() { Some(m) => [m[0] + rect[0] * m[2], m[1] + rect[1] * m[2], rect[2] * m[2], rect[3] * m[2]], None => rect };
        let whole_rect = rect;
        let rect = cut(cut(rect, clip), SCREEN);
        let clipped = (rect[2] - whole_rect[2]).abs() > 1.0 || (rect[3] - whole_rect[3]).abs() > 1.0;
        if rect[2] < 1.0 || rect[3] < 1.0 || !rect.iter().all(|v| v.is_finite()) { continue; }
        let boxed = c.kind == Kind::Box;
        let (fill, border) = if boxed { c.box_fill() } else { ([0.0; 4], None) };
        let fill = if fill.iter().all(|v| v.is_finite()) { fill } else { [0.0; 4] };
        if rect[2] >= 1880.0 && rect[3] >= 1040.0 {
            if (boxed && fill[3] >= 0.9) || c.kind == Kind::Sprite { items.clear(); layer = 0; } else if boxed && fill[3] >= 0.15 { layer += 1; }
            continue;
        }
        // 경기 속 이름표(움직이는 20픽셀 상자 + 「Lv.」 글자)는 게임이 경기 자리 밖을 잘라 감추지만 명령에는 남는다.
        if hole.is_some() {
            let tag = |n: &Cmd| n.kind == Kind::Text && n.text.starts_with("Lv.") && (n.f(164) - 20.0).abs() < 2.0 && n.key.ends_with("light");
            if tag(c) || (boxed && (c.f(100) - 20.0).abs() < 1.0 && list.get(index + 1).is_some_and(tag)) { continue; }
        }
        // 경기 그림 자리 안의 반투명 겹침(채팅 등)은 테이블 쪽에서 다루지 않는다.
        if let Some(h) = hole {
            let over = area(cut(rect, h)) / area(rect).max(1.0);
            if over > 0.6 && rect[1] + rect[3] * 0.5 > h[1] + 52.0 {
                let solid = boxed && fill[3] >= 0.8 && area(rect) >= 2500.0;
                if !solid && !items.iter().any(|i| i.boxed && i.fill[3] >= 0.8 && area(i.rect) >= 2500.0 && area(cut(i.rect, h)) > 0.6 * area(i.rect) && inside(rect, i.rect, 2.0)) { continue; }
            }
        }
        let picture = matches!(c.kind, Kind::Sprite | Kind::Patch) && c.tex != 0 && !c.key.ends_with("sprite/white");
        items.push(Item { picture, rect, boxed, fill, stroke: border.is_some_and(|b| b[3] > 0.05), radius: if boxed { c.f(12).clamp(0.0, 60.0) } else { 0.0 }, layer, index, cutout: picture && !clipped && (c.key.contains("champions/") || c.key.contains("team_logo") || c.key.contains("player") || c.key.contains("item")), big: c.key.contains("champions/") });
    }
    // 창 고르기
    let mut wins: Vec<Win> = Vec::new();
    let mut owner: Vec<Option<usize>> = vec![None; items.len()];
    let mut spill: Vec<(usize, Rect)> = Vec::new();
    for (k, it) in items.iter().enumerate() {
        let visible = it.boxed && (it.fill[3] >= 0.2 || it.stroke);
        let home = wins.iter().rposition(|w| w.layer == it.layer && inside(it.rect, w.rect, 3.0));
        if let Some(h) = home { owner[k] = Some(h); continue; }
        // 스크롤로 창 위·아래 밖으로 흘러나온 줄은 그 창 것으로 보고 넘친 만큼 버린다.
        let r = it.rect;
        if let Some(h) = wins.iter().rposition(|w| { let b = w.rect; w.layer == it.layer && !w.bare && b[3] >= 200.0 && r[0] >= b[0] - 3.0 && r[0] + r[2] <= b[0] + b[2] + 3.0 && r[3] < b[3] * 0.6 && ((r[1] > b[1] && r[1] < b[1] + b[3] - 4.0 && r[1] + r[3] > b[1] + b[3] && b[1] + b[3] < 1078.0) || (r[1] < b[1] && r[1] + r[3] > b[1] + 4.0 && r[1] + r[3] < b[1] + b[3] && b[1] > 2.0)) && k > 0 && items[..k].iter().zip(&owner[..k]).any(|(o, w2)| w2.is_some() && o.boxed && (o.rect[0] - r[0]).abs() < 2.0 && (o.rect[2] - r[2]).abs() < 2.0 && inside(o.rect, b, 3.0)) }) { owner[k] = Some(h); spill.push((k, cut(r, wins[h].rect))); continue; }
        if visible && area(it.rect) >= 6000.0 && it.rect[2] >= 40.0 && it.rect[3] >= 24.0 {
            // 불투명한 새 창이 먼저 있던 창을 완전히 덮으면 그 창은 보이지 않는다.
            if it.fill[3] >= 0.95 { for w in wins.iter_mut() { if w.layer == it.layer && inside(w.rect, it.rect, 1.0) { w.rect = [0.0; 4]; } } }
            wins.push(Win { rect: it.rect, radius: it.radius, fill: it.fill, layer: it.layer, level: 0, bare: false, plates: Vec::new(), zones: Vec::new(), cuts: Vec::new(), order: it.index, scene: Vec::new(), ghost: false });
            owner[k] = Some(wins.len() - 1);
        }
    }
    for (k, r) in spill { items[k].rect = r; }
    // 어느 창에도 안 든 것들은 가까운 것끼리 묶어 테두리 없는 창으로.
    let loose: Vec<usize> = (0..items.len()).filter(|&k| owner[k].is_none() && !(items[k].boxed && items[k].fill[3] < 0.2 && !items[k].stroke)).collect();
    let mut groups: Vec<(Rect, u32, Vec<usize>)> = Vec::new();
    for &k in &loose {
        let (r, l) = (items[k].rect, items[k].layer);
        let center = [r[0] + r[2] * 0.5, r[1] + r[3] * 0.5];
        if let Some(h) = wins.iter().rposition(|w| w.layer == l && area(w.rect) > 0.0 && has(w.rect, center[0], center[1])) { owner[k] = Some(h); continue; }
        let wide = [r[0] - 16.0, r[1] - 14.0, r[2] + 32.0, r[3] + 28.0];
        let mut merged = (r, l, vec![k]);
        let mut j = 0;
        while j < groups.len() { if groups[j].1 == l && area(cut(groups[j].0, wide)) > 0.0 { let g = groups.swap_remove(j); merged.0 = union(merged.0, g.0); merged.2.extend(g.2); j = 0; } else { j += 1; } }
        groups.push(merged);
    }
    for (r, l, order) in ghosts { let r = cut(r, SCREEN); if area(r) > 16.0 { wins.push(Win { rect: r, radius: 0.0, fill: [0.0; 4], layer: l, level: 0, bare: true, plates: Vec::new(), zones: Vec::new(), cuts: Vec::new(), order, scene: Vec::new(), ghost: true }); } }
    for (r, l, ks) in groups {
        let pad = [r[0] - 10.0, r[1] - 8.0, r[2] + 20.0, r[3] + 16.0];
        let order = ks.iter().map(|&k| items[k].index).min().unwrap_or(0);
        wins.push(Win { rect: cut(pad, SCREEN), radius: 10.0, fill: [0.0; 4], layer: l, level: 0, bare: true, plates: Vec::new(), zones: Vec::new(), cuts: Vec::new(), order, scene: Vec::new(), ghost: false });
        let w = wins.len() - 1;
        for k in ks { owner[k] = Some(w); }
    }
    // 판·자리·잘라낼 그림
    for (k, it) in items.iter().enumerate() {
        let Some(w) = owner[k] else {
            // 투명한 자리(누를 수 있는 줄)는 위치로 창을 찾는다.
            if it.boxed { if let Some(w) = wins.iter_mut().rev().find(|w| w.layer == it.layer && inside(it.rect, w.rect, 6.0)) { if it.rect[2] >= 24.0 && it.rect[3] >= 16.0 { w.zones.push((it.rect, it.radius)); } } }
            continue;
        };
        if area(wins[w].rect) <= 0.0 { continue; }
        let whole = area(wins[w].rect);
        if area(it.rect) <= 0.0 { continue; }
        if it.boxed && it.rect != wins[w].rect {
            let usable = it.rect[2] >= 24.0 && it.rect[3] >= 16.0 && area(it.rect) <= whole * 0.8;
            if usable { wins[w].zones.push((it.rect, it.radius)); }
            let visible = it.fill[3] >= 0.5 || (it.stroke && it.fill[3] >= 0.05);
            if usable && visible && it.rect[3] >= 18.0 {
                let parent = wins[w].plates.iter().rev().find(|p| inside(it.rect, p.rect, 2.0));
                let depth = parent.map(|p| p.depth + 1).unwrap_or(1);
                let under = parent.map(|p| [p.fill[0], p.fill[1], p.fill[2]]).unwrap_or([wins[w].fill[0], wins[w].fill[1], wins[w].fill[2]]);
                let button = it.rect[3] <= 96.0 && it.rect[2] <= 560.0 && (it.radius >= 3.0 || it.stroke);
                if depth <= 3 { wins[w].plates.push(Plate { rect: it.rect, radius: it.radius, depth, fill: it.fill, under, button }); }
            }
        } else if it.cutout && it.rect[2] >= 14.0 {
            let parent = wins[w].plates.iter().rev().find(|p| inside(it.rect, p.rect, 6.0));
            let depth = parent.map(|p| p.depth).unwrap_or(0);
            // 나중에 그려진 불투명한 상자가 덮은 그림(목록 아래로 흘러 설명 칸에 가려진 줄 등)은 게임에서도 가려져 있으니 띄우지 않는다.
            if items[k + 1..].iter().any(|o| o.boxed && o.layer == it.layer && o.fill[3] >= 0.8 && area(cut(o.rect, it.rect)) > 0.10 * area(it.rect)) { continue; }
            // 그림 위에 놓인 그림은 바탕색을 모르므로 덮지도 띄우지도 않는다.
            if items[..k].iter().zip(&owner[..k]).any(|(o, w2)| *w2 == Some(w) && o.picture && area(cut(o.rect, it.rect)) > 0.6 * area(it.rect)) { continue; }
            let under = parent.map(|p| p.fill).or(if wins[w].bare { None } else { Some(wins[w].fill) }).filter(|f| f[3] >= 0.9).map(|f| [f[0], f[1], f[2]]);
            wins[w].cuts.push(Cut { index: it.index, rect: it.rect, depth, under, big: it.big });
        }
    }
    // 그림 장면(게이밍하우스처럼 배경 그림 여러 장 + 사람·가구): 층을 나눠 입체로 그린다.
    for (w, win) in wins.iter_mut().enumerate() {
        let pictures: Vec<usize> = (0..items.len()).filter(|&k| owner[k] == Some(w) && items[k].picture).collect();
        let covers = pictures.iter().filter(|&&k| area(items[k].rect) > 0.6 * area(win.rect)).count();
        if covers >= 2 && pictures.len() >= 20 && area(win.rect) > 200_000.0 { win.scene = (0..items.len()).filter(|&k| owner[k] == Some(w) && !items[k].boxed).map(|k| items[k].index).collect(); win.cuts.clear(); }
    }
    wins.retain(|w| area(w.rect) > 0.0);
    wins.sort_by_key(|w| (w.layer, w.order));
    for j in 0..wins.len() {
        let mut level = 0;
        for i in 0..j { if wins[i].layer == wins[j].layer { let o = area(cut(wins[i].rect, wins[j].rect)); if o > 0.1 * area(wins[i].rect).min(area(wins[j].rect)) { level = level.max(wins[i].level + 1); } } }
        wins[j].level = level.min(4);
    }
    let top_layer = wins.iter().map(|w| w.layer).max().unwrap_or(0);
    stage.sort_by_key(|s| s.0);
    Shelf { wins, top_layer, stage, fade, shows }
}
impl Win {
    /// 가리킨 자리: 그 점을 품은 가장 작은 자리.
    pub fn zone(&self, x: f32, y: f32) -> Option<(Rect, f32)> { self.zones.iter().filter(|z| has(z.0, x, y)).min_by(|a, b| area(a.0).total_cmp(&area(b.0))).copied() }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn boxed(r: Rect, a: f32) -> Cmd {
        let mut raw = [0u8; 208];
        for (o, v) in [(88, r[0]), (92, r[1]), (96, r[2]), (100, r[3]), (12, 8.0), (112, 0.1), (116, 0.1), (120, 0.12), (124, a)] { raw[o..o + 4].copy_from_slice(&f32::to_le_bytes(v)); }
        Cmd { kind: Kind::Box, raw, key: String::new(), tex: 0, shader: String::new(), style: vec![], text: String::new(), data: vec![] }
    }
    #[test]
    fn 창과_버튼과_팝업을_나눈다() {
        let list = vec![boxed(SCREEN, 1.0), boxed([0.0, 0.0, 264.0, 1080.0], 1.0), boxed([0.0, 16.0, 264.0, 48.0], 0.0), boxed([300.0, 20.0, 700.0, 300.0], 1.0), boxed([320.0, 250.0, 120.0, 40.0], 1.0), boxed(SCREEN, 0.5), boxed([600.0, 300.0, 700.0, 400.0], 1.0)];
        let s = analyze(&list, None, &|_| None);
        assert_eq!(s.wins.len(), 3);
        assert_eq!(s.top_layer, 1);
        assert_eq!(s.wins[2].layer, 1);
        let card = &s.wins[1];
        assert_eq!(card.plates.len(), 1);
        assert!(card.plates[0].button);
        assert_eq!(s.wins[0].zones.len(), 1);
        assert_eq!(card.zone(330.0, 260.0).unwrap().0, [320.0, 250.0, 120.0, 40.0]);
    }
    #[test]
    fn 전체를_덮는_새_화면은_앞의_것을_지운다() {
        let list = vec![boxed([10.0, 10.0, 500.0, 500.0], 1.0), boxed(SCREEN, 1.0), boxed([700.0, 100.0, 300.0, 300.0], 1.0)];
        let s = analyze(&list, None, &|_| None);
        assert_eq!(s.wins.len(), 1);
    }
}

#[cfg(test)]
mod move_tests {
    use super::*;
    fn cmd(kind: Kind, set: &[(usize, f32)]) -> Cmd { let mut c = Cmd { kind, raw: [0; 208], key: String::new(), tex: 0, shader: String::new(), style: vec![], text: String::new(), data: vec![] }; for (at, v) in set { c.raw[*at..*at + 4].copy_from_slice(&v.to_le_bytes()); } c }
    #[test]
    fn 커지는_단추는_제자리에서_커진다() {
        // 타이틀 단추에 마우스를 올렸을 때의 실제 명령(0.7.3).
        let list = vec![
            cmd(Kind::Box, &[(88, 950.0), (92, 0.0), (96, 970.0), (100, 1080.0), (124, 1.0)]),
            cmd(Kind::Other(18), &[(8, 1121.0), (12, 480.0), (16, 608.0), (20, 50.0), (28, 0.5), (32, 0.5), (36, 1.024), (40, 1.024)]),
            cmd(Kind::Other(18), &[(8, -1425.0), (12, -505.0), (36, 1.0), (40, 1.0)]),
            cmd(Kind::Box, &[(88, 1121.0), (92, 480.0), (96, 608.0), (100, 50.0), (124, 1.0)]),
            cmd(Kind::Other(19), &[]), cmd(Kind::Other(19), &[]),
        ];
        let shelf = analyze(&list, None, &|_| None);
        assert!(shelf.shows.is_empty());
        let w = &shelf.wins[0];
        let r = w.plates.first().map(|p| p.rect).or(shelf.wins.get(1).map(|w| w.rect)).unwrap();
        assert!((r[0] + r[2] * 0.5 - 1425.0).abs() < 1.0 && (r[1] + r[3] * 0.5 - 505.0).abs() < 1.0 && (r[2] - 622.6).abs() < 1.0, "{r:?}");
    }
}
