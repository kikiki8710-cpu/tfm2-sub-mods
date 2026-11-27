//! sim — 게임의 리플레이 재생 핸들러(0x2d70330)가 하는 일을 그대로 따라 해 경기를 재시뮬한다 (0.7.4).
//! 근거 = mods_report\tfm2_battle_lab\RE\2026-11-23_리플레이재생경로-런처-ctrl-오라클-헤드리스-0.7.3.md
//!
//! 순서: AI 팩토리 등록 → 버전 자산 4종 Arc 포장 → 런처(ctrl) → 로스터 blue/red(포지션 정렬 → 챔피언 조회 → 참가자 조립 → 추가)
//!       → 전략 세터 ×2 → 오라클(1틱) 반복(out[0]==-1 = 종료) → World 결과 판독.
//! ⚠소유권: Database 의 값을 비트 복사해 넘긴 것(전략 24B 등)은 Database 와 힙 포인터를 공유한다 — 이 프로세스는
//!   ctrl·Database 를 일부러 해제하지 않는다(누수 = 의도). Arc 는 strong=2 로 만들어 게임이 0 까지 내리지 못하게 한다.
use crate::{rd, rstr};

// ── 0.7.4 RVA ──
const AI_FACTORY_FN_SLOT: usize = 0x57166e0;
const AI_FACTORY_STATE: usize = 0x57166e8;
const AI_FACTORY: usize = 0x110d490;
const LAUNCHER: usize = 0x19a3370;
const CHAMP_LOOKUP: usize = 0x1dc1a30;
const PART_ASSEMBLE: usize = 0x2c26bf0;
const ADD_PART: usize = 0x19a5ed0;
const ORACLE: usize = 0x199bb30;
const FRAME_DROP: usize = 0x2045060;
const ORACLE_SENTINEL: usize = 0x5716008;
const OBSERVER_FLAG: usize = 0x58010e0;
// ── Database(0x180d0) 오프셋 ──
const DB_CHAMP_INFO: usize = 0x9308; // 0x7a98
const DB_GAME_SETTING: usize = 0x122d8; // 0x1538
const DB_MAP_SETTING: usize = 0x13810; // 0x58
const DB_ITEM_SETTING: usize = 0x13868; // 0x3040
const DB_PRE_PATCH: usize = 0x17cb0; // HashMap<String, PrePatchData 0x9018> 버킷 0x9030
// ── World / ctrl ──
const CTRL_SIZE: usize = 0x1e10;
const CTRL_WORLD: usize = 0x1b40;
const CTRL_WORLD_VT: usize = 0x1b48;
const W_SEED: usize = 0x142f8;
const W_TICK: usize = 0x14300;
const W_KILL0: usize = 0x14308;
const W_KILL1: usize = 0x14310;
const WVT_IS_END: usize = 0x138;
const WVT_SET_STRATEGY: usize = 0x198;
const ATH_STRIDE: usize = 0x270;
const ATH_POS: usize = 0x268;
const ATH_ID: usize = 0x238;
const PART_SIZE: usize = 0x8b8;
const FRAME_SIZE: usize = 0x128;

type Launcher = unsafe extern "win64" fn(out: *mut u8, mode: u8, seed: u64, flag: u8, a5: usize, a6: usize, a7: usize, a8: usize) -> *mut u8;
type Assemble = unsafe extern "win64" fn(out: *mut u8, ath: usize, name: *const u8, name_len: usize, team: u64, arc: usize, vt: usize) -> *mut u8;
type AddPart = unsafe extern "win64" fn(ctrl: *mut u8, part: *mut u8);
type Oracle = unsafe extern "win64" fn(out: *mut u8, ctrl: *mut u8, ctx: *mut u8, rec: u8, b: u8) -> *mut u8;
type Drop1 = unsafe extern "win64" fn(p: *mut u8);
type SetStrategy = unsafe extern "win64" fn(world: usize, idx: u32, strat: *mut u8);
type WorldBool = unsafe extern "win64" fn(world: usize) -> u8;

fn alloc16(size: usize) -> *mut u8 {
    unsafe { std::alloc::alloc_zeroed(std::alloc::Layout::from_size_align(size, 16).unwrap()) }
}

