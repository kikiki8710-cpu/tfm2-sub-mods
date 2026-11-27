//! 헤드셋 흐름: --play. 게임이 없으면 스팀으로 켜고, 게임이 꺼지면 같이 끝난다.
use super::{game, Play};
use crate::lang::t;
use crate::{gpu::{Gpu, Target}, live::feed::Feed, log::Log, manipulation::{Grab, Table}, scene::TABLE, window::Window, xrflow::projection};
use anyhow::{bail, Context, Result};
use glam::{Quat, Vec3};
use openxr as xr;
use std::{path::{Path, PathBuf}, sync::{atomic::{AtomicBool, Ordering}, Arc}, thread, time::{Duration, Instant}};
use windows::{core::Interface, Win32::{Foundation::LUID, Graphics::{Direct3D11::ID3D11Texture2D, Dxgi::Common::*}}};

const VIEW: xr::ViewConfigurationType = xr::ViewConfigurationType::PRIMARY_STEREO;
fn loader() -> Result<PathBuf> {
    let mut list = Vec::new();
    // 창작마당 패키지는 게임 모드 폴더 안에 있어 .dll 이면 게임 로더가 모드로 읽다 실패해 모드가 꺼진다 → .dat 로 둔다(전체 경로 로드라 확장자 무관).
    if let Ok(exe) = std::env::current_exe() { if let Some(d) = exe.parent() { list.push(d.join("openxr_loader.dll")); list.push(d.join("openxr_loader.dat")); } }
    list.push("C:/Program Files (x86)/Steam/steamapps/common/SteamVR/bin/win64/openxr_loader.dll".into());
    list.into_iter().find(|p| p.exists()).context(crate::lang::t("openxr_loader.dll 이 없습니다(프로그램 폴더 또는 SteamVR)"))
}
struct Hands { set: xr::ActionSet, trigger: xr::Action<f32>, squeeze: xr::Action<f32>, recenter: xr::Action<bool>, typing: xr::Action<bool>, stick: xr::Action<xr::Vector2f>, grip: xr::Action<xr::Posef>, aim: xr::Action<xr::Posef>, haptic: xr::Action<xr::Haptic>, spaces: Option<([xr::Space; 2], [xr::Space; 2])>, paths: [xr::Path; 2], grab: Grab, last: Instant, recentered: [bool; 2], typed: [bool; 2], clicking: Option<usize>, wheel: f32 }
fn axis(v: f32) -> f32 { if v.abs() < 0.18 { 0.0 } else { v.signum() * (v.abs() - 0.18) / 0.82 } }
fn pose(l: &xr::SpaceLocation) -> Option<(Vec3, Quat)> {
    if !l.location_flags.contains(xr::SpaceLocationFlags::POSITION_VALID | xr::SpaceLocationFlags::ORIENTATION_VALID) { return None; }
    let (p, q) = (l.pose.position, l.pose.orientation);
    Some((Vec3::new(p.x, p.y, p.z), Quat::from_xyzw(q.x, q.y, q.z, q.w)))
}
#[derive(Default)]
struct HandFrame { lasers: Vec<(Vec3, Vec3, Option<f32>)>, points: Vec<(Vec3, bool)>, pointer: Option<(super::Hit, bool)> }
impl Hands {
    fn new(instance: &xr::Instance, log: &Log) -> Result<Self> {
        let path = |s: &str| instance.string_to_path(s);
        let paths = [path("/user/hand/left")?, path("/user/hand/right")?];
        let set = instance.create_action_set("play", t("VR 조작"), 0)?;
        let this = Self {
            trigger: set.create_action("press", t("누르기·잡기"), &paths)?, squeeze: set.create_action("hold", t("테이블 잡기"), &paths)?, recenter: set.create_action("recenter", t("처음 배치"), &paths)?, typing: set.create_action("typing", t("자판"), &paths)?, stick: set.create_action("adjust", t("스크롤·테이블 조절"), &paths)?,
            grip: set.create_action("grip_pose", t("손 위치"), &paths)?, aim: set.create_action("aim_pose", t("가리키기"), &paths)?, haptic: set.create_action("feedback", t("진동"), &paths)?,
            spaces: None, paths, grab: Grab::default(), last: Instant::now(), recentered: [false; 2], typed: [false; 2], clicking: None, wheel: 0.0, set,
        };
        for profile in ["/interaction_profiles/oculus/touch_controller", "/interaction_profiles/valve/index_controller", "/interaction_profiles/microsoft/motion_controller", "/interaction_profiles/khr/simple_controller"] {
            let mut b = Vec::new();
            for hand in ["left", "right"] {
                let p = |s: &str| path(&format!("/user/hand/{hand}/{s}"));
                b.push(xr::Binding::new(&this.grip, p("input/grip/pose")?));
                b.push(xr::Binding::new(&this.aim, p("input/aim/pose")?));
                b.push(xr::Binding::new(&this.haptic, p("output/haptic")?));
                if profile.ends_with("simple_controller") { b.push(xr::Binding::new(&this.recenter, p("input/menu/click")?)); b.push(xr::Binding::new(&this.trigger, p("input/select/click")?)); continue; }
                b.push(xr::Binding::new(&this.trigger, p("input/trigger/value")?));
                b.push(xr::Binding::new(&this.squeeze, p(if profile.contains("microsoft") { "input/squeeze/click" } else { "input/squeeze/value" })?));
                b.push(xr::Binding::new(&this.stick, p("input/thumbstick")?));
                b.push(xr::Binding::new(&this.typing, p(if profile.contains("oculus") { if hand == "left" { "input/y/click" } else { "input/b/click" } } else if profile.contains("valve") { "input/b/click" } else { "input/thumbstick/click" })?));
                b.push(xr::Binding::new(&this.recenter, p(if profile.contains("oculus") { if hand == "left" { "input/x/click" } else { "input/a/click" } } else if profile.contains("valve") { "input/a/click" } else { "input/menu/click" })?));
            }
            if let Err(e) = instance.suggest_interaction_profile_bindings(path(profile)?, &b) { log.line(format!("컨트롤러 프로필 미지원 {profile} {e:?}")); } else { log.line(format!("컨트롤러 연결 {}", profile.rsplit('/').next().unwrap_or(""))); }
        }
        Ok(this)
    }
    fn attach(&mut self, session: &xr::Session<xr::D3D11>) -> Result<()> {
        session.attach_action_sets(&[&self.set])?;
        let space = |a: &xr::Action<xr::Posef>, i: usize| a.create_space(session, self.paths[i], xr::Posef::IDENTITY);
        self.spaces = Some(([space(&self.grip, 0)?, space(&self.grip, 1)?], [space(&self.aim, 0)?, space(&self.aim, 1)?]));
        Ok(())
    }
    fn release(&mut self, play: &mut Play) { self.grab.cancel(); self.clicking = None; if let Some(m) = play.mouse() { m.button(false); } }
    #[allow(clippy::too_many_arguments)]
    fn update(&mut self, session: &xr::Session<xr::D3D11>, space: &xr::Space, time: xr::Time, play: &mut Play, head: Vec3, q: Quat, dt: f32, floor: f32) -> Result<HandFrame> {
        session.sync_actions(&[xr::ActiveActionSet::new(&self.set)])?;
        let mut out = HandFrame::default();
        let (mut down, mut hold, mut grips, mut aims, mut sticks, mut recenter) = ([false; 2], [false; 2], [None; 2], [None; 2], [(0.0f32, 0.0f32); 2], false);
        {
            let (gs, is) = self.spaces.as_ref().context("손 공간")?;
            for i in 0..2 {
                let p = self.paths[i];
                let t = self.trigger.state(session, p)?;
                down[i] = t.is_active && t.current_state > if self.clicking == Some(i) { 0.35 } else { 0.55 };
                let s = self.squeeze.state(session, p)?;
                hold[i] = s.is_active && s.current_state > 0.5;
                let v = self.stick.state(session, p)?;
                if v.is_active { sticks[i] = (axis(v.current_state.x), axis(v.current_state.y)); }
                let r = self.recenter.state(session, p)?;
                let pressed = r.is_active && r.current_state;
                recenter |= pressed && !self.recentered[i];
                self.recentered[i] = pressed;
                let y = self.typing.state(session, p)?;
                let y_down = y.is_active && y.current_state;
                if y_down && !self.typed[i] { play.keyboard.toggle(head, q); }
                self.typed[i] = y_down;
                if self.grip.is_active(session, p)? { grips[i] = pose(&gs[i].locate(space, time)?); }
                if self.aim.is_active(session, p)? { aims[i] = pose(&is[i].locate(space, time)?); }
            }
        }
        if recenter { self.release(play); play.table_height = floor + TABLE; play.table_scale = 1.0; play.recenter(head, q); return Ok(out); }
        // 가리키기: 화면을 가리키는 손은 마우스, 아니면 테이블 손.
        let mut pointing = [None; 2];
        let mut hits: [Option<super::Hit>; 2] = [None, None];
        let (mut keys, mut on_keys, mut docked) = (None::<bool>, [false; 2], None::<bool>);
        for i in 0..2 {
            let Some((o, r)) = aims[i] else { continue };
            let d = (r * Vec3::NEG_Z).normalize_or_zero();
            let hit = play.hit(o, d);
            // 자판이 더 가까우면 자판을 누른다.
            if let Some(t) = play.keyboard.hit(o, d).filter(|t| hit.as_ref().is_none_or(|h| *t < h.distance)) {
                keys = Some(keys.unwrap_or(false) | down[i]);
                out.lasers.push((o, d, Some(t)));
                on_keys[i] = true;
                continue;
            }
            if let Some(t) = play.dock.hit(o, d).filter(|t| hit.as_ref().is_none_or(|h| *t < h.distance)) {
                docked = Some(docked.unwrap_or(false) | down[i]);
                out.lasers.push((o, d, Some(t)));
                on_keys[i] = true;
                continue;
            }
            if !hold[i] { if let Some(h) = &hit { pointing[i] = Some(Play::uv(h)); } }
            out.lasers.push((o, d, hit.as_ref().filter(|_| !hold[i]).map(|h| h.distance)));
            if !hold[i] { hits[i] = hit; }
        }
        match play.dock.step(docked.is_some(), docked.unwrap_or(false), dt) {
            Some(super::dock::Action::Flat) => { let f = !play.flat; play.set_flat(f); }
            Some(super::dock::Action::Center) => { self.release(play); play.table_height = floor + TABLE; play.table_scale = 1.0; play.recenter(head, q); return Ok(out); }
            Some(super::dock::Action::Keyboard) => play.keyboard.toggle(head, q),
            None => {}
        }
        if let Some(vk) = play.keyboard.step(keys.is_some(), keys.unwrap_or(false), dt) { if vk != 0 { if let Some(m) = play.mouse() { m.focus(); m.key(vk); } } }
        let active = self.clicking.filter(|&i| aims[i].is_some()).or_else(|| if pointing[1].is_some() { Some(1) } else if pointing[0].is_some() { Some(0) } else { None });
        if let Some(m) = play.mouse() {
            match active {
                Some(i) => {
                    m.focus();
                    if let Some(uv) = pointing[i] { m.move_to(uv.x, uv.y); }
                    if down[i] && pointing[i].is_some() && self.clicking.is_none() { self.clicking = Some(i); m.button(true); let v = xr::HapticVibration::new().amplitude(0.3).frequency(xr::FREQUENCY_UNSPECIFIED).duration(xr::Duration::from_nanos(18_000_000)); let _ = self.haptic.apply_feedback(session, self.paths[i], &v); }
                    if !down[i] && self.clicking == Some(i) { self.clicking = None; m.button(false); }
                    self.wheel += sticks[i].1 * dt * 1400.0;
                    let notch = (self.wheel / 120.0).trunc();
                    if notch != 0.0 { self.wheel -= notch * 120.0; m.wheel((notch * 120.0) as i32); }
                }
                None => { if self.clicking.take().is_some() { m.button(false); } }
            }
        }
        if let Some(i) = active { out.pointer = hits[i].take().map(|h| (h, self.clicking == Some(i))); }
        if !play.in_match() { self.grab.cancel(); return Ok(out); }
        let held: [bool; 2] = std::array::from_fn(|i| hold[i] || (down[i] && !on_keys[i] && pointing[i].is_none() && self.clicking != Some(i)));
        let positions: [Option<Vec3>; 2] = std::array::from_fn(|i| grips[i].map(|g: (Vec3, Quat)| g.0));
        let mut table = Table { center: Vec3::new(play.table_origin.x, play.table_height, play.table_origin.z), scale: play.table_scale, yaw: play.table_yaw };
        let input_dt = self.last.elapsed().as_secs_f32();
        self.last = Instant::now();
        let r = self.grab.step(held, positions, input_dt, &mut table, head, floor);
        for i in 0..2 {
            if let Some(p) = r.points[i] { out.points.push((p, r.grabbing[i])); }
            if r.haptic[i] { let v = xr::HapticVibration::new().amplitude(0.25).frequency(xr::FREQUENCY_UNSPECIFIED).duration(xr::Duration::from_nanos(30_000_000)); let _ = self.haptic.apply_feedback(session, self.paths[i], &v); }
        }
        if !self.grab.busy() {
            let free: [(f32, f32); 2] = std::array::from_fn(|i| if pointing[i].is_some() { (0.0, 0.0) } else { sticks[i] });
            let mut forward = table.center - head;
            forward.y = 0.0;
            let forward = if forward.length_squared() < 0.0001 { q * Vec3::NEG_Z } else { forward }.normalize_or_zero();
            table.center += forward * free[0].0 * dt * 0.65;
            table.center.y += free[0].1 * dt * 0.45;
            table.yaw += free[1].0 * dt * std::f32::consts::FRAC_PI_2;
            table.scale *= (free[1].1 * dt * 0.6).exp();
            if free[0].0.abs() + free[0].1.abs() + free[1].0.abs() + free[1].1.abs() > 0.001 { table.constrain(head, floor); }
        }
        (play.table_origin.x, play.table_origin.z, play.table_height, play.table_scale, play.table_yaw) = (table.center.x, table.center.z, table.center.y, table.scale, table.yaw);
        Ok(out)
    }
}
struct Eye { targets: Vec<Target>, swapchain: xr::Swapchain<xr::D3D11>, width: u32, height: u32 }
enum Outcome { Quit, Restart }

