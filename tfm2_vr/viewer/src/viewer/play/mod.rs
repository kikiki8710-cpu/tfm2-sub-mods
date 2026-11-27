//! VR 판 본 흐름: 게임 화면 패널 + 경기 테이블. 경기(게임 화면 명령)가 있으면 테이블을 펴고, 화면에서 경기 그림을 뺀 나머지만 패널로 둔다.
pub mod capture;
pub mod desktop;
pub mod dock;
pub mod game;
pub mod keyboard;
pub mod room;
pub mod shelf;
pub mod show;
pub mod xr;

use crate::{geometry::{self, Instance}, gpu::Gpu, live::{feed::Feed, Live}, log::Log, scene::TABLE};
use anyhow::Result;
use capture::Capture;
use game::Mouse;
pub use room::Hit;
use room::{Place, Room};
use shelf::Shelf;
use glam::{Mat4, Quat, Vec2, Vec3};
use std::time::{Duration, Instant};
use windows::Win32::Foundation::HWND;

const UI: [f32; 2] = [1920.0, 1080.0];
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Layout { Waiting, Screen, Match([i32; 4]) }
pub struct Play {
    pub table_height: f32, pub table_scale: f32, pub table_origin: Vec3, pub table_yaw: f32, pub floor: f32,
    pub live: Live, capture: Option<Capture>, making: Option<std::sync::mpsc::Receiver<Made>>, stalled: Option<Instant>, steady: Instant, mouse: Option<Mouse>, hwnd: Option<HWND>, search: Instant, pub launched: bool, seen_game: bool, lost: Option<Instant>,
    pub room: Room, pub keyboard: keyboard::Keyboard, pub dock: dock::Dock, pub flat: bool, started: Instant, fresh: Instant, shelf: Shelf, shelf_frame: u64, shelf_ms: f64, room_ms: f64, pub steps: [f64; 4], pub layout: Layout, home: (Vec3, f32), match_since: Option<Instant>, last_match: Instant,
    laser_id: usize, marker_id: usize, log: Log, crop: [f32; 4], failed: Instant, capture_seen: Instant, capture_retry: Instant, cover: Option<game::Cover>, covered: Instant, pub frozen: bool, pub status: &'static str,
}
/// 다른 흐름으로 넘기는 것들(D3D11 장치와 창 손잡이는 어느 흐름에서 써도 된다).
struct Pack(windows::Win32::Graphics::Direct3D11::ID3D11Device, HWND);
unsafe impl Send for Pack {}
struct Made(HWND, Result<Capture>);
unsafe impl Send for Made {}
struct Old(Capture);
unsafe impl Send for Old {}
fn settings_flat() -> bool { std::fs::read_to_string(crate::log::root().join("play_settings.txt")).is_ok_and(|s| s.contains("flat=1")) }
impl Play {
    pub fn new(gpu: &mut Gpu, feed: Feed, log: &Log) -> Result<Self> {
        let boxes = [(Vec3::new(2.52, 0.12, 2.16), Vec3::new(0.0, 0.695, 0.0), 0x334b4au32), (Vec3::new(2.4, 0.035, 2.04), Vec3::new(0.0, 0.735, 0.0), 0x354b42), (Vec3::new(0.62, 0.66, 0.62), Vec3::new(0.0, 0.33, 0.0), 0x1a3035)].iter().map(|&(s, p, c)| geometry::solid_instance(Mat4::from_scale_rotation_translation(s, Quat::IDENTITY, p), geometry::color_hex(c, 1.0))).collect();
        gpu.add_mesh(&geometry::mesh("live0table", geometry::box_vertices(), boxes, false, true), 4)?;
        let room = Room::new(gpu)?;
        let keyboard = keyboard::Keyboard::new(gpu)?;
        let dock = dock::Dock::new(gpu)?;
        let live = Live::new(gpu, feed, log)?;
        let mut dynamic = |name: &str, v, blend, write| -> Result<usize> { let mut m = geometry::mesh(name, v, vec![], blend, write); m.dynamic = true; gpu.add_mesh(&m, 256) };
        let laser_id = dynamic("live0laser", geometry::box_vertices(), false, true)?;
        let marker_id = dynamic("live0marker", geometry::icosahedron(), false, true)?;
        gpu.live = true;
        let now = Instant::now();
        Ok(Self { table_height: TABLE, table_scale: 1.0, table_origin: Vec3::new(0.0, 0.0, -1.4), table_yaw: 0.0, floor: 0.0, live, capture: None, making: None, stalled: None, steady: now, mouse: None, hwnd: None, search: now - Duration::from_secs(5), launched: false, seen_game: false, lost: None, room, keyboard, dock, flat: settings_flat(), started: now, fresh: now, shelf: Shelf::default(), shelf_frame: u64::MAX, shelf_ms: 0.0, room_ms: 0.0, steps: [0.0; 4], layout: Layout::Waiting, home: (Vec3::new(0.0, 1.3, 0.0), 0.0), match_since: None, last_match: now - Duration::from_secs(60), laser_id, marker_id, log: log.clone(), crop: [0.0, 0.0, 1.0, 1.0], failed: now - Duration::from_secs(60), capture_seen: now, capture_retry: now, cover: None, covered: now, frozen: false, status: "게임을 찾는 중" })
    }
    pub fn world(&self) -> Mat4 { Mat4::from_scale_rotation_translation(Vec3::splat(self.table_scale), Quat::from_rotation_y(self.table_yaw), Vec3::new(self.table_origin.x, self.table_height, self.table_origin.z)) * Mat4::from_translation(Vec3::new(0.0, -TABLE, 0.0)) }
    pub fn in_match(&self) -> bool { matches!(self.layout, Layout::Match(_)) }
    /// 휜 화면과 평평한 화면을 바꾸고 다음 실행에도 기억한다.
    pub fn set_flat(&mut self, flat: bool) {
        self.flat = flat;
        self.log.line(if flat { "화면 평면" } else { "화면 곡면" });
        let _ = std::fs::write(crate::log::root().join("play_settings.txt"), format!("flat={}\n", flat as u8));
    }
    /// 보는 방향 앞에 다시 놓는다.
    pub fn recenter(&mut self, head: Vec3, rotation: Quat) {
        let mut d = rotation * Vec3::NEG_Z;
        d.y = 0.0;
        let d = if d.length_squared() < 0.0001 { Vec3::NEG_Z } else { d.normalize() };
        self.table_origin = head + d * 1.4;
        self.table_yaw = (-d.x).atan2(-d.z);
        self.home = (head, self.table_yaw);
        self.dock.place(head, self.table_yaw);
    }
    /// 게임이 꺼진 지 몇 초 지났으면 true(프로그램 종료).
    pub fn finished(&self) -> bool { self.seen_game && self.lost.is_some_and(|t| t.elapsed() > Duration::from_secs(6)) }
    fn watch_game(&mut self, gpu: &Gpu) {
        if let Some(h) = self.hwnd { if !game::alive(h) || self.capture.as_ref().is_some_and(|c| c.closed()) { self.log.line("게임 창이 사라짐"); self.hwnd = None; self.capture = None; self.mouse = None; self.lost = Some(Instant::now()); self.status = "게임을 찾는 중"; } }
        if self.hwnd.is_none() && self.search.elapsed() > Duration::from_millis(800) {
            self.search = Instant::now();
            if let Some(h) = game::find() { self.hwnd = Some(h); self.mouse = Some(Mouse::new(h)); self.seen_game = true; self.lost = None; game::favor(h); self.log.line("게임 창 발견"); self.status = "게임 화면 연결 중"; }
        }
        if let Some(h) = self.hwnd { if self.covered.elapsed() > Duration::from_millis(500) { self.covered = Instant::now(); if self.cover.is_none() { self.cover = game::Cover::new(); } if let Some(c) = &mut self.cover { c.follow(h); } } } else { self.cover = None; }
        if let Some(rx) = &self.making {
            match rx.try_recv() {
                Ok(Made(h, Ok(c))) => { self.making = None; if self.hwnd.is_some() { if self.hwnd != Some(h) { self.hwnd = Some(h); if let Some(m) = &mut self.mouse { m.retarget(h); } } self.capture = Some(c); self.capture_seen = Instant::now(); self.log.line("게임 화면 받기 시작"); } }
                Ok(Made(_, Err(e))) => { self.making = None; self.failed = Instant::now(); self.log.line(format!("게임 화면 받기 실패: {e:#}")); }
                Err(std::sync::mpsc::TryRecvError::Disconnected) => { self.making = None; self.failed = Instant::now(); }
                Err(_) => {}
            }
        }
        if let (Some(h), None, None) = (self.hwnd, &self.capture, &self.making) {
            if self.failed.elapsed() > Duration::from_secs(2) {
                // 연결은 0.1초쯤 걸려 헤드셋 화면이 끊기므로 따로 만든다.
                let (tx, rx) = std::sync::mpsc::channel();
                let pack = Pack(gpu.device.clone(), h);
                std::thread::spawn(move || { let pack = pack; let Pack(device, h) = pack; let h = game::find().unwrap_or(h); let _ = tx.send(Made(h, Capture::new(&device, h))); });
                self.making = Some(rx);
            }
        }
    }
    /// 시험용: 저장한 화면 그림을 패널로 쓴다.
    pub fn force(&mut self, gpu: &mut Gpu, layout: Layout) { self.layout = layout; gpu.live = true; }
    /// 경기 그림이 놓인 자리(UI 픽셀). 없으면 전체 화면 패널.
    fn game_rect(&self) -> Option<[i32; 4]> {
        let f = self.live.current()?;
        let c = f.list("UI").iter().find(|c| c.key == "#Game")?;
        let (w, h) = (c.f(0x70) * 2048.0, c.f(0x74) * 2048.0);
        let (x, y) = (c.f(0x78), c.f(0x7c));
        (w > 300.0 && h > 300.0).then_some([x as i32, y as i32, w as i32, h as i32])
    }
    pub fn hit(&self, origin: Vec3, direction: Vec3) -> Option<Hit> { self.room.hit(origin, direction) }
    pub fn uv(hit: &Hit) -> Vec2 { Vec2::new(hit.x / UI[0], hit.y / UI[1]) }
    pub fn mouse(&mut self) -> Option<&mut Mouse> { self.mouse.as_mut() }
    pub fn dump(&self) { for w in &self.shelf.wins { self.log.line(format!("창 {:?} 층{} 단{} 맨{} 채움{:?} 판{} 그림{} 장면{} 순서{}", w.rect.map(|v| v as i32), w.layer, w.level, w.bare, w.fill, w.plates.len(), w.cuts.len(), w.scene.len(), w.order)); } }
    /// 시험용: 이 맞은 점이 가리키는 (창 사각형, 자리 사각형).
    pub fn target(&self, hit: &Hit) -> Option<(shelf::Rect, Option<shelf::Rect>)> { self.shelf.wins.iter().find(|w| room::key(w) == hit.key).map(|w| (w.rect, hit.zone)) }
    pub fn summary(&self) -> String { let w = &self.shelf.wins; format!("나누기 {:.2}ms 방 {:.2}ms 창 {} 판 {} 자리 {} 그림 {} 층 {}", self.shelf_ms, self.room_ms, w.len(), w.iter().map(|w| w.plates.len()).sum::<usize>(), w.iter().map(|w| w.zones.len()).sum::<usize>(), w.iter().map(|w| w.cuts.len()).sum::<usize>(), self.shelf.top_layer) }
    pub fn capture_frames(&self) -> u64 { self.capture.as_ref().map(|c| c.frames).unwrap_or(0) }
    /// lasers: (손 위치, 방향, 맞은 거리) · points: 잡은 손.
    pub fn update(&mut self, gpu: &mut Gpu, head: Vec3, rotation: Quat, lasers: &[(Vec3, Vec3, Option<f32>)], points: &[(Vec3, bool)], pointer: Option<(&Hit, bool)>, dt: f32) -> Result<()> {
        let t0 = Instant::now();
        if !self.frozen { self.watch_game(gpu); }
        self.steps[0] = t0.elapsed().as_secs_f64() * 1000.0;
        let t0 = Instant::now();
        if let Some(c) = &mut self.capture {
            let context = gpu.context.clone();
            match c.poll(&gpu.device, &context) { Ok(true) => { self.capture_seen = Instant::now(); if c.frames % 60 == 1 { if let Some(h) = self.hwnd { self.crop = game::client_uv(h); } } } Ok(false) => {} Err(e) => { self.log.line(format!("게임 화면 받기 오류: {e:#}")); self.capture = None; } }
            if let Some(v) = self.capture.as_ref().and_then(|c| c.view.clone()) { gpu.panel = Some(v); }
        }
        // 게임이 멈췄는지 살피고 기록한다(불러오는 중·절전·다른 창 등을 나중에 가릴 수 있게).
        let silent = self.live.silence();
        if !self.frozen && self.live.stats.frames > 0 {
            if silent > 1.0 && self.stalled.is_none() { self.stalled = Some(Instant::now()); self.log.line(format!("게임 장면 멈춤 · {}", self.hwnd.map(game::state).unwrap_or_default())); }
            if silent < 0.3 { if let Some(t) = self.stalled.take() { self.log.line(format!("게임 장면 다시 옴({:.1}초 만)", t.elapsed().as_secs_f32() + 1.0)); } }
        }
        if silent > 0.3 { self.steady = Instant::now(); }
        // 창 크기·전체 화면 전환 뒤에는 받기가 멈출 수 있다: 게임이 장면을 계속 보내는데 화면만 안 오면 다시 연결한다.
        let drawing = if self.live.stats.frames > 0 { self.steady.elapsed() > Duration::from_millis(1500) } else { self.capture_seen.elapsed() > Duration::from_secs(6) };
        if self.capture.is_some() && drawing && self.capture_seen.elapsed() > Duration::from_millis(1500) && self.capture_retry.elapsed() > Duration::from_secs(4) {
            self.capture_retry = Instant::now();
            self.capture_seen = Instant::now();
            if let Some(old) = self.capture.take() { let old = Old(old); std::thread::spawn(move || { let old = old; drop(old.0); }); }
            self.log.line("게임 화면 다시 연결");
        }
        if self.hwnd.is_none() && !self.frozen { gpu.panel = None; }
        self.steps[1] = t0.elapsed().as_secs_f64() * 1000.0;
        let t0 = Instant::now();
        let world = self.world();
        let local = world.inverse();
        self.live.update(gpu, local.transform_point3(head), Quat::from_rotation_y(-self.table_yaw) * rotation, 0.45, 30f32.to_radians())?;
        self.steps[2] = t0.elapsed().as_secs_f64() * 1000.0;
        let t0 = Instant::now();
        let rect = if self.live.in_match() { self.game_rect() } else { None };
        if rect.is_some() { self.last_match = Instant::now(); if self.match_since.is_none() { self.match_since = Some(Instant::now()); } } else if self.last_match.elapsed() > Duration::from_millis(600) { self.match_since = None; }
        let next = match rect { Some(r) if self.match_since.is_some_and(|t| t.elapsed() > Duration::from_millis(300)) => Layout::Match(r), None if self.last_match.elapsed() < Duration::from_millis(600) => self.layout, _ if gpu.panel.is_some() => Layout::Screen, _ => Layout::Waiting };
        if next != self.layout { self.log.line(format!("화면 배치 {:?} → {next:?}", self.layout)); self.layout = next; }
        let show_table = self.in_match();
        gpu.mask(|n| show_table || !n.starts_with("live") || n.contains("room") || n.contains("key") || n.contains("laser") || n.contains("marker"));
        let c = self.crop;
        let place = Place { head: self.home.0, yaw: self.home.1, floor: self.floor, table: show_table.then_some(self.table_height), flat: self.flat };
        if gpu.panel.is_none() || self.layout == Layout::Waiting {
            let dots = ["", ".", "..", "..."][(self.started.elapsed().as_millis() / 400 % 4) as usize];
            let line = format!("{}{dots}", crate::lang::t(if self.launched && self.hwnd.is_none() { "게임이 켜지기를 기다리는 중" } else { self.status }));
            self.room.message(gpu, &mut self.live, &place, local, &[crate::lang::t("팀파이트 매니저 2"), &line, crate::lang::t("A / X 로 다시 가운데에 놓기")])?;
        } else if !self.live.alive() && self.live.stats.frames == 0 { self.room.screen(gpu, &place, local, c)?; } else if let Some(frame) = self.live.take() {
            let list = frame.list("UI");
            let stamp = frame.number * 2 + show_table as u64;
            if stamp != self.shelf_frame {
                self.shelf_frame = stamp;
                self.fresh = Instant::now();
                let hole = match self.layout { Layout::Match(r) => Some(r.map(|v| v as f32)), _ => None };
                let live = &self.live;
                let t = Instant::now();
                self.shelf = shelf::analyze(list, hole, &|c| live.sprite_size(c));
                self.shelf_ms = t.elapsed().as_secs_f64() * 1000.0;
            }
            let hole = match self.layout { Layout::Match(r) => Some(r.map(|v| v as f32)), _ => None };
            let t = Instant::now();
            // 게임이 불러오느라 새 장면을 못 보내는 동안 멈춘 것처럼 보이지 않게 알린다.
            let stall = self.fresh.elapsed().as_secs_f32();
            self.room.note = (stall > 1.5).then(|| crate::lang::t("불러오는 중").to_string());
            let result = self.room.update(gpu, &mut self.live, &frame, &self.shelf, &place, local, c, pointer.map(|(h, down)| (h.key, h.zone, down)), dt, hole);
            self.room_ms = t.elapsed().as_secs_f64() * 1000.0;
            self.live.put(frame);
            result?;
        }
        self.keyboard.draw(gpu, &mut self.live, local)?;
        self.dock.draw(gpu, &mut self.live, local, self.flat)?;
        let beams: Vec<Instance> = lasers.iter().map(|(o, d, hit)| { let length = hit.unwrap_or(0.25); let m = Mat4::from_rotation_translation(Quat::from_rotation_arc(Vec3::NEG_Z, *d), *o + *d * (length * 0.5)) * Mat4::from_scale(Vec3::new(0.0024, 0.0024, length)); geometry::solid_instance(local * m, geometry::color_hex(if hit.is_some() { 0x6fe3d0 } else { 0x54707a }, 1.0)) }).collect();
        gpu.update(self.laser_id, &beams)?;
        let mut marks: Vec<Instance> = lasers.iter().filter_map(|(o, d, hit)| hit.map(|t| geometry::solid_instance(local * Mat4::from_scale_rotation_translation(Vec3::splat(0.008), Quat::IDENTITY, *o + *d * t), geometry::color_hex(0xf6fffb, 1.0)))).collect();
        marks.extend(points.iter().map(|(p, grab)| geometry::solid_instance(local * Mat4::from_scale_rotation_translation(Vec3::splat(0.014), Quat::IDENTITY, *p), geometry::color_hex(if *grab { 0xffc857 } else { 0x9bd3ef }, 1.0))));
        gpu.update(self.marker_id, &marks)?;
        self.steps[3] = t0.elapsed().as_secs_f64() * 1000.0;
        Ok(())
    }
}