/// Rust ABI 팻포인터 반환(rax:rdx)을 받는 3인자 호출.
pub unsafe fn call3_ret2(f: usize, a: usize, b: usize, c: usize) -> (usize, usize) {
    let (r0, r1): (usize, usize);
    // ⚠Windows x64: 호출 직전 rsp 는 16 정렬(+ shadow 0x20). 처음 `sub rsp,0x28` 로 써서 8 어긋났고,
    //   게임 함수 머리의 `movaps [rbp+..],xmm6` 가 GP 예외(접근 주소 -1)로 죽었다(실측 2026-11-23).
    //   ⟹ 현재 rsp 를 r15(비휘발)에 보관하고 명시적으로 16 정렬한 뒤 shadow 0x20 을 둔다.
    std::arch::asm!(
        "mov r15, rsp",
        "and rsp, -16",
        "sub rsp, 0x20",
        "call {f}",
        "mov rsp, r15",
        f = in(reg) f,
        in("rcx") a, in("rdx") b, in("r8") c,
        lateout("rax") r0, lateout("rdx") r1, out("r15") _,
        clobber_abi("win64"),
    );
    (r0, r1)
}

/// ArcInner{strong, weak, data} 를 프로세스 힙(게임과 같은 할당자)에 만든다. strong=2 → 게임이 해제하지 못함.
unsafe fn make_arc(src: usize, size: usize) -> usize {
    let p = alloc16(0x10 + size) as usize;
    *(p as *mut usize) = 2;
    *((p + 8) as *mut usize) = 1;
    std::ptr::copy_nonoverlapping(src as *const u8, (p + 0x10) as *mut u8, size);
    p
}

/// pre_patch_data 에서 버전 키를 찾는다 → (GameSetting*, ChampionInfo*) · 없으면 None.
unsafe fn pre_patch_lookup(db: usize, ver: &str) -> Option<(usize, usize)> {
    let ctrl: usize = rd(db + DB_PRE_PATCH);
    let mask: usize = rd(db + DB_PRE_PATCH + 8);
    let items: usize = rd(db + DB_PRE_PATCH + 0x18);
    if ctrl == 0 || items == 0 { return None; }
    for i in 0..=mask {
        let c: u8 = rd(ctrl + i);
        if c & 0x80 != 0 { continue; }
        let bucket = ctrl - (i + 1) * 0x9030;
        if rstr(bucket) == ver {
            let val = bucket + 0x18;
            return Some((val, val + 0x1538));
        }
    }
    None
}

