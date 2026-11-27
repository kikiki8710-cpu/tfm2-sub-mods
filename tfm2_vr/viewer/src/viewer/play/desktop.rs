//! 헤드셋 없이 본 흐름을 시험한다: --play-desktop [--game-exe 경로] [--seconds N] [--shots 폴더] [--shot-every 초] [--click 초,x,y]…
use super::{game, Play};
use crate::{gpu::Gpu, live::feed::Feed, log::Log};
use anyhow::Result;
use glam::{Mat4, Quat, Vec3};
use std::{path::PathBuf, time::{Duration, Instant}};

pub fn run(log: &Log, args: &[String]) -> Result<()> {
    let value = |flag: &str| args.iter().position(|a| a == flag).and_then(|i| args.get(i + 1)).cloned();
    let seconds: f64 = value("--seconds").and_then(|s| s.parse().ok()).unwrap_or(60.0);
    let every: f64 = value("--shot-every").and_then(|s| s.parse().ok()).unwrap_or(5.0);
    let shots = PathBuf::from(value("--shots").unwrap_or_else(|| "shots".into()));
    std::fs::create_dir_all(&shots)?;
    let mut clicks: Vec<(f64, f32, f32)> = args.iter().enumerate().filter(|(_, a)| *a == "--click").filter_map(|(i, _)| args.get(i + 1)).filter_map(|s| { let v: Vec<f64> = s.split(',').filter_map(|x| x.parse().ok()).collect(); (v.len() == 3).then(|| (v[0], v[1] as f32, v[2] as f32)) }).collect();
    clicks.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut gpu = Gpu::new(log, None, 0xb000)?;
    let mut play = Play::new(&mut gpu, Feed::game()?, log)?;
    let (texture, target) = gpu.offscreen(1280, 960)?;
    let head = Vec3::new(0.0, 1.45, 0.0);
    play.recenter(head, Quat::IDENTITY);
    if game::find().is_none() { if let Some(exe) = value("--game-exe") { game::launch(Some(&exe))?; } }
    if args.iter().any(|a| a == "--flat") { play.flat = true; } else if args.iter().any(|a| a == "--curved") { play.flat = false; }
    game::priority();
    let start = Instant::now();
    let (mut next, mut n, mut frames, mut worst) = (0.0f64, 0, 0u64, 0.0f64);
    let mut report = Instant::now();
    let mut previous = Instant::now();
    let sweep = args.iter().any(|a| a == "--sweep");
    let (mut swept, mut bad, mut held): ((usize, u32, Option<(super::shelf::Rect, Option<super::shelf::Rect>)>), u32, u32) = ((0, 0, None), 0, 0);
    while start.elapsed().as_secs_f64() < seconds && !play.finished() {
        let t = Instant::now();
        let pitch = if play.in_match() { -0.42 } else { 0.0 };
        let rotation = Quat::from_rotation_x(pitch);
        // 클릭 시험: 레이저 대신 화면 좌표(UI 픽셀)를 바로 누른다.
        let now = start.elapsed().as_secs_f64();
        if let Some(&(at, x, y)) = clicks.first() {
            if let Some(m) = play.mouse() {
                if now > at - 1.2 { m.focus(); m.move_to(x / 1920.0, y / 1080.0); }
                if now > at { m.button(true); }
                if now > at + 0.12 { m.button(false); clicks.remove(0); log.line(format!("클릭 {x},{y}")); }
            }
        }
        // --sweep: 레이저를 여러 곳에 1.6초씩 **가만히** 대고 헤드셋처럼 마우스를 움직여, 게임의 반응(설명 창·커지는 단추)까지 합쳐 가리킴이 저절로 바뀌는지 센다.
        let spot = (now / 1.6) as usize;
        let direction = if sweep { let (c, r) = (spot % 9, (spot / 9) % 6); Quat::from_rotation_y((4.0 - c as f32) * 0.105) * Quat::from_rotation_x((2.2 - r as f32) * 0.085) * Vec3::NEG_Z } else { rotation * Vec3::NEG_Z };
        let hand = head + Vec3::new(0.2, -0.3, -0.2);
        let hit = play.hit(hand, direction);
        if sweep {
            let target = hit.as_ref().and_then(|h| play.target(h));
            if spot != swept.0 { if swept.1 > 1 { log.line(format!("흔들린 자리 {} 번: {}회 {:?}", swept.0, swept.1, swept.2)); bad += 1; } if swept.2.is_some() { held += 1; } swept = (spot, 0, target); }
            else if now - spot as f64 * 1.6 < 0.35 { swept.2 = target; }
            else if target != swept.2 { swept.1 += 1; swept.2 = target; }
            if let (Some(h), Some(m)) = (&hit, play.mouse()) { m.focus(); m.move_to(h.x / 1920.0, h.y / 1080.0); }
        }
        play.update(&mut gpu, head, rotation, &[(hand, direction, hit.as_ref().map(|h| h.distance))], &[], hit.as_ref().map(|h| (h, false)), previous.elapsed().as_secs_f32().min(0.1))?;
        previous = Instant::now();
        let cpu = t.elapsed().as_secs_f64() * 1000.0;
        let view = Mat4::from_rotation_translation(rotation, head).inverse();
        gpu.render(&target, Mat4::perspective_rh(80f32.to_radians(), 1280.0 / 960.0, 0.01, 40.0) * view, play.world(), true);
        { let all = t.elapsed().as_secs_f64() * 1000.0; if all > 14.0 { log.line(format!("느린 장 {all:.1}ms 준비 {cpu:.1} [감시 {:.1} 화면 {:.1} 장면 {:.1} 방 {:.1}] 그리기 {:.1}", play.steps[0], play.steps[1], play.steps[2], play.steps[3], all - cpu)); } }
        worst = worst.max(t.elapsed().as_secs_f64() * 1000.0);
        frames += 1;
        if now >= next { gpu.save_png(&texture, &shots.join(format!("play_{n:03}.png")))?; n += 1; next = now + every; }
        if report.elapsed() > Duration::from_secs(5) { log.line(format!("{:.0}초 배치 {:?} 화면 {} {} 장면 {:?} 최악 {worst:.1}ms 그린 {frames}", now, play.layout, play.capture_frames(), play.summary(), play.live.stats)); report = Instant::now(); worst = 0.0; }
        std::thread::sleep(Duration::from_millis(6));
    }
    gpu.check()?;
    if sweep { log.line(format!("머문 자리 {held} 흔들린 자리 {bad}")); }
    log.line(format!("본 흐름 시험 끝 {:.0}초 그린 {frames}", start.elapsed().as_secs_f64()));
    Ok(())
}

