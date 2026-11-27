//! 게임 0.7.4 상수·구조체 접근 (근거 = mods_report\tfm2_inhibitor\RE\2026-11-26_*.md · 정적 RE).
//! ⚠RVA·오프셋은 버전 종속 — 패치 때 MIG 매니페스트(tfm2_inhibitor.json)로 재핀.
#![allow(dead_code)]
use crate::mem::*;

// ── RVA (0.7.4) ──────────────────────────────────────────────────────────
pub const RVA_MOBATICK: usize = 0x1c1ca30;     // game.rs L2082 · fn(&mut World, &GameContext, &mut Rng, &mut Option<&mut Vec<Event>>)
pub const RVA_TOWER_UPDATE: usize = 0x1be3ea0; // tower.rs L210 · fn(World, &vt, &mut Rng, &GameContext, [rsp+28] id, [rsp+30] events)
pub const RVA_SPAWN_MINION: usize = 0x19ae550; // fn(out, &GameSetting, tutorial u8, tick, team, line u8, buffed u8, is_melee u8)
pub const RVA_ENTITY_CLONE: usize = 0x1c75a10; // <Entity as Clone>::clone(out, &src)
pub const RVA_ADD_ENTITY: usize = 0x1c12a20;   // World::add_entity(World, &mut Entity /*move*/) -> id
pub const RVA_ADD_BUFF: usize = 0x1be25a0;     // Entity::add_buff(&mut Entity, &BuffState 0x120)
pub const RVA_RECALC: usize = 0x1bd6e00;       // 계산 스탯 재계산(+0x6B0 dirty 일 때만)
pub const RVA_VT_MOBA_A: usize = 0x42213F0;    // Moba World vtable(내부 &dyn) — 타워 update rdx 대조
/// 위 세 진입 훅 공통 원본 12B (push rbp,r15,r14,r13,r12,rsi,rdi,rbx — rip 상대 없음)
pub const PRO_PUSH8: [u8; 12] = [0x55, 0x41, 0x57, 0x41, 0x56, 0x41, 0x55, 0x41, 0x54, 0x56, 0x57, 0x53];
pub const PRO_MOBATICK_TAIL: [u8; 4] = [0xb8, 0xd8, 0x1b, 0x00];
pub const PRO_TOWER_TAIL: [u8; 4] = [0x48, 0x81, 0xec, 0x78];
pub const PRO_SPAWN_TAIL: [u8; 4] = [0xb8, 0x78, 0x10, 0x00];

// ── World ────────────────────────────────────────────────────────────────
pub const W_DENSE_PTR: usize = 0x730;
pub const W_DENSE_LEN: usize = 0x738;
pub const W_SLOTS_PTR: usize = 0x748; // u32 dense idx, u32::MAX = 빈칸
pub const W_SLOTS_LEN: usize = 0x750;
pub const W_TICK: usize = 0x14300;
// kind 별 id 목록 Vec<u64>(insert L72 가 push · RE-C §4) — 타워 / 넥서스
pub const W_TOWERS_PTR: usize = 0x10630;
pub const W_TOWERS_LEN: usize = 0x10638;
pub const W_NEXUS_PTR: usize = 0x10678;
pub const W_NEXUS_LEN: usize = 0x10680;
pub const W_INIT_DONE: usize = 0x146AC; // u8 — L2053(포탑·넥서스·챔피언 생성) 끝나면 1
pub const ENT_STRIDE: usize = 0x6B8;

// ── GameContext ──────────────────────────────────────────────────────────
pub const C_SETTING: usize = 0x8;
pub const C_MAPDEF: usize = 0x20;
pub const C_MODE: usize = 0x38;      // u8 TutorialType — 0/8 = 5v5 풀맵(쌍둥이 있음)
pub const C_NO_MINION: usize = 0x3A; // u8 — 0 이 아니면 미니언 단계 건너뜀(추정)
// MapDef.towers Vec<TowerDef{pos:[(u64,u64);2], ty:u8}> (0x28B)
pub const M_TOWERS_PTR: usize = 0x50;
pub const M_TOWERS_LEN: usize = 0x58;
pub const TD_SIZE: usize = 0x28;
pub const TD_TY: usize = 0x20;