/// 진단: 선수 기록 items · 팀 개인 전술 · World 선수 상태 안의 "items 와 같은 usize 배열 Vec" 위치를 찍는다.
unsafe fn dump_builds(mrd: usize, world: usize) {
    // 1) 선수 기록 items(Vec<usize> 추정: cap@+0x18 ptr@+0x20 len@+0x28)
    let mut first_items: Vec<u64> = Vec::new();
    for (team, vec_off) in [(0, 0x30usize), (1, 0x48)] {
        let ptr: usize = rd(mrd + vec_off + 8);
        let len: usize = rd(mrd + vec_off + 16);
        for i in 0..len {
            let a = ptr + i * ATH_STRIDE;
            let ip: usize = rd(a + 0x20);
            let il: usize = rd(a + 0x28);
            let items: Vec<u64> = if ip != 0 && il < 64 { (0..il).map(|k| rd::<u64>(ip + k * 8)).collect() } else { vec![] };
            if team == 0 && i == 0 { first_items = items.clone(); }
            println!("      [빌드] team{team} pos{} {:?} 기록 items={items:?}", rd::<u32>(a + ATH_POS), rstr(a));
        }
    }
    // 2) 개인 전술 HashMap<String,[ItemBuildOverride;3]> @+0x298(blue)/+0x2c8(red) — 버킷 크기 0x20 가정
    for (team, off) in [(0, 0x298usize), (1, 0x2c8)] {
        let ctrl: usize = rd(mrd + off);
        let mask: usize = rd(mrd + off + 8);
        let items: usize = rd(mrd + off + 0x18);
        let mut ents = Vec::new();
        if ctrl != 0 && items > 0 && mask < 4096 {
            for i in 0..=mask {
                if rd::<u8>(ctrl + i) & 0x80 != 0 { continue; }
                let b = ctrl - (i + 1) * 0x20;
                ents.push(format!("{}={:?}", rstr(b), [rd::<u8>(b + 0x18), rd::<u8>(b + 0x19), rd::<u8>(b + 0x1a)]));
            }
        }
        println!("      [개인전술] team{team} 항목 {items} : {}", ents.join(" · "));
    }
    // 3) World vt+0x220 = 선수 슬라이스 (rax=시작, rdx=마지막 원소 주소 · SimVtableV1 player_count 가 (rdx-rax-8)/8+1 로 셈)
    let wvt: usize = rd(world + 0); // 사용 안 함(자리표시)
    let _ = wvt;
    let (s, e) = call3_ret2(rd::<usize>(WORLD_VT.with(|c| c.get()) + 0x220), world, 0, 0);
    let n = if e >= s { (e - s) / 8 } else { 0 };
    println!("      [World] 선수 슬라이스 시작=0x{s:x} 끝=0x{e:x} → {n}명");
    if n == 0 || n > 32 { return; }
    let s8: Vec<String> = (0..n).map(|i| format!("0x{:x}", srd(s + i * 8).unwrap_or(0))).collect();
    println!("      [World] 원소 8B 값: {}", s8.join(" "));
    // 원소가 포인터면 따라가서 "기록 items 와 같은 usize Vec" 을 찾는다(안전 읽기)
    for i in 0..n.min(10) {
        let Some(el) = srd(s + i * 8).map(|v| v as usize) else { continue };
        let mut hit = String::new();
        if el > 0x10000 && el < 0x7fff_ffff_ffff {
            for off in (0..0x1800usize).step_by(8) {
                let (Some(cap), Some(vp), Some(vl)) = (srd(el + off), srd(el + off + 8), srd(el + off + 16)) else { break };
                let (cap, vp, vl) = (cap as usize, vp as usize, vl as usize);
                if vl < 2 || vl > 16 || cap < vl || cap > 64 || vp < 0x10000 || vp > 0x7fff_ffff_ffff { continue; }
                let v: Vec<u64> = (0..vl).filter_map(|k| srd(vp + k * 8)).collect();
                if v.len() == vl && v.iter().all(|x| *x < 200) {
                    hit.push_str(&format!(" +0x{off:x}={v:?}{}", if i == 0 && v == first_items { "★" } else { "" }));
                }
            }
        }
        println!("      [World] 선수{i} 원소=0x{el:x}{hit}");
    }
}
thread_local! { static WORLD_VT: std::cell::Cell<usize> = const { std::cell::Cell::new(0) }; }

#[link(name = "kernel32")]
extern "system" {
    fn ReadProcessMemory(p: usize, addr: usize, buf: *mut u8, n: usize, got: *mut usize) -> i32;
    fn GetCurrentProcess() -> usize;
}
/// 진단용 안전 읽기(잘못된 주소면 None) — 직접 역참조는 AV 로 프로세스가 죽는다.
pub fn srd(addr: usize) -> Option<u64> {
    let mut v = 0u64;
    let mut got = 0usize;
    let ok = unsafe { ReadProcessMemory(GetCurrentProcess(), addr, &mut v as *mut u64 as *mut u8, 8, &mut got) };
    if ok != 0 && got == 8 { Some(v) } else { None }
}

/// 모드 API 함수표 SimVtableV1.strategy_get_json(+536) 으로 팀 전술 문서 전체를 JSON 으로.
unsafe fn strategy_json(base: usize, fat: &[usize; 2], team: usize) -> Option<String> {
    type F = unsafe extern "C" fn(state: *const u8, team: usize, path: *const u8, plen: usize, buf: *mut u8, cap: usize, out: *mut usize) -> bool;
    let f: F = std::mem::transmute(rd::<usize>(base + 0x4385b90 + 536));
    let st = fat.as_ptr() as *const u8;
    let mut len = 0usize;
    if !f(st, team, b"".as_ptr(), 0, std::ptr::null_mut(), 0, &mut len) { return None; }
    let mut buf = vec![0u8; len + 16];
    let mut full = 0usize;
    if !f(st, team, b"".as_ptr(), 0, buf.as_mut_ptr(), buf.len(), &mut full) { return None; }
    buf.truncate(full.min(len));
    String::from_utf8(buf).ok()
}

pub struct SimResult {
    pub ticks: usize,
    /// 선수별 킬 합(팀 0, 팀 1) — 기록 performance.kills 합과 같은 기준
    pub pkills: (u64, u64),
    pub world_tick: usize,
    pub kills: (u64, u64),
    pub seed_ok: bool,
    pub wall: std::time::Duration,
}

