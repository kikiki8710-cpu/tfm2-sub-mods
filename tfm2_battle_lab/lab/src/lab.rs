//! lab — 임의 경기 실험실(시제품, 0.7.4).
//! 시나리오 파일(줄 단위)을 읽어 세이브의 리플레이 하나를 틀로 삼고, 선수 챔피언·스탯·아이템·시드·새 챔피언 파일·틱별 개입을 얹어 게임 코드로 돌린다.
//! 출력 = NDJSON (틱 표본 · 행동 시작 · 이펙트 · 마지막 통계).
//!
//! 시나리오 문법(# 주석 · 공백 구분)
//!   key <n>                      기준 리플레이 key
//!   seed <u64>                   시드 바꿈(0x 가능)
//!   file <경로> [fill|replace]   .data_champion 추가(replace = 세이브 항목을 이 파일로 교체 + 그 챔피언의 행동 패치 비움)
//!   swap <team> <pos> <champ>    선수 챔피언 교체(pos = 기록 포지션 0~4)
//!   stat <team> <pos> <필드|all> <값>   선수 능력치(AthleteStat 17 수치)
//!   items <team> <pos> a,b,c,d   목표 빌드(게임 id 목록 · -1 로 비움)
//!   only <team> <pos>            이 선수만 출전(여러 줄) · mode <n> 게임 모드(0 MOBA)
//!   until <tick>                 이 틱에서 멈춤(0 = 끝까지)
//!   sample <n>                   n 틱마다 표본(기본 60)
//!   at <tick> <명령...>          틱 시작 직후 개입
//!      pos <선수#> <x> <y> | hp <선수#> <v> | gold <선수#> <v> | addgold <선수#> <±v> | heal <선수#> <v>
//!      freeze <선수#> | free <선수#> | cast <선수#> <0~3> target <선수#>/pos x y/dir dx dy | move <선수#> x y | cd0 <선수#>
//!      shield <선수#> <양> <틱> | dmg <공격자#> <대상#> <ad> <ap> | clearcc <선수#> | end <0|1 이긴팀 블루> | strat <team> <경로> <json>
//!   frames <간격> <경로>           화면용 이진 프레임(BLF1)
//!   out <경로>                   출력 파일
use crate::{rd, rstr, sim};
use std::fmt::Write as _;

const CHAMP_LOOKUP: usize = 0x1dc1a30;
const PARSE_DC: usize = 0x3ce990;
const CONV_DC: usize = 0x1dd65d0;
const FILL_DC: usize = 0x1dd7720;
const GROW_DC: usize = 0x3fb4a20;
const PAST_DC: usize = 0x13b5280;
const PATCH_ONE: usize = 0x1dd57b0;
const PATCH_VER: usize = 0x13dbf30;
const VT_DC: usize = 0x42440d0;
const SIMVT: usize = 0x4385b90;
const DB_LIVE: usize = 0x9308;
const DB_PRE: usize = 0x17cb0;
const DB_PATCH: usize = 0x17c80;
const ENTRY: usize = 0xb38;

#[derive(Clone, Debug)]
pub struct Cmd { pub tick: usize, pub words: Vec<String> }
#[derive(Default)]
pub struct Scenario {
    pub key: Option<usize>, pub seed: Option<u64>, pub files: Vec<(String, bool)>,
    pub swaps: Vec<(u64, u32, String)>, pub stats: Vec<(u64, u32, String, f64)>, pub items: Vec<(u64, u32, Vec<i64>)>,
    pub keep: Vec<(u64, u32)>, pub mode: u8, pub until: usize, pub sample: usize, pub cmds: Vec<Cmd>, pub out: String, pub frames: Option<(usize, String)>, pub settings: Vec<(String, String, String)>, pub full: Vec<(String, String)>,
}

pub fn parse(text: &str) -> Result<Scenario, String> {
    let mut s = Scenario { sample: 60, out: "lab_out.ndjson".into(), ..Default::default() };
    for (ln, raw) in text.lines().enumerate() {
        let l = raw.trim();
        if l.is_empty() || l.starts_with('#') { continue; }
        let w: Vec<&str> = l.split_whitespace().collect();
        let e = |m: &str| format!("{}줄 {m}: {l}", ln + 1);
        let num = |i: usize| -> Result<usize, String> { w.get(i).and_then(|x| x.parse().ok()).ok_or_else(|| e("숫자 아님")) };
        match w[0] {
            "key" => s.key = Some(num(1)?),
            "seed" => { let v = w.get(1).ok_or_else(|| e("값 없음"))?; s.seed = Some(if let Some(h) = v.strip_prefix("0x") { u64::from_str_radix(h, 16) } else { v.parse() }.map_err(|_| e("시드"))?); }
            "file" => { // 경로에 공백 가능: 마지막 단어가 모드면 분리
                let rest = l[4..].trim();
                let (p, m) = match rest.rsplit_once(' ') { Some((p, m)) if m == "replace" || m == "fill" => (p.trim(), m), _ => (rest, "fill") };
                s.files.push((p.to_string(), m == "replace"));
            }
            "swap" => s.swaps.push((num(1)? as u64, num(2)? as u32, w.get(3).ok_or_else(|| e("챔피언"))?.to_string())),
            "stat" => s.stats.push((num(1)? as u64, num(2)? as u32, w.get(3).ok_or_else(|| e("필드"))?.to_string(), w.get(4).and_then(|x| x.parse().ok()).ok_or_else(|| e("값"))?)),
            "items" => s.items.push((num(1)? as u64, num(2)? as u32, w.get(3).ok_or_else(|| e("목록"))?.split(',').map(|x| x.parse::<i64>()).collect::<Result<_, _>>().map_err(|_| e("목록"))?)),
            "only" => s.keep.push((num(1)? as u64, num(2)? as u32)), // 이 선수만 출전(여러 줄)
            "mode" => s.mode = num(1)? as u8,
            "until" => s.until = num(1)?,
            "sample" => s.sample = num(1)?.max(1),
            // setting <gs|ci|is> <원본 json>|<고친 json> : 게임 파서로 둘 다 읽어 달라진 칸만 세이브 설정에 덮는다
            "setting" => { let rest = l[7..].trim(); let (k, p) = rest.split_once(' ').ok_or_else(|| e("setting"))?; let (p0, p1) = p.split_once('|').ok_or_else(|| e("setting 경로"))?; s.settings.push((k.to_string(), p0.trim().to_string(), p1.trim().to_string())); }
            // setting_full <gs|ci|is> <json> : 그 설정을 통째로 이 파일 내용으로(설정 교체 모드 · 세이브의 시즌 패치 값은 버림)
            "setting_full" => { let rest = l[12..].trim(); let (k, p) = rest.split_once(' ').ok_or_else(|| e("setting_full"))?; s.full.push((k.to_string(), p.trim().to_string())); }
            "frames" => { let e: usize = num(1)?; s.frames = Some((e, l.splitn(3, ' ').nth(2).unwrap_or("frames.bin").trim().to_string())); } // frames <간격> <경로>
            "out" => s.out = l[3..].trim().to_string(),
            "at" => s.cmds.push(Cmd { tick: num(1)?, words: w[2..].iter().map(|x| x.to_string()).collect() }),
            _ => return Err(e("모르는 명령")),
        }
    }
    s.cmds.sort_by_key(|c| c.tick);
    Ok(s)
}

const FIELDS: [&str; 17] = ["last_hit", "skill_avoid", "skill_hit", "positioning", "control_speed", "concentration", "mental", "judgement", "order", "roaming", "aggressive", "ego", "top", "jungle", "mid", "bottom", "support"];

unsafe fn za(n: usize) -> usize { std::alloc::alloc_zeroed(std::alloc::Layout::from_size_align(n, 16).unwrap()) as usize }
unsafe fn halloc(n: usize) -> usize {
    #[link(name = "kernel32")]
    extern "system" { fn GetProcessHeap() -> usize; fn HeapAlloc(h: usize, f: u32, n: usize) -> usize; }
    HeapAlloc(GetProcessHeap(), 8, n)
}
unsafe fn sheets(db: usize) -> Vec<(String, usize)> {
    let mut v = vec![("live".to_string(), db + DB_LIVE)];
    let (ctrl, mask): (usize, usize) = (rd(db + DB_PRE), rd(db + DB_PRE + 8));
    if ctrl != 0 { for i in 0..=mask { if rd::<u8>(ctrl + i) & 0x80 == 0 { let b = ctrl - (i + 1) * 0x9030; v.push((rstr(b), b + 0x18 + 0x1538)); } } }
    v
}
unsafe fn find(ci: usize, id: &str) -> usize {
    let (p, n): (usize, usize) = (rd(ci + 8), rd(ci + 16));
    (0..n).map(|i| p + i * ENTRY).find(|&e| rstr(e + 0x10) == id).unwrap_or(0)
}