/// --ui-compare <폴더> <이름> [match]: 저장한 명령 1장(frame_*.bin, tex_*.bin)과 화면 그림(screen.png)으로 방을 그려 본다.
pub fn compare(log: &Log, args: &[String]) -> Result<()> {
    use super::Layout;
    let folder = PathBuf::from(&args[2]);
    let label = args.get(3).cloned().unwrap_or_else(|| "ui".into());
    let mut gpu = Gpu::new(log, None, 0xb000)?;
    let mut play = Play::new(&mut gpu, Feed::folder(&folder)?, log)?;
    let image = image::open(folder.join("screen.png"))?.to_rgba8();
    gpu.panel_image(&image)?;
    let (texture, target) = gpu.offscreen(1600, 1200)?;
    let head = Vec3::new(0.0, 1.45, 0.0);
    play.recenter(head, Quat::IDENTITY);
    play.frozen = true;
    play.flat = args.iter().any(|a| a == "--flat");
    let views = [("front", 0.0f32, 0.0f32, Vec3::ZERO, 82f32), ("left", 0.45, -0.05, Vec3::new(-0.25, 0.05, 0.0), 82.0), ("near", -0.35, 0.02, Vec3::new(0.35, -0.05, -0.75), 70.0)];
    let mut aim = (Vec3::new(0.18, 1.15, -0.2), Vec3::new(-0.10, 0.10, -1.0).normalize());
    if let Some(v) = args.get(4).filter(|v| v.contains(',')) { let p: Vec<f32> = v.split(',').filter_map(|x| x.parse().ok()).collect(); if p.len() == 2 { aim.1 = Vec3::new(p[0], p[1], -1.0).normalize(); } }
    for (name, yaw, pitch, shift, fov) in views {
        let rotation = Quat::from_rotation_y(yaw) * Quat::from_rotation_x(pitch);
        let eye = head + shift;
        for _ in 0..70 {
            let hit = play.hit(aim.0, aim.1);
            play.update(&mut gpu, eye, rotation, &[(aim.0, aim.1, hit.as_ref().map(|h| h.distance))], &[], hit.as_ref().map(|h| (h, false)), 0.016)?;
            std::thread::sleep(Duration::from_millis(6));
        }
        gpu.render(&target, Mat4::perspective_rh(fov.to_radians(), 1600.0 / 1200.0, 0.01, 40.0) * Mat4::from_rotation_translation(rotation, eye).inverse(), play.world(), true);
        gpu.save_png(&texture, &crate::log::root().join(format!("_tmp/shots/{label}_{name}.png")))?;
    }
    if args.iter().any(|a| a == "--dump") { play.dump(); }
    let s = play.summary();
    log.line(format!("{label} 배치 {:?} {s}", play.layout as Layout));
    Ok(())
}