/// 실험실용: 경기를 만들기만 하고(런처→로스터→전략) 틱은 호출자가 진행한다.
pub struct Game { base: usize, ctrl: *mut u8, ctx: *mut u8, out: *mut u8, pub world: usize, pub wvt: usize, pub gs: usize, pub mode: u8 }
impl Game {
    /// 1틱. rec=1 이면 프레임 이벤트가 채워진다. 끝났으면 None. 프레임은 drop_frame 으로 해제.
    pub unsafe fn step(&self, rec: u8) -> Option<usize> {
        let oracle: Oracle = std::mem::transmute(self.base + ORACLE);
        oracle(self.out, self.ctrl, self.ctx, rec, 1);
        if rd::<u64>(self.out as usize) == u64::MAX { None } else { Some(self.out as usize) }
    }
    pub unsafe fn drop_frame(&self, f: usize) { let d: Drop1 = std::mem::transmute(self.base + FRAME_DROP); d(f as *mut u8) }
    /// 승패 코드: 1 = 블루 승, 0 = 레드 승, 그 밖 = 안 끝남(게임 함수 값 그대로)
    pub unsafe fn winner(&self) -> u8 { let f: WorldBool = std::mem::transmute(rd::<usize>(self.wvt + WVT_IS_END)); f(self.world) }
    pub fn ctrl_addr(&self) -> usize { self.ctrl as usize }
    pub unsafe fn blue_win(&self) -> bool { self.winner() == 1 }
}
pub unsafe fn start(base: usize, db: usize, mrd: usize, mode: u8, keep: &[(u64, u32)]) -> Result<Game, String> {
    *((base + AI_FACTORY_FN_SLOT) as *mut usize) = base + AI_FACTORY;
    *((base + AI_FACTORY_STATE) as *mut u32) = 0;
    let ver = rstr(mrd + 0x60);
    let (gs, ci) = match pre_patch_lookup(db, &ver) { Some((g, c)) => (g, c), None => (db + DB_GAME_SETTING, db + DB_CHAMP_INFO) };
    let (a_gs, a_map, a_item, a_ci) = (make_arc(gs, 0x1538), make_arc(db + DB_MAP_SETTING, 0x58), make_arc(db + DB_ITEM_SETTING, 0x3040), make_arc(ci, 0x7a98));
    let seed: u64 = rd(mrd + 0x310);
    let ctrl = alloc16(CTRL_SIZE);
    let launcher: Launcher = std::mem::transmute(base + LAUNCHER);
    launcher(ctrl, mode, seed, 0, a_gs, a_map, a_item, a_ci);
    let (world, wvt): (usize, usize) = (rd(ctrl as usize + CTRL_WORLD), rd(ctrl as usize + CTRL_WORLD_VT));
    if world == 0 || wvt == 0 { return Err("런처가 World 를 만들지 않았다".into()); }
    let assemble: Assemble = std::mem::transmute(base + PART_ASSEMBLE);
    let add: AddPart = std::mem::transmute(base + ADD_PART);
    for (team, vec_off) in [(0u64, 0x30usize), (1, 0x48)] {
        let (ptr, len): (usize, usize) = (rd(mrd + vec_off + 8), rd(mrd + vec_off + 16));
        let mut aths: Vec<usize> = (0..len).map(|i| ptr + i * ATH_STRIDE).collect();
        aths.sort_by_key(|a| rd::<u32>(a + ATH_POS));
        for a in aths {
            if !keep.is_empty() && !keep.contains(&(team, rd::<u32>(a + ATH_POS))) { continue; }
            let champ = rstr(a);
            let (arc, vt) = call3_ret2(base + CHAMP_LOOKUP, ci, champ.as_ptr() as usize, champ.len());
            if arc == 0 { return Err(format!("챔피언 조회 실패 {champ:?}")); }
            let name = format!("athlete_{}", rd::<usize>(a + ATH_ID));
            let part = alloc16(PART_SIZE + 0x40);
            assemble(part, a, name.as_ptr(), name.len(), team, arc, vt);
            if rd::<u64>(part as usize) == u64::MAX { return Err(format!("참가자 조립 실패 team={team} {champ:?}")); }
            add(ctrl, part);
        }
    }
    let set_strat: SetStrategy = std::mem::transmute(rd::<usize>(wvt + WVT_SET_STRATEGY));
    for (idx, off) in [(0u32, 0x78usize), (1, 0x90)] {
        let st = alloc16(0x20);
        std::ptr::copy_nonoverlapping((mrd + off) as *const u8, st, 0x18);
        set_strat(world, idx, st);
    }
    let ctx = alloc16(0x40);
    *((ctx as usize + 0x10) as *mut usize) = base + ORACLE_SENTINEL;
    Ok(Game { base, ctrl, ctx, out: alloc16(FRAME_SIZE + 0x40), world, wvt, gs, mode })
}