/// 챔피언 파일 하나를 게임 파서로 읽어 시트에 넣는다.
pub unsafe fn add_file(base: usize, db: usize, path: &str, replace: bool) -> Result<String, String> {
    let mut raw = std::fs::read(path).map_err(|e| format!("{path}: {e}"))?;
    if raw.starts_with(&[0xef, 0xbb, 0xbf]) { raw.drain(..3); }
    let out = za(0x9f0);
    let sr = [raw.as_ptr() as usize, raw.len(), 0usize];
    let parse: unsafe extern "system" fn(usize, usize) -> usize = std::mem::transmute(base + PARSE_DC);
    parse(out, sr.as_ptr() as usize);
    if rd::<i64>(out) == -1 {
        let eb: usize = rd(out + 8);
        let msg = if rd::<usize>(eb) == 0 { let (mp, ml): (usize, usize) = (rd(eb + 8), rd(eb + 0x10)); String::from_utf8_lossy(std::slice::from_raw_parts(mp as *const u8, ml.min(400))).into_owned() } else { format!("코드 {}", rd::<usize>(eb)) };
        return Err(format!("파싱 실패 {path}: {msg} (line {} col {})", rd::<usize>(eb + 0x18), rd::<usize>(eb + 0x20)));
    }
    let id = rstr(out + 0x10);
    let arc = halloc(0xa00);
    *(arc as *mut usize) = 1 << 20;
    *((arc + 8) as *mut usize) = 1;
    std::ptr::copy_nonoverlapping(out as *const u8, (arc + 0x10) as *mut u8, 0x9f0);
    let (d, vt) = (arc + 0x10, base + VT_DC);
    let conv: unsafe extern "system" fn(usize, usize, usize, usize) -> usize = std::mem::transmute(base + CONV_DC);
    let fill: unsafe extern "system" fn(usize, usize) = std::mem::transmute(base + FILL_DC);
    let grow: unsafe extern "system" fn(usize) = std::mem::transmute(base + GROW_DC);
    let past: unsafe extern "system" fn(usize, usize) = std::mem::transmute(base + PAST_DC);
    for (tag, ci) in sheets(db) {
        if !replace && tag != "live" { continue; }
        let e = find(ci, &id);
        if e == 0 {
            let tmp = za(ENTRY);
            conv(tmp, d, arc, vt);
            let (cap, len): (usize, usize) = (rd(ci), rd(ci + 16));
            if cap == len { grow(ci); }
            std::ptr::copy_nonoverlapping(tmp as *const u8, (rd::<usize>(ci + 8) + len * ENTRY) as *mut u8, ENTRY);
            *((ci + 16) as *mut usize) = len + 1;
        } else if replace {
            let tmp = za(ENTRY);
            conv(tmp, d, arc, vt);
            std::ptr::copy_nonoverlapping(tmp as *const u8, e as *mut u8, ENTRY);
        } else {
            fill(e, d);
            if rd::<u8>(e + 0xb34) == 0 { *((e + 0x9a0) as *mut usize) = arc; *((e + 0x9a8) as *mut usize) = vt; }
        }
    }
    if !replace { past(db, d); }
    Ok(id)
}
/// 행동 패치 채움(게임이 재생 전에 하는 단계). skip = 파일로 교체한 챔피언은 패치 없이.
pub unsafe fn patch_actions(base: usize, db: usize, skip: &[String]) {
    let f1: unsafe extern "system" fn(usize, usize) = std::mem::transmute(base + PATCH_ONE);
    let f2: unsafe extern "system" fn(usize, usize) = std::mem::transmute(base + PATCH_VER);
    f1(db + DB_LIVE, db + DB_PATCH);
    f2(db + DB_PRE, db + DB_PATCH);
    for (_, ci) in sheets(db) { for id in skip { let e = find(ci, id); if e != 0 { *((e + 0x9b0) as *mut usize) = 0; } } }
}

/// 틀 리플레이의 선수(팀, 포지션) → MatchReplayAthlete 주소
pub unsafe fn athlete(mrd: usize, team: u64, pos: u32) -> Option<usize> {
    let o = if team == 0 { 0x30 } else { 0x48 };
    let (p, n): (usize, usize) = (rd(mrd + o + 8), rd(mrd + o + 16));
    (0..n).map(|i| p + i * 0x270).find(|&a| rd::<u32>(a + 0x268) == pos)
}
pub unsafe fn apply_athletes(mrd: usize, s: &Scenario) -> Result<(), String> {
    for (t, p, c) in &s.swaps {
        let a = athlete(mrd, *t, *p).ok_or("swap: 선수 없음")?;
        let b: &'static str = Box::leak(c.clone().into_boxed_str());
        *(a as *mut usize) = 0; *((a + 8) as *mut usize) = b.as_ptr() as usize; *((a + 16) as *mut usize) = b.len();
    }
    for (t, p, f, v) in &s.stats {
        let a = athlete(mrd, *t, *p).ok_or("stat: 선수 없음")?;
        let idx: Vec<usize> = if f == "all" { (0..12).collect() } else { vec![FIELDS.iter().position(|x| x == f).ok_or_else(|| format!("stat: 필드 {f} 모름 ({})", FIELDS.join(" ")))?] };
        for i in idx { *((a + 0xc0 + i * 8) as *mut u64) = *v as u64; } // 정수 스탯(자료형은 실측 확인 필요)
    }
    for (t, p, v) in &s.items {
        let a = athlete(mrd, *t, *p).ok_or("items: 선수 없음")?;
        let buf: Vec<u64> = v.iter().filter(|x| **x >= 0).map(|x| *x as u64).collect();
        let lk: &'static mut [u64] = Box::leak(buf.into_boxed_slice());
        *((a + 0x18) as *mut usize) = lk.len(); *((a + 0x20) as *mut usize) = lk.as_ptr() as usize; *((a + 0x28) as *mut usize) = lk.len();
    }
    Ok(())
}