// ── Entity ───────────────────────────────────────────────────────────────
pub const E_FACTION: usize = 0x0;  // u8 0 = Team
pub const E_TEAM: usize = 0x8;
pub const E_LAST_ATK_TAG: usize = 0x28;
pub const E_KIND: usize = 0x68;    // u32: 3 Minion 4 Tower 5 Nexus
pub const E_TW_STATE: usize = 0x70;   // Tower: 0 Idle / 1 Attacking
pub const E_TW_NEAREST: usize = 0x88; // Tower: nearest_enemy 태그 0 = None
pub const E_TW_LASTHIT: usize = 0xA8; // Tower: last_hit 태그 −1 = None
pub const E_TW_STAT_HP: usize = 0xD8; // Tower stat 사본(+0xC8) 의 hp
pub const E_TW_CD: usize = 0x110;
pub const E_TW_TY: usize = 0x128;  // u8 TowerType
pub const E_MN_LINE: usize = 0x11A; // Minion line u8 0 Top 1 Mid 2 Bot
pub const E_MN_IS_RANGE: usize = 0x118; // Minion is_range bool
pub const E_ID: usize = 0x5B8;
pub const E_BASE_HP: usize = 0x5D8;
pub const E_MAX_HP: usize = 0x620;
pub const E_POS_X: usize = 0x658;
pub const E_POS_Y: usize = 0x660;
pub const E_HP: usize = 0x668;
pub const E_SPAWN_TICK: usize = 0x670;
pub const E_DMG_WIN: usize = 0x680; // +team*8 (0x680/0x688), +0x690/+0x698(block)
pub const E_ATK_TYPE: usize = 0x6A8;
pub const E_DIRTY: usize = 0x6B0;
pub const E_CAN_TARGET: usize = 0x6B1;

pub const KIND_MINION: u32 = 3;
pub const KIND_TOWER: u32 = 4;
pub const KIND_NEXUS: u32 = 5;
pub const TY_TWIN_A: u8 = 3;
pub const TY_TWIN_B: u8 = 4;

// ── BuffState (0x120) ────────────────────────────────────────────────────
pub const BUFF_SIZE: usize = 0x120;
pub const B_NAME_LEN: usize = 0x0;
pub const B_NAME: usize = 0x4;
pub const B_DURATION: usize = 0x48; // 0 Permanent
pub const B_ATTACK: usize = 0x58;
pub const B_ATTACK_MULT: usize = 0x5C;
pub const B_DEFENCE: usize = 0x68;
pub const B_HP: usize = 0x70;
pub const B_MR: usize = 0x78;
pub const B_HP_MULT: usize = 0x84;
pub const B_RADIUS_MULT: usize = 0x100;

/// 미니언 스폰점(하드코딩 · MapDef lanes 를 안 봄) [team][line] = (x, y)
pub const MINION_SPAWN: [[(u64, u64); 3]; 2] = [
    [(80000, 820000), (144000, 817000), (144000, 880000)],
    [(820000, 80000), (817000, 144000), (880000, 144000)],
];

pub type FnClone = extern "win64" fn(*mut u8, *const u8) -> usize;
pub type FnAddEntity = extern "win64" fn(usize, *mut u8) -> u64;
pub type FnAddBuff = extern "win64" fn(usize, *const u8);
pub type FnRecalc = extern "win64" fn(usize);

/// id → 엔티티 포인터 (get_entity 0x1c157e0 과 같은 식, 유령 슬롯 폴백 제외)
pub unsafe fn entity_by_id(w: usize, id: u64) -> Option<usize> {
    let slen = rd_u64(w + W_SLOTS_LEN)?;
    if id >= slen { return None; }
    let sptr = rd_u64(w + W_SLOTS_PTR)? as usize;
    let idx = rd_u32(sptr + 4 * id as usize)?;
    if idx == u32::MAX { return None; }
    let dlen = rd_u64(w + W_DENSE_LEN)?;
    if idx as u64 >= dlen { return None; }
    let dptr = rd_u64(w + W_DENSE_PTR)? as usize;
    if !ptr_ok(dptr) { return None; }
    Some(dptr + ENT_STRIDE * idx as usize)
}

/// 살아 있는 엔티티 dense 배열 (ptr, len)
pub unsafe fn dense(w: usize) -> Option<(usize, usize)> {
    let p = rd_u64(w + W_DENSE_PTR)? as usize;
    let n = rd_u64(w + W_DENSE_LEN)? as usize;
    if n > 100_000 || (n > 0 && !ptr_ok(p)) { return None; }
    Some((p, n))
}

/// MapDef 에서 ty 의 첫 TowerDef pos[team]
pub unsafe fn mapdef_tower_pos(ctx: usize, ty: u8, team: usize) -> Option<(u64, u64)> {
    let m = rd_u64(ctx + C_MAPDEF)? as usize;
    if !ptr_ok(m) { return None; }
    let p = rd_u64(m + M_TOWERS_PTR)? as usize;
    let n = rd_u64(m + M_TOWERS_LEN)? as usize;
    if n == 0 || n > 64 || !ptr_ok(p) { return None; }
    for i in 0..n {
        let td = p + i * TD_SIZE;
        if rd_u8(td + TD_TY)? == ty {
            let x = rd_u64(td + team * 16)?;
            let y = rd_u64(td + team * 16 + 8)?;
            return Some((x, y));
        }
    }
    None
}