/// --ui-play <폴더> <이름> [걸음] [--flat]: 저장한 명령 여러 장(frame_*.bin)을 차례로 흘려 연출을 본다. 걸음마다 그림 1장.
pub fn play(log: &Log, args: &[String]) -> Result<()> {
    let folder = PathBuf::from(&args[2]);
    let label = args.get(3).cloned().unwrap_or_else(|| "play".into());
    let every: usize = args.get(4).and_then(|v| v.parse().ok()).unwrap_or(6);
    let mut gpu = Gpu::new(log, None, 0xb000)?;
    let mut play = Play::new(&mut gpu, Feed::folder(&folder)?, log)?;
    let image = image::open(folder.join("screen.png"))?.to_rgba8();
    gpu.panel_image(&image)?;
    let (texture, target) = gpu.offscreen(1280, 960)?;
    let head = Vec3::new(0.0, 1.45, 0.0);
    play.recenter(head, Quat::IDENTITY);
    play.frozen = true;
    play.flat = args.iter().any(|a| a == "--flat");
    play.live.hold = true;
    let mut files: Vec<_> = std::fs::read_dir(&folder)?.flatten().map(|e| e.path()).filter(|p| p.file_name().is_some_and(|n| n.to_string_lossy().starts_with("frame_"))).collect();
    files.sort();
    let out = crate::log::root().join("_tmp/shots").join(&label);
    std::fs::create_dir_all(&out)?;
    let mut worst = 0.0f64;
    // 그림 준비: 흐름을 여러 번 돌려 칸을 채운 뒤 한 번 더 흘리며 찍는다.
    for pass in 0..4 {
        // 게임 연출이 끝난 뒤에도 카드가 남으므로 마지막 장을 더 흘린다.
        for (i, f) in files.iter().chain(std::iter::repeat_n(files.last().unwrap(), 70)).enumerate() {
            play.live.put(crate::live::frame::parse(&std::fs::read(f)?)?);
            let t = Instant::now();
            play.update(&mut gpu, head, Quat::IDENTITY, &[], &[], None, 0.020)?;
            if pass == 3 { worst = worst.max(t.elapsed().as_secs_f64() * 1000.0); }
            if pass < 3 { std::thread::sleep(Duration::from_millis(2)); continue; }
            if i % every == 0 {
                gpu.render(&target, Mat4::perspective_rh(74f32.to_radians(), 1280.0 / 960.0, 0.01, 40.0) * Mat4::from_rotation_translation(Quat::IDENTITY, head).inverse(), play.world(), true);
                gpu.save_png(&texture, &out.join(format!("{i:03}.png")))?;
            }
        }
    }
    log.line(format!("{label} 장 {} 최악 {worst:.2}ms", files.len()));
    Ok(())
}

