//! view — 앱(화면)용 출력: ①세이브 검사 JSON ②프레임 이진 파일(BLF1) ③선수·개체 이름.
use crate::{lab::Sim, rd, read_word, rstr};
use std::collections::HashMap;
use std::fmt::Write as _;
use std::io::Write as _;

fn esc(s: &str) -> String { s.chars().map(|c| if c == '"' { '\'' } else if c == char::from(92) { '/' } else if c.is_control() { ' ' } else { c }).collect() }

/// Database 의 DataTable(+0x18 개수 · +0x20 헤드 센티넬) 노드 전부 → (노드, 키). link = 노드 안 링크 칸(= 값 크기), 키 = link+0x10.
/// 0.7.4 실측(2026-11-26 LAB_PROBE): 리플레이 0x328 · 경기 0xa0 · 팀 0x5d8 · 리그 0x80 · 대회 0x70 · 리그 대회 0xd8 · 국제 대회 차수 0xb8 · 솔랭 0xb0
unsafe fn table(db: usize, off: usize, link: usize) -> Vec<(usize, usize)> {
    let (len, hd): (usize, usize) = (rd(db + off + 0x18), rd(db + off + 0x20));
    let mut v = vec![];
    if hd < 0x10000 || len > 5_000_000 { return v; }
    let mut n: usize = rd(hd + link);
    while n != hd && n > 0x10000 && v.len() < len { v.push((n, rd::<usize>(n + link + 0x10))); n = rd(n + link); }
    v
}
/// Vec<usize> {cap, ptr, len} 의 값들
unsafe fn ids(vec: usize) -> Vec<usize> {
    let (p, l): (usize, usize) = (rd(vec + 8), rd(vec + 16));
    if p < 0x10000 || l > 1_000_000 { return vec![]; }
    (0..l).map(|i| rd::<usize>(p + i * 8)).collect()
}
const T_MATCHES: usize = 0x17af8;
const T_TEAMS: usize = 0x17900;
const T_LEAGUES: usize = 0x17890;
const T_TOURNAMENTS: usize = 0x178c8;
const T_LEAGUE_COMP: usize = 0x17b68;
const T_TOURN_COMP: usize = 0x17ba0;
const T_SOLO: usize = 0x17c48;

/// 리플레이 key → 경기 종류 이름(리그·국제 대회 이름 / "솔랭" / "연습" / ""), 팀 id → 이름
unsafe fn leagues(db: usize, replays: &[(usize, usize, usize)]) -> (HashMap<usize, String>, Vec<(usize, String)>) {
    // 대회 차수 → 리그·대회 이름(긴 이름 +0x18)
    let mut comp_name: HashMap<(u8, usize), String> = HashMap::new();
    for (n, _) in table(db, T_LEAGUES, 0x80) { let nm = rstr(n + 0x18); for c in ids(n + 0x30) { comp_name.insert((0, c), nm.clone()); } }
    for (n, _) in table(db, T_TOURNAMENTS, 0x70) { let nm = rstr(n + 0x18); for c in ids(n + 0x30) { comp_name.insert((1, c), nm.clone()); } }
    // 경기 id → 이름
    let mut match_name: HashMap<usize, String> = HashMap::new();
    for (n, k) in table(db, T_LEAGUE_COMP, 0xd8) { if let Some(nm) = comp_name.get(&(0, k)) { for o in [0usize, 0x18, 0x30] { for m in ids(n + o) { match_name.insert(m, nm.clone()); } } } }
    for (n, k) in table(db, T_TOURN_COMP, 0xb8) { if let Some(nm) = comp_name.get(&(1, k)) { for o in [0usize, 0x18] { for m in ids(n + o) { match_name.insert(m, nm.clone()); } } } }
    // 리플레이 id → 경기
    let mut rep: HashMap<usize, String> = HashMap::new();
    let (mut practice, mut named) = (0usize, 0usize);
    for (n, k) in table(db, T_MATCHES, 0xa0) {
        let nm = match match_name.get(&k) { Some(x) => { named += 1; x.clone() } None => if rd::<u8>(n + 0x9c) != 0 || rd::<u8>(n + 0x9d) != 0 { practice += 1; "연습".to_string() } else { String::new() } };
        for r in ids(n + 0x68) { rep.insert(r, nm.clone()); }
    }
    let solo: std::collections::HashSet<usize> = table(db, T_SOLO, 0xb0).iter().map(|(_, k)| *k).collect();
    let mut out = HashMap::new();
    let (mut by_key, mut by_id, mut by_solo, mut none) = (0, 0, 0, 0);
    for &(key, id, _) in replays {
        let v = if let Some(x) = rep.get(&key) { by_key += 1; x.clone() } else if let Some(x) = rep.get(&id) { by_id += 1; x.clone() } else if solo.contains(&id) || solo.contains(&key) { by_solo += 1; "솔랭".into() } else { none += 1; String::new() };
        out.insert(key, v);
    }
    println!("INSPECT 경기 종류: 대회 경기 {named} · 연습 {practice} · 리플레이 연결 key {by_key} / id {by_id} / 솔랭 {by_solo} / 모름 {none} (솔랭 표 {})", solo.len());
    let teams = table(db, T_TEAMS, 0x5d8).iter().map(|&(n, k)| (k, rstr(n + 0xf8))).collect();
    (out, teams)
}