/// state = {World, World vtable, 이벤트 수집 포인터(0=없음), 읽기 전용 u8} — +0x18 이 0 이 아니면 바꾸는 슬롯이 전부 거절(0x350f36c 실측).
pub struct Sim { base: usize, fat: [usize; 4], pub ctx: [usize; 8] }
macro_rules! slot { ($s:expr, $o:expr, $t:ty) => { std::mem::transmute::<usize, $t>(rd::<usize>($s.base + SIMVT + $o)) } }
impl Sim {
    pub unsafe fn new(base: usize, world: usize, wvt: usize) -> Self { Sim { base, fat: [world, wvt, 0, 0], ctx: [0; 8] } }
    fn st(&self) -> *const u8 { self.fat.as_ptr() as *const u8 }
    fn stm(&self) -> *mut u8 { self.fat.as_ptr() as *mut u8 }
    pub unsafe fn tick(&self) -> usize { slot!(self, 8, unsafe extern "C" fn(*const u8) -> usize)(self.st()) }
    pub unsafe fn players(&self) -> Vec<u64> { let n = slot!(self, 192, unsafe extern "C" fn(*const u8) -> usize)(self.st()); let f = slot!(self, 200, unsafe extern "C" fn(*const u8, usize) -> u64); (0..n).map(|i| f(self.st(), i)).collect() }
    pub unsafe fn pget(&self, o: usize, h: u64) -> u64 { slot!(self, o, unsafe extern "C" fn(*const u8, u64) -> u64)(self.st(), h) }
    pub unsafe fn ent(&self, h: u64) -> u64 { self.pget(216, h) }
    pub unsafe fn pos(&self, e: u64) -> Option<(u64, u64)> { let (mut x, mut y) = (0, 0); if slot!(self, 72, unsafe extern "C" fn(*const u8, u64, *mut u64, *mut u64) -> bool)(self.st(), e, &mut x, &mut y) { Some((x, y)) } else { None } }
    pub unsafe fn hp(&self, e: u64) -> Option<(usize, usize)> { let (mut a, mut b) = (0, 0); if slot!(self, 80, unsafe extern "C" fn(*const u8, u64, *mut usize, *mut usize) -> bool)(self.st(), e, &mut a, &mut b) { Some((a, b)) } else { None } }
    pub unsafe fn cds(&self, h: u64) -> Option<[usize; 4]> { let mut c = [0usize; 4]; if slot!(self, 480, unsafe extern "C" fn(*const u8, u64, *mut usize, *mut usize, *mut usize, *mut usize) -> bool)(self.st(), h, &mut c[0], &mut c[1], &mut c[2], &mut c[3]) { Some(c) } else { None } }
    pub unsafe fn ent_bool(&self, o: usize, e: u64) -> bool { slot!(self, o, unsafe extern "C" fn(*const u8, u64) -> bool)(self.st(), e) }
    unsafe fn two(&self, f: &dyn Fn(*mut u8, usize, *mut usize) -> bool) -> Option<String> {
        let mut buf = vec![0u8; 8192]; let mut n = 0usize;
        if !f(buf.as_mut_ptr(), buf.len(), &mut n) { return None; }
        if n > buf.len() { buf = vec![0; n + 16]; if !f(buf.as_mut_ptr(), buf.len(), &mut n) { return None; } }
        Some(String::from_utf8_lossy(&buf[..n.min(buf.len())]).into_owned())
    }
    pub unsafe fn name(&self, e: u64) -> String { self.two(&|b, c, n| slot!(self, 504, unsafe extern "C" fn(*const u8, u64, *mut u8, usize, *mut usize) -> bool)(self.st(), e, b, c, n)).unwrap_or_default() }
    pub unsafe fn stats_json(&self, h: u64, path: &str) -> String { self.two(&|b, c, n| slot!(self, 552, unsafe extern "C" fn(*const u8, u64, *const u8, usize, *mut u8, usize, *mut usize) -> bool)(self.st(), h, path.as_ptr(), path.len(), b, c, n)).unwrap_or_else(|| "null".into()) }
    /// &mut Player (World vt+0x180 · 0x350f38e)
    pub unsafe fn player_mut(&self, h: u64) -> usize { if h == 0 { return 0; } let f: unsafe extern "win64" fn(usize, usize) -> usize = std::mem::transmute(rd::<usize>(self.fat[1] + 0x180)); f(self.fat[0], h as usize - 1) }
    pub unsafe fn entities(&self) -> Vec<u64> { let n = slot!(self, 40, unsafe extern "C" fn(*const u8) -> usize)(self.st()); let f = slot!(self, 48, unsafe extern "C" fn(*const u8, usize) -> u64); (0..n).map(|i| f(self.st(), i)).filter(|h| *h != 0).collect() }
    pub unsafe fn projectiles(&self) -> Vec<u64> { let n = slot!(self, 400, unsafe extern "C" fn(*const u8) -> usize)(self.st()); let f = slot!(self, 408, unsafe extern "C" fn(*const u8, usize) -> u64); (0..n).map(|i| f(self.st(), i)).collect() }
    pub unsafe fn proj_info(&self, h: u64) -> Option<(u64, u64, usize, usize)> { let mut o = [0u64; 6]; if slot!(self, 424, unsafe extern "C" fn(*const u8, u64, *mut u64) -> bool)(self.st(), h, o.as_mut_ptr()) { Some((o[0], o[1], o[2] as usize, o[3] as usize)) } else { None } }
    pub unsafe fn strategy_json(&self, team: usize) -> String { self.two(&|b, c, n| slot!(self, 536, unsafe extern "C" fn(*const u8, usize, *const u8, usize, *mut u8, usize, *mut usize) -> bool)(self.st(), team, "".as_ptr(), 0, b, c, n)).unwrap_or_else(|| "null".into()) }
    pub unsafe fn item_keys(&self, h: u64) -> Vec<String> { let n = slot!(self, 488, unsafe extern "C" fn(*const u8, u64) -> usize)(self.st(), h); (0..n.min(16)).map(|k| self.two(&|b, c, m| slot!(self, 496, unsafe extern "C" fn(*const u8, u64, usize, *mut u8, usize, *mut usize) -> bool)(self.st(), h, k, b, c, m)).unwrap_or_default()).collect() }
    pub unsafe fn buffs(&self, e: u64) -> usize { slot!(self, 160, unsafe extern "C" fn(*const u8, u64) -> usize)(self.st(), e) }
    unsafe fn cmd(&self, w: &[String], ps: &[u64]) -> String {
        let pn = |i: usize| -> u64 { w.get(i).and_then(|x| x.parse::<usize>().ok()).and_then(|k| ps.get(k).copied()).unwrap_or(0) };
        let n = |i: usize| -> u64 { w.get(i).and_then(|x| x.parse::<u64>().ok()).unwrap_or(0) };
        let r = match w.first().map(|s| s.as_str()).unwrap_or("") {
            "pos" => { ANCHOR[n(1) as usize & 15][0].store(n(2), std::sync::atomic::Ordering::Relaxed); ANCHOR[n(1) as usize & 15][1].store(n(3), std::sync::atomic::Ordering::Relaxed); slot!(self, 440, unsafe extern "C" fn(*mut u8, u64, u64, u64) -> bool)(self.stm(), self.ent(pn(1)), n(2), n(3)) }
            "hp" => slot!(self, 432, unsafe extern "C" fn(*mut u8, u64, usize) -> bool)(self.stm(), self.ent(pn(1)), n(2) as usize),
            "gold" => slot!(self, 456, unsafe extern "C" fn(*mut u8, u64, usize) -> bool)(self.stm(), pn(1), n(2) as usize),
            "addgold" => slot!(self, 464, unsafe extern "C" fn(*mut u8, u64, i64) -> bool)(self.stm(), pn(1), w.get(2).and_then(|x| x.parse().ok()).unwrap_or(0)),
            "shield" => slot!(self, 576, unsafe extern "C" fn(*mut u8, u64, usize, usize) -> bool)(self.stm(), self.ent(pn(1)), n(2) as usize, n(3) as usize),
            "clearcc" => { slot!(self, 568, unsafe extern "C" fn(*mut u8, u64) -> usize)(self.stm(), self.ent(pn(1))); true }
            "heal" => { let e = self.ent(pn(1)); slot!(self, 312, unsafe extern "C" fn(*mut u8, usize, usize, usize))(self.stm(), e as usize - 1, e as usize - 1, n(2) as usize); true }
            "dmg" => { slot!(self, 304, unsafe extern "C" fn(*mut u8, usize, usize, usize, usize, u32))(self.stm(), self.ent(pn(1)) as usize - 1, self.ent(pn(2)) as usize - 1, n(3) as usize, n(4) as usize, 0); true }
            "end" => { slot!(self, 472, unsafe extern "C" fn(*mut u8, bool))(self.stm(), n(1) == 1); true }
            "strat" => { let (p, j) = (w.get(2).cloned().unwrap_or_default(), w[3.min(w.len())..].join(" ")); slot!(self, 544, unsafe extern "C" fn(*mut u8, usize, *const u8, usize, *const u8, usize) -> bool)(self.stm(), n(1) as usize, p.as_ptr(), p.len(), j.as_ptr(), j.len()) }
            // level <선수#> <레벨> : 게임의 경험치 함수(0x19e6980)로 올린다(스킬 해금·스탯·체력까지 게임이 처리)
            "level" => { let h = pn(1); if h == 0 { false } else {
                let give: unsafe extern "win64" fn(usize, usize, usize, usize, usize, u8, usize) = std::mem::transmute(self.base + 0x19e6980);
                let sink: usize = 0; // &Option<&mut Vec<이벤트>> = None
                let want = n(2);
                for k in 0..64 { if self.pget(224, h) >= want { break; } give(self.fat[0], self.fat[1], self.ctx.as_ptr() as usize, h as usize - 1, if k == 0 { 200_000 } else { 0 }, 4, &sink as *const usize as usize); }
                let p = self.player_mut(h); if p != 0 { *((p + 0x870) as *mut usize) = 0; }
                self.pget(224, h) == want } }
            // useult <시전자#> <주인#> [target#] : 「주인」 엔티티 궁 슬롯의 효과를 시전자 이름으로 대기열(World vt+0x260)에 넣는다(0.7.4 · 강탈 시험)
            //   InputTarget 은 주인 궁의 시전 방식(ent+0x560)에 맞춘다: 0 대상 · 1 좌표 · 2 방향 · 3 없음. casting_target 4(자기)면 시전자 자신.
            "useult" => {
                let (cs, owner) = (self.ent(pn(1)), self.ent(pn(2)));
                let tg = if w.len() > 3 { self.ent(pn(3)) } else { owner };
                if cs == 0 || owner == 0 || tg == 0 { return "선수/개체 없음".into(); }
                let getf: unsafe extern "win64" fn(usize, usize) -> usize = std::mem::transmute(rd::<usize>(self.fat[1] + 0x208));
                let e = getf(self.fat[0], owner as usize - 1);
                if e == 0 { return "엔티티 없음".into(); }
                let casting: u32 = rd(e + 0x560);
                if casting == u32::MAX { return "궁 없음".into(); }
                let (arc, evt, atk, ctgt): (usize, usize, u32, u32) = (rd(e + 0x530), rd(e + 0x538), rd(e + 0x55c), rd(e + 0x558));
                std::sync::atomic::fence(std::sync::atomic::Ordering::SeqCst);
                (*(arc as *const std::sync::atomic::AtomicUsize)).fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                let (tx, ty) = self.pos(tg).unwrap_or((0, 0));
                let (cx, cy) = self.pos(cs).unwrap_or((0, 0));
                let input: [u64; 3] = match casting {
                    0 => [0, if ctgt == 4 { cs - 1 } else { tg - 1 }, 0],
                    1 => [2, tx, ty],
                    2 => [1, (tx as i64 - cx as i64) as u64, (ty as i64 - cy as i64) as u64],
                    _ => [3, 0, 0],
                };
                let q: unsafe extern "win64" fn(usize, usize, usize, usize, u32, usize, *const u64) = std::mem::transmute(rd::<usize>(self.fat[1] + 0x260));
                q(self.fat[0], 0, arc, evt, atk, cs as usize - 1, input.as_ptr());
                return format!("ok casting={casting} ctgt={ctgt} atk={atk} evt=+0x{:x} w={:x?}", evt.wrapping_sub(self.base), (0..7).map(|k| rd::<u64>(e + 0x530 + k * 8)).collect::<Vec<_>>());
            }
            "freeze" => { set_frozen(n(1) as usize, true); true }
            "free" => { set_frozen(n(1) as usize, false); crate::ai::clear_override(n(1) as usize); true }
            // cast <선수#> <0평타|1스킬1|2스킬2|3궁> target <선수#> | pos <x> <y> | dir <dx> <dy>
            "cast" => {
                let tag = 2 + n(2) as usize;
                let i = |k: usize| -> i64 { w.get(k).and_then(|x| x.parse::<i64>().ok()).unwrap_or(0) };
                let v = match w.get(3).map(|s| s.as_str()) {
                    Some("target") => [tag, 0, self.ent(pn(4)) as usize - 1, 0],
                    Some("dir") => [tag, 1, i(4) as usize, i(5) as usize],
                    Some("pos") => [tag, 2, i(4) as usize, i(5) as usize],
                    // auto <선수#> : 대상 → 좌표 → 방향 → 없음 순으로 12틱마다 바꿔 가며 시도(60틱 뒤 포기)
                    Some("auto") => {
                        let tgt = self.ent(pn(4));
                        if tgt == 0 || self.ent(pn(1)) == 0 { return "대상/시전자 없음".into(); }
                        let (tx, ty) = self.pos(tgt).unwrap_or((0, 0));
                        let (cx, cy) = self.pos(self.ent(pn(1))).unwrap_or((0, 0));
                        let list = vec![[tag, 0, tgt as usize - 1, 0], [tag, 2, tx as usize, ty as usize], [tag, 1, (tx as i64 - cx as i64) as usize, (ty as i64 - cy as i64) as usize], [tag, 0, self.ent(pn(1)) as usize - 1, 0]];
                        set_frozen(n(1) as usize, true);
                        AUTO.lock().unwrap_or_else(|e| e.into_inner()).insert(n(1) as usize & 15, (list, 0, 0));
                        return "ok auto".into();
                    }
                    _ => return "대상 형식".into(),
                };
                set_frozen(n(1) as usize, true); crate::ai::set_override(n(1) as usize, v); true }
            "move" => { crate::ai::set_override(n(1) as usize, [0, n(2) as usize, n(3) as usize, 0]); true }
            "cd0" => { let p = self.player_mut(pn(1)); if p == 0 { false } else { for k in 0..4 { *((p + 0x890 + k * 8) as *mut usize) = 0; } true } }
            "cdset" => { let p = self.player_mut(pn(1)); if p == 0 { false } else { *((p + 0x890 + n(2) as usize * 8) as *mut usize) = n(3) as usize; true } }
            "poke" => { let p = self.player_mut(pn(1)); if p == 0 { false } else { *((p + usize::from_str_radix(w[2].trim_start_matches("0x"), 16).unwrap_or(0)) as *mut u64) = n(3); true } }
            x => return format!("모르는 명령 {x}"),
        };
        if r { "ok".into() } else { "거절(false)".into() }
    }
}
/// 허수아비 표시: 매 틱 「제자리 이동」 입력을 준다(입력 없음은 직전 이동 명령이 이어진다 — 2026-11-25 실측).
/// 자동 시전: 선수# → (시도 목록, 다음 번째, 다음 교체 틱(0=즉시))
static AUTO: std::sync::Mutex<std::collections::BTreeMap<usize, (Vec<[usize; 4]>, usize, usize)>> = std::sync::Mutex::new(std::collections::BTreeMap::new());
static HOLD: [std::sync::atomic::AtomicUsize; 16] = [const { std::sync::atomic::AtomicUsize::new(0) }; 16];
/// 모드 dll 이 붙인 아이템 key(번호 = 기본 30개 + 순서)
static MOD_ITEMS: std::sync::Mutex<Vec<String>> = std::sync::Mutex::new(Vec::new());
static ANCHOR: [[std::sync::atomic::AtomicU64; 2]; 16] = [const { [const { std::sync::atomic::AtomicU64::new(u64::MAX) }; 2] }; 16];
static FROZEN: [std::sync::atomic::AtomicBool; 16] = [const { std::sync::atomic::AtomicBool::new(false) }; 16];
fn set_frozen(i: usize, on: bool) {
    let i = i & 15;
    FROZEN[i].store(on, std::sync::atomic::Ordering::Relaxed);
    crate::ai::MUTE[i].store(on as usize, std::sync::atomic::Ordering::Relaxed);
}
fn esc(s: &str) -> String { s.replace(char::from(92), "/").replace('"', "'") }