/// --ui-jitter <폴더>: 저장한 명령을 차례로 흘리며 같은 창(UI 사각형이 거의 같은)이 프레임 사이에 얼마나 움직이는지 잰다.
pub fn jitter(log: &Log, args: &[String]) -> Result<()> {
    let folder = PathBuf::from(&args[2]);
    let mut gpu = Gpu::new(log, None, 0xb000)?;
    let mut play = Play::new(&mut gpu, Feed::folder(&folder)?, log)?;
    let image = image::open(folder.join("screen.png"))?.to_rgba8();
    gpu.panel_image(&image)?;
    let head = Vec3::new(0.0, 1.45, 0.0);
    play.recenter(head, Quat::IDENTITY);
    play.frozen = true;
    play.live.hold = true;
    let mut files: Vec<_> = std::fs::read_dir(&folder)?.flatten().map(|e| e.path()).filter(|p| p.file_name().is_some_and(|n| n.to_string_lossy().starts_with("frame_"))).collect();
    files.sort();
    let mut worst: Vec<(f32, super::shelf::Rect)> = Vec::new();
    for pass in 0..3 {
        let mut last: Vec<(super::shelf::Rect, Vec3)> = Vec::new();
        for (i, f) in files.iter().enumerate() {
            let data = std::fs::read(f)?;
            for _ in 0..(if pass == 0 && i == 0 { 60 } else { 1 }) { play.live.put(crate::live::frame::parse(&data)?); play.update(&mut gpu, head, Quat::IDENTITY, &[], &[], None, 0.02)?; if pass == 0 { std::thread::sleep(Duration::from_millis(3)); } }
            let now = play.room.placed();
            if pass == 2 { for (r, p) in &now { if let Some((_, q)) = last.iter().find(|(s, _)| (0..4).all(|k| (s[k] - r[k]).abs() < 1.0)) { worst.push(((*p - *q).length(), *r)); } } }
            if pass == 2 { log.line(format!("창 {} 새 창 {}", now.len(), now.iter().filter(|(r, _)| !last.iter().any(|(s, _)| (0..4).all(|k| (s[k] - r[k]).abs() < 1.0))).count())); }
            if pass == 2 && args.iter().any(|a| a == "--dump") { for (r, p) in now.iter().filter(|(r, _)| !last.iter().any(|(s, _)| (0..4).all(|k| (s[k] - r[k]).abs() < 1.0))) { log.line(format!("새 {:?} @ {:.3?}", r.map(|v| v as i32), p.to_array())); } for (r, _) in last.iter().filter(|(r, _)| !now.iter().any(|(s, _)| (0..4).all(|k| (s[k] - r[k]).abs() < 1.0))) { log.line(format!("사라짐 {:?}", r.map(|v| v as i32))); } }
            last = now;
        }
    }
    worst.sort_by(|a, b| b.0.total_cmp(&a.0));
    for w in worst.iter().take(8) { log.line(format!("움직임 {:.1}mm {:?}", w.0 * 1000.0, w.1.map(|v| v as i32))); }
    Ok(())
}

