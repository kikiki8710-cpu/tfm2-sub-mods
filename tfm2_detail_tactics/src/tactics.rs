//! tactics — 상세전술 32개 표 · 단계 선택 · 단계 값 · 내 팀 판별. (2026-10-22, 0.6.5)
//!
//! 다른 모듈은 여기만 본다:
//!   · UI/저장  → `set_level` / `level` / `state_text` / `apply_state_text`
//!   · 설정 파일 → `set_value`
//!   · 적용 장치 → `value_for(tactic, athlete_id, pos)` — None 이면 원본 그대로.
//! 모든 상태는 원자 변수라 어느 스레드(시뮬 스레드 포함)에서 읽어도 된다. 락·할당 없음.
use std::sync::atomic::{AtomicBool, AtomicI64, AtomicU32, AtomicU64, AtomicU8, Ordering::Relaxed};
use std::sync::{Mutex,MutexGuard,atomic::Ordering::{Acquire,AcqRel,Release}};
static CONFIG_WRITE:Mutex<()>=Mutex::new(());
static CONFIG_REV:AtomicU32=AtomicU32::new(0);
pub struct ConfigUpdate(MutexGuard<'static,()>);
pub fn config_update()->ConfigUpdate{
    let guard=CONFIG_WRITE.lock().unwrap_or_else(|e|e.into_inner());
    CONFIG_REV.fetch_add(1,AcqRel);ConfigUpdate(guard)
}
impl Drop for ConfigUpdate{fn drop(&mut self){CONFIG_REV.fetch_add(1,Release);}}
pub fn config_revision()->Option<u32>{let v=CONFIG_REV.load(Acquire);(v&1==0).then_some(v)}
pub fn config_unchanged(v:u32)->bool{
    std::sync::atomic::fence(Acquire);
    CONFIG_REV.load(Relaxed)==v
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Scope {
    /// 포지션(탑·정글·미드·원딜·서폿)마다 따로 고른다.
    Player,
    /// 팀 전체에 하나.
    Team,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Kind {
    /// 숫자 문턱 — 단계: 기본 / 낮게 / 높게.
    Value,
    /// 켬·끔 — 단계: 기본(켬) / 끔.
    Toggle,
}

pub struct TacticDef {
    /// 설정 파일 [섹션] 이름 · 저장 키.
    pub id: &'static str,
    /// tactics_verify 노브 id(지점 표 `knobs32_065.json` 의 id).
    pub knob: &'static str,
    pub name: &'static str,
    pub desc: &'static str,
    pub scope: Scope,
    pub kind: Kind,
    /// 게임 원본값(표시용).
    pub default: i64,
    /// 설정 파일이 없을 때 쓸 낮게/높게 기본값.
    pub low: i64,
    pub high: i64,
    /// 허용 범위(즉치 폭 한계 — tactics_verify KNOBS min/max).
    pub min: i64,
    pub max: i64,
}

const fn v(id: &'static str, knob: &'static str, name: &'static str, desc: &'static str, scope: Scope,
           default: i64, low: i64, high: i64, min: i64, max: i64) -> TacticDef {
    TacticDef { id, knob, name, desc, scope, kind: Kind::Value, default, low, high, min, max }
}
const fn t(id: &'static str, knob: &'static str, name: &'static str, desc: &'static str, scope: Scope) -> TacticDef {
    TacticDef { id, knob, name, desc, scope, kind: Kind::Toggle, default: 1, low: 0, high: 1, min: 0, max: 1 }
}

use Scope::{Player as P, Team as T};

/// 표 순서 = 전술 번호(저장·적용 장치가 이 번호를 쓴다 — 바꾸지 말고 뒤에 추가할 것).
pub static TACTICS: &[TacticDef] = &[
    // ── 후퇴·교전 회피
    // 0.7.2 상한 126: 라인 수비 사본 창 0x11519b7 이 `cmp rax,v+1` 이라 127 은 1바이트 즉치를 넘는다(100 이상은 어느 값이든 「항상」)
    v("combat_retreat_hp", "combat_retreat_hp_35", "본진 위협 중 후퇴 체력(%)", "본진이 공격받는 동안에는 체력이 이 값 이하여야 교전에서 물러난다. 높을수록 일찍 빠진다. 본진 위협이 없으면 영향 없다", P, 35, 20, 50, 0, 126),
    v("line_chase_retreat_hp", "line_chase_retreat_hp_45", "포탑 위험 피하기 체력(%)", "체력이 이 값 이하면 이동·추격 길이 적 포탑 위험 자리인지 항상 확인해 피한다. 높을수록 조심한다", P, 45, 30, 60, 0, 127),
    v("line_chase_target_guard_hp", "line_chase_target_guard_hp_65", "포탑에 맞은 뒤 위험 피하기 체력(%)", "방금 적 포탑에 맞았고 체력이 이 값 이하면 적 포탑 위험 자리를 피한다. 높을수록 조심한다", P, 65, 50, 80, 0, 127),
    v("lane_retreat_final", "lane_retreat_final_50", "라인 후퇴 점수 문턱", "라인 후퇴 점수가 이 값 이상이면 후퇴한다. 낮을수록 잘 빠진다. 낮은 체력 같은 즉시 후퇴 이유는 따로 판단한다", P, 50, 35, 65, 0, 127),
    v("crowd_retreat_weight", "crowd_retreat_weight_plus2_300", "수적 열세 후퇴 가중치", "적이 2명 이상 많을 때 후퇴 쪽에 주는 무게", P, 300, 150, 600, 0, 100000),
    v("combat_avoid_hp", "combat_avoid_hp_41", "도주 중 궁극기 사용 체력(%)", "이미 도망치는 중 체력이 이 값보다 낮으면 도주에 쓸 수 있는 궁극기도 쓴다. 높을수록 잘 쓴다", P, 41, 25, 60, 0, 127),
    v("combat_avoid_count_margin", "combat_avoid_count_margin_2", "도주 중 궁극기 사용 인원 차(명)", "이미 도망치는 중 적이 아군보다 이만큼 많으면 도주에 쓸 수 있는 궁극기도 쓴다. 작을수록 잘 쓴다", P, 2, 1, 3, 0, 127),
    v("self_risk_ignore_hp", "self_risk_ignore_hp_35", "본진 위협 중 위험 무시 체력(%)", "본진이 공격받을 때 체력이 이 값보다 높으면 주변 위험을 무시하고 싸운다. 본진 위협이 없으면 영향 없다", P, 35, 20, 55, 0, 127),
    // ── 갱킹·정글
    v("jg_gank_hp_start", "jg_gank_hp_start_50", "눈에 띈 적 갱 시작 체력(%)", "정글러 체력이 이 값 이상이면 눈에 띈 적을 바로 노리는 갱까지 본다. 낮아도 일정에 따른 갱은 갈 수 있다", P, 50, 35, 70, 0, 127),
    v("jg_gank_hp_floor", "jg_gank_hp_floor_40", "갱 출발 최소 체력(%)", "정글러 체력이 이 값 이하면 갱을 새로 잡지 않는다. 갱 도중에 그만두는 체력은 「갱 포기 체력」에서 정한다", P, 40, 25, 60, 0, 127),
    v("gank_abort_hp", "gank_abort_hp_41", "갱 포기 체력(%)", "갱 진행 중 체력이 이 값보다 낮으면 포기", P, 41, 25, 60, 0, 127),
    t("gank_abort_tower", "gank_abort_tower_reason9", "로밍 중 내 라인 포탑 공격받으면 중단", "로밍 갱 중 떠나온 라인의 우리 포탑이 미니언과 싸우면 그만둔다. 끄면 계속 간다. 라인 개입 특성 선수의 로밍에만 적용", P),
    v("gank_call_accept_hp", "gank_call_accept_hp_60", "갱 콜 수락 최소 체력(%)", "라인 계획 중 갱 콜 수락에 필요한 체력. 정글도 라인 계획이면 적용", P, 60, 40, 80, 1, 127),
    v("jungle_safe_radius", "jungle_safe_radius_150000", "상대 정글 매복 안전 반경", "상대 정글 캠프에 매복할 때 이 거리 안에 적 라이너·서포터가 있으면 매복하지 않는다. 높을수록 조심한다", P, 150000, 100000, 250000, 1000, 2000000),
    v("jungle_stop_hp", "jungle_stop_hp_41", "정글링 중단 체력(%)", "정글러 체력이 이 값 아래면 정글링을 멈춘다. 회복 수단이 있으면 20 낮은 값을 쓴다. 갱 출발 판단에도 쓰인다", P, 41, 31, 55, 30, 127),
    v("minion_damage_ratio", "minion_damage_ratio_49", "챔피언 공격 시 미니언 피해 민감도(%)", "라인전 단계 뒤 적 챔피언을 칠 자리의 미니언 예상 피해율이 이 값을 넘으면 위험으로 본다. 낮을수록 조심한다", P, 49, 39, 65, 39, 127),
    // ── 라인 운영·로밍
    v("lane_assign_hp", "lane_assign_hp_30", "라인 배정 체력(%)", "라인 첫 포탑이 하나라도 무너진 뒤 라인 담당을 뽑을 때 탑·미드·원딜에게 필요한 체력. 정글·서폿은 제외", P, 30, 15, 50, 0, 127),
    v("lane_opportunity_own_min", "lane_opportunity_own_min_2", "라인 기회 — 건강 아군 최소(명)", "라인 영역에 체력 50% 이상 아군이 이만큼 있어야 푸시 기회로 본다. 팀 전체 인원 조건은 따로 있다", P, 2, 1, 3, 0, 127),
    t("support_minion_block", "support_minion_block_role_4", "서폿 라인 미니언 처리 금지", "끄면 서폿도 라인 미니언을 치러 자리를 옮긴다. 사거리 안의 미니언은 켜 두어도 친다", P),
    t("bot_duo_partner", "bot_duo_partner_required", "원딜·서폿 짝 동행 확인", "끄면 원딜·서폿이 다른 라인 갱에 합류할 때 짝의 상태를 확인하지 않는다", P),
    v("liner_roam_self_hp", "liner_roam_self_hp_41", "로밍 최소 자기 체력(%)", "라이너가 이 체력 이상일 때만 로밍", P, 41, 25, 60, 0, 127),
    v("liner_roam_lane_healthy_hp", "liner_roam_lane_healthy_hp_41", "로밍 대상 라인 — 건강 아군 기준(%)", "라인 개입 특성 선수가 로밍 갈 라인을 고를 때 이 체력 이상인 그 라인 아군만 센다. 높을수록 덜 간다", P, 41, 25, 60, 0, 127),
    v("liner_roam_call_chance", "liner_roam_call_chance_700", "로밍 팀 콜 확률(/1000)", "로밍할 때 팀 콜을 보낼 확률", P, 700, 400, 900, 0, 1001),
    v("hazard_avoid_padding", "hazard_avoid_padding_6000", "위험 구역 비켜 걷기 여유", "적 투사체·장판을 이만큼 더 넓게 피해 걷는다. 회피 능력치가 낮은 선수는 효과가 작다", P, 6000, 3000, 12000, 0, 1000000),
    // ── 푸시·타워
    v("push_sustain_min_healthy", "push_sustain_min_healthy_3", "교전 뒤 푸시 시작 최소 인원(명)", "교전 뒤 8초 안에 새 푸시를 시작할 때 필요한 건강한 아군 수. 뒤의 다른 인원 조건 때문에 4 이하는 차이가 거의 없다", T, 3, 2, 4, 0, 127),
    v("push_sustain_margin", "push_sustain_margin_2", "교전 뒤 푸시 시작 인원 여유(명)", "교전 뒤 8초 안에 새 푸시를 시작하려면 건강한 아군이 적보다 이만큼 많아야 한다. 1은 원래 조건에 이미 들어 있다", T, 2, 1, 3, 0, 126),
    v("tower_chase_own_hp", "tower_chase_own_hp_35", "타워 아래 추격 내 체력(%)", "적 타워 아래까지 쫓으려면 필요한 내 체력(25/35/45 단계를 함께 이동)", P, 35, 25, 50, 10, 117),
    // ── 기지 방어
    v("base_defense_min_enemies", "base_defense_min_enemies_6", "기지 방어 출동 기준 적 미니언 수", "포탑이 모두 무너진 라인에서 넥서스 가까이 온 적 미니언이 이 수 이상이면 방어하러 간다", T, 6, 4, 8, 1, 127),
    v("base_defense_hold_retreat_hp", "base_defense_hold_retreat_hp_45", "기지 방어 중 후퇴 체력(%)", "기지 방어 중 미니언에게 큰 피해를 받을 자리이고 체력이 이 값 이하면 물러난다", P, 45, 30, 60, 0, 127),
    v("base_defense_hold_nexus_hp", "base_defense_hold_nexus_hp_50", "넥서스 위험 체력(%)", "넥서스가 위협받고 체력이 이 값 이하면 기지 방어에서 미니언 피해가 커도 버틴다. 회복 귀환 같은 다른 판단은 50 그대로", T, 50, 35, 70, 0, 127),
    v("base_defense_hold_dmg_ratio", "base_defense_hold_dmg_ratio_49", "기지 방어 미니언 피해율 문턱(%)", "기지 방어 중 2초 동안 미니언에게 받을 피해율이 이 값을 넘으면 위험으로 본다. 다음 단계 조건 때문에 29 이상은 모두 같다", P, 49, 30, 70, 0, 127),
    // ── 궁극기
    v("lowhp_ult_bonus_low", "lowhp_ult_bonus_low_14", "도주 중 저체력 군중제어 가산점", "도망치는 중 체력 45% 미만일 때 군중제어 기술(스킬·궁극기) 점수에 더하는 값. 높을수록 그냥 도망치기보다 기술을 쓴다", P, 14, 5, 40, 0, 127),
    // BEGIN GENERATED EXTRAS — tools/gen_extras.py
    v("gank_call_rng","extra_c001","갱 콜 수락 난수","음수 평가일 때만 적용합니다. 높을수록 콜 수락이 까다롭습니다.",P,350,250,450,0,999),
    v("pickoff_hp_bands","extra_c002","픽오프 저체력 기준(%)","하단 체력 문턱과 그보다20 높은 문턱을 함께 조절한다. 높을수록 체력 있는 적도 우선한다",P,61,41,71,1,80),
    v("structure_retreat_keep_hp","extra_c003","추격 중 포탑에 맞았을 때 후퇴 체력(%)","적을 쫓다 적 포탑에 맞았고 길이 포탑 사정권을 지나면, 체력이 이 값 이하일 때 잠시 물러난다. 높을수록 잘 물러난다",P,65,50,80,0,100),
    v("pre_gank_engage_profile","extra_c004","갱 도착 전 라인 선공","100이 원본 HP35·거리200000입니다. 높을수록 선공 허용 범위가 넓습니다. 0.7.2부터는 근처에 보이는 적 챔피언이 있을 때만 선공을 검토합니다(그 거리는 「솔로킬 시도 시 적 확인 거리」).",P,100,80,120,50,150),
    v("gank_travel_deadline","extra_c005","정글 갱 이동 기한","정글 갱 계획 기한과 이동시간 승인 문턱에 같은 초를 사용합니다.",P,12,8,16,2,30),
    v("battle_call_response_seconds","extra_c006","교전 콜 응답 시간","도착 승인과 진단의 응답 초를 맞춥니다. 별도 궁극기6초 하한은 유지합니다.",P,5,3,7,1,15),
    v("objective_power_allowance","extra_c007","오브젝트 전력 보완값","아군 체력 합에 더하는 여유. 높게 하면 진입 조건을 완화한다",P,200,100,300,0,1000),
    v("same_lane_gank_retry_seconds","extra_c008","같은 라인 갱 재시도","동일 라인 재시도를 막는 추가 초입니다. 통계용 기록 시간은 유지합니다.",P,10,6,14,1,30),
    v("chase_speed_ratio","extra_c010","추격 속도비 기준(10분율)","적 이동속도×10과 내 속도×이 값을 비교한다. 낮을수록 추격을 멈추기 쉽다. 일반 5대5 경기에는 쓰이지 않는다",P,9,7,11,1,20),
    v("tower_tank_hits","extra_c011","타워 탱킹 필요 체력(타)","적 포탑을 대신 맞아 줄 아군에게 필요한 체력(포탑 몇 타). 팀 공통이며 낮을수록 위험을 감수한다. 순서상 첫 후보에만 적용",T,3,2,4,1,10),
    v("kill_bonus_cap","extra_c013","처치 가산점 상한","적을 처치할 가능성의 가산점 상한. 높을수록 마무리 공격·기술을 우선한다",P,100,50,150,0,1000),
    v("ally_spacing_radius","extra_c014","아군 간격 반경","가까운 아군끼리 벌어지는 반경. 높을수록 서로 넓게 선다. 데스매치 전용(일반 5대5 경기에는 쓰이지 않는다)",P,48000,32000,60000,1000,65535),
    v("near_ally_link_margin","extra_c015","근접 아군 연계 거리 여유","아군의 공격 범위 밖에서도 연계할 수 있다고 인정하는 거리 여유. 높을수록 넓게 합을 맞춘다",P,30000,15000,60000,0,300000),
    v("line_defense_approval_profile","extra_c016","라인 공격 위험 허용치(%)","미니언·포탑을 칠 후보를 위험 때문에 버리는 기준(100 = 자리 위험 25·10, 받을 피해 35%). 높을수록 위험을 무릅쓰고 친다",P,100,80,120,50,150),
    v("base_battle_call_range","extra_c017","기지 교전 콜 허용 폭","1000범위 난수의 허용 폭입니다. 기지 지원 계획 자체의 승인과는 별개입니다.",P,650,500,800,1,1000),
    v("shared_battle_role_markers","extra_c018","집중 대상 따르기 최소 인원(명)","6초 안에 합류할 우리 편이 이 수 이상인 교전이면 그 교전의 집중 대상을 따른다. 높을수록 덜 따른다. 교전 제안·콜에 적용",P,2,1,3,1,5),
    v("basic_trade_hp_divisor","extra_c019","기본공격 교환 체력 여유(분모)","내 최대 체력/이 값과 적 기본공격 피해를 합친 체력보다 높을 때 안전하다고 본다. 높을수록 교환에 적극적",P,4,2,8,1,32),
    v("support_cap_multiplier","extra_c020","회복·실드 평가 허용 배수","회복은 잃은 체력+피해평가×이 값, 보호막은 피해평가×(이 값+1)까지 가치로 센다. 광역 보호막의 주변 아군 몫은 제외",P,2,1,4,0,10),
    v("buff_arrival_value_seconds","extra_c021","지원 기술 도착 가치 기간(초)","도착이 늦는 지원 기술의 가치를 이 기간에 걸쳐 낮춘다. 높을수록 늦게 도착해도 높게 평가한다",P,6,3,10,1,30),
    v("push_start_base","extra_c022","푸시 개시 기본 점수","능력치 보정 전 개시 문턱. 높게 하면 더 높은 점수를 요구한다",P,130,110,150,0,1000),
    v("other_lane_collapse_profile","extra_c023","다른 라인 붕괴 경계","다른 라인 지표와 단계 문턱을 함께 조절. 높게 하면 일찍 푸시를 포기한다",P,100,75,125,50,150),
    v("gank_reevaluate_seconds","extra_c025","갱·매복 결정 뒤 쉬는 시간(초)","정글러가 갱이나 매복을 결정한 뒤 이 초 동안 새 갱을 판단하지 않는다. 높을수록 뜸해진다",P,3,2,5,1,15),
    v("gank_candidate_margin","extra_c026","갱 후보 인원 여유","내 편 후보가 있을 때 허용할 상대 후보 수의 추가 여유입니다.",P,1,0,2,0,4),
    v("ambush_engage_radius","extra_c027","매복 교전 대상 반경","매복 판단 중 적 후보를 살피는 반경. 시야·최근 관측 조건은 유지한다",P,250000,200000,300000,1000,2000000),
    v("objective_lane_pressure_profile","extra_c028","오브젝트 전 라인 압박 문턱","라인 단계·지표·인원 문턱을 함께 조절. 높게 하면 라인 압박 후보를 넓힌다",P,100,50,150,50,150),
    v("roam_retry_extra_seconds","extra_c029","로밍 재시도 추가 간격","원래 기본 간격에 추가되는 초입니다. 마지막 로밍 기록이 없으면 우회합니다.",P,10,6,14,1,30),
    v("defense_leave_hp","extra_c030","방어 계획 이탈 체력(%)","일반 방어 체력 문턱. 위협 경로는 이 값보다10 낮게 적용한다",P,31,21,41,11,100),
    v("structure_skill_bonus","extra_c031","구조물 기술 후보 가산","넥서스 공격 계획에서 종류4 구조물을 겨냥한 첫 두 기술 점수에만 가산",P,3,0,6,0,100),
    v("epic_final_allies_min","extra_c032","에픽 최종 아군 인원 문턱","지터 보정 후 인원 문턱. 상대 보정값0 예외와 선승인은 유지한다",P,3,2,4,0,5),
    v("boss_respawn_wait_seconds","extra_c033","보스 리스폰 대기(초)","현재 보스가 없을 때 남은 리스폰 시간이 이 값보다 크면 계획을 끝낸다",P,15,10,20,1,120),
    v("boss_reposition_improvement","extra_c034","보스 사냥 재배치 개선값","국소 재배치에 요구하는 점수 개선. 기존 거리 대체조건은 유지한다",P,8,4,12,0,1000),
    v("boss_poke_damage_factor","extra_c035","보스 견제 예상피해 가중치","예상피해를 체력과 비교하는 배수. 높게 하면 위험 판단이 보수적이다",P,2,1,3,1,10),
    v("active_recall_slack_profile","extra_c037","마무리 전략의 귀환 종료 여유(초)","현재 전략0은 선택 초, 전략1은1초 적게 적용. 큰 계획 종료 판단만 조절한다",P,3,2,4,1,30),
    v("position_isolation_risk_percent","extra_c038","고립 위치 위험 가산(%)","주변에 아군이 없고 적이 2명 이상이면 적 1명당(4명까지) 챔피언 위험을 이 %만큼 더한다. 높을수록 혼자 들어가지 않는다",P,25,10,50,0,1000),
    v("front_ally_risk_scale","extra_c039","튼튼한 아군보다 앞섬 억제(%)","튼튼한 아군보다 앞선 위치의 위험 점수와 두 상한을 함께 비례 조정한다. 높을수록 아군 뒤에 서려 한다",P,100,50,150,1,400),
    v("battle_skill_safety_margin","extra_c040","교전 기술 안전 거리 여유","적 사거리에 더하는 안전 여유. 높을수록 더 떨어진 위치를 안전하다고 본다. 일반 5대5 경기에는 쓰이지 않는다",P,50000,25000,75000,0,300000),
    v("wave_position_risk_divisor","extra_c041","라인 웨이브 위치 위험 할인(분모)","원래 웨이브 안전 조건을 만족할 때 위치 위험을 이 값으로 나눈다. 높을수록 웨이브 주변을 덜 위험하게 본다",P,4,2,8,1,32),
    v("movement_skill_period_margin","extra_c042","이동기 절약 보행 여유(틱)","이동기 절약 모드에서 걷는 시간과 기술 주기를 비교할 때 더하는 여유. 높을수록 이동기를 아낀다. 맴돌기 등 일부 이동은 제외",P,60,30,90,0,600),
    v("early_gank_target_min_hp","extra_c044","조기 갱 대상 최소 체력","조기 갱 대상으로 검토할 적의 최소 체력입니다. 저체력 선호 설정과 다릅니다.",P,40,30,50,1,90),
    v("camp_time_margin_profile","extra_c045","캠프 경쟁 시간 여유(%)","생존 여유1초와 상대 처리시간 비교의 반초 여유를 같은 비율로 조절한다",P,100,50,150,0,300),
    v("battle_risk_retreat_threshold","extra_c048","교전 위험 후퇴 문턱","적 스킬·투사체·돌진에 걸려 있고 위치 위험이 이 값 이상이면 후퇴를 후보에 넣는다. 관련 문턱도 함께 움직인다. 낮을수록 잘 물러난다",P,10,5,15,1,50),
    v("unseen_self_risk_divisor","extra_c049","시야 밖 자기 위험 할인(분모)","적 시야에 잡히지 않을 때 기본 자기위험을 이 값으로 나눈다. 다른 위험 가산은 유지하며 높을수록 은폐를 믿는다",P,3,2,6,1,32),
    v("battle_reposition_distance_scale","extra_c050","교전 재배치 거리 폭(%)","교전 중 재배치·카이팅 거리와 포탑 평타 사거리 여유를 함께 조정한다. 높으면 적에게 조금 더 붙고, 막히면 더 멀리서 자리를 찾는다",P,100,75,125,25,200),
    v("candidate_move_budget_margin","extra_c051","챔피언 공격 후보 이동 여유(틱)","교전 밖(라인·정글·보스·넥서스)에서 적 챔피언을 노릴 때 사거리에 더하는 이동 여유. 높을수록 먼 적도 노린다",P,30,15,60,0,120),
    v("forward_damage_credit_percent","extra_c052","전진 후 피해 상쇄 비율(%)","이동 후 위험에서 현재 위치 피해의 이 비율을 빼고0하한을 적용한다. 높을수록 전진을 덜 위험하게 본다",P,50,25,75,0,100),
    v("battle_move_candidate_penalty","extra_c053","압박 유지 시 도망 감점","거들 대상 곁에 적이 있고 내가 안전할 때 도망·귀환 후보에서 빼는 점수. 높을수록 물러나지 않고 버틴다",P,6,0,12,0,1000),
    v("lane_attack_move_allowance","extra_c054","라인 공격 접근 여유(틱)","라인 미니언·구조물을 칠 때 사거리에 더하는 이동 여유. 높을수록 먼 대상도 검토한다",P,30,20,40,0,60),
    v("lane_defense_support_margin","extra_c055","라인 방어 지원 인원 여유","주변 다른 아군 수에 더할 지원 여유입니다. 원래 능력 조건도 필요합니다.",P,1,0,2,0,4),
    v("lane_exchange_advantage_ticks","extra_c056","라인 교환 시간 우위","최종 라인 후보의 교환 시간차 문턱입니다. 업데이트 틱이며 별도 인원 우회는 유지합니다.",P,61,41,81,1,180),
    v("lane_superiority_profile","extra_c057","라인 수적우세 확정 신중도","낮게:담당HP20·1명, 기본:25·2명, 높게:30·3명. 팀 공통 정책입니다.",T,1,0,2,0,2),
    v("lane_call_travel_seconds","extra_c058","라인 갱 콜 이동 상한","콜 응답 이동비용 상한과 수락 계획 기한에 같은 초를 적용합니다.",P,10,6,14,2,30),
    v("mia_notice_caution_profile","extra_c060","실종 알림 신중도","실종 알림을 띄우는 조건만 바꾼다(100 = 대기 1·4초, 체력 60%, 거리 170000). 높을수록 까다롭다. 동료의 판단은 바꾸지 않는다",P,100,80,120,50,150),
    v("relay_undying_limit_centiseconds","extra_c061","운영 신호 불사 잔여 허용","100이 원본1초입니다. 0.01초 단위의 불사 잔여 문턱이며 통과 후 기존 계획을 해제합니다.",P,100,50,150,1,300),
    v("gank_lane_tower_wave_profile","extra_d001","갱 후보 포탑·웨이브 경계","포탑 반경·미니언 차이·웨이브 위치 필터를 함께 조절. 높게 하면 보수적이다",P,100,75,125,50,150),
    v("lane_wait_front_minion_distance","extra_d002","선두 아군 미니언 대기 거리","선두 아군 미니언이 우리 포탑에서 이 거리 안이면 포탑 곁에서, 아니면 그 미니언보다 이 거리만큼 뒤에서 기다린다",P,180000,150000,210000,50000,300000),
    v("support_roam_adc_min_hp","extra_d004","서폿 로밍 원딜 최소 체력","알려진 서폿 로밍 세 경로에서 같은 팀 원딜 체력을 검사하는 문턱입니다.",P,51,40,65,1,95),
    v("purchase_recall_walk_ticks","extra_n001","구매 귀환 최소 도보시간(틱)","기지까지 걸어가는 시간이 이 값보다 짧으면 구매 귀환을 하지 않는다. 높을수록 멀리 있을 때만 귀환한다",P,211,150,300,1,10000),
    v("purchase_recall_threat_radius","extra_n002","구매 귀환 위협 반경","이 반경 안에 적 챔피언·미니언·포탑 등이 하나라도 있으면 구매 귀환을 하지 않는다. 높을수록 덜 귀환한다",P,200000,100000,300000,0,1000000),
    v("purchase_recall_threat_memory","extra_n003","구매 귀환 위협 기억","구매 귀환 판단에서 최근 본 적을 위협으로 기억하는 기간입니다. 높을수록 오래 기억합니다. 초 단위가 아닙니다.",P,120,60,180,0,10000),
    v("same_lane_partner_radius","extra_n004","같은라인 동료 근접반경","같은 라인 동료가 이 거리 안이면 웨이브를 붙잡고 기다리지 않는다. 높을수록 덜 기다린다. 거리 밖의 별도 조건은 그대로다",P,150000,100000,200000,0,700000),
    v("support_zero_net_penalty","extra_n005","지원 기술 평가 0 감점","아군·자신에게 쓰는 회복·보호막·버프 기술의 평가(기술 비용을 뺀 값)가 정확히 0일 때 주는 감점. 높을수록 덜 쓴다",P,10,0,30,0,1000),
    v("camp_deadline_arrival_seconds","extra_n006","캠프 재생성 전 갱·귀환 검토 여유(초)","다음 캠프에 재생성보다 이 초 넘게 일찍 도착할 때만 그사이 갱·구매 귀환을 검토한다. 높을수록 덜 검토한다",P,1,0,3,0,10),
    v("enemy_base_allies_radius","extra_n007","넥서스 끝내기 아군 집결 반경","살아 있는 아군 전원이 적 넥서스에서 이 거리 안이어야 끝내기 공격을 허용한다. 높을수록 쉽게 허용. 공격적 마무리 전술은 제외",P,600000,450000,700000,1000,2000000),
    v("ambush_wait_accept_radius","extra_n008","강 건너 매복 경유 거리","매복할 수풀이 강 건너편일 때 경유점(모르가드·세르펜 자리)에 이 거리 안으로 오면 바로 수풀로 간다. 높을수록 일찍 질러간다",P,100000,50000,150000,1000,300000),
    v("tracked_target_chase_bonus","extra_n009","등록 추적대상의 사거리 밖 추격 가산","등록된 추적 대상이 사거리 밖에 있을 때 추격 후보에 더하는 점수입니다. 높을수록 그 대상을 더 쫓습니다.",P,10,0,20,0,1000),
    v("champion_ultimate_waste_penalty","extra_n010","챔피언 대상 궁극기 낭비 억제 감점","원래 낭비 검사 조건을 만족한 챔피언 대상 궁극기 후보의 감점입니다. 높을수록 해당 궁극기 사용을 아낍니다. 실제 위력은 유지합니다.",P,30,0,60,0,1000),
    v("lane_opportunity_enemy_limit","extra_n011","라인 기회 상대 수 상한","라인 기회를 검토할 때 허용하는 상대 인원 상한입니다. 높을수록 상대가 더 많아도 기회 후보를 검토합니다.",P,1,0,2,0,5),
    v("skill_arrival_prediction_margin","extra_n012","투사체 도달 예측 단축(틱)","투사체 비행 시간 예측에서 빼는 값(틱). 높을수록 멀리서 일찍 쏜다. 적 챔피언에게 쏘는 판단에는 거의 쓰이지 않는다",P,10,5,15,0,1000),
    v("nexus_finish_start_range","extra_e001","넥서스 직행 거리(%)","집중 공성 중 적 넥서스·넥서스 포탑이 이 거리 안이면 싸움을 접고 넥서스로 간다(100 = 25만). 높을수록 멀리서도 직행한다",P,100,70,150,25,400),
    v("weak_team_finish_caution","extra_e002","판단력 낮을 때 마무리 신중도(%)","팀 평균 판단력이 낮을수록 넥서스 마무리 시작을 더 조심한다(100 = 원본 · 최대 1.9배). 0이면 판단력과 상관없이 시도한다",T,100,50,130,0,130),
    v("enemy_approach_engage_range","extra_e003","적 접근 시 교전 요청 거리(%)","체력 60% 이상일 때 보이는 적이 이 거리 안이면 교전을 검토한다(100 = 25만). 낮추면 덜 건다. 100 위는 뒤의 거리 조건 때문에 효과가 없다",P,100,75,125,25,400),
    v("pre_gank_enemy_check_range","extra_e004","솔로킬 시도 시 적 확인 거리(%)","전투 중이 아닐 때 보이는 적이 이 거리 안에 있어야 솔로킬을 검토한다(100 = 25만). 낮추면 덜 시도한다. 100 위는 효과가 없다",P,100,75,150,25,400),
    // END GENERATED EXTRAS
];

pub const N: usize = 104;
const _: () = assert!(TACTICS.len() == N);

/// 단계: 0 기본(원본) · 1 낮게(켬·끔이면 끔) · 2 높게.
pub const LV_BASE: u8 = 0;
pub const LV_LOW: u8 = 1;
pub const LV_HIGH: u8 = 2;
/// 칸: 0..4 = 포지션(탑·정글·미드·원딜·서폿), 5 = 팀.
pub const COL_TEAM: usize = 5;
pub const POS_NAMES: [&str; 5] = ["탑", "정글", "미드", "원딜", "서폿"];

static LEVEL: [[AtomicU8; 6]; N] = [const { [const { AtomicU8::new(0) }; 6] }; N];
/// [전술][0 = 낮게, 1 = 높게]. i64::MIN = 아직 안 읽음(정의 기본값 사용).
static VALUE: [[AtomicI64; 2]; N] = [const { [const { AtomicI64::new(i64::MIN) }; 2] }; N];
static STATE_VER: AtomicU32 = AtomicU32::new(0);
/// 단계 값(설정 파일)이 바뀔 때마다 증가 — 적용 장치가 값 반영 시점을 안다.
static VALUES_VER: AtomicU32 = AtomicU32::new(0);

/// 전술별 적용 상태(적용 장치가 쓴다 · UI 가 표시).
pub const ST_UNKNOWN: u8 = 0;
pub const ST_ACTIVE: u8 = 1;
pub const ST_UNSUPPORTED: u8 = 2;
pub const ST_FAILED: u8 = 3;
static STATUS: [AtomicU8; N] = [const { AtomicU8::new(ST_UNKNOWN) }; N];
pub fn set_status(t: usize, st: u8) { if t < N { STATUS[t].store(st, Relaxed); } }
pub fn status(t: usize) -> u8 { if t < N { STATUS[t].load(Relaxed) } else { ST_UNKNOWN } }
pub fn index_of_knob(knob: &str) -> Option<usize> { TACTICS.iter().position(|d| d.knob == knob) }

/// 계약 명단 athlete_id(최대 32). ID 0도 유효하므로 빈 슬롯은 u64::MAX.
static MY_ATHLETES: [AtomicU64; 32] = [const { AtomicU64::new(u64::MAX) }; 32];
/// 적용 스위치 — 세이브 설정 로드 + 내 팀 확보 뒤 켠다.
static ARMED: AtomicBool = AtomicBool::new(false);

pub fn index_of(id: &str) -> Option<usize> { TACTICS.iter().position(|d| d.id == id) }

pub fn level(t: usize, col: usize) -> u8 { if t < N && col < 6 { LEVEL[t][col].load(Relaxed) } else { 0 } }
/// 빼 둔 전술(2026-10-29 유저 지시). 카탈로그·세이브 형식은 그대로 두고 단계만 항상 0(원본)으로 둔다.
/// 0.7.0 의 새 로밍 경로(코드 4 = Player+0x2c5)가 라인 로밍이 아니라 「상대 정글 침투 선호」 특성으로 밝혀져,
/// 라인 로밍용 값이 상대 정글 침투 결정에 섞이는 것을 막으려고 세 전술을 통째로 뺐다.
pub const RETIRED: &[&str] = &["liner_roam_self_hp", "liner_roam_call_chance", "roam_retry_extra_seconds"];
pub fn retired(t: usize) -> bool { t < N && RETIRED.contains(&TACTICS[t].id) }
pub fn set_level(t: usize, col: usize, lv: u8) {
    if t >= N || col >= 6 { return; }
    let max = if retired(t) { LV_BASE } else if TACTICS[t].kind == Kind::Toggle { LV_LOW } else { LV_HIGH };
    LEVEL[t][col].store(lv.min(max), Relaxed);
    STATE_VER.fetch_add(1, Relaxed);
}
pub fn state_version() -> u32 { STATE_VER.load(Relaxed) }
pub fn clear_levels() { for row in LEVEL.iter() { for c in row { c.store(0, Relaxed); } } STATE_VER.fetch_add(1, Relaxed); }

/// 단계 값(낮게/높게). 범위 밖이면 거절(false).
pub fn set_value(t: usize, lv: u8, value: i64) -> bool {
    if t >= N || !(lv == LV_LOW || lv == LV_HIGH) { return false; }
    let d = &TACTICS[t];
    if value < d.min || value > d.max { return false; }
    VALUE[t][(lv - 1) as usize].store(value, Relaxed);
    VALUES_VER.fetch_add(1, Relaxed);
    true
}
pub fn values_version() -> u32 { VALUES_VER.load(Relaxed) }
/// 설정 파일 다시 읽기 전 — 전부 정의 기본값으로.
pub fn reset_values() {
    for row in VALUE.iter() { for c in row { c.store(i64::MIN, Relaxed); } }
    VALUES_VER.fetch_add(1, Relaxed);
}
/// 단계의 실제 값(기본 = 원본값).
pub fn value_of(t: usize, lv: u8) -> i64 {
    let d = &TACTICS[t];
    match lv {
        LV_LOW | LV_HIGH => {
            if d.kind == Kind::Toggle { return if lv == LV_LOW { 0 } else { 1 }; }
            let v = VALUE[t][(lv - 1) as usize].load(Relaxed);
            if v == i64::MIN { if lv == LV_LOW { d.low } else { d.high } } else { v }
        }
        _ => d.default,
    }
}

pub fn set_my_athletes(ids: &[u64]) {
    for (i, slot) in MY_ATHLETES.iter().enumerate() { slot.store(ids.get(i).copied().unwrap_or(u64::MAX), Relaxed); }
}
pub fn my_athletes() -> Vec<u64> { MY_ATHLETES.iter().map(|a| a.load(Relaxed)).filter(|a| *a != u64::MAX).collect() }
#[inline]
pub fn is_my_athlete(ath: u64) -> bool { ath != u64::MAX && MY_ATHLETES.iter().any(|a| a.load(Relaxed) == ath) }
pub fn set_armed(on: bool) { ARMED.store(on, Relaxed); }
pub fn armed() -> bool { ARMED.load(Relaxed) }

/// ★적용 장치용 단일 진입점. 판단 중인 선수(athlete_id, 포지션 0..4)에게 쓸 값.
/// None = 원본 그대로(내 팀이 아님 · 단계 기본 · 비무장 · 범위 밖 입력).
#[inline]
pub fn value_for(t: usize, athlete_id: u64, pos: usize) -> Option<i64> {
    let lv = level_for(t, athlete_id, pos);
    if lv == LV_BASE { None } else { Some(value_of(t, lv)) }
}
/// 같은 판정의 단계 번호판(0 = 원본 그대로). 단계별로 미리 만든 코드를 고르는 쪽이 쓴다.
#[inline]
pub fn level_for(t: usize, athlete_id: u64, pos: usize) -> u8 {
    if t >= N || !armed() || !is_my_athlete(athlete_id) || pos > 4 { return LV_BASE; }
    let col = if TACTICS[t].scope == Scope::Team { COL_TEAM } else { pos };
    LEVEL[t][col].load(Relaxed)
}

// ───────── 세이브 저장 형식: 줄마다 "<전술 id> <칸6개>" (칸 = 0/1/2, 순서 탑 정글 미드 원딜 서폿 팀) ─────────
pub fn state_text() -> String {
    let mut s = String::from("v1\n");
    for (t, d) in TACTICS.iter().enumerate() {
        let cols: Vec<String> = (0..6).map(|c| level(t, c).to_string()).collect();
        if cols.iter().all(|c| c == "0") { continue; }
        s.push_str(&format!("{} {}\n", d.id, cols.join(" ")));
    }
    s
}
/// 반환 = 읽은 전술 수. 모르는 id·잘못된 줄은 건너뛴다(다음 버전 호환).
pub fn apply_state_text(txt: &str) -> usize {
    let _update=config_update();
    clear_levels();
    let mut n = 0;
    for line in txt.lines() {
        let mut it = line.split_whitespace();
        let Some(id) = it.next() else { continue };
        let Some(t) = index_of(id) else { continue };
        let Ok(cols): Result<Vec<u8>,_> = it.map(|x| x.parse::<u8>()).collect() else { continue; };
        if cols.len() != 6 { continue; }
        for (c, lv) in cols.iter().enumerate() { set_level(t, c, *lv); }
        n += 1;
    }
    n
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]fn all_100_saved_ids_roundtrip_and_config_revision_rejects_partial_update(){
        clear_levels();let revision=config_revision().unwrap();
        {let _update=config_update();assert!(config_revision().is_none());
         for t in 0..N{for col in 0..6{set_level(t,col,LV_LOW);}}}
        assert!(!config_unchanged(revision));
        let txt=state_text();assert_eq!(apply_state_text(&txt),N-RETIRED.len()); // 뺀 전술은 0 이라 저장 줄이 없다
        for t in 0..N{for col in 0..6{assert_eq!(level(t,col),if retired(t){LV_BASE}else{LV_LOW});}}
        clear_levels();
    }
    #[test]
    fn table_is_consistent() {
        for d in TACTICS {
            assert!(d.min <= d.default && d.default <= d.max, "{}", d.id);
            assert!(d.min <= d.low && d.low <= d.max && d.min <= d.high && d.high <= d.max, "{}", d.id);
        }
        let mut ids: Vec<&str> = TACTICS.iter().map(|d| d.id).collect();
        ids.sort(); ids.dedup(); assert_eq!(ids.len(), N);
    }
    #[test]
    fn malformed_saved_line_is_rejected_as_a_whole() {
        assert_eq!(apply_state_text("combat_retreat_hp 1 0 0 0 0 0 extra\n"),0);
        assert_eq!(apply_state_text("combat_retreat_hp 1 0 0 0 0 0\n"),1);
        assert_eq!(level(0,0),1);
        clear_levels();
    }
    #[test]
    fn retired_tactics_stay_at_base_even_from_old_saves() {
        clear_levels();
        set_my_athletes(&[101]);
        set_armed(true);
        assert_eq!(apply_state_text("v1\nliner_roam_self_hp 2 2 2 2 2 0\nliner_roam_call_chance 1 1 1 1 1 0\nroam_retry_extra_seconds 2 0 2 2 2 0\ncombat_retreat_hp 2 0 0 0 0 0\n"), 4);
        for id in RETIRED { let t = index_of(id).unwrap(); assert!(retired(t)); for c in 0..6 { assert_eq!(level(t, c), LV_BASE); } assert_eq!(value_for(t, 101, 0), None); }
        assert_eq!(level(index_of("combat_retreat_hp").unwrap(), 0), LV_HIGH); // 나머지는 그대로
        assert!(!state_text().contains("liner_roam_self_hp"));
        set_armed(false);
        clear_levels();
    }
    #[test]
    fn value_for_respects_team_level_and_armed() {
        clear_levels();
        set_my_athletes(&[101, 102]);
        let t = index_of("combat_retreat_hp").unwrap();
        set_level(t, 0, LV_HIGH);
        set_armed(false);
        assert_eq!(value_for(t, 101, 0), None);
        set_armed(true);
        assert_eq!(value_for(t, 101, 0), Some(50));
        assert_eq!(value_for(t, 101, 1), None); // 정글 칸은 기본
        assert_eq!(value_for(t, 999, 0), None); // 남의 팀
        set_my_athletes(&[0,101,102]); // 실제 세이브의 첫 선수 ID도 0이다.
        assert_eq!(value_for(t,0,0),Some(50));
        assert_eq!(my_athletes(),vec![0,101,102]);
        assert!(set_value(t, LV_HIGH, 60));
        assert_eq!(value_for(t, 102, 0), Some(60));
        assert!(!set_value(t, LV_HIGH, 500)); // 범위 밖
        let team = index_of("push_sustain_min_healthy").unwrap();
        set_level(team, COL_TEAM, LV_LOW);
        assert_eq!(value_for(team, 101, 3), Some(2));
        let tog = index_of("bot_duo_partner").unwrap();
        set_level(tog, 4, LV_HIGH); // 켬·끔은 낮게(끔)까지만
        assert_eq!(level(tog, 4), LV_LOW);
        assert_eq!(value_for(tog, 101, 4), Some(0));
        let txt = state_text();
        set_level(t, 0, LV_BASE);
        assert_eq!(apply_state_text(&txt), 3);
        assert_eq!(level(t, 0), LV_HIGH);
        set_armed(false);
        clear_levels();
    }
}