/// 리플레이 노드(MatchReplayData*) 1건을 재시뮬.
pub unsafe fn simulate(base: usize, db: usize, mrd: usize, log: bool) -> Result<SimResult, String> {
    // ① AI 팩토리(main 이 하던 등록)
    *((base + AI_FACTORY_FN_SLOT) as *mut usize) = base + AI_FACTORY;
    *((base + AI_FACTORY_STATE) as *mut u32) = 0;
    let obs: u8 = rd(base + OBSERVER_FLAG);
    // ② 버전 자산
    let ver = rstr(mrd + 0x60);
    let (gs, ci, src) = match pre_patch_lookup(db, &ver) {
        Some((g, c)) => (g, c, "pre_patch_data"),
        None => (db + DB_GAME_SETTING, db + DB_CHAMP_INFO, "live"),
    };
    let a_gs = make_arc(gs, 0x1538);
    let a_map = make_arc(db + DB_MAP_SETTING, 0x58);
    let a_item = make_arc(db + DB_ITEM_SETTING, 0x3040);
    let a_ci = make_arc(ci, 0x7a98);
    if log { println!("    자산 출처={src} (version {ver:?}) · observer_flag={obs}"); }
    // ③ 런처
    let seed: u64 = rd(mrd + 0x310);
    let ctrl = alloc16(CTRL_SIZE);
    let launcher: Launcher = std::mem::transmute(base + LAUNCHER);
    launcher(ctrl, 0, seed, 0, a_gs, a_map, a_item, a_ci);
    let world: usize = rd(ctrl as usize + CTRL_WORLD);
    let wvt: usize = rd(ctrl as usize + CTRL_WORLD_VT);
    if world == 0 || wvt == 0 { return Err("런처가 World 를 만들지 않았다".into()); }
    let wseed: u64 = rd(world + W_SEED);
    if log { println!("    런처 OK · World vt = game+0x{:x} (MOBA 기대 0x41cd8b8) · World seed 0x{wseed:016x}", wvt - base); }
    // ④ 로스터
    let assemble: Assemble = std::mem::transmute(base + PART_ASSEMBLE);
    let add: AddPart = std::mem::transmute(base + ADD_PART);
    for (team, vec_off) in [(0u64, 0x30usize), (1, 0x48)] {
        let ptr: usize = rd(mrd + vec_off + 8);
        let len: usize = rd(mrd + vec_off + 16);
        let mut aths: Vec<usize> = (0..len).map(|i| ptr + i * ATH_STRIDE).collect();
        aths.sort_by_key(|a| rd::<u32>(a + ATH_POS));
        for a in aths {
            // LAB_SWAP="team,pos,champ;..." : 이 프로세스 안 사본의 챔피언 이름만 바꾼다(파일은 안 건드림)
            if let Ok(sw) = std::env::var("LAB_SWAP") {
                for one in sw.split(';') {
                    let f: Vec<&str> = one.split(',').collect();
                    if f.len() == 3 && f[0].parse() == Ok(team) && f[1].parse() == Ok(rd::<u32>(a + ATH_POS)) {
                        let b: &'static str = Box::leak(f[2].to_string().into_boxed_str());
                        println!("    SWAP team{team} pos{} {:?} -> {b:?}", f[1], rstr(a));
                        *(a as *mut usize) = 0; *((a + 8) as *mut usize) = b.as_ptr() as usize; *((a + 16) as *mut usize) = b.len();
                    }
                }
            }
            let champ = rstr(a);

            let (arc, vt) = call3_ret2(base + CHAMP_LOOKUP, ci, champ.as_ptr() as usize, champ.len());
            if arc == 0 { return Err(format!("챔피언 조회 실패 {champ:?}")); }
            // 이름 = 라이브 DB 의 선수 이름(게임은 CD 선수 맵에서) — 여기선 id 문자열(⚠sim 영향 없음 = 추정)
            let name = format!("athlete_{}", rd::<usize>(a + ATH_ID));
            let part = alloc16(PART_SIZE + 0x40);

            assemble(part, a, name.as_ptr(), name.len(), team, arc, vt);

            if rd::<u64>(part as usize) == u64::MAX { return Err(format!("참가자 조립 실패 team={team} {champ:?}")); }
            add(ctrl, part);
        }
    }
    if log { WORLD_VT.with(|c| c.set(wvt)); dump_builds(mrd, world); }
    let fat: [usize; 2] = [world, wvt]; // SimVtableV1 state = &dyn AbstractGame
    let strat_before = if log { [strategy_json(base, &fat, 0), strategy_json(base, &fat, 1)] } else { [None, None] };
    // ⑤ 전략
    let set_strat: SetStrategy = std::mem::transmute(rd::<usize>(wvt + WVT_SET_STRATEGY));
    for (idx, off) in [(0u32, 0x78usize), (1, 0x90)] {
        let s = alloc16(0x20);
        std::ptr::copy_nonoverlapping((mrd + off) as *const u8, s, 0x18);
        set_strat(world, idx, s);
    }
    if log {
        for t in 0..2 {
            let after = strategy_json(base, &fat, t);
            let b = strat_before[t].clone().unwrap_or_default();
            let a = after.clone().unwrap_or_default();
            println!("      [팀전술] team{t} 주입 전 {}B / 후 {}B · {}", b.len(), a.len(), if a == b { "⚠변화 없음" } else { "바뀜" });
            println!("        전: {}", b.chars().take(600).collect::<String>());
            println!("        후: {}", a.chars().take(2000).collect::<String>());
            let raw: Vec<String> = (0..3).map(|k| format!("0x{:x}", rd::<u64>(mrd + if t == 0 { 0x78 } else { 0x90 } + k * 8))).collect();
            println!("        기록 24B = {}", raw.join(" "));
        }
    }
    // ⑥ 틱 루프 (헤드리스 0x22518e0 과 같은 상한 0x927bf)
    let oracle: Oracle = std::mem::transmute(base + ORACLE);
    let fdrop: Drop1 = std::mem::transmute(base + FRAME_DROP);
    let ctx = alloc16(0x40);
    *((ctx as usize + 0x10) as *mut usize) = base + ORACLE_SENTINEL;
    let out = alloc16(FRAME_SIZE + 0x40);
    let t0 = std::time::Instant::now();
    let mut n = 0usize;
    while n <= 0x927bf {
        oracle(out, ctrl, ctx, 0, 1);
        if rd::<u64>(out as usize) == u64::MAX { break; }
        fdrop(out);
        n += 1;
    }
    let is_end: WorldBool = std::mem::transmute(rd::<usize>(wvt + WVT_IS_END));
    let _ = is_end(world);
    // 선수별 킬 합(기록 performance.kills 와 같은 기준) — SimVtableV1 player_count(+192)/player_at(+200)/player_team(+248)/player_kills(+272)
    let fat2: [usize; 2] = [world, wvt];
    let st = fat2.as_ptr() as *const u8;
    let tbl = base + 0x4385b90;
    let pc: unsafe extern "C" fn(*const u8) -> usize = std::mem::transmute(rd::<usize>(tbl + 192));
    let pat: unsafe extern "C" fn(*const u8, usize) -> u64 = std::mem::transmute(rd::<usize>(tbl + 200));
    let pteam: unsafe extern "C" fn(*const u8, u64) -> usize = std::mem::transmute(rd::<usize>(tbl + 248));
    let pk: unsafe extern "C" fn(*const u8, u64) -> usize = std::mem::transmute(rd::<usize>(tbl + 272));
    let mut pkills = (0u64, 0u64);
    for i in 0..pc(st) {
        let h = pat(st, i);
        let k = pk(st, h) as u64;
        if pteam(st, h) == 0 { pkills.0 += k } else { pkills.1 += k }
    }
    Ok(SimResult {
        ticks: n,
        pkills,
        world_tick: rd(world + W_TICK),
        kills: (rd(world + W_KILL0), rd(world + W_KILL1)),
        seed_ok: wseed == seed,
        wall: t0.elapsed(),
    })
}