/// i 번째 선수의 기록 챔피언(팀별 포지션 순 = 조립 순서)
unsafe fn champ_of(mrd: usize, i: usize, n: usize) -> String {
    let half = n / 2;
    let (o, k) = if i < half { (0x30usize, i) } else { (0x48, i - half) };
    let (p, l): (usize, usize) = (rd(mrd + o + 8), rd(mrd + o + 16));
    let mut v: Vec<usize> = (0..l).map(|j| p + j * 0x270).collect();
    v.sort_by_key(|a| rd::<u32>(a + 0x268));
    v.get(k).map(|a| rstr(*a)).unwrap_or_default()
}
/// 판단 기록 1건(0x410B)을 문자열까지 풀어 한 줄 JSON 으로. String{cap,ptr,len} 로 보이는 8B 정렬 3워드를 전부 찾아 (오프셋, 내용) 으로.
unsafe fn dump_rec(out: &mut String, tick: usize, r: usize) {
    let _ = write!(out, "{{\"t\":\"tr\",\"tick\":{tick},\"w\":[");
    for k in 0..130 { let _ = write!(out, "{}\"{:x}\"", if k > 0 { "," } else { "" }, crate::read_word(r + k * 8).unwrap_or(0)); }
    // 사유 Vec @+0x268 {cap, ptr, len} 원소 0x28: 앞 워드들과 문자열 후보
    out.push_str("],\"r\":[");
    let (rp, rn) = (crate::read_word(r + 0x270).unwrap_or(0), crate::read_word(r + 0x278).unwrap_or(0));
    if rn > 0 && rn <= 64 { for k in 0..rn { let e = rp + k * 0x28; let _ = write!(out, "{}[\"{:x}\",\"{:x}\",\"{:x}\",\"{:x}\",\"{:x}\"]", if k > 0 { "," } else { "" }, crate::read_word(e).unwrap_or(0), crate::read_word(e + 8).unwrap_or(0), crate::read_word(e + 16).unwrap_or(0), crate::read_word(e + 24).unwrap_or(0), crate::read_word(e + 32).unwrap_or(0)); let (sp, sn) = (crate::read_word(e + 8).unwrap_or(0), crate::read_word(e + 16).unwrap_or(0)); if sn > 0 && sn < 100 && crate::read_word(sp).is_some() { let b: Vec<u8> = (0..sn).map(|i| { let w = crate::read_word((sp + i) & !7).unwrap_or(0); (w >> (((sp + i) & 7) * 8)) as u8 }).collect(); if let Ok(s) = String::from_utf8(b) { let _ = write!(out, ",\"{}\"", esc(&s)); } } } }
    out.push_str("],\"s\":[");
    let mut first = true;
    let mut o = 0;
    while o + 24 <= 0x410 {
        let (c, p, n) = (crate::read_word(r + o).unwrap_or(0), crate::read_word(r + o + 8).unwrap_or(0), crate::read_word(r + o + 16).unwrap_or(0));
        if n > 0 && n < 400 && c >= n && c < 100_000 && p > 0x10000 && p < (1 << 47) && crate::read_word(p).is_some() {
            let b: Vec<u8> = (0..n).map(|i| { let w = crate::read_word((p + i) & !7).unwrap_or(0); (w >> (((p + i) & 7) * 8)) as u8 }).collect();
            if let Ok(s) = String::from_utf8(b) { if s.chars().all(|ch| !ch.is_control()) { let _ = write!(out, "{}[\"{o:x}\",\"{}\"]", if first { "" } else { "," }, esc(&s)); first = false; } }
        }
        o += 8;
    }
    out.push_str("]}\n");
}
/// stable dll 로더(0x34a6080)를 그대로 부른다: out 0xb8 {mod_id String@0, champions Vec@0x18, items@0x30, …}. out[0]==-2 구식, -1 오류(+8 코드).
unsafe fn load_mods(base: usize, db: usize) -> Vec<crate::modhost::Ext> {
    let mut exts = vec![];
    let dlls = crate::MOD_DLLS.lock().unwrap().clone();
    if dlls.is_empty() { return exts; }
    let gv: [u32; 4] = [1, 0, 7, 4]; // 모드에 알려 주는 게임 버전(패치 때 바꿀 것)
    let setv: unsafe extern "win64" fn(*const u32) = std::mem::transmute(base + 0x3fdca8d);
    setv(gv.as_ptr());
    let loader: unsafe extern "win64" fn(usize, *const usize, usize, usize, usize) -> usize = std::mem::transmute(base + 0x34a6080);
    for (path, h) in &dlls {
        let out = za(0xc0);
        let hm = *h;
        loader(out, &hm, 0, 0, 0);
        let tag: i64 = rd(out);
        if tag == -2 { println!("LAB 모드 {path}: 구식(stable 아님)"); continue; }
        if tag == -1 { println!("LAB 모드 {path}: 오류 코드 0x{:x}", rd::<u64>(out + 8)); continue; }
        println!("LAB 모드 {path}: id={:?} champions={} items={} native_effect={} native_passive={} level={} map={} hook={}", rstr(out), rd::<usize>(out + 0x28), rd::<usize>(out + 0x40), rd::<usize>(out + 0x78), rd::<usize>(out + 0x80), rd::<u32>(out + 0xb0), rd::<u8>(out + 0xb4), rd::<u8>(out + 0xb5));
        let nm = rstr(out);
        if std::env::var("LAB_EXTDBG").is_ok() { for (lab, o) in [("server", 0x88usize), ("client", 0x98)] { let p: usize = rd(out + o); let rw = |x: usize| crate::read_word(x).unwrap_or(0); println!("LAB EXT {lab} arc={p:x} vt=+0x{:x} words={}", rd::<usize>(out + o + 8).wrapping_sub(base), (0..8).map(|k| format!("{:x}", rw(p + k * 8))).collect::<Vec<_>>().join(" ")); let q = rw(p + 0x10); println!("     *0x10: {}", (0..6).map(|k| format!("{:x}", rw(q + k * 8))).collect::<Vec<_>>().join(" ")); let q2 = rw(p + 0x18); println!("     *0x18: {}", (0..9).map(|k| format!("{:x}", rw(q2 + k * 8))).collect::<Vec<_>>().join(" ")); } }
        if let Some(e) = crate::modhost::find_ext(&nm, rd(out + 0x98), false) { exts.push(e); }
        if let Some(e) = crate::modhost::find_ext(&nm, rd(out + 0x88), true) { exts.push(e); }
        let nitems: usize = rd(out + 0x40);
        if nitems > 0 {
            let content = za(0x80);
            std::ptr::copy_nonoverlapping((out + 0x30) as *const u8, (content + 0x30) as *mut u8, 24);
            for o in [0x0usize, 0x18, 0x48, 0x60] { *((content + o + 8) as *mut usize) = 8; } // 빈 Vec = {0, 8, 0}
            let res = za(0x20);
            let attach: unsafe extern "win64" fn(usize, usize, usize) -> usize = std::mem::transmute(base + 0x3508780);
            attach(res, content, db + 0x13868);
            { let ip: usize = rd(out + 0x38); let mut l = MOD_ITEMS.lock().unwrap_or_else(|e| e.into_inner()); for k in 0..nitems { l.push(rstr(ip + k * 0x28)); } }
            println!("LAB 모드 아이템 {nitems} 개 붙임 · 실패 {} 개", rd::<usize>(res + 16));
        }
        // dll 챔피언: 시트마다 Arc strong++ 후 0x3513c90(sheet, String*, arc, vt, 0, 1)
        let (cp, cn): (usize, usize) = (rd(out + 0x20), rd(out + 0x28));
        for k in 0..cn {
            let e = cp + k * 0x28;
            let (id, arc, vt) = (rstr(e), rd::<usize>(e + 0x18), rd::<usize>(e + 0x20));
            let put: unsafe extern "win64" fn(usize, usize, usize, usize, u8, u8) = std::mem::transmute(base + 0x3513c90);
            for (_, ci) in sheets(db).into_iter().take(1) {
                (*(arc as *const std::sync::atomic::AtomicUsize)).fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                let buf = halloc(id.len().max(1)); std::ptr::copy_nonoverlapping(id.as_ptr(), buf as *mut u8, id.len());
                let st: [usize; 3] = [id.len().max(1), buf, id.len()];
                put(ci, st.as_ptr() as usize, arc, vt, 0, 1);
            }
            println!("LAB 모드 챔피언 {id} 붙임");
        }
    }
    exts
}
/// 게임 파서로 JSON 을 읽는다. (파서 rva, 크기) → 구조체 사본 주소
unsafe fn parse_json(base: usize, rva: usize, size: usize, path: &str) -> Result<usize, String> {
    let raw: &'static [u8] = Box::leak(std::fs::read(path).map_err(|e| format!("{path}: {e}"))?.into_boxed_slice());
    let raw = raw.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(raw);
    let out = za(size + 0x40);
    let sr: [usize; 3] = [raw.as_ptr() as usize, raw.len(), 0];
    let f: unsafe extern "system" fn(usize, usize) -> usize = std::mem::transmute(base + rva);
    f(out, sr.as_ptr() as usize);
    // Err 표시는 종류마다 다르므로 두 번 읽어 내용이 같은지로 확인한다(호출자가 비교). -1 워드면 오류.
    if std::env::var("LAB_PARSEDBG").is_ok() { println!("LAB PARSE 0x{rva:x} 첫 워드들 {}", (0..6).map(|k| format!("{:x}", rd::<u64>(out + k * 8))).collect::<Vec<_>>().join(" ")); }
    if rd::<i64>(out) == -1 || rd::<i64>(out) == i64::MIN { return Err(format!("파싱 실패 {path}: 첫 워드 {:x} · 둘째 {:x}", rd::<u64>(out), rd::<u64>(out + 8))); }
    Ok(out)
}
/// 설정을 통째로 교체. 파서 결과를 대상마다 비트 복사(힙은 공유 · 이 프로세스는 해제하지 않음).
unsafe fn replace_setting(base: usize, db: usize, kind: &str, p: &str) -> Result<String, String> {
    let (rva, size) = match kind { "gs" => (0x3cd060, 0x1538usize), "ci" => (0x3ce410, 0x7a98), "is" => (0x3ce250, 0x3040), _ => return Err("종류는 gs/ci/is".into()) };
    let a = parse_json(base, rva, size, p)?;
    let mut targets = vec![];
    match kind {
        "gs" => { targets.push(db + 0x122d8); for (tag, ci) in sheets(db) { if tag != "live" { targets.push(ci - 0x1538); } } }
        "ci" => for (_, ci) in sheets(db) { targets.push(ci); },
        _ => targets.push(db + 0x13868),
    }
    for g in &targets {
        let keep: [u64; 3] = rd(*g); // ChampionInfo 맨 앞 = mod_champions Vec — 세이브 것을 남긴다
        std::ptr::copy_nonoverlapping(a as *const u8, *g as *mut u8, size);
        if kind == "ci" { *(*g as *mut [u64; 3]) = keep; }
    }
    Ok(format!("{} 곳 교체", targets.len()))
}
/// 달라진 8바이트 칸만 덮기. 같은 JSON 을 두 번 읽어 다른 칸(= 포인터·힙)은 비교에서 뺀다.
unsafe fn patch_setting(base: usize, db: usize, kind: &str, p0: &str, p1: &str) -> Result<String, String> {
    let (rva, size) = match kind { "gs" => (0x3cd060, 0x1538usize), "ci" => (0x3ce410, 0x7a98), "is" => (0x3ce250, 0x3040), _ => return Err("종류는 gs/ci/is".into()) };
    let (a, a2, m) = (parse_json(base, rva, size, p0)?, parse_json(base, rva, size, p0)?, parse_json(base, rva, size, p1)?);
    let mut diffs: Vec<(usize, u64)> = vec![];
    let mut ptrs = 0;
    for o in (0..size).step_by(8) {
        let (x, y, z) = (rd::<u64>(a + o), rd::<u64>(a2 + o), rd::<u64>(m + o));
        if x != y { ptrs += 1; continue; }
        if x != z { diffs.push((o, z)); }
    }
    // 대상: live + 과거 버전(GameSetting·ChampionInfo), 아이템은 live 한 곳
    let mut targets = vec![];
    match kind {
        "gs" => { targets.push(db + 0x122d8); for (tag, ci) in sheets(db) { if tag != "live" { targets.push(ci - 0x1538); } } }
        "ci" => for (_, ci) in sheets(db) { targets.push(ci); },
        _ => targets.push(db + 0x13868),
    }
    let nt = targets.len();
    for g in targets { for (o, v) in &diffs { *((g + o) as *mut u64) = *v; } }
    Ok(format!("달라진 칸 {} 개 · 포인터 칸 {} 개(무시) · 대상 {} 곳", diffs.len(), ptrs, nt))
}
/// 승패: 1 블루 · 0 레드 · 2 모름. ⚠World vt+0x138 은 「끝났는가」일 뿐 승패가 아니다(0.7.3 0x1690190 디스어셈 · 124건 대조로 확인 2026-11-25).
/// MOBA: 강제 종료면 World+0x146ae(2=없음), 아니면 남은 넥서스의 팀.
unsafe fn winner_code(sm: &Sim, g: &sim::Game, base: usize) -> u8 {
    if g.wvt != base + 0x41cd8b8 { return 2; }
    let f: u8 = rd(g.world + 0x146ae);
    if f != 2 { return f; }
    let mut alive = [false; 2];
    for h in sm.entities() { if sm.pget(696, h) as u32 == 6 && sm.ent_bool(104, h) { let t = sm.pget(88, h) as usize; if t < 2 { alive[t] = true; } } }
    match alive { [true, false] => 1, [false, true] => 0, _ => 2 }
}
pub unsafe fn run(base: usize, db: usize, head: usize, path: &str) -> bool {
    let text = match std::fs::read_to_string(path) { Ok(t) => t, Err(e) => { println!("LAB FAIL 시나리오 {path}: {e}"); return false; } };
    let s = match parse(&text) { Ok(s) => s, Err(e) => { println!("LAB FAIL {e}"); return false; } };
    for (k, p) in &s.full { match replace_setting(base, db, k, p) { Ok(m) => println!("LAB 설정 교체 {k}: {m}"), Err(e) => { println!("LAB FAIL 설정 교체 {k}: {e}"); return false; } } }
    for (k, p0, p1) in &s.settings { match patch_setting(base, db, k, p0, p1) { Ok(m) => println!("LAB 설정 {k}: {m}"), Err(e) => { println!("LAB FAIL 설정 {k}: {e}"); return false; } } }
    let exts = load_mods(base, db);
    if !exts.is_empty() || crate::SNAP.lock().unwrap().is_some() {
        // 게임이 하던 순서 흉내: 확장 초기화 → 서버 시작 → 타이틀 화면 몇 프레임 → 경기 중 화면 여러 프레임(늦게 설치하는 모드용)
        let frames: usize = std::env::var("LAB_FRAMES").ok().and_then(|v| v.parse().ok()).unwrap_or(400);
        crate::modhost::set_scene(1);
        for e in &exts { if !e.server { crate::modhost::call(e, 2, 0); } }
        for e in &exts { if e.server { crate::modhost::call(e, 2, 0); } }
        for k in 0..frames { if k == 30 { crate::modhost::set_scene(4); } for e in &exts { if !e.server { crate::modhost::call(e, 3, 16667); crate::modhost::call(e, 4, 16667); } } }
        println!("LAB 확장 {} 개 · 화면 갱신 {frames} 회", exts.len());
        if let Some(sn) = crate::SNAP.lock().unwrap().as_ref() {
            let d = crate::modhost::diff(base, sn);
            println!("LAB 바뀐 곳 {} 군데", d.len());
            for (sec, rva, len, f) in d.iter().take(400) { println!("LAB 변경 {sec} 0x{rva:x} 길이 {len} 함수 0x{f:x}"); }
        }
    }
    // 1) 챔피언 파일
    let mut skip = Vec::new();
    for (f, rep) in &s.files {
        match add_file(base, db, f, *rep) { Ok(id) => { println!("LAB 파일 {id} ({})", if *rep { "교체" } else { "추가/채움" }); if *rep { skip.push(id) } } Err(e) => { println!("LAB FAIL {e}"); return false; } }
    }
    // 2) 행동 패치
    patch_actions(base, db, &skip);
    // 3) 틀 리플레이
    let mut n: usize = rd(head + 0x330);
    let mut mrd = 0usize;
    while n != head { if rd::<u8>(n + 0x321) == 0 && s.key.map_or(true, |k| rd::<usize>(n + 0x338) == k) { mrd = n; break; } n = rd(n + 0x330); }
    if mrd == 0 { println!("LAB FAIL 기준 리플레이를 못 찾음 key={:?}", s.key); return false; }
    if let Some(v) = s.seed { *((mrd + 0x310) as *mut u64) = v; }
    if let Err(e) = apply_athletes(mrd, &s) { println!("LAB FAIL {e}"); return false; }
    if std::env::var("LAB_NOAI").is_err() { println!("LAB AI 훅 {}", crate::ai::install(base)); }
    // 4) 시작
    let g = match sim::start(base, db, mrd, s.mode, &s.keep) { Ok(g) => g, Err(e) => { println!("LAB FAIL 시작: {e}"); return false; } };
    let mut sm = Sim::new(base, g.world, g.wvt);
    sm.ctx[1] = g.gs; sm.ctx[7] = g.mode as usize; // Context{+8 GameSetting, +0x38 mode}
    let mut out = String::new();
    let ps = sm.players();
    // 선수 개체는 첫 틱 뒤에 생긴다 — head 는 1틱 진행 뒤에 쓴다.
    let _ = sm.tick();
    crate::arm_hw_bp(base);
    let mut ents: Vec<u64> = vec![0; ps.len()];
    let _ = write!(out, "{{\"t\":\"head\",\"seed\":{},\"players\":[", rd::<u64>(mrd + 0x310));
    for (i, h) in ps.iter().enumerate() { let t = sm.pget(248, *h); let _ = write!(out, "{}{{\"i\":{i},\"team\":{t},\"champ\":\"{}\"}}", if i > 0 { "," } else { "" }, champ_of(mrd, i, ps.len())); }
    out.push_str("]}\n");
    let trace = std::env::var("LAB_TRACE").is_ok();
    if trace { *((g.ctrl_addr() + 0x1dfe) as *mut u8) = 1; println!("LAB 판단기록 켬 (ctrl+0x1dfe)"); }
    let trace_from: usize = std::env::var("LAB_TRACE_FROM").ok().and_then(|v| v.parse().ok()).unwrap_or(0);
    let trace_to: usize = std::env::var("LAB_TRACE_TO").ok().and_then(|v| v.parse().ok()).unwrap_or(0);
    let mut ntrace = 0u64; let mut tdump = 0u32;
    // 5) 틱 루프 (기록 인자 r9b=1 — 이벤트가 채워짐)
    let mut ci = 0usize;
    let mut casts = vec![[0u32; 4]; ps.len()];
    let mut fx: std::collections::BTreeMap<(usize, String), u32> = Default::default();
    let cast_hold: usize = std::env::var("LAB_CAST_HOLD").ok().and_then(|v| v.parse().ok()).unwrap_or(90);
    let once = std::env::var("LAB_HOLD").is_err(); // 기본: 시전 시작되면 그 입력을 멈춤(허수아비로)
    let ilog = std::env::var("LAB_INPUT").is_ok();
    let mut inputs: std::collections::BTreeMap<(usize, u32), (u64, [usize; 4])> = Default::default();
    let mut rec = match &s.frames { Some((e, p)) => match crate::view::Rec::open(p, *e) { Ok(r) => Some(r), Err(x) => { println!("LAB FAIL 프레임 {p}: {x}"); return false; } }, None => None };
    let _ = writeln!(out, "{{\"t\":\"strategy\",\"blue\":{},\"red\":{}}}", sm.strategy_json(0), sm.strategy_json(1));
    let mut ent_said = vec![0u64; ps.len()];
    { let l = MOD_ITEMS.lock().unwrap_or_else(|e| e.into_inner()); if !l.is_empty() { let _ = writeln!(out, "{{\"t\":\"moditems\",\"list\":{:?}}}", *l); } }
    let mut ended = false;
    let bwant = std::env::var("LAB_BUFFN").is_ok();
    let mut bnames: std::collections::BTreeMap<String, u64> = Default::default();
    let vw = std::env::var("LAB_VIEW").is_ok();
    let mut vw_prev: std::collections::BTreeMap<usize, String> = Default::default();
    let mut vtag: std::collections::BTreeMap<u64, (String, (u32, u32))> = Default::default();
    let mut rg_prev: std::collections::BTreeMap<usize, String> = Default::default();
    let mut ticks = 0usize;
    let prog = std::env::var("LAB_PROGRESS").ok();
    if let Some(p) = &prog { let _ = std::fs::write(p, "0"); }
    let census = std::env::var("LAB_TAGS").is_ok();
    let mut tags: std::collections::BTreeMap<(u8, u64), (u64, String)> = Default::default();
    loop {
        if ticks > 0x927bf || (s.until > 0 && ticks >= s.until) { break; }
        while ci < s.cmds.len() && s.cmds[ci].tick <= ticks {
            let r = sm.cmd(&s.cmds[ci].words, &ps);
            let _ = writeln!(out, "{{\"t\":\"cmd\",\"tick\":{ticks},\"cmd\":\"{}\",\"r\":\"{r}\"}}", esc(&s.cmds[ci].words.join(" ")));
            ci += 1;
        }
        {
            let mut g2 = AUTO.lock().unwrap_or_else(|e| e.into_inner());
            let mut drop_list = vec![];
            for (i, (list, k, next)) in g2.iter_mut() {
                if *next != 0 && *next > ticks { continue; }
                if *k >= list.len() { drop_list.push(*i); continue; }
                crate::ai::set_override(*i, list[*k]);
                *k += 1; *next = ticks + 12;
            }
            for i in drop_list { g2.remove(&i); crate::ai::freeze(i); let _ = writeln!(out, "{{\"t\":\"castfail\",\"tick\":{ticks},\"i\":{i}}}"); }
        }
        for (i, _) in ps.iter().enumerate() {
            if !FROZEN[i & 15].load(std::sync::atomic::Ordering::Relaxed) { continue; }
            let o = crate::ai::OVERRIDE[i][0].load(std::sync::atomic::Ordering::Relaxed);
            if o != usize::MAX && o != 0 && o != usize::MAX - 1 { continue; } // 시전 명령 우선
            if HOLD[i & 15].load(std::sync::atomic::Ordering::Relaxed) > ticks { continue; } // 시전 동작 중 = 입력 비움(제자리 이동이 시전을 끊는다 — 2026-11-25)
            let rel = std::sync::atomic::Ordering::Relaxed;
            if ANCHOR[i & 15][0].load(rel) == u64::MAX { if let Some((x, y)) = ents[i].checked_sub(1).and_then(|_| sm.pos(ents[i])) { ANCHOR[i & 15][0].store(x, rel); ANCHOR[i & 15][1].store(y, rel); } }
            let (x, y) = (ANCHOR[i & 15][0].load(rel), ANCHOR[i & 15][1].load(rel));
            if x != u64::MAX { crate::ai::set_override(i, [0, x as usize, y as usize, 8]); }
        }
        for (i, h) in ps.iter().enumerate() { crate::ai::PTRS[i & 15].store(sm.player_mut(*h), std::sync::atomic::Ordering::Relaxed); }
        let Some(frame) = g.step(1) else { ended = true; break; };
        for (i, h) in ps.iter().enumerate() { let e = sm.ent(*h); if e != 0 { ents[i] = e; } }
        // 이벤트: ptr@8 len@0x10 원소 0x90 · 태그 byte@0 · 상자 포인터@8
        let (ep, en): (usize, usize) = (rd(frame + 8), rd(frame + 16));
        if ep != 0 && en < 100_000 {
            for k in 0..en {
                let e = ep + k * 0x90;
                let tag: u8 = rd(e);
                let pl: usize = rd(e + 8);
                let sw = |x: usize| crate::read_word(x).unwrap_or(0) as u64;
                if census {
                let cen = tags.entry((tag, if tag == 0 { sw(pl) } else { 0 })).or_insert((0, String::new()));
                cen.0 += 1;
                if cen.1.is_empty() { cen.1 = (0..8).map(|w| format!("{:x}", rd::<u64>(e + w * 8))).collect::<Vec<_>>().join(" "); if pl > 0x10000 && pl < 1 << 47 { cen.1 += " | "; cen.1 += &(0..10).map(|w| format!("{:x}", sw(pl.wrapping_add(w * 8)))).collect::<Vec<_>>().join(" "); } }
                }
                // 화면 동작 이름 흉내(0.7.4 0x239ff40 · 0x23a0290): 이동 이벤트 17 · 정지 18 이 run/idle 로 덮는 규칙
                if (tag == 17 || tag == 18) && (vw || rec.is_some()) {
                    let id = rd::<u32>(e + 4) as u64;
                    if ents.contains(&(id + 1)) {
                        let v = vtag.entry(id).or_insert_with(|| ("idle".into(), (u32::MAX, u32::MAX)));
                        let np = if tag == 17 { (rd::<u32>(e + 8), rd::<u32>(e + 12)) } else { v.1 };
                        let keep = if tag == 18 { v.0.ends_with("idle") } else if np == v.1 { v.0.ends_with("idle") || v.0.starts_with("idle") } else { v.0.ends_with("run") || v.0.starts_with("run") };
                        let to = if tag == 18 || np == v.1 { "idle" } else { "run" };
                        v.1 = np;
                        if !keep { v.0 = to.into(); let _ = writeln!(out, "{{\"t\":\"an\",\"tick\":{ticks},\"e\":{id},\"k\":{tag},\"n\":\"{to}\"}}"); }
                    }
                    continue;
                }
                if pl < 0x10000 || pl >= 1 << 47 { continue; }
                if vw {
                    // 그림 조사: 이벤트마다 개체 번호·들어 있는 글자를 남긴다
                    let w: Vec<u64> = (0..16).map(|k| sw(pl + k * 8)).collect();
                    let strs = |b: usize, n: usize| -> Vec<String> { (0..n).filter_map(|k| { let (c, p, l) = (sw(b + k * 8), sw(b + k * 8 + 8) as usize, sw(b + k * 8 + 16) as usize); if l == 0 || l > 64 || c < l as u64 || c > 256 || p < 0x10000 || p >= 1 << 47 { return None; }
                        let mut v = vec![]; for j in 0..l { let x = crate::read_word((p + j) & !7)?; v.push((x as u64 >> (((p + j) & 7) * 8)) as u8); }
                        if v.iter().all(|c| (0x20..0x7f).contains(c)) { String::from_utf8(v).ok() } else { None } }).collect() };
                    let mut st = strs(pl, 14);
                    if tag == 0 && w[1] > 0x10000 && w[1] < 1 << 47 { st.extend(strs(w[1] as usize, 6)); }
                    let who: Vec<usize> = if tag == 0 { ents.iter().enumerate().filter(|(_, x)| **x == w[2] + 1).map(|(i, _)| i).collect() } else { vec![] };
                    if tag != 0 || !who.is_empty() || !st.is_empty() { let _ = writeln!(out, "{{\"t\":\"ev\",\"tick\":{ticks},\"tag\":{tag},\"code\":{},\"who\":{who:?},\"s\":{st:?},\"w\":{:?}}}", if tag == 0 { w[0] } else { 0 }, &w[..8].iter().map(|x| format!("{x:x}")).collect::<Vec<_>>()); }
                }
                match tag {
                    0 => { let code: u64 = rd(pl); let raw: u64 = rd(pl + 16);
                        if (vw || rec.is_some()) && (code == 3 || code == 13) && ents.contains(&(raw + 1)) { // 동작 이름(3 = 시전 · 13 = 궁/스킬 중 동작)
                            let b = sw(pl + 8) as usize; let (sp, sn) = (sw(b + 8) as usize, sw(b + 16) as usize);
                            if sp > 0x10000 && sn > 0 && sn <= 64 { let v: Vec<u8> = (0..sn).map(|j| (crate::read_word((sp + j) & !7).unwrap_or(0) as u64 >> (((sp + j) & 7) * 8)) as u8).collect(); vtag.entry(raw).or_insert_with(|| (String::new(), (u32::MAX, u32::MAX))).0 = String::from_utf8_lossy(&v).into_owned(); let (av, ad) = if code == 3 { (sw(b + 0x30), sw(b + 0x38)) } else { (0, 0) }; let _ = writeln!(out, "{{\"t\":\"an\",\"tick\":{ticks},\"e\":{raw},\"k\":{code},\"n\":\"{}\",\"v\":{av},\"d\":{ad}}}", esc(&String::from_utf8_lossy(&v))); } }

                        if (16..=19).contains(&code) { if let Some(i) = ents.iter().position(|x| *x == raw + 1) { casts[i][(code - 16) as usize] += 1; if once { let o = crate::ai::OVERRIDE[i][0].load(std::sync::atomic::Ordering::Relaxed); if o == 2 + (code - 16) as usize { AUTO.lock().unwrap_or_else(|e| e.into_inner()).remove(&i); crate::ai::freeze(i); HOLD[i & 15].store(ticks + cast_hold, std::sync::atomic::Ordering::Relaxed); } } let _ = writeln!(out, "{{\"t\":\"cast\",\"tick\":{ticks},\"i\":{i},\"slot\":{}}}", code - 16); } } }
                    13 => { let (sp, sn): (usize, usize) = (rd(pl + 8), rd(pl + 16)); let (tgt, cst): (u64, u64) = (rd(pl + 32), rd(pl + 48));
                        if sp > 0x10000 && sn <= 64 { let nm = String::from_utf8_lossy(std::slice::from_raw_parts(sp as *const u8, sn)).into_owned();
                            if let Some(i) = ents.iter().position(|x| *x == cst + 1) { *fx.entry((i, nm.clone())).or_default() += 1; let k: u32 = rd(pl + 24); let y: u64 = rd(pl + 40); let _ = writeln!(out, "{{\"t\":\"fx\",\"tick\":{ticks},\"i\":{i},\"target\":{},\"k\":{k},\"x\":{tgt},\"y\":{y},\"c\":{cst},\"name\":\"{}\"}}", tgt + 1, esc(&nm)); } } }
                    2 => { // 투사체 생성: 이름 String@+0x40(cap,ptr,len) · 시전자 raw = [12]
                        let (sp, sn): (usize, usize) = (sw(pl + 0x48) as usize, sw(pl + 0x50) as usize); let cst = sw(pl + 12 * 8);
                        if sp > 0x10000 && sn > 0 && sn <= 64 { let nm = String::from_utf8_lossy(std::slice::from_raw_parts(sp as *const u8, sn)).into_owned();
                            if let Some(i) = ents.iter().position(|x| *x == cst + 1) { *fx.entry((i, format!("proj:{nm}"))).or_default() += 1; } } }
                    _ => {}
                }
            }
        }
        if trace {
            let bx: usize = rd(frame + 0x118);
            if bx != 0 { let (_cap, p, l): (usize, usize, usize) = (rd(bx), rd(bx + 8), rd(bx + 16));
                ntrace += l as u64;
                if std::env::var("LAB_TRACE_TXT").is_ok() && ticks >= trace_from && ticks < trace_to { for k in 0..l { dump_rec(&mut out, ticks, p + k * 0x410); } }
                if let Ok(dir) = std::env::var("LAB_TRACE_DIR") { if l < 64 { for k in 0..l { if tdump < 4000 { tdump += 1; let b = std::slice::from_raw_parts((p + k * 0x410) as *const u8, 0x410); std::fs::write(format!("{dir}/t{ticks:05}_{k}.bin"), b).ok(); } } } }
            }
        }
        g.drop_frame(frame);
        for i in 0..ps.len() { if ents[i] != 0 && ents[i] != ent_said[i] { ent_said[i] = ents[i]; let _ = writeln!(out, "{{\"t\":\"pl\",\"i\":{i},\"ent\":{}}}", ents[i] - 1); } }
        if let Some(r) = rec.as_mut() { if ticks % r.every == 0 { r.frame(&sm, g.world, ticks, &mut out); } }
        if let Ok(pd) = std::env::var("LAB_PDUMP") { let ts: Vec<usize> = pd.split(',').filter_map(|x| x.parse().ok()).collect();
            if ts.contains(&ticks) { let p = sm.player_mut(ps[0]); println!("PDUMP tick {ticks} lvl {} :", sm.pget(224, ps[0])); for o in (0x820..0x8f0usize).step_by(0x20) { println!("   +{o:x}: {}", (0..4).map(|k| format!("{:x}", rd::<u64>(p + o + k * 8))).collect::<Vec<_>>().join(" ")); } } }
        if ilog { for i in 0..ps.len() { let w = crate::ai::last(i); if w[0] != usize::MAX { *inputs.entry((i, w[0] as u32)).or_insert((0u64, w)) = { let o = inputs.get(&(i, w[0] as u32)).copied().unwrap_or((0, w)); (o.0 + 1, w) }; if ticks < 4000 && (w[0] as u32) != 0 { let _ = writeln!(out, "{{\"t\":\"in\",\"tick\":{ticks},\"i\":{i},\"w\":[{},{},{},{}]}}", w[0], w[1], w[2], w[3]); } } } }
        if bwant && ticks % 300 == 0 { for h in &ps { let e = sm.ent(*h); if e == 0 { continue; } for k in 0..sm.buffs(e).min(64) { let mut buf = [0u64; 80];
            if slot!(sm, 168, unsafe extern "C" fn(*const u8, u64, usize, *mut u64) -> bool)(sm.fat.as_ptr() as *const u8, e, k, buf.as_mut_ptr()) { let p = buf.as_ptr() as *const u8; let n = (buf[8] as usize).min(64); let nm = String::from_utf8_lossy(std::slice::from_raw_parts(p, n)).into_owned(); let nm = if nm == "serpen_permanent_buff" { let v: Vec<String> = (0..18).map(|z| std::ptr::read_unaligned(p.add(88 + z * 4) as *const i32).to_string()).collect(); format!("{nm} 팀{} [{}]", sm.pget(248, *h), v.join(" ")) } else { nm }; *bnames.entry(nm).or_insert(0u64) += 1; } } } }
        if vw || rec.as_ref().map_or(false, |r| r.every == 1) { for (i, h) in ps.iter().enumerate() { let e = sm.ent(*h); if e == 0 { continue; }
            let mut bl = vec![]; for k in 0..sm.buffs(e).min(64) { let mut buf = [0u64; 80]; if slot!(sm, 168, unsafe extern "C" fn(*const u8, u64, usize, *mut u64) -> bool)(sm.fat.as_ptr() as *const u8, e, k, buf.as_mut_ptr()) { let n = (buf[8] as usize).min(64); bl.push(String::from_utf8_lossy(std::slice::from_raw_parts(buf.as_ptr() as *const u8, n)).into_owned()); } }
            let mut cl = vec![]; for k in 0..(sm.pget(176, e) as usize).min(32) { let mut buf = [0u64; 24]; if slot!(sm, 184, unsafe extern "C" fn(*const u8, u64, usize, *mut u64) -> bool)(sm.fat.as_ptr() as *const u8, e, k, buf.as_mut_ptr()) { let n = (buf[14] as usize).min(64); cl.push(format!("{}:{}", buf[0] as u32, String::from_utf8_lossy(std::slice::from_raw_parts((buf.as_ptr() as *const u8).add(48), n)))); } }
            bl.sort(); let line = format!("\"b\":{bl:?},\"c\":{cl:?},\"nm\":{:?}", sm.name(e)); if vw_prev.get(&i) != Some(&line) { let _ = writeln!(out, "{{\"t\":\"st\",\"tick\":{ticks},\"i\":{i},{line}}}"); vw_prev.insert(i, line); } }
            if vw && ticks % 10 == 0 { let v: Vec<String> = sm.entities().iter().filter(|e| !ents.contains(e)).map(|e| sm.name(*e)).collect(); let mut m: std::collections::BTreeMap<String, u32> = Default::default(); for x in v { *m.entry(x).or_default() += 1; } let line = format!("{m:?}"); if vw_prev.get(&99) != Some(&line) { let _ = writeln!(out, "{{\"t\":\"ents\",\"tick\":{ticks},\"m\":{line}}}"); vw_prev.insert(99, line); } }
        }
        // 사거리: Entity+0x488 칸 4개(보폭 0x38 · range+0x10 · growth+0x18 · target+0x28 · casting+0x30) · 레벨+0x5c0 · 버프 사거리+0x430 · 반경+0x678·보정+0x468 (0.7.4 명세_074)
        if rec.is_some() && ticks % 10 == 0 {
            let getf: unsafe extern "win64" fn(usize, usize) -> usize = std::mem::transmute(rd::<usize>(sm.fat[1] + 0x208));
            for (i, h) in ps.iter().enumerate() {
                let eh = sm.ent(*h); if eh == 0 { continue; }
                let e = getf(sm.fat[0], eh as usize - 1); if e < 0x10000 { continue; }
                let g = |o: usize| crate::read_word(e + o).unwrap_or(0);
                let (lv, br, r0, rm) = (g(0x5c0).clamp(1, 60), g(0x430) as i64, g(0x678), g(0x468) as u32 as i32);
                let rad = if rm == 0 { r0 } else { r0.wrapping_mul((100 + rm as i64).max(0) as usize) / 100 };
                let sl: Vec<String> = (0..4).map(|k| { let b = 0x488 + k * 0x38; let ca = g(b + 0x30) as u32; if ca == u32::MAX { return "null".into(); } format!("[{},{ca},{}]", (g(b + 0x10) + g(b + 0x18).wrapping_mul(lv - 1)) as i64 + br, g(b + 0x28) as u32) }).collect();
                let line = format!("\"lv\":{lv},\"rad\":{rad},\"s\":[{}]", sl.join(","));
                if rg_prev.get(&i) != Some(&line) { let _ = writeln!(out, "{{\"t\":\"rg\",\"tick\":{ticks},\"i\":{i},{line}}}"); rg_prev.insert(i, line); }
            }
        }
        if ticks % s.sample == 0 {
            let _ = write!(out, "{{\"t\":\"s\",\"tick\":{ticks},\"p\":[");
            for (i, h) in ps.iter().enumerate() {
                let e = sm.ent(*h);
                let (x, y) = sm.pos(e).unwrap_or((0, 0));
                let (hp, mx) = sm.hp(e).unwrap_or((0, 0));
                let c = sm.cds(*h).unwrap_or([0; 4]);
                let _ = write!(out, "{}[{},{},{},{},{},{},{},[{},{},{},{}],{},{}]", if i > 0 { "," } else { "" }, sm.ent_bool(104, e) as u8, x, y, hp, mx, sm.pget(224, *h), sm.pget(232, *h), c[0], c[1], c[2], c[3], sm.pget(272, *h), sm.pget(280, *h));
                let sh = sm.pget(136, e) as usize; let bn = sm.buffs(e); let cn = sm.pget(176, e) as usize;
                out.pop(); let _ = write!(out, ",{bn},{cn},{sh}]");
            }
            out.push_str("]}\n");
        }
        ticks += 1;
        if ticks % 200 == 0 { if let Some(p) = &prog { let _ = std::fs::write(p, ticks.to_string()); } }
    }
    let _ = write!(out, "{{\"t\":\"end\",\"ticks\":{ticks},\"ended\":{ended},\"winner_code\":{},\"p\":[", winner_code(&sm, &g, base));
    for (i, h) in ps.iter().enumerate() {
        let _ = write!(out, "{}{{\"i\":{i},\"k\":{},\"d\":{},\"a\":{},\"cs\":{},\"items\":{:?},\"casts\":{:?},\"fx\":{{", if i > 0 { "," } else { "" }, sm.pget(272, *h), sm.pget(280, *h), sm.pget(288, *h), sm.pget(296, *h), sm.item_keys(*h), casts[i]);
        for (k, ((_, nm), c)) in fx.iter().filter(|((j, _), _)| *j == i).enumerate() { let _ = write!(out, "{}\"{}\":{c}", if k > 0 { "," } else { "" }, esc(nm)); }
        let _ = write!(out, "}},\"stat\":{}}}", sm.stats_json(*h, ""));
    }
    out.push_str("]}\n");
    for ((i, k), (n, w)) in &inputs { println!("INPUT 선수{i} 종류워드 0x{k:x} x{n} 예 {:x} {:x} {:x} {:x}", w[0], w[1], w[2], w[3]); }
    for ((tg, cd), (n, sample)) in &tags { println!("TAG {tg} code {cd} x{n} {sample}"); }
    if let Some(r) = rec.as_mut() { r.finish(); let _ = writeln!(out, "{{\"t\":\"projnames\",\"names\":[{}]}}", r.pnames.iter().map(|x| format!("\"{}\"", esc(x))).collect::<Vec<_>>().join(",")); }
    if let Err(e) = std::fs::write(&s.out, &out) { println!("LAB FAIL 출력 {}: {e}", s.out); return false; }
    if std::env::var("LAB_BUFFN").is_ok() { for (nm, c) in &bnames { println!("BUFF {nm} 선수틱표본 {c}"); } }
    if std::env::var("LAB_ITEMKEYS").is_ok() { for (i, h) in ps.iter().enumerate() { println!("ITEMS 선수{i} {:?}", sm.item_keys(*h)); } }
    crate::report_hw_bp();
    if trace { println!("LAB 판단기록 {ntrace} 건"); }
    println!("LAB think 호출 {} · 선수 대조 실패 {}", crate::ai::count(), crate::ai::MISS.load(std::sync::atomic::Ordering::Relaxed));
    let mut pk = (0u64, 0u64);
    for h in &ps { let k = sm.pget(272, *h); if sm.pget(248, *h) == 0 { pk.0 += k } else { pk.1 += k } }
    println!("LAB OK 틱 {ticks} · 선수킬합 {}:{} · 승자 {} · 출력 {}", pk.0, pk.1, match (ended, winner_code(&sm, &g, base)) { (false, _) => "미종료", (_, 1) => "블루", (_, 0) => "레드", _ => "모름" }, s.out);
    true
}
