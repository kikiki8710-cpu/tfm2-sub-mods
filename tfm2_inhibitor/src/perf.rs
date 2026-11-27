//! 처리 시간 계측(cfg perf_log=1) — rdtsc 누적(호출당 ~10ns). TSC→시간 환산은 QPC 로 시작~현재 비율을 잡는다.
//! 보고: MOBATICK 원본 1회 평균 / 우리 pre·post / 타워 update 판별 / AI 빌더 보정 — 틱당 µs 와 원본 대비 %.
#![allow(dead_code)]
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

extern "system" {
    fn QueryPerformanceCounter(c: *mut i64) -> i32;
    fn QueryPerformanceFrequency(f: *mut i64) -> i32;
}
pub static ON: AtomicBool = AtomicBool::new(false);
pub static T_PRE: AtomicU64 = AtomicU64::new(0);
pub static T_ORIG: AtomicU64 = AtomicU64::new(0);
pub static T_POST: AtomicU64 = AtomicU64::new(0);
pub static T_TOWER: AtomicU64 = AtomicU64::new(0);
pub static N_TOWER: AtomicU64 = AtomicU64::new(0);
pub static T_AI: AtomicU64 = AtomicU64::new(0);
pub static N_AI: AtomicU64 = AtomicU64::new(0);
pub static N_MOBA: AtomicU64 = AtomicU64::new(0);
static TSC0: AtomicU64 = AtomicU64::new(0);
static QPC0: AtomicU64 = AtomicU64::new(0);

#[inline(always)]
pub fn now() -> u64 { unsafe { core::arch::x86_64::_rdtsc() } }
#[inline(always)]
pub fn on() -> bool { ON.load(Ordering::Relaxed) }

pub fn start() {
    let mut q = 0i64;
    unsafe { QueryPerformanceCounter(&mut q); }
    QPC0.store(q as u64, Ordering::Relaxed);
    TSC0.store(now(), Ordering::Relaxed);
    ON.store(true, Ordering::Relaxed);
}

/// TSC 1µs 당 틱 수(시작 이후 평균)
fn tsc_per_us() -> f64 {
    let (mut q, mut f) = (0i64, 0i64);
    unsafe { QueryPerformanceCounter(&mut q); QueryPerformanceFrequency(&mut f); }
    let dq = (q as u64).saturating_sub(QPC0.load(Ordering::Relaxed)) as f64;
    let dt = now().saturating_sub(TSC0.load(Ordering::Relaxed)) as f64;
    if dq <= 0.0 || f <= 0 { return 0.0; }
    let us = dq * 1e6 / f as f64;
    if us <= 0.0 { 0.0 } else { dt / us }
}

pub fn report() -> String {
    if !on() { return String::from("perf=off"); }
    let k = tsc_per_us();
    let r = Ordering::Relaxed;
    let n = N_MOBA.load(r).max(1) as f64;
    if k <= 0.0 { return String::from("perf=calibrating"); }
    let us = |t: &AtomicU64| t.load(r) as f64 / k / n;
    let (pre, orig, post, tower, ai) = (us(&T_PRE), us(&T_ORIG), us(&T_POST), us(&T_TOWER), us(&T_AI));
    // 타워 판별은 원본 안에서 돈다 → 순수 원본 = orig − tower
    let base = (orig - tower).max(0.001);
    let ours = pre + post + tower + ai;
    format!("perf ticks={} per-tick us: orig={:.1} (of which tower_check={:.2}) pre={:.2} post={:.2} ai_fix={:.2} | mod overhead={:.2}us = {:.1}% of vanilla tick | tower calls={} ai calls={}",
        N_MOBA.load(r), orig, tower, pre, post, ai, ours, ours / base * 100.0, N_TOWER.load(r), N_AI.load(r))
}