/// 세이브 검사: 리플레이 목록·선수단·챔피언 목록·패치 버전 → JSON 한 개
pub unsafe fn inspect(db: usize, head: usize, out: &str, save: &str) -> bool {
    let mut s = String::new();
    // 리플레이 (key, id, 노드) — 경기 종류를 먼저 이어 둔다
    let mut reps = vec![];
    { let mut m: usize = rd(head + 0x330); while m != head && m > 0x10000 { reps.push((rd::<usize>(m + 0x338), rd::<usize>(m + 0x2f8), m)); m = rd(m + 0x330); } }
    let (lg, teams) = leagues(db, &reps);
    let _ = write!(s, "{{\"save\":\"{}\",\"teams\":{{", esc(save));
    for (i, (k, n)) in teams.iter().enumerate() { let _ = write!(s, "{}\"{k}\":\"{}\"", if i > 0 { "," } else { "" }, esc(n)); }
    let _ = write!(s, "}},\"versions\":[");
    let (ctrl, mask): (usize, usize) = (rd(db + 0x17cb0), rd(db + 0x17cb8));
    let mut first = true;
    if ctrl != 0 { for i in 0..=mask { if rd::<u8>(ctrl + i) & 0x80 == 0 { let _ = write!(s, "{}\"{}\"", if first { "" } else { "," }, esc(&rstr(ctrl - (i + 1) * 0x9030))); first = false; } } }
    let _ = write!(s, "],\"data_champions\":[");
    let (p, n): (usize, usize) = (rd(db + 0x9308 + 8), rd(db + 0x9308 + 16));
    for i in 0..n { let _ = write!(s, "{}\"{}\"", if i > 0 { "," } else { "" }, esc(&rstr(p + i * 0xb38 + 0x10))); }
    let _ = write!(s, "],\"replays\":[");
    let mut m: usize = rd(head + 0x330);
    let mut first = true;
    while m != head {
        if rd::<u8>(m + 0x321) == 0 {
            let k = |q: usize| (0..5).map(|i| rd::<u64>(q + 0x50 + i * 8)).sum::<u64>();
            let _ = write!(s, "{}{{\"key\":{},\"tick\":{},\"blue_win\":{},\"k0\":{},\"k1\":{},\"t0\":{},\"t1\":{},\"seed\":\"{}\",\"ver\":\"{}\",\"lg\":\"{}\",\"roster\":[", if first { "" } else { "," },
                rd::<usize>(m + 0x338), rd::<usize>(m + 0x318), rd::<u8>(m + 0x320), k(m + 0xa8), k(m + 0x1a0), rd::<usize>(m + 0x300), rd::<usize>(m + 0x308), rd::<u64>(m + 0x310), esc(&rstr(m + 0x60)), esc(lg.get(&rd::<usize>(m + 0x338)).map(|x| x.as_str()).unwrap_or("")));
            first = false;
            let mut f2 = true;
            for (t, off) in [(0, 0x30usize), (1, 0x48)] {
                let (rp, rn): (usize, usize) = (rd(m + off + 8), rd(m + off + 16));
                let mut v: Vec<usize> = (0..rn).map(|i| rp + i * 0x270).collect();
                v.sort_by_key(|a| rd::<u32>(a + 0x268));
                for a in v {
                    let (ip, il): (usize, usize) = (rd(a + 0x20), rd(a + 0x28));
                    let items: Vec<String> = if ip != 0 && il < 32 { (0..il).map(|k| rd::<u64>(ip + k * 8).to_string()).collect() } else { vec![] };
                    let st: Vec<String> = (0..17).map(|k| rd::<u64>(a + 0xc0 + k * 8).to_string()).collect();
                    let _ = write!(s, "{}{{\"team\":{t},\"pos\":{},\"champ\":\"{}\",\"ath\":{},\"stat\":[{}],\"items\":[{}]}}", if f2 { "" } else { "," }, rd::<u32>(a + 0x268), esc(&rstr(a)), rd::<usize>(a + 0x238), st.join(","), items.join(","));
                    f2 = false;
                }
            }
            s.push_str("]}");
        }
        m = rd(m + 0x330);
    }
    s.push_str("]}");
    match std::fs::write(out, s) { Ok(_) => true, Err(e) => { println!("INSPECT FAIL {e}"); false } }
}