/// --ui-hover <폴더> [--flat]: 화면 하나를 고정하고, 레이저를 여러 방향으로 **가만히** 둔 채 가리킴이 저절로 바뀌는지(깜박임) 센다.
pub fn hover(log: &Log, args: &[String]) -> Result<()> {
    let folder = PathBuf::from(&args[2]);
    let mut gpu = Gpu::new(log, None, 0xb000)?;
    let mut play = Play::new(&mut gpu, Feed::folder(&folder)?, log)?;
    let image = image::open(folder.join("screen.png"))?.to_rgba8();
    gpu.panel_image(&image)?;
    let head = Vec3::new(0.0, 1.45, 0.0);
    play.recenter(head, Quat::IDENTITY);
    play.frozen = true;
    play.flat = args.iter().any(|a| a == "--flat");
    for _ in 0..120 { play.update(&mut gpu, head, Quat::IDENTITY, &[], &[], None, 0.02)?; std::thread::sleep(Duration::from_millis(3)); }
    play.live.hold = true;
    let (mut rays, mut touched, mut flicker, mut flips, mut drift) = (0, 0, 0, 0, 0.0f32);
    // 손 떨림(도): 두 주파수를 섞는다.
    let tremor: f32 = args.iter().position(|a| a == "--tremor").and_then(|i| args.get(i + 1)).and_then(|v| v.parse().ok()).unwrap_or(0.0);
    if args.iter().any(|a| a == "--cross") {
        // 다른 곳을 가리키다 옮겨 왔을 때, 처음부터 거기를 가리킨 것과 같은가(달라붙어 안 넘어가는지).
        let ray = |k: u32| { let f = |n: u32| { let mut x = (k * 2 + n).wrapping_mul(0x9E37_79B9); x ^= x >> 15; x = x.wrapping_mul(0x85eb_ca6b); x ^= x >> 13; (x & 0xffff) as f32 / 65535.0 }; Quat::from_rotation_y((f(0) - 0.5) * 1.6) * Quat::from_rotation_x((f(1) - 0.5) * 0.9) * Vec3::NEG_Z };
        let hand = Vec3::new(0.25, 1.10, -0.25);
        let (mut tried, mut stuck, mut fresh_worse) = (0, 0, 0);
        for k in 0..6000u32 {
            let d = ray(k);
            for _ in 0..14 { play.update(&mut gpu, head, Quat::IDENTITY, &[], &[], None, 0.05)?; }
            let Some(h) = play.hit(hand, d) else { continue };
            let clean = (h.key, h.zone);
            // 자리 안쪽 15픽셀 이상일 때만 본다(가장자리는 일부러 여유를 둔다).
            let Some(z) = h.zone else { continue };
            let inner = h.x > z[0] + 15.0 && h.x < z[0] + z[2] - 15.0 && h.y > z[1] + 15.0 && h.y < z[1] + z[3] - 15.0;
            if !inner { continue; }
            // 근처(±3도)를 먼저 가리키다 옮겨 온다.
            let near = Quat::from_rotation_y(0.05 * ((k % 5) as f32 - 2.0)) * Quat::from_rotation_x(0.05 * ((k % 3) as f32 - 1.0)) * d;
            for _ in 0..30 { let q = play.hit(hand, near); play.update(&mut gpu, head, Quat::IDENTITY, &[], &[], q.as_ref().map(|q| (q, false)), 0.0111)?; }
            let mut last = None;
            for _ in 0..30 { let q = play.hit(hand, d); last = q.as_ref().map(|q| (q.key, q.zone)); play.update(&mut gpu, head, Quat::IDENTITY, &[], &[], q.as_ref().map(|q| (q, false)), 0.0111)?; }
            tried += 1;
            if last != Some(clean) { stuck += 1; if stuck < 4 { log.line(format!("다름 {clean:?} → {last:?}")); } }
            let _ = &mut fresh_worse;
        }
        log.line(format!("옮겨 가리키기 {} · 시험 {tried} 안 넘어감 {stuck}", folder.display()));
        return Ok(());
    }
    for hand in [Vec3::new(0.25, 1.10, -0.25), Vec3::new(-0.30, 0.95, -0.15)] {
        for yi in -32..=32 { for pi in -18..=20 {
            rays += 1;
            let d = Quat::from_rotation_y(yi as f32 * 0.0275) * Quat::from_rotation_x(pi as f32 * 0.0275) * Vec3::NEG_Z;
            // 앞 가리킴의 흔적을 지운다.
            for _ in 0..14 { play.update(&mut gpu, head, Quat::IDENTITY, &[], &[], None, 0.05)?; }
            let (mut last, mut n, mut lo, mut hi) = (None, 0, [f32::INFINITY; 2], [f32::NEG_INFINITY; 2]);
            let mut any = false;
            for f in 0..60 {
                let t = f as f32 * 0.0111;
                let d = Quat::from_rotation_y((tremor * ((t * 51.0).sin() + 0.5 * (t * 23.0).sin())).to_radians()) * Quat::from_rotation_x((tremor * ((t * 44.0).cos() + 0.5 * (t * 19.0).sin())).to_radians()) * d;
                let hit = play.hit(hand, d);
                let now = hit.as_ref().and_then(|h| play.target(h));
                any |= now.is_some();
                if f > 0 && now != last { n += 1; }
                if let Some(h) = &hit { if f > 0 { lo = [lo[0].min(h.x), lo[1].min(h.y)]; hi = [hi[0].max(h.x), hi[1].max(h.y)]; } }
                last = now;
                play.update(&mut gpu, head, Quat::IDENTITY, &[(hand, d, hit.as_ref().map(|h| h.distance))], &[], hit.as_ref().map(|h| (h, false)), 0.0111)?;
            }
            if any { touched += 1; if n > 1 { flicker += 1; flips += n; } if n <= 1 && hi[0] >= lo[0] { drift = drift.max(hi[0] - lo[0]).max(hi[1] - lo[1]); } }
        } }
    }
    log.line(format!("가리킴 시험 {} · 방향 {rays} 닿음 {touched} 깜박임 {flicker}({:.2}%) 바뀐 횟수 {flips} · 안 깜박인 방향의 최대 밀림 {drift:.1}px", folder.display(), flicker as f32 * 100.0 / touched.max(1) as f32));
    Ok(())
}