fn cycle(log: &Log, window: &mut Window, stop: &Arc<AtomicBool>, launched: &mut bool, args: &[String]) -> Result<Outcome> {
    let entry = unsafe { xr::Entry::load_from(Path::new(&loader()?), &()) }?;
    if !entry.enumerate_extensions()?.khr_d3d11_enable { bail!(t("지금 켜진 VR 런타임에 D3D11 지원이 없습니다")); }
    let mut extensions = xr::ExtensionSet::default();
    extensions.khr_d3d11_enable = true;
    let instance = entry.create_instance(&xr::ApplicationInfo { application_name: "tfm2_vr", application_version: 1, engine_name: "TFM2 VR", engine_version: 1, api_version: xr::Version::new(1, 0, 0) }, &extensions, &[], &())?;
    let props = instance.properties()?;
    log.line(format!("VR 런타임 {} {}", props.runtime_name, props.runtime_version));
    let mut hands = Hands::new(&instance, log)?;
    let system = loop {
        if !window.poll() || stop.load(Ordering::SeqCst) { return Ok(Outcome::Quit); }
        match instance.system(xr::FormFactor::HEAD_MOUNTED_DISPLAY) { Ok(s) => break s, Err(xr::sys::Result::ERROR_FORM_FACTOR_UNAVAILABLE) => thread::sleep(Duration::from_millis(200)), Err(e) => return Err(e.into()) }
    };
    let need = instance.graphics_requirements::<xr::D3D11>(system)?;
    let mut gpu = Gpu::new(log, Some(LUID { LowPart: need.adapter_luid.LowPart, HighPart: need.adapter_luid.HighPart }), need.min_feature_level)?;
    let (session, mut waiter, mut stream) = unsafe { instance.create_session::<xr::D3D11>(system, &xr::d3d::SessionCreateInfoD3D11 { device: gpu.device.as_raw().cast() }) }?;
    hands.attach(&session)?;
    let stage = session.enumerate_reference_spaces()?.contains(&xr::ReferenceSpaceType::STAGE);
    let space = session.create_reference_space(if stage { xr::ReferenceSpaceType::STAGE } else { xr::ReferenceSpaceType::LOCAL }, xr::Posef::IDENTITY)?;
    let configs = instance.enumerate_view_configuration_views(system, VIEW)?;
    let formats = session.enumerate_swapchain_formats()?;
    let format = [DXGI_FORMAT_R8G8B8A8_UNORM_SRGB, DXGI_FORMAT_B8G8R8A8_UNORM_SRGB].into_iter().find(|f| formats.contains(&(f.0 as u32))).context("지원 색상 형식 없음")?;
    let limits = instance.system_properties(system)?.graphics_properties;
    let mut eyes = Vec::new();
    for c in configs {
        let [w, h] = gpu.settings.eye_size([c.recommended_image_rect_width, c.recommended_image_rect_height], [c.max_image_rect_width.min(limits.max_swapchain_image_width), c.max_image_rect_height.min(limits.max_swapchain_image_height)]);
        let swapchain = session.create_swapchain(&xr::SwapchainCreateInfo { create_flags: xr::SwapchainCreateFlags::EMPTY, usage_flags: xr::SwapchainUsageFlags::COLOR_ATTACHMENT, format: format.0 as u32, sample_count: 1, width: w, height: h, face_count: 1, array_size: 1, mip_count: 1 })?;
        let mut targets = Vec::new();
        for raw in swapchain.enumerate_images()? { let p = raw.cast(); let t = unsafe { ID3D11Texture2D::from_raw_borrowed(&p) }.context("색상 텍스처")?; targets.push(gpu.target(t, w, h, format)?); }
        log.line(format!("눈 렌더 크기 {w}x{h}"));
        eyes.push(Eye { targets, swapchain, width: w, height: h });
    }
    if eyes.len() != 2 { bail!("양안 구성이 아닙니다"); }
    let mut play = Play::new(&mut gpu, Feed::game()?, log)?;
    if !*launched && game::find().is_none() && !args.iter().any(|a| a == "--no-launch") {
        let direct = args.iter().position(|a| a == "--game-exe").and_then(|i| args.get(i + 1)).map(String::as_str);
        log.line(t("게임을 켭니다"));
        game::launch(direct)?;
    }
    *launched = true;
    game::priority();
    let (mut running, mut focused, mut recenter, mut exiting, mut floor) = (false, false, true, None::<Instant>, 0.0f32);
    let (mut previous, mut events) = (Instant::now(), xr::EventDataBuffer::new());
    let (mut frames, mut report, mut cpu_worst, mut cpu_sum) = (0u64, Instant::now(), 0.0f64, 0.0f64);
    loop {
        if !window.poll() || play.finished() { if play.finished() && !stop.load(Ordering::SeqCst) { log.line(t("게임이 꺼져 VR 도 끝냅니다")); } stop.store(true, Ordering::SeqCst); }
        while let Some(event) = instance.poll_event(&mut events)? {
            match event {
                xr::Event::SessionStateChanged(e) => {
                    log.line(format!("세션 상태 {:?}", e.state()));
                    focused = e.state() == xr::SessionState::FOCUSED;
                    if !focused { hands.release(&mut play); }
                    match e.state() {
                        xr::SessionState::READY if !running => { session.begin(VIEW)?; running = true; recenter = true; previous = Instant::now(); }
                        xr::SessionState::STOPPING => { if running { session.end()?; running = false; } if stop.load(Ordering::SeqCst) { return Ok(Outcome::Quit); } }
                        xr::SessionState::EXITING | xr::SessionState::LOSS_PENDING => return Ok(if stop.load(Ordering::SeqCst) { Outcome::Quit } else { Outcome::Restart }),
                        _ => {}
                    }
                }
                xr::Event::InstanceLossPending(_) => return Ok(Outcome::Restart),
                _ => {}
            }
        }
        if stop.load(Ordering::SeqCst) {
            if !running { return Ok(Outcome::Quit); }
            match exiting { None => { let _ = session.request_exit(); exiting = Some(Instant::now()); } Some(t) if t.elapsed() > Duration::from_secs(8) => return Ok(Outcome::Quit), _ => {} }
        }
        if !running { thread::sleep(Duration::from_millis(10)); continue; }
        let frame = waiter.wait()?;
        let cpu = Instant::now();
        let dt = previous.elapsed().as_secs_f32().min(0.1);
        previous = Instant::now();
        stream.begin()?;
        let mut views = Vec::new();
        if frame.should_render {
            let (flags, located) = session.locate_views(VIEW, frame.predicted_display_time, &space)?;
            if flags.contains(xr::ViewStateFlags::POSITION_VALID | xr::ViewStateFlags::ORIENTATION_VALID) && located.len() == 2 { views = located; }
        }
        if !views.is_empty() {
            let (a, b) = (views[0].pose, views[1].pose);
            let head = Vec3::new(a.position.x + b.position.x, a.position.y + b.position.y, a.position.z + b.position.z) * 0.5;
            let q = Quat::from_xyzw(a.orientation.x, a.orientation.y, a.orientation.z, a.orientation.w).slerp(Quat::from_xyzw(b.orientation.x, b.orientation.y, b.orientation.z, b.orientation.w), 0.5);
            if recenter { floor = if stage { 0.0 } else { head.y - 1.6 }; play.table_height = floor + TABLE; play.recenter(head, q); recenter = false; log.line(format!("처음 배치 머리 {:?}", head.to_array())); }
            let h = if focused { hands.update(&session, &space, frame.predicted_display_time, &mut play, head, q, dt, floor)? } else { hands.release(&mut play); HandFrame::default() };
            play.floor = floor;
            play.update(&mut gpu, head, q, &h.lasers, &h.points, h.pointer.as_ref().map(|p| (&p.0, p.1)), dt)?;
            for (eye, view) in eyes.iter_mut().zip(&views) {
                let i = eye.swapchain.acquire_image()?;
                eye.swapchain.wait_image(xr::Duration::INFINITE)?;
                gpu.render(&eye.targets[i as usize], projection(view), play.world(), true);
                unsafe { gpu.context.Flush(); }
                eye.swapchain.release_image()?;
            }
            let layers: Vec<_> = eyes.iter().zip(&views).map(|(e, v)| xr::CompositionLayerProjectionView::new().pose(v.pose).fov(v.fov).sub_image(xr::SwapchainSubImage::new().swapchain(&e.swapchain).image_array_index(0).image_rect(xr::Rect2Di { offset: xr::Offset2Di { x: 0, y: 0 }, extent: xr::Extent2Di { width: e.width as i32, height: e.height as i32 } }))).collect();
            let layer = xr::CompositionLayerProjection::new().space(&space).views(&layers);
            stream.end(frame.predicted_display_time, xr::EnvironmentBlendMode::OPAQUE, &[&layer])?;
        } else { stream.end(frame.predicted_display_time, xr::EnvironmentBlendMode::OPAQUE, &[])?; }
        let ms = cpu.elapsed().as_secs_f64() * 1000.0;
        frames += 1;
        cpu_worst = cpu_worst.max(ms);
        cpu_sum += ms;
        if report.elapsed() > Duration::from_secs(5) { let s = report.elapsed().as_secs_f64(); log.line(format!("{:.1}fps CPU 평균 {:.2}ms 최악 {cpu_worst:.2}ms 배치 {:?} 화면 {} {} 장면 {:?}", frames as f64 / s, cpu_sum / frames.max(1) as f64, play.layout, play.capture_frames(), play.summary(), play.live.stats)); gpu.check()?; (frames, cpu_worst, cpu_sum, report) = (0, 0.0, 0.0, Instant::now()); }
    }
}
pub fn run(log: &Log, args: &[String]) -> Result<()> {
    let stop = Arc::new(AtomicBool::new(false));
    let s = stop.clone();
    ctrlc::set_handler(move || s.store(true, Ordering::SeqCst))?;
    let mut window = Window::new(false)?;
    let mut launched = false;
    while !stop.load(Ordering::SeqCst) && window.poll() {
        match cycle(log, &mut window, &stop, &mut launched, args)? { Outcome::Quit => break, Outcome::Restart => { log.line(t("VR 연결이 끊겨 다시 준비합니다")); thread::sleep(Duration::from_millis(500)); } }
    }
    log.line(t("VR 정상 종료"));
    Ok(())
}