/// 화면용 프레임 기록기(BLF1): 매 레코드 = u32 tick, u16 개체수, u16 투사체수, 개체 16B × n, 투사체 12B × n
pub struct Rec { f: std::io::BufWriter<std::fs::File>, pub every: usize, pn: HashMap<String, u16>, pub pnames: Vec<String>, known: HashMap<u64, ()> }
impl Rec {
    pub fn open(path: &str, every: usize) -> std::io::Result<Self> {
        let mut f = std::io::BufWriter::with_capacity(1 << 20, std::fs::File::create(path)?);
        f.write_all(b"BLF1")?;
        Ok(Rec { f, every: every.max(1), pn: HashMap::new(), pnames: vec![], known: HashMap::new() })
    }
    /// 한 틱 기록. 처음 보는 개체는 ndjson 에 이름 줄을 남긴다.
    pub unsafe fn frame(&mut self, sm: &Sim, world: usize, tick: usize, nd: &mut String) {
        let ids = sm.entities();
        let mut eb: Vec<u8> = Vec::with_capacity(ids.len() * 16);
        let mut cnt = 0u16;
        for h in ids {
            let Some((x, y)) = sm.pos(h) else { continue };
            let (hp, mx) = sm.hp(h).unwrap_or((0, 0));
            let kind = sm.pget(696, h) as u32;
            let team = sm.pget(88, h);
            let alive = sm.ent_bool(104, h);
            if !self.known.contains_key(&h) {
                self.known.insert(h, ());
                let _ = writeln!(nd, "{{\"t\":\"ent\",\"id\":{},\"name\":\"{}\",\"kind\":{kind},\"team\":{team},\"r\":{}}}", h - 1, esc(&sm.name(h)), sm.pget(144, h));
            }
            eb.extend_from_slice(&((h - 1) as u32).to_le_bytes());
            eb.extend_from_slice(&((x / 16).min(65535) as u16).to_le_bytes());
            eb.extend_from_slice(&((y / 16).min(65535) as u16).to_le_bytes());
            eb.extend_from_slice(&(hp.min(65535) as u16).to_le_bytes());
            eb.extend_from_slice(&(mx.min(65535) as u16).to_le_bytes());
            eb.extend_from_slice(&[kind as u8, team as u8, alive as u8, 0]);
            cnt += 1;
        }
        // 투사체: World+0x808 Vec(ptr@0x808+8? → VR 확인: ptr 0x808 len 0x810) 이름 String@+0x90 · 위치는 함수표
        let pl = sm.projectiles();
        let (vp, vn): (usize, usize) = (read_word(world + 0x808).unwrap_or(0), read_word(world + 0x810).unwrap_or(0));
        let mut pb: Vec<u8> = Vec::with_capacity(pl.len() * 12);
        let mut pc = 0u16;
        for (i, h) in pl.iter().enumerate() {
            let Some((x, y, caster, team)) = sm.proj_info(*h) else { continue };
            let name = if i < vn && vp > 0x10000 { let e = vp + i * 0x138; let (sp, sn) = (read_word(e + 0x98).unwrap_or(0), read_word(e + 0xa0).unwrap_or(0)); if sp > 0x10000 && sn > 0 && sn < 80 { let b: Vec<u8> = (0..sn).map(|k| { let w = read_word((sp + k) & !7).unwrap_or(0); (w >> (((sp + k) & 7) * 8)) as u8 }).collect(); String::from_utf8(b).unwrap_or_default() } else { String::new() } } else { String::new() };
            let idx = match self.pn.get(&name) { Some(v) => *v, None => { let v = self.pnames.len() as u16; self.pnames.push(name.clone()); self.pn.insert(name, v); v } };
            pb.extend_from_slice(&(caster as u32).to_le_bytes());
            pb.extend_from_slice(&((x / 16).min(65535) as u16).to_le_bytes());
            pb.extend_from_slice(&((y / 16).min(65535) as u16).to_le_bytes());
            pb.extend_from_slice(&idx.to_le_bytes());
            pb.extend_from_slice(&[team as u8, 0]);
            pc += 1;
        }
        let _ = self.f.write_all(&(tick as u32).to_le_bytes());
        let _ = self.f.write_all(&cnt.to_le_bytes());
        let _ = self.f.write_all(&pc.to_le_bytes());
        let _ = self.f.write_all(&eb);
        let _ = self.f.write_all(&pb);
    }
    pub fn finish(&mut self) { let _ = self.f.flush(); }
}
