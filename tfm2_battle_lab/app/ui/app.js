'use strict';
// ── 언어(한국어/English) ── 위쪽에서 고르고 localStorage 에 기억한다(바꾸면 다시 읽는다)
const LANG = (() => { try { const v = localStorage.getItem('bl_lang'); if (v === 'ko' || v === 'en') return v; } catch {} return 'en'; })();  // 기본 = 영어(유저 지시 2026-11-27 · 창작마당 공개) · 오른쪽 위 칸에서 한국어로 바꾸면 bl_lang 에 기억
const EN = {"리그": "League", "솔랭": "Solo Rank", "연습 경기": "Practice", "기타": "Other", "팀 ": "Team ", "<tr><th class=\"ck\">키</th><th class=\"cl\">리그</th><th>블루</th><th>레드</th><th class=\"cr\">결과</th></tr>": "<tr><th class=\"ck\">Key</th><th class=\"cl\">League</th><th>Blue</th><th>Red</th><th class=\"cr\">Result</th></tr>", "버전": "Version","연도": "Year","전체": "All", "최근 경기부터": "newest first", "더 보기": "Show more", "남은 ": "remaining ", "탑": "Top","정글": "Jungle", "미드": "Mid", "원딜": "Bot", "서폿": "Support", "적성 탑": "Aptitude Top", "적성 정글": "Aptitude Jungle", "적성 미드": "Aptitude Mid", "적성 원딜": "Aptitude Bot", "적성 서폿": "Aptitude Support", "초": "s", "경기를 계산하는 중입니다": "Simulating the match", " (판단 기록 포함 — 오래 걸립니다)": " (with decision log — takes longer)", "실패: ": "Failed: ", "계산 중…": "Simulating…", "게임과 세이브를 읽는 중": "Loading the game and the save", "경기 시간 ": "Match time ", "약 ": "~", " — 예상보다 길어지고 있습니다": " — taking longer than expected", "화면을 준비하는 중입니다": "Preparing the view", "<span class=\"bad\">실패: ": "<span class=\"bad\">Failed: ", "모드 ": "Mod ", " · 스킬 효과 ": " · skill effects ", " · 패시브 ": " · passives ", " · 경기 훅(난수가 달라짐)": " · match hook (changes RNG)", "이전 버전": "Legacy", "모드들이 바꾼 게임 코드 ": "Game code spots patched by mods: ", "군데": "", "멈춤": "Pause", "그림": "art", "aseprite 파일이 아님": "not an aseprite file", "왼쪽에서 경기를 고르거나 새로 실행하세요": "Pick a match on the left or run a new one", " (틱 ": " (tick ", "<span class=\"b\">블루 ": "<span class=\"b\">Blue ", " 레드</span>": " Red</span>", " · 중간에 멈춤": " · stopped early", " · 블루 승": " · Blue wins", " · 레드 승": " · Red wins", " · 승자 판정 안 됨": " · no winner decided", "체력 ": "HP ", "사망": "Dead", "<span class=\"dim\">「판단 기록」을 켜고 실행하면 여기에 이유가 나옵니다.</span>": "<span class=\"dim\">Run with \"Record decisions\" on to see the reasons here.</span>", "시각": "Time", "틱 ": "Tick ", "목표": "Goal", "큰 플랜": "Main plan", "대상": "Target", "서브플랜": "Subplan", "세부": "Detail", "행동": "Action", "배정 사유": "Assignment", "점수 항목": "Score terms", "명령 ": "Command ", "시전 실패 선수 ": "Cast failed, player ", "평타": "Attack", "스킬1": "Skill 1", "스킬2": "Skill 2", "궁": "Ult", " 사용": " cast", "없음": "None", "리플레이가 들어 있는 세이브를 고르세요": "Pick a save that contains replays", "재생 가능한 리플레이 ": "Playable replays: ", "건 · 누르면 다시 계산해 보여 줍니다": " · click one to re-simulate and show it", "판단 기록 켜기(파일이 커집니다)": "Record decisions (bigger files)", "<tr><th>키</th><th>블루</th><th>결과</th><th>버전</th></tr>": "<tr><th>Key</th><th>Blue</th><th>Result</th><th>Version</th></tr>", "레드 ": "Red ", "블루 ": "Blue ", "팀 전술 바꾸기 (": "Change team tactics (", "기록 그대로": "As recorded", " [모드]": " [mod]", "키 ": "Key ", "바탕 경기": "Base match", "비우면 기록 시드": "Empty = recorded seed", "시드": "Seed", "정식 경기": "Normal match", "데스매치": "Deathmatch", "모드": "Mode", "끝 틱(0=끝까지)": "End tick (0 = full)", "판단 기록 켜기": "Record decisions", "레드 팀": "Red team", "블루 팀": "Blue team", "능력치·빌드": "Stats & build", "목표 빌드": "Target build", "경기 시작": "Start match", "저장 이름": "Preset name", "설정 저장": "Save preset", "저장한 설정": "Saved presets", "고르기…": "Choose…", "[개발]": "[dev]", "궁 주인": "Ult owner", "원본으로 보기": "Show original", "◀ 이전": "◀ Prev", "다음 ▶": "Next ▶", " 궁만 되풀이": " Loop the ult only", "직접 짜기": "Custom setup", "원본 ": "Original ", " · 동작: ": " · anims: ", "배치 끝내기": "Done placing", "지도에서 끌어서 배치": "Place on the map", "시전자": "Caster", "레벨": "Level", "스킬2는 3레벨, 궁은 5레벨부터 효과가 납니다": "Skill 2 works from level 3, the ult from level 5", "위치": "Position", "기본 거리": "Default distance", "기본 배치로": "Reset layout", "시전자는 지도 가운데, 허수아비는 기본 거리만큼 옆에 세웁니다": "Puts the caster at the map center and the dummies at the default distance", "좌표는 0~960000, 스킬 사거리(range)와 같은 단위입니다": "Coordinates are 0–960000, the same unit as skill range", "허수아비(AI 없음)": "Dummies (no AI)", "적": "Enemy", "아군": "Ally", "0 = 가득": "0 = full", "번": "", "번 ": " ", "체력": "HP", "빼기": "Remove", "허수아비 추가": "Add dummy", "시전 순서": "Cast order", "대상 허수아비": "Target dummy", "적 ": "enemy ", "아군 ": "ally ", "자동 = 스킬 방식에 맞춰 · 대상 = 그 허수아비 · 좌표 = 그 허수아비가 서 있는 자리 · 방향 = 그 허수아비 쪽": "Auto = depends on the skill · Target = that dummy · Position = where that dummy stands · Direction = toward that dummy", "자동": "Auto", "좌표": "Position", "방향": "Direction", "틱": "Tick", "시전 추가": "Add cast", "끝 틱": "End tick", "미니언 나오기": "Spawn minions", "실행": "Run", "거리 ": "dist ", " ⚠ 적 타워 사거리 안": " ⚠ inside enemy tower range", "재생": "Play", "캐릭터를 끌어서 옮기세요 · 빈 곳 끌기 = 지도 이동 · 휠 = 확대": "Drag characters to move them · drag empty space = pan · wheel = zoom", " (대상 없음)": " (no target)", "사거리를 읽지 못했습니다: ": "Could not read the range: ", " ⚠타워": " ⚠tower", "아군 4 · 적 5 까지": "Up to 4 allies and 5 enemies", "<span class=\"bad\">시전 실패 ": "<span class=\"bad\">Cast failures: ", "건</span>": "</span>", "\">시전 ": "\">Casts ", "건": "", " (안 나간 것은 사거리 밖이거나 시전 방식이 안 맞습니다 — 「자동」으로 해 보세요)": " (missing casts were out of range or used the wrong cast type — try \"Auto\")", " <span class=\"bad\">· 그 자리에 못 섬: ": " <span class=\"bad\">· could not stand there: ", " (벽 등)</span>": " (wall etc.)</span>", "mod.override_info 형식 오류": "mod.override_info format error", "모드 아이템 ": "Added ", "개를 빌드 목록에 추가했습니다": " mod items to the build list", "게임 규칙": "Game rules", "챔피언 수치": "Champion stats", "아이템": "Items", "이름 검색(예: archer, kill_gold)": "Search by name (e.g. archer, kill_gold)", "검색하면 숫자를 고칠 수 있습니다. 고친 값은 모든 실행에 적용됩니다.": "Search to edit numbers. Edited values apply to every run.", "원래 ": "was ", "이 종류 되돌리기": "Reset this kind", "세이브의 설정 위에 고친 숫자만 덮습니다. 목록 안의 항목 수를 늘리는 편집은 아직 안 됩니다.": "Only the edited numbers are laid over the save's settings. Adding entries to lists is not supported yet.", "세이브가 없습니다 — 게임에서 경기를 한 번 이상 하고 저장한 뒤 「새로 고침」을 누르세요": "No saves found — play and save at least one match in the game, then press \"Reload\"", "세이브 읽는 중…": "Reading the save…", "세이브를 읽는 중입니다": "Reading the save", "<span class=\"bad\">이 세이브에는 재생할 수 있는 리플레이가 없습니다. 다른 세이브를 고르세요.</span>": "<span class=\"bad\">This save has no playable replays. Pick another save.</span>", "추가/채움": "Add/fill", "교체": "Replace", "챔피언 ": "Champion ", "설정 교체: ": "Setting override: ", "(병합)": " (merge)", "(통째)": " (whole)", " — 미지원": " — unsupported", "공식 API": "Official API", "예전 방식(클래식 API) dll 이라 켤 수 없습니다": "Old-style (classic API) dll — cannot be enabled", "게임 함수 후킹": "Hooks game functions", "예) D:\\SteamLibrary\\steamapps\\common\\Teamfight Manager2": "e.g. D:\\SteamLibrary\\steamapps\\common\\Teamfight Manager2", "이 폴더 쓰기": "Use this folder", " 확인 중…": " Checking…", " 다시 시작하는 중… 첫 실행이면 그림을 꺼내느라 몇 초 걸립니다": " Restarting… the first run takes a few seconds to extract graphics", " 다시 켜지지 않았습니다. 프로그램을 직접 다시 실행해 주세요": " It did not come back. Please start the program again yourself", "게임 폴더": "Game folder", "지난번에 고른 모드 중 찾지 못한 것: ": "Mods selected last time but not found: ", "지난번에 고른 모드 ": "Restored ", "개를 켠 상태입니다": " mods selected last time", "TFM2 전투 실험실": "TFM2 Battle Lab", "전투 실험실": "Battle Lab", "세이브": "Save", "새로 고침": "Reload", "세이브·모드 목록 다시 읽기": "Reload saves and mods", "적용할 모드": "Mods", "리플레이": "Replay", "임의 경기": "Custom match", "스킬 실험실": "Skill lab", "규칙": "Rules", "1초 뒤로 (Shift+←)": "1 second back (Shift+←)", "◀ 1틱": "◀ 1 tick", "1틱 뒤로 (←)": "1 tick back (←)", "재생/멈춤 (스페이스)": "Play/pause (Space)", "1틱 ▶": "1 tick ▶", "1틱 앞으로 (→)": "1 tick forward (→)", "1초 앞으로 (Shift+→)": "1 second forward (Shift+→)", "1배": "1×", "2배": "2×", "4배": "4×", "8배": "8×", "1/4배": "1/4×", "1/10배": "1/10×", "틱 번호를 적고 Enter": "Type a tick number and press Enter", "선택 따라가기": "Follow selected", "사거리": "Range", "고른 선수(배치 화면에서는 시전자)의 사거리. 상대 챔피언의 중심이 원 안에 들어오면 닿습니다.": "Range of the selected player (the caster on the placement screen). It reaches when the center of an enemy champion is inside the circle.", "선수": "Players", "판단 이유": "Decision reasons", "선수를 고르고 「판단 기록」을 켠 경기에서 볼 수 있습니다.": "Select a player in a match run with \"Record decisions\".", "사건": "Events", "모두 끄기": "Turn all off", "닫기": "Close", "챔피언 파일은 세이브에 같은 이름이 있으면 「교체」를 골라야 지금 파일의 내용이 쓰입니다. dll 모드는 앱 폴더(modgame)의 사본으로 올리고, 시뮬레이터가 게임인 것처럼 환경을 꾸며 줍니다. 무엇이 바뀌었는지는 실행 뒤 「사건」에 나옵니다.": "If the save already has a champion with the same name, choose \"Replace\" to use the file's current content. dll mods are loaded from a copy in the app folder (modgame), and the simulator sets up the environment as if it were the game. What changed is listed under \"Events\" after a run."};
const T = s => LANG === 'en' && s in EN ? EN[s] : s;
const NO = n => LANG === 'en' ? '#' + n : n + '번'; // 허수아비 번호
// 서버(앱)가 한국어로 보내는 메시지
const SVP = [[/^게임을 찾지 못했습니다\((.*)\)\..*/, 'Game not found ($1). Enter the game folder path below (or write it on one line in "game_path.txt" in this program\'s folder).'],
  [/^이 프로그램은 게임 (\S+) 전용입니다\. 설치된 게임의 버전이 다릅니다\(exe 크기 (\d+), 필요한 크기 (\d+)\)\./, 'This program only works with game $1. The installed game is a different version (exe size $2, expected $3).'],
  [/^계산기\(lab_worker\.exe\)가 없습니다.*/, 'The simulator (lab_worker.exe) is missing. Check whether your antivirus removed it, and extract the zip again.'],
  [/^그림 준비가 안 됐습니다.*/, 'Graphics are not prepared (check that the program folder is writable).'],
  [/^경로가 비어 있습니다$/, 'The path is empty'], [/^그 폴더에 (\S+) 가 없습니다$/, 'There is no $1 in that folder'], [/^저장 실패: /, 'Could not save: '],
  [/^세이브를 찾을 수 없음/, 'Save not found'], [/^세이브 읽기 실패: /, 'Could not read the save: '], [/^모드 복사 실패 /, 'Could not copy mod '],
  [/^게임 사본 준비 실패: /, 'Could not prepare the game copy: '], [/^시나리오 쓰기 실패/, 'Could not write the scenario'], [/^허용되지 않은 /, 'Not allowed: ']];
const SV = m => { if (LANG !== 'en') return m; for (const [r, t] of SVP) if (r.test(m)) return m.replace(r, t); return m; };
// index.html 의 고정 글자·title·placeholder
function i18nStatic() {
  document.documentElement.lang = LANG; document.title = T(document.title.trim());
  const w = document.createTreeWalker(document.body, NodeFilter.SHOW_TEXT); for (let n; (n = w.nextNode());) { const t = n.nodeValue.trim(); if (t && t in EN && LANG === 'en') n.nodeValue = n.nodeValue.replace(t, EN[t]); }
  for (const e of document.querySelectorAll('[title],[placeholder]')) for (const a of ['title', 'placeholder']) { const v = e.getAttribute(a); if (v) e.setAttribute(a, T(v)); }
  const ls = document.getElementById('lang'); if (ls) { ls.value = LANG; ls.onchange = () => { try { localStorage.setItem('bl_lang', ls.value); } catch {} location.reload(); }; }
}
// 전투 실험실 화면. 서버(127.0.0.1) 외에 아무 데도 연결하지 않는다.
const $ = s => document.querySelector(s), ce = (t, p = {}, ...k) => { const e = Object.assign(document.createElement(t), p); for (const x of k) e.append(x); return e; };
const api = async (u, o) => { const r = await fetch(u, o); const t = await r.text(); if (!r.ok) throw new Error(t || r.status); return t; };
const J = async u => JSON.parse(await api(u));
const POS = [T('탑'), T('정글'), T('미드'), T('원딜'), T('서폿')];
const STATS = ['last_hit', 'skill_avoid', 'skill_hit', 'positioning', 'control_speed', 'concentration', 'mental', 'judgement', 'order', 'roaming', 'aggressive', 'ego', T('적성 탑'), T('적성 정글'), T('적성 미드'), T('적성 원딜'), T('적성 서폿')];
const KEYS = ['last_hit', 'skill_avoid', 'skill_hit', 'positioning', 'control_speed', 'concentration', 'mental', 'judgement', 'order', 'roaming', 'aggressive', 'ego', 'top', 'jungle', 'mid', 'bottom', 'support'];
const BUILTIN = 'android archer bard barrier_magician berserker bomber boomerang_hunter cavalry_knight chef circus_blade clown dancer dark_mage demon dokkaebi druid dual_blader enchanter executioner exorcist fighter gambler ghost guardian_spirit gunner hammerer hitman hunter ice_mage illusionist inquisitor jiangshi knight lancer lightning_mage magic_knight monk necromancer ninja ogre plague_doctor poison_dart_hunter pole_warrior priest prisoner pyromancer pythoness shadowmancer shield_bearer siege_breaker soldier spirit_caller swordman taoist vampire voodoo_shaman werewolf whip_master white_mage wind_mage'.split(' ');
const S = { save: null, ins: null, mods: [], sel: { champs: {}, dlls: {}, sets: {} }, names: {}, art: null, modItems: [] };

// ── 공통 ──────────────────────────────────────────────
const nm = k => (S.names[k] ? S.names[k] + ' (' + k + ')' : k);
const champList = () => {
  const set = new Set(BUILTIN);
  for (const c of (S.ins ? S.ins.data_champions : [])) set.add(c);
  for (const id of Object.keys(S.sel.champs)) set.add(id);
  return [...set].sort((a, b) => nm(a).localeCompare(nm(b), LANG));
};
const champSel = v => { const s = ce('select'); for (const c of champList()) s.append(ce('option', { value: c, textContent: nm(c), selected: c === v })); return s; };
function modLines() { const o = []; for (const [id, m] of Object.entries(S.sel.champs)) o.push('file ' + m.path + ' ' + m.mode); return o; }
function modDlls() { return Object.keys(S.sel.dlls).map(p => 'dll=' + p); }
// 기다리는 동안 화면 가운데에 안내를 띄우고, 다시 누르지 못하게 막는다
const B = { on: false, t: 0, iv: 0, note: '' };
function busy(text) { if (!B.on) { B.on = true; B.t = performance.now(); B.iv = setInterval(busySec, 200); document.body.classList.add('busy'); $('#busy').hidden = false; } $('#busyT').textContent = text; B.note = ''; $('#busyBar').hidden = true; busySec(); }
// 진행 막대: 0~1 (null = 모름 → 왔다 갔다)
function busyBar(v, note = '') { const b = $('#busyBar'); b.hidden = false; b.classList.toggle('wait', v === null); b.firstChild.style.width = v === null ? '' : (v * 100).toFixed(1) + '%'; B.note = note; busySec(); }
function busySec() { $('#busyS').textContent = Math.floor((performance.now() - B.t) / 1000) + T('초') + (B.note ? ' · ' + B.note : ''); }
function idle() { B.on = false; clearInterval(B.iv); document.body.classList.remove('busy'); $('#busy').hidden = true; }
async function startRun(lines, opt = {}) {
  if (B.on) return null; // 이미 계산 중
  busy(T('경기를 계산하는 중입니다') + (opt.trace ? T(' (판단 기록 포함 — 오래 걸립니다)') : ''));
  try { return await runOnce(lines, opt); } catch (e) { $('#msg').textContent = ''; $('#msg').append(ce('span', { className: 'bad', textContent: T('실패: ') + SV(e.message) })); return null; } finally { idle(); }
}
const mmss = t => { const s = Math.floor(t / 60); return Math.floor(s / 60) + ':' + String(s % 60).padStart(2, '0'); };
// 예상 길이(틱): 끝 틱을 정했으면 그 값, 아니면 바탕 경기의 기록 길이
const runExact = lines => lines.some(l => /^until [1-9]/.test(l));
function runTotal(lines) { const u = lines.map(l => l.match(/^until (\d+)/)).find(Boolean); if (u && +u[1]) return +u[1]; const k = lines.map(l => l.match(/^key (\d+)/)).find(Boolean); return (k && S.ins?.replays.find(r => r.key === +k[1])?.tick) || 0; }
async function runOnce(lines, opt) {
  $('#msg').textContent = T('계산 중…');
  const setLines = [...await modSettingLines(), ...await ruleLines(opt.rules)];
  const body = ['save=' + S.save, ...modDlls(), 'every=' + (opt.every || 1), ...(opt.trace ? ['trace=1'] : []), ...setLines, ...modLines(), ...lines].join('\n');
  const { id } = JSON.parse(await api('/api/run', { method: 'POST', body }));
  S.runMode = +((lines.find(l => /^mode \d/.test(l)) || '').split(' ')[1] || 0);
  const total = runTotal(lines); let seen = -1; busyBar(null, T('게임과 세이브를 읽는 중'));
  for (;;) {
    await new Promise(r => setTimeout(r, 250));
    const s = await J('/api/run/' + id);
    if (s.state === 'running' && s.tick >= 0 && s.tick >= seen) { seen = s.tick; const left = total && seen < total; busyBar(total ? Math.min(0.99, seen / total) : null, T('경기 시간 ') + mmss(seen) + (total ? (left ? ' / ' + (runExact(lines) ? '' : T('약 ')) + mmss(total) + ' (' + Math.floor(seen / total * 100) + '%)' : T(' — 예상보다 길어지고 있습니다')) : '')); }
    if (s.state === 'done') { $('#msg').textContent = ''; busy(T('화면을 준비하는 중입니다')); busyBar(1); S.modLog = modSummary(s.log); return loadRun(id); }
    if (s.state === 'fail') { $('#msg').innerHTML = T('<span class="bad">실패: ') + s.log.replace(/</g, '&lt;').split('\n').slice(-2).join(' / ') + '</span>'; return null; }
  }
}

// 모드 적용 요약(계산기 로그에서): 바뀐 게임 함수 수 · 경기 훅 · 오류
function modSummary(log) {
  const out = [], m = log.match(/LAB 바뀐 곳 (\d+) 군데/);
  if (!m && !/LAB 모드/.test(log)) return out;
  for (const l of log.split('\n')) {
    const r = l.match(/id="([^"]+)".*native_effect=(\d+) native_passive=(\d+).*hook=(\d+)/);
    if (r) out.push(T('모드 ') + r[1] + (r[2] !== '0' ? T(' · 스킬 효과 ') + r[2] : '') + (r[3] !== '0' ? T(' · 패시브 ') + r[3] : '') + (r[4] !== '0' ? T(' · 경기 훅(난수가 달라짐)') : ''));
    if (/오류|구식|LoadLibrary 실패/.test(l)) out.push('⚠ ' + l.replace('LAB ', '').replace('구식', T('이전 버전')));
  }
  if (m) out.push(T('모드들이 바꾼 게임 코드 ') + m[1] + T('군데'));
  return out;
}

// ── 기록 읽기 ─────────────────────────────────────────
const V = { run: null, t: 0, play: false, sel: 0, zoom: 1, cx: 640, cy: 640 };
async function loadRun(id) {
  P.on = false;
  const [nd, fb] = await Promise.all([api('/api/run/' + id + '/run.ndjson'), fetch('/api/run/' + id + '/frames.bin').then(r => r.arrayBuffer())]);
  const run = { head: null, ents: {}, plEnt: {}, casts: [], fx: [], an: {}, st: {}, cmds: [], rg: {}, traces: [], proj: [], end: null, fails: [] };
  for (const l of nd.split('\n')) {
    if (!l) continue; const o = JSON.parse(l);
    if (o.t === 'head') run.head = o; else if (o.t === 'ent') run.ents[o.id] = o; else if (o.t === 'pl') (run.plEnt[o.i] ||= []).push([o.ent]);
    else if (o.t === 'cast') run.casts.push(o); else if (o.t === 'fx') run.fx.push(o); else if (o.t === 'an') (run.an[o.e] ||= []).push([o.tick, o.n, o.k]); else if (o.t === 'st') (run.st[o.i] ||= []).push([o.tick, o.b, o.c]); else if (o.t === 'rg') (run.rg[o.i] ||= []).push([o.tick, o]); else if (o.t === 'cmd') run.cmds.push(o);
    else if (o.t === 'tr') run.traces.push(o); else if (o.t === 'projnames') run.proj = o.names; else if (o.t === 'end') run.end = o; else if (o.t === 'castfail') run.fails.push(o);
  }
  const dv = new DataView(fb); run.dv = dv; run.off = []; run.tick = [];
  let o = 4; while (o + 8 <= fb.byteLength) { run.off.push(o); run.tick.push(dv.getUint32(o, true)); const ne = dv.getUint16(o + 4, true), np = dv.getUint16(o + 6, true); o += 8 + ne * 16 + np * 12; }
  const el = {}; for (const [k, v] of Object.entries(run.plEnt)) for (const [e] of v) el[e] = +k; run.entPl = el; await loadChampArt(Object.values(run.ents).filter(e => e.kind === 0).map(e => e.name));
  run.mode = S.runMode || 0; V.run = run; flipOf = {}; V.t = 0; V.play = true; $('#tickMax').textContent = '/ ' + (run.tick.at(-1) ?? 0); $('#tickIn').max = run.tick.at(-1) ?? 0; $('#play').textContent = T('멈춤'); $('#seek').max = run.off.length - 1; V.zoom = Math.min(canvas.clientHeight, canvas.clientWidth) / 1280 || 0.6; V.cx = V.cy = 640;
  if (run.focus) { V.cx = run.focus[0]; V.cy = run.focus[1]; }
  fillPlayers(); fillEvents(); return run;
}
function frame(i) {
  const r = V.run, o = r.off[i], dv = r.dv, ne = dv.getUint16(o + 4, true), np = dv.getUint16(o + 6, true), es = [], ps = [];
  let q = o + 8;
  for (let k = 0; k < ne; k++, q += 16) es.push({ id: dv.getUint32(q, true), x: dv.getUint16(q + 4, true) * 16, y: dv.getUint16(q + 6, true) * 16, hp: dv.getUint16(q + 8, true), mx: dv.getUint16(q + 10, true), kind: dv.getUint8(q + 12), team: dv.getUint8(q + 13), alive: dv.getUint8(q + 14) });
  for (let k = 0; k < np; k++, q += 12) ps.push({ c: dv.getUint32(q, true), x: dv.getUint16(q + 4, true) * 16, y: dv.getUint16(q + 6, true) * 16, n: r.proj[dv.getUint16(q + 8, true)] || '', team: dv.getUint8(q + 10) });
  return { tick: r.tick[i], es, ps };
}

// ── 그림 ─────────────────────────────────────────────
const canvas = $('#cv'), cx = canvas.getContext('2d'), sheets = {}; let SHV = Date.now(); // 「새로 고침」하면 그림을 다시 읽는다(좌표만 새로 읽고 그림은 옛것을 쓰는 섞임 방지)
// 그림 찾는 순서: 시트(#sheet.png + #anim.fanim) → aseprite 원본 → 그림 한 장(.png)
const loadImg = u => new Promise(r => { const i = new Image(); i.onload = () => r(i); i.onerror = () => r(null); i.src = u; });
async function loadSheet(s, u) {
  const v = '?v=' + SHV;
  try {
    const [im, an] = await Promise.all([loadImg(u + '%23sheet.png' + v), fetch(u + '%23anim.fanim' + v).then(r => r.ok ? r.json() : null).catch(() => null)]);
    if (im && an?.anims) { s.img = im; s.an = an.anims; s.ok = true; return; }
    const r = await fetch(u + '.aseprite' + v); if (r.ok) { const a = await aseSheet(await r.arrayBuffer()); s.img = a.img; s.an = a.anims; s.ok = true; return; }
    const one = await loadImg(u + '.png' + v); if (one) { s.img = one; s.single = true; s.ok = true; }
  } catch (e) { console.warn(T('그림'), u, e); }
}
// aseprite 파일 → 프레임을 격자로 편 캔버스 + 동작표(fanim 과 같은 모양)
const ASEB = { 1: 'multiply', 2: 'screen', 3: 'overlay', 4: 'darken', 5: 'lighten', 6: 'color-dodge', 7: 'color-burn', 8: 'hard-light', 9: 'soft-light', 10: 'difference', 11: 'exclusion', 12: 'hue', 13: 'saturation', 14: 'color', 15: 'luminosity', 16: 'lighter' };
async function inflate(b) { return new Uint8Array(await new Response(new Blob([b]).stream().pipeThrough(new DecompressionStream('deflate'))).arrayBuffer()); }
async function aseSheet(buf) {
  const D = new DataView(buf), u8 = new Uint8Array(buf), g16 = o => D.getUint16(o, true), i16 = o => D.getInt16(o, true), g32 = o => D.getUint32(o, true), str = o => new TextDecoder().decode(u8.subarray(o + 2, o + 2 + g16(o)));
  if (g16(4) !== 0xA5E0) throw new Error(T('aseprite 파일이 아님'));
  const nf = g16(6), W = g16(8), H = g16(10), dep = g16(12), hop = g32(14) & 1, tidx = u8[28], layers = [], cels = [], tags = [], dur = [], pal = [];
  let o = 128;
  for (let f = 0; f < nf; f++) {
    const len = g32(o), old = g16(o + 6), nc = g32(o + 12) || old; dur.push(g16(o + 8) / 1000 || 0.1); let q = o + 16;
    for (let c = 0; c < nc; c++) {
      const sz = g32(q), t = g16(q + 4), p = q + 6; if (sz < 6) break;
      if (t === 0x2004) { const fl = g16(p), lv = g16(p + 4); let vis = !!(fl & 1) && !(fl & 64); for (let k = layers.length - 1, need = lv - 1; k >= 0 && need >= 0; k--) if (layers[k].lv === need) { vis &&= layers[k].vis; need--; } layers.push({ vis, lv, type: g16(p + 2), bm: g16(p + 10), op: hop ? u8[p + 12] : 255 }); }
      else if (t === 0x2005) { const k = { f, l: g16(p), x: i16(p + 2), y: i16(p + 4), op: u8[p + 6], ty: g16(p + 7), z: i16(p + 9) }; if (k.ty === 1) k.link = g16(p + 16); else if (k.ty === 0 || k.ty === 2) { k.w = g16(p + 16); k.h = g16(p + 18); k.raw = u8.slice(p + 20, q + sz); } cels.push(k); }
      else if (t === 0x2019) { const a = g32(p + 4), b = g32(p + 8); let r = p + 20; for (let k = a; k <= b; k++) { const fl = g16(r); pal[k] = [u8[r + 2], u8[r + 3], u8[r + 4], u8[r + 5]]; r += 6; if (fl & 1) r += 2 + g16(r); } }
      else if (t === 0x2018) { let r = p + 10; for (let k = g16(p); k > 0; k--) { tags.push({ a: g16(r), b: g16(r + 2), d: u8[r + 4], n: str(r + 17) }); r += 19 + g16(r + 17); } }
      q += sz;
    }
    o += len;
  }
  for (const k of cels) if (k.raw) {
    const px = k.ty === 2 ? await inflate(k.raw) : k.raw, im = new ImageData(k.w, k.h), n = k.w * k.h;
    if (dep === 32) im.data.set(px.subarray(0, n * 4)); else for (let e = 0; e < n; e++) { const c = dep === 16 ? [px[e * 2], px[e * 2], px[e * 2], px[e * 2 + 1]] : px[e] === tidx ? [0, 0, 0, 0] : pal[px[e]] || [0, 0, 0, 0]; im.data.set(c, e * 4); }
    k.c = ce('canvas', { width: k.w, height: k.h }); k.c.getContext('2d').putImageData(im, 0, 0); k.raw = null;
  }
  for (const k of cels) if (k.link !== undefined) k.c = cels.find(x => x.l === k.l && x.f === k.link)?.c;
  const cols = Math.max(1, Math.min(nf, Math.floor(8192 / W))), img = ce('canvas', { width: cols * W, height: Math.ceil(nf / cols) * H }), g = img.getContext('2d'), box = f => ({ x: (f % cols) * W, y: Math.floor(f / cols) * H, w: W, h: H });
  for (let f = 0; f < nf; f++) {
    const b = box(f); g.save(); g.beginPath(); g.rect(b.x, b.y, W, H); g.clip();
    for (const k of cels.filter(x => x.f === f && x.c && layers[x.l]?.vis && layers[x.l].type === 0).sort((x, y) => (x.l + x.z) - (y.l + y.z) || x.z - y.z)) { const L = layers[k.l]; g.globalAlpha = k.op / 255 * L.op / 255; g.globalCompositeOperation = ASEB[L.bm] || 'source-over'; g.drawImage(k.c, b.x + k.x, b.y + k.y); }
    g.restore();
  }
  const anims = {}, seq = t => { const l = []; for (let f = t.a; f <= t.b && f < nf; f++) l.push(f); if (t.d === 1 || t.d === 3) l.reverse(); if (t.d >= 2 && l.length > 2) l.push(...l.slice(1, -1).reverse()); return l; };
  for (const t of tags) anims[t.n] ||= { frames: seq(t).map(f => ({ duration: dur[f], data: box(f) })) };
  if (!tags.length) anims.idle = { frames: seq({ a: 0, b: nf - 1, d: 0 }).map(f => ({ duration: dur[f], data: box(f) })) };
  return { img, anims };
}
function sheet(key) {
  if (key in sheets) return sheets[key];
  const s = sheets[key] = { img: null, an: null, ok: false };
  const base = key.startsWith('asset/') ? modKey(key) : key;
  if (!base) return s;
  loadSheet(s, '/asset/' + base.split('/').map(encodeURIComponent).join('/'));
  return s;
}
// 'asset/<모드id>/경로' → 게임 mods 폴더(모드 챔피언 그림)
function modKey(k) { const p = k.split('/'); if (p[1] === 'base') return k.replace(/^asset\/base\/(aseprite_resources\/)?/, ''); return 'mods/' + p.slice(1).join('/'); }
const ANIM = { 0: ['attack'], 1: ['skill1', 'skill'], 2: ['skill2'], 3: ['ult', 'ult_pre', 'ult_on'] };
function spriteKey(e, run) {
  const n = run.ents[e.id]?.name || ''; const t = e.team === 0 ? 'blue' : 'red';
  if (e.kind === 0) return S.champArt?.[n] || 'champions/' + n;
  if (e.kind === 5) return 'ingame/' + t + '_tower'; if (e.kind === 6) return 'ingame/' + t + '_nexus';
  const m = { bee_monster: 'bee', mushroom_monster: 'mushroom', stump_monster: 'stump', rhino_monster: 'rhino', serpen_monster: 'serpen', epic_monster: 'epic' }[n];
  return m ? 'ingame/' + m : null;
}
function drawAnim(key, tags, tms, x, y, flip, scale = 1, o = {}) {
  const s = sheet(key); if (!s.ok) return false;
  if (s.single) { cx.save(); cx.translate(x, y); if (flip) cx.scale(-1, 1); cx.drawImage(s.img, -s.img.width / 2 * scale, -s.img.height / 2 * scale, s.img.width * scale, s.img.height * scale); cx.restore(); return true; }
  if (!s.an) return false;
  let tag = tags.find(t => s.an[t]) || (s.an.idle ? 'idle' : Object.keys(s.an)[0]); const fr = s.an[tag]?.frames; if (!fr || !fr.length) return false;
  const tot = fr.reduce((a, f) => a + f.duration, 0) || 0.1; let t = tot > 0 ? (o.once ? Math.min(tms / 1000, tot - 1e-4) : (tms / 1000) % tot) : 0, f = fr[0]; for (const k of fr) { if (t < k.duration) { f = k; break; } t -= k.duration; }
  const d = f.data; cx.save(); cx.translate(x, y); if (flip) cx.scale(-1, 1);
  cx.drawImage(s.img, d.x, d.y, d.w, d.h, -d.w / 2 * scale, o.mid ? -d.h / 2 * scale : -d.h * scale + 4, d.w * scale, d.h * scale); cx.restore(); return true;
}
// 모드 챔피언의 시트·이펙트 그림(data_champion 안의 sprite · view_*)
// used = 화면에 나오는 챔피언. 고른 모드가 먼저, 안 고른 챔피언은 설치된 모드에서 같은 id 를 찾아 그림만 쓴다.
const ART = { v: 0, j: {} };
async function champJson(path, fresh) {
  if (ART.v !== SHV) { ART.v = SHV; ART.j = {}; }
  if (fresh || !(path in ART.j)) { try { ART.j[path] = JSON.parse((await api('/api/modsetting?path=' + encodeURIComponent(path))).replace(/^\uFEFF/, '')); } catch (e) { console.warn(e); ART.j[path] = null; } }
  return ART.j[path];
}
async function loadChampArt(used = []) {
  S.champArt = {}; S.modArt = { proj: {}, fx: {}, buff: {} };
  const put = j => { if (!j) return; if (j.sprite) S.champArt[j.id] = j.sprite; for (const [k, l] of [['proj', j.view_projectiles], ['fx', j.view_effects], ['buff', j.view_buffs]]) for (const v of l || []) if (v.anim) S.modArt[k][v.name] = { k: v.anim, t: [v.tag] }; };
  for (const id of new Set(used)) { if (S.sel.champs[id] || BUILTIN.includes(id)) continue; const c = S.mods.flatMap(m => m.champions).find(x => x.id === id); if (c) put(await champJson(c.path)); }
  for (const m of Object.values(S.sel.champs)) put(await champJson(m.path, true)); // 고른 것은 늘 파일을 다시 읽는다(만드는 중인 모드)
}
const art = (kind, n) => S.modArt?.[kind]?.[n] || S.art?.[kind]?.[n];
const MID = 14; // 챔피언·이펙트는 게임처럼 프레임 가운데를 기준으로 그린다(발은 14px 아래)
// 원본 챔피언 전용 화면 코드가 해 주던 것(0.7.3 RE): 궁 버프 동안 시트 교체 · 겹침
const FORM = { clown: 'clown_ult', ghost: 'ghost_ult', jiangshi: 'jiangshi_ult', magic_knight: 'magic_knight_ult', siege_breaker: 'siege_breaker_ult' };
const OWNFX = { magic_knight: [['magic_knight_ult', 'champions/magic_knight', 'ult_effect_loop']], jiangshi: [['jiangshi_ult', 'champions/jiangshi', 'ult_on_loop']], gunner: [['gunner_ult', 'skill_effect/gunner_ult_effect', 'ult_on']] };
function last(l, tick) { if (!l) return null; let a = 0, b = l.length; while (a < b) { const m = (a + b) >> 1; if (l[m][0] <= tick) a = m + 1; else b = m; } return a ? l[a - 1] : null; }
// 지금 몸 동작: [꼬리표, 지난 ms, 한 번만, 몸 안 보임]
function bodyAnim(e, sk, f, moving) {
  const an = sheet(sk).an, l = last(V.run.an[e.id], f.tick); if (!an || !l) return null;
  const [t, n, kind] = l, age = (f.tick - t) * 1000 / 60;
  if (n === 'idle' || n === 'run') return null;
  if (!an[n]) return kind === 3 ? null : [n, age, true, age < 1500]; // 시트에 없는 이름: 시전(3)은 무시, 궁 중(13)은 게임에서도 몸이 안 그려진다
  if (/^run.+idle$/.test(n)) return [n, age, false]; // 대기·달리기가 덮지 않는 이름(변신 색판) — 다음 동작이 올 때까지 유지
  const loop = /(loop|idle|run)$/.test(n), tot = an[n].frames.reduce((a, x) => a + x.duration, 0) * 1000;
  const st = last(V.run.st[V.run.entPl[e.id]], f.tick), held = st ? (st[2] || []).some(c => c.startsWith('10:')) : false; // 동작 상태(CC)가 남아 있으면 마지막 프레임을 붙든다(게임과 같게)
  if (loop) return held || f.tick - t < 4 || (!st && age < 3000) ? [n, age, false] : null;
  return age < tot || (held && kind === 13) ? [n, age, true] : null;
}
const mp = v => v / 1000 + 160; // 지도 좌표(0~960000) → 그림 좌표(1280 안, 여백 160)
// 맵 = 게임의 5대5 그림 6장. 캐릭터 아래 5장은 처음 한 번 한 장(1280×1280)으로 합쳐 두고, 앞쪽 벽만 캐릭터 위에 그린다.
const MAPL = ['background_5v5', 'bush_shadow_5v5', 'wall_shadow_5v5', 'wall_5v5', 'bush_5v5', 'wall_5v5_front'].map(n => { const i = new Image(); i.src = '/asset/ingame/5v5/' + n + '.png'; return i; });
const MAPC = { img: null, n: -1 };
const imgOk = i => i.complete && i.naturalWidth > 0;
function mapImg() {
  const n = MAPL.slice(0, 5).filter(imgOk).length; if (MAPC.n === n) return MAPC.img; MAPC.n = n; if (!imgOk(MAPL[0])) return MAPC.img = null;
  const c = MAPC.img || ce('canvas', { width: 1280, height: 1280 }), g = c.getContext('2d'); g.imageSmoothingEnabled = false; g.clearRect(0, 0, 1280, 1280); for (const i of MAPL.slice(0, 5)) if (imgOk(i)) g.drawImage(i, 0, 0, 1280, 1280); return MAPC.img = c;
}
// 5대5(모드 0)가 아니면 벽이 다르므로 바닥만
function drawMap(mode) { if (mode) { if (imgOk(MAPL[0])) cx.drawImage(MAPL[0], 0, 0, 1280, 1280); return; } const m = mapImg(); if (m) cx.drawImage(m, 0, 0, 1280, 1280); }
function drawMapFront(mode) { if (!mode && imgOk(MAPL[5])) cx.drawImage(MAPL[5], 0, 0, 1280, 1280); }
let flipOf = {};
function draw(now) {
  const dpr = devicePixelRatio || 1, W = canvas.clientWidth, H = canvas.clientHeight;
  if (canvas.width !== W * dpr || canvas.height !== H * dpr) { canvas.width = W * dpr; canvas.height = H * dpr; }
  cx.setTransform(dpr, 0, 0, dpr, 0, 0); cx.imageSmoothingEnabled = false; cx.fillStyle = '#0c0f13'; cx.fillRect(0, 0, W, H);
  if (P.on && sk) return drawPlace(now, W, H);
  if (!V.run || !V.run.off.length) { cx.fillStyle = '#93a0b3'; cx.textAlign = 'center'; cx.font = '15px "Malgun Gothic"'; cx.fillText(T('왼쪽에서 경기를 고르거나 새로 실행하세요'), W / 2, H / 2); return; }
  const i = Math.min(Math.floor(V.t), V.run.off.length - 1), f = frame(i);
  const selEnt = f.es.find(e => V.run.entPl[e.id] === V.sel);
  if ($('#follow').checked && selEnt) { V.cx = mp(selEnt.x); V.cy = mp(selEnt.y); }
  cx.translate(W / 2, H / 2); cx.scale(V.zoom, V.zoom); cx.translate(-V.cx, -V.cy);
  drawMap(V.run.mode);
  const tms = f.tick * 1000 / 60, pf = {}; if (i > 0) for (const q of frame(i - 1).es) if (q.kind === 0) pf[q.id] = q; // 그림 시각 = 경기 시각(멈추면 그림도 멈춘다)
  drawProj(f, tms, true); const order = [...f.es].sort((a, b) => a.y - b.y);
  if (selEnt && selEnt.alive && selEnt.hp > 0) drawRange(last(V.run.rg[V.sel], f.tick)?.[1], selEnt.x, selEnt.y);
  for (const e of order) {
    if (!e.alive && e.kind !== 5) continue; if (e.hp <= 0 && e.kind === 0) continue;
    const X = mp(e.x), Y = mp(e.y); let sk = spriteKey(e, V.run), tags = ['idle'], flip = false, at = tms;
    if (e.kind === 0) {
      const pl = V.run.entPl[e.id], c = lastCast(pl, f.tick), p = pf[e.id]; flip = !!flipOf[e.id];
      if (p && p.alive && (Math.abs(p.x - e.x) > 60 || Math.abs(p.y - e.y) > 60)) { tags = ['run', 'idle']; if (p.x !== e.x) flip = e.x < p.x; }
      const st = last(V.run.st[pl], f.tick), bl = st ? st[1] : [], nm0 = V.run.ents[e.id]?.name || '';
      if (FORM[nm0] && bl.includes(FORM[nm0])) sk = 'champions/' + FORM[nm0];
      const b = bodyAnim(e, sk, f, tags[0] === 'run');
      for (const [bn, k2, t2] of OWNFX[nm0] || []) if (bl.includes(bn)) drawAnim(k2, [t2], tms, X, Y - MID, flip, 1, { mid: 1 });
      for (const bn of bl) { const a = art('buff', bn); if (a) drawAnim(a.k, a.t, tms, X, Y - MID, flip, 1, { mid: 1 }); }
      if (b) { tags = [b[0]]; at = b[1]; e._once = b[2]; e._gone = b[3]; }
      else if (!V.run.an[e.id] && c && f.tick - c.tick < 40) { tags = [...ANIM[c.slot], 'attack']; at = (f.tick - c.tick) * 1000 / 60; }
      e._tag = tags[0];
      flipOf[e.id] = flip;
    }
    if (!e._gone && !(sk && drawAnim(sk, tags, at, X, e.kind === 0 ? Y - MID : Y, flip, 1, { mid: e.kind === 0, once: e._once }))) { cx.beginPath(); cx.fillStyle = e.kind === 1 ? (e.team === 0 ? '#4c86d8' : '#c65353') : e.kind === 0 ? (e.team === 0 ? '#5aa2ff' : '#ff6b6b') : '#8d9a5b'; cx.arc(X, Y, e.kind === 0 ? 7 : 4, 0, 7); cx.fill(); }
    const w = e.kind === 0 ? (V.run.showcase ? 24 : 34) : e.kind === 1 ? 12 : 26, top = Y - (e.kind === 0 ? 42 : 24);
    if (e.kind !== 1 || e.hp < e.mx) { cx.fillStyle = '#000a'; cx.fillRect(X - w / 2 - 1, top - 1, w + 2, 5); cx.fillStyle = e.team === 0 ? '#4d9bff' : e.team === 1 ? '#ff5d5d' : '#d6c24a'; cx.fillRect(X - w / 2, top, w * Math.max(0, Math.min(1, e.hp / (e.mx || 1))), 3); }
    if (e.kind === 0) { const pl = V.run.entPl[e.id]; if (pl === V.sel) { cx.strokeStyle = '#f2c14e'; cx.lineWidth = 1.5; cx.strokeRect(X - w / 2 - 3, top - 3, w + 6, 9); } cx.fillStyle = '#fff'; cx.font = (V.zoom > 1.4 ? 13 / V.zoom : 10) + 'px "Malgun Gothic"'; cx.textAlign = 'center'; { const cn = V.run.ents[e.id]?.name || ''; cx.fillText(S.names[cn] || cn, X, top - 3); } if (V.run.showcase && pl === 0 && e._tag) { cx.fillStyle = '#ffd37a'; cx.font = 15 / V.zoom + 'px "Malgun Gothic"'; cx.fillText(e._tag, X, Y + 14); } }
  }
  drawMapFront(V.run.mode);
  drawProj(f, tms, false);
  for (const x of V.run.fx) {
    const d = f.tick - x.tick; if (d < 0 || d > 60) continue; let px, py;
    if (x.k === 2) { px = mp(x.x); py = mp(x.y); } else { const e = f.es.find(q => q.id === (x.k ? x.c : x.target - 1)); if (!e) continue; px = mp(e.x); py = mp(e.y); }
    const a = art('fx', x.name); let ok = false;
    if (a) { const sh = sheet(a.k), tg = sh.an && a.t.find(t => sh.an[t]), tot = tg ? sh.an[tg].frames.reduce((q, z) => q + z.duration, 0) * 60 : 45; if (d > Math.max(6, tot)) continue; ok = drawAnim(a.k, a.t, d * 1000 / 60, px, py - MID, false, 1, { mid: 1, once: 1 }); }
    if (!ok && !a && d < 24) { const al = 1 - d / 24; cx.strokeStyle = 'rgba(255,210,110,' + al + ')'; cx.lineWidth = 1; cx.beginPath(); cx.arc(px, py - 6, 5 + d * .7, 0, 7); cx.stroke(); if (V.run.showcase) { cx.fillStyle = 'rgba(255,210,110,' + al + ')'; cx.font = 11 / V.zoom + 'px sans-serif'; cx.textAlign = 'center'; cx.fillText(x.name, px, py - 16 - d * .3); } }
  }
  cx.setTransform(dpr, 0, 0, dpr, 0, 0);
  const sec = Math.floor(f.tick / 60); $('#clock').textContent = String(Math.floor(sec / 60)).padStart(2, '0') + ':' + String(sec % 60).padStart(2, '0') + T(' (틱 ') + f.tick + ')';
  drawInfo(f);
}
const GROUND = /road|area|ground|cloud|field|zone|circle|knight_ult$|guardian_spirit_ult|shadowmancer_ult_end|icemage_skill2/; // 바닥에 깔리는 것은 몸보다 먼저
function drawProj(f, tms, under) {
  for (const p of f.ps) { if (GROUND.test(p.n) !== under) continue; const X = mp(p.x), Y = mp(p.y), a = art('proj', p.n); if (!(a && drawAnim(a.k, a.t, tms, X, Y - 8, false, 1, { mid: 1 }))) { cx.beginPath(); cx.fillStyle = p.team === 0 ? '#9cc7ff' : '#ffb0b0'; cx.arc(X, Y, 3, 0, 7); cx.fill(); } }
}
function lastCast(pl, tick) { let b = null; const a = V.run.castBy ||= byPlayer(V.run.casts); const l = a[pl] || []; for (const c of l) { if (c.tick > tick) break; b = c; } return b; }
function byPlayer(l) { const o = {}; for (const c of l) (o[c.i] ||= []).push(c); return o; }

// ── 오른쪽 정보 ───────────────────────────────────────
let lastInfo = 0;
function fillPlayers() {
  const d = $('#players'); d.replaceChildren();
  V.run.head.players.forEach((p, i) => { const c = ce('div', { className: 'card' + (i === V.sel ? ' sel' : ''), id: 'pl' + i, onclick: () => { V.sel = i; document.querySelectorAll('#players .card').forEach(x => x.classList.toggle('sel', x.id === 'pl' + i)); } }); const cn = V.run.ents[V.run.plEnt[i]?.at(-1)?.[0]]?.name || p.champ; c.innerHTML = '<b class="' + (p.team ? 'r' : 'b') + '">' + nm(cn) + '</b> <span class="dim" id="pi' + i + '"></span>'; d.append(c); });
  if (V.run.end) { const b = V.run.end.p.filter(x => V.run.head.players[x.i].team === 0).reduce((a, x) => a + x.k, 0), r = V.run.end.p.filter(x => V.run.head.players[x.i].team === 1).reduce((a, x) => a + x.k, 0); $('#score').innerHTML = T('<span class="b">블루 ') + b + '</span> : <span class="r">' + r + T(' 레드</span>') + (!V.run.end.ended ? T(' · 중간에 멈춤') : V.run.end.winner_code === 1 ? T(' · 블루 승') : V.run.end.winner_code === 0 ? T(' · 레드 승') : T(' · 승자 판정 안 됨')); }
}
function drawInfo(f) {
  if (performance.now() - lastInfo < 250) return; lastInfo = performance.now();
  V.run.head.players.forEach((p, i) => { const e = f.es.find(q => V.run.entPl[q.id] === i), sp = $('#pi' + i); if (sp) sp.textContent = e && e.hp > 0 ? T('체력 ') + e.hp + '/' + e.mx : T('사망'); });
  const last = V.run.traces.length ? latestTrace(f.tick) : null, w = $('#why');
  if (last) { w.className = ''; w.replaceChildren(...traceRows(last)); } else if (V.run.traces.length === 0) w.innerHTML = T('<span class="dim">「판단 기록」을 켜고 실행하면 여기에 이유가 나옵니다.</span>');
}
function pos3d(w, o) { const v = BigInt('0x' + (w[o / 8] || '0')); return Number(BigInt.asIntN(64, v)); }
function latestTrace(tick) {
  const sel = V.run.head.players[V.sel]; if (!sel) return null; let b = null;
  for (const x of V.run.traces) { if (x.tick > tick) break; const tm = pos3d(x.w, 0x3d0), po = pos3d(x.w, 0x3d8); const idx = V.run.head.players.findIndex((p, i) => p.team === tm && i % 5 === po); if (idx === V.sel) b = x; } return b;
}
function traceRows(x) {
  const g = o => (x.s.find(q => q[0] === o) || [])[1]; const out = []; const add = (k, v) => { if (v) out.push(ce('div', { className: 'row' }, ce('span', { className: 'dim', textContent: k }), ce('span', { textContent: v }))); };
  add(T('시각'), T('틱 ') + x.tick); add(T('목표'), g('1d8')); add(T('큰 플랜'), g('1f0')); add(T('대상'), g('208')); add(T('서브플랜'), g('220')); add(T('세부'), g('238')); add(T('행동'), g('250')); add(T('배정 사유'), g('158') || g('378'));
  const rs = []; (x.r || []).forEach((q, i) => { if (typeof q === 'string' && Array.isArray(x.r[i - 1])) rs.push([q, x.r[i - 1][3]]); }); if (rs.length) { out.push(ce('div', { className: 'dim', textContent: T('점수 항목') })); for (const [n, v] of rs) out.push(ce('div', { className: 'row' }, ce('span', { textContent: n }), ce('b', { textContent: String(Number(BigInt.asIntN(64, BigInt('0x' + v)))) }))); } return out;
}
function fillEvents() {
  const d = $('#events'); d.replaceChildren(); const r = V.run; const out = []; for (const x of (S.modLog || [])) d.append(ce('div', { className: 'ev', textContent: x }));
  for (const c of r.cmds) if (!/^freeze|^pos|^cd0/.test(c.cmd)) out.push([c.tick, T('명령 ') + c.cmd + ' → ' + c.r]); for (const f of r.fails) out.push([f.tick, T('시전 실패 선수 ') + f.i]); for (const c of r.casts) if (r.head.players.length <= 4 || false) out.push([c.tick, nm(r.head.players[c.i].champ) + ' ' + [T('평타'), T('스킬1'), T('스킬2'), T('궁')][c.slot] + T(' 사용')]);
  out.sort((a, b) => a[0] - b[0]); for (const [t, s] of out.slice(0, 200)) d.append(ce('div', { className: 'ev', textContent: T('틱 ') + t + ' · ' + s }));
  if (!out.length && !(S.modLog || []).length) d.textContent = T('없음');
}

// ── 리플레이 탭 ──────────────────────────────────────
function buildReplay() {
  const d = $('#tab-replay'); d.replaceChildren(); if (!S.ins) { d.textContent = T('리플레이가 들어 있는 세이브를 고르세요'); return; }
  d.append(ce('div', { className: 'dim', textContent: T('재생 가능한 리플레이 ') + S.ins.replays.length + T('건 · 누르면 다시 계산해 보여 줍니다') }));
  const tr = ce('label', { className: 'row' }, ce('input', { type: 'checkbox', id: 'rtrace' }), T('판단 기록 켜기(파일이 커집니다)')); d.append(tr);
  // 여러 해 진행한 세이브는 리플레이가 수만 건이다 → 연도·리그로 나누고 최근 경기부터 500건씩만 그린다(전부 그리면 화면이 멈춘다)
  const year = r => String(r.ver).split('.')[0], cnt = {};
  for (const r of S.ins.replays) cnt[year(r)] = (cnt[year(r)] || 0) + 1;
  const ys = Object.keys(cnt).sort();
  if (S.ryear !== '*' && !(S.ryear in cnt)) S.ryear = ys[ys.length - 1];
  const t = ce('table'), more = ce('button', { onclick: () => { S.rshow += 500; fill(); } });
  const ysel = ce('select', { onchange: () => { S.ryear = ysel.value; S.rshow = 500; fillLg(); fill(); } });
  for (const y of [...ys].reverse()) ysel.append(ce('option', { value: y, textContent: y + ' (' + cnt[y] + ')', selected: y === S.ryear }));
  ysel.append(ce('option', { value: '*', textContent: T('전체') + ' (' + S.ins.replays.length + ')', selected: S.ryear === '*' }));
  const lsel = ce('select', { onchange: () => { S.rlg = lsel.value; S.rshow = 500; fill(); } });
  const inYear = () => S.ins.replays.filter(r => S.ryear === '*' || year(r) === S.ryear);
  // 리그 목록: 그 연도에 있는 것만(1부 → 2부 → 국제 대회 → 그 밖, 이름순) · 솔랭은 게임이 리플레이로 남기지 않아 없다
  function fillLg() {
    const c = {}; for (const r of inYear()) c[r.lg || ''] = (c[r.lg || ''] || 0) + 1;
    const rank = k => k === '연습' ? 8 : !k ? 10 : /tournament|컵/.test(k) ? 3 : /2$|2부/.test(k) ? 2 : 1;
    const ks = Object.keys(c).sort((a, b) => rank(a) - rank(b) || lgName(a).localeCompare(lgName(b)));
    if (S.rlg !== '*' && !(S.rlg in c)) S.rlg = '*';
    lsel.replaceChildren(ce('option', { value: '*', textContent: T('전체'), selected: S.rlg === '*' }));
    for (const k of ks) lsel.append(ce('option', { value: k, textContent: lgName(k) + ' (' + (c[k] || 0) + ')', selected: k === S.rlg }));
  }
  S.rlg = S.rlg || '*'; fillLg();
  d.append(ce('div', { className: 'row' }, T('연도'), ysel, ce('span', { className: 'dim', textContent: T('최근 경기부터') })), ce('div', { className: 'row' }, T('리그'), lsel));
  S.rshow = S.rshow || 500;
  const esc = x => String(x).replace(/&/g, '&amp;').replace(/</g, '&lt;');
  function fill() {
    t.className = 'rlist'; t.innerHTML = T('<tr><th class="ck">키</th><th class="cl">리그</th><th>블루</th><th>레드</th><th class="cr">결과</th></tr>');
    const list = inYear().filter(r => S.rlg === '*' || (r.lg || '') === S.rlg);
    for (let i = list.length - 1, n = 0; i >= 0 && n < S.rshow; i--, n++) { const r = list[i]; const solo = r.lg === '솔랭'; const row = ce('tr', { className: 'pick', onclick: async () => { if (B.on) return; t.querySelectorAll('.sel').forEach(x => x.classList.remove('sel')); row.classList.add('sel'); await startRun(['key ' + r.key], { trace: $('#rtrace').checked }); } }); row.innerHTML = '<td class="dim">' + r.key + '</td><td class="dim">' + esc(lgName(r.lg)) + '</td><td>' + esc(solo ? T('솔랭') : teamName(r.t0)) + '</td><td>' + esc(solo ? T('솔랭') : teamName(r.t1)) + '</td><td class="' + (r.blue_win ? 'b' : 'r') + '">' + r.k0 + ':' + r.k1 + '</td>'; row.title = lgName(r.lg) + ' · ' + T('버전') + ' ' + r.ver + '\n' + (solo ? T('솔랭') : teamName(r.t0) + ' vs ' + teamName(r.t1)) + '\n' + r.roster.map(x => (x.team ? T('레드 ') : T('블루 ')) + POS[x.pos] + ' ' + nm(x.champ)).join('\n'); t.append(row); }
    if (!list.length) t.append(ce('tr', {}, ce('td', { colSpan: 5, style: 'white-space:normal', className: 'dim', textContent: T('없음') })));
    const left = list.length - S.rshow; more.hidden = left <= 0; more.textContent = T('더 보기') + ' (' + T('남은 ') + left + T('건') + ')';
  }
  fill();
  d.append(t, more);
}
// 리그·대회 이름(세이브엔 번역 키나 옛 한국어 이름으로 들어 있다 · assets/league_names.json)
function lgName(k) { if (k === '솔랭') return T('솔랭'); if (k === '연습') return T('연습 경기'); if (!k) return T('기타'); const e = (S.lgn || {})[k]; return e ? (LANG === 'en' ? e.en : e.ko) : k; }
function teamName(id) { return (S.ins && S.ins.teams && S.ins.teams[id]) || T('팀 ') + id; }

// ── 임의 경기 탭 ─────────────────────────────────────
let custom = null;
// 팀 전술: 고른 항목만 바꾼다(나머지는 기록 그대로). custom.strat[t] = {항목: 값}
function stratBox(t) {
  if (typeof custom.strat[t] !== 'object' || !custom.strat[t]) custom.strat[t] = {};
  const det = ce('details'); det.append(ce('summary', { textContent: T('팀 전술 바꾸기 (') + Object.keys(custom.strat[t]).length + ')' }));
  for (const [f, x] of Object.entries(S.nt ? S.nt.strat : {})) {
    const s = ce('select', { onchange: () => { if (s.value === '') delete custom.strat[t][f]; else custom.strat[t][f] = JSON.parse(s.value); det.firstChild.textContent = T('팀 전술 바꾸기 (') + Object.keys(custom.strat[t]).length + ')'; } });
    s.append(ce('option', { value: '', textContent: T('기록 그대로') }));
    for (const v of x.values) { const j = JSON.stringify(v.v); s.append(ce('option', { value: j, textContent: LANG === 'en' && v.en || v.name, selected: JSON.stringify(custom.strat[t][f]) === j })); }
    det.append(ce('label', { className: 'row' }, ce('span', { textContent: LANG === 'en' && x.en || x.name, style: 'flex:1' }), s));
  }
  return det;
}
// 목표 빌드: 4칸 고르기(아이템 번호 순서 = 게임 설정 순서)
function buildBox(sl) {
  const d = ce('div', { className: 'row' }), cur = sl.items.split(',').map(x => +x).filter(x => !Number.isNaN(x));
  for (let i = 0; i < 4; i++) { const s = ce('select', { style: 'max-width:130px', onchange: () => { cur[i] = +s.value; sl.items = cur.filter(x => x !== undefined && x >= 0).join(','); } }); s.append(ce('option', { value: -1, textContent: T('없음') })); for (const it of [...(S.nt ? S.nt.items : []), ...S.modItems.map((k, j) => ({ id: 30 + j, ko: k, tier: -1 }))]) s.append(ce('option', { value: it.id, textContent: (LANG === 'en' && it.en || it.ko) + (it.tier < 0 ? T(' [모드]') : ' T' + (it.tier + 1)), selected: cur[i] === it.id })); d.append(s); }
  return d;
}
function buildCustom() {
  const d = $('#tab-custom'); d.replaceChildren(); if (!S.ins) { d.textContent = T('리플레이가 들어 있는 세이브를 고르세요'); return; }
  const base = S.ins.replays.find(r => r.key === custom?.key) || S.ins.replays[0]; custom = { key: base.key, seed: '', trace: false, until: 0, mode: 0, slots: base.roster.map(x => ({ team: x.team, pos: x.pos, champ: x.champ, stat: [...x.stat], items: x.items.join(',') })), strat: [{}, {}], ...(custom || {}) };
  const ks = ce('select', { onchange: () => { custom = null; const r = S.ins.replays.find(q => q.key == ks.value); custom = { key: r.key, seed: '', trace: false, until: 0, mode: 0, strat: [{}, {}], slots: r.roster.map(x => ({ team: x.team, pos: x.pos, champ: x.champ, stat: [...x.stat], items: x.items.join(',') })) }; buildCustom(); } }); const kl = S.ins.replays.slice(-300).reverse(); if (!kl.some(r => r.key === custom.key)) { const c = S.ins.replays.find(r => r.key === custom.key); if (c) kl.unshift(c); } for (const r of kl) ks.append(ce('option', { value: r.key, textContent: T('키 ') + r.key + ' (' + lgName(r.lg) + ' · ' + teamName(r.t0) + ' vs ' + teamName(r.t1) + ')', selected: r.key === custom.key }));
  d.append(ce('div', { className: 'row' }, T('바탕 경기'), ks)); const sd = ce('input', { placeholder: T('비우면 기록 시드'), value: custom.seed, oninput: () => custom.seed = sd.value }); d.append(ce('div', { className: 'row' }, T('시드'), sd));
  if (custom.mode === 1) custom.mode = 0; const md = ce('select', { onchange: () => custom.mode = +md.value }); [[0, T('정식 경기')], [2, T('데스매치')]].forEach(([v, t]) => md.append(ce('option', { value: v, textContent: t, selected: custom.mode === v }))); const un = ce('input', { type: 'number', value: custom.until, min: 0, oninput: () => custom.until = +un.value }); d.append(ce('div', { className: 'row' }, T('모드'), md, T('끝 틱(0=끝까지)'), un));
  const ct = ce('input', { type: 'checkbox', checked: custom.trace, onchange: () => custom.trace = ct.checked }); d.append(ce('label', { className: 'row' }, ct, T('판단 기록 켜기')));
  for (const t of [0, 1]) { d.append(ce('h2', { className: t ? 'r' : 'b', textContent: t ? T('레드 팀') : T('블루 팀') })); d.append(stratBox(t));
    for (const sl of custom.slots.filter(x => x.team === t)) { const cs = champSel(sl.champ); cs.onchange = () => sl.champ = cs.value;  const det = ce('details'); det.append(ce('summary', { textContent: T('능력치·빌드') })); const g = ce('div', { className: 'grid2' }); KEYS.forEach((k, i) => { const inp = ce('input', { type: 'number', min: 0, max: 100, value: sl.stat[i], oninput: () => sl.stat[i] = +inp.value }); g.append(ce('label', {}, STATS[i], inp)); }); det.append(g, ce('div', { className: 'dim', textContent: T('목표 빌드') }), buildBox(sl)); d.append(ce('div', { className: 'card' }, ce('div', { className: 'row' }, POS[sl.pos], cs), det)); } }
  const go = ce('button', { className: 'primary', textContent: T('경기 시작'), onclick: runCustom }); const nmI = ce('input', { placeholder: T('저장 이름') }); const sv = ce('button', { textContent: T('설정 저장'), onclick: async () => { if (!nmI.value) return; await api('/api/preset/' + encodeURIComponent(nmI.value), { method: 'POST', body: JSON.stringify({ kind: 'custom', save: S.save, ...custom }) }); await loadPresets(); } }); const ps = ce('select', { id: 'presetSel', onchange: async () => { if (!ps.value) return; const p = await J('/api/preset/' + encodeURIComponent(ps.value)); custom = p; buildCustom(); } }); d.append(ce('div', { className: 'row' }, go), ce('div', { className: 'row' }, nmI, sv), ce('div', { className: 'row' }, T('저장한 설정'), ps)); loadPresets();
}
async function loadPresets() { const s = $('#presetSel'); if (!s) return; s.replaceChildren(ce('option', { value: '', textContent: T('고르기…') })); for (const n of await J('/api/presets')) if (!n.startsWith('_')) s.append(ce('option', { value: n, textContent: n })); }
function runCustom() {
  const base = S.ins.replays.find(r => r.key === custom.key); const L = ['key ' + custom.key]; if (custom.seed !== '') L.push('seed ' + custom.seed); if (custom.mode) L.push('mode ' + custom.mode); if (custom.until) L.push('until ' + custom.until);
  for (const sl of custom.slots) { const b = base.roster.find(x => x.team === sl.team && x.pos === sl.pos); if (sl.champ !== b.champ) L.push('swap ' + sl.team + ' ' + sl.pos + ' ' + sl.champ); sl.stat.forEach((v, i) => { if (v !== b.stat[i]) L.push('stat ' + sl.team + ' ' + sl.pos + ' ' + KEYS[i] + ' ' + v); }); if (sl.items !== b.items.join(',')) L.push('items ' + sl.team + ' ' + sl.pos + ' ' + sl.items); }
  custom.strat.forEach((o, t) => { if (o && typeof o === 'object') for (const [k, v] of Object.entries(o)) L.push('at 1 strat ' + t + ' ' + k + ' ' + JSON.stringify(v)); });
  return startRun(L, { trace: custom.trace });
}

// ── 스킬 실험실 탭 ────────────────────────────────────
let sk = null;
function buildSkill() {
  const d = $('#tab-skill'); d.replaceChildren(); if (!S.ins) { d.textContent = T('리플레이가 들어 있는 세이브를 고르세요'); return; }
  sk ||= { champ: 'archer', level: 6, dummies: [{ team: 1, champ: 'knight', hp: 0, level: 1, n: 1 }], steps: [{ t: 60, slot: 1 }, { t: 220, slot: 2 }, { t: 380, slot: 3 }], dist: 20000, end: 700, minion: false };
  layoutSkill();
  const cs = champSel(sk.champ); cs.onchange = () => { sk.champ = cs.value; placeArt(); }; const lv = ce('input', { type: 'number', min: 1, max: 18, value: sk.level, oninput: () => sk.level = +lv.value }); const ds = ce('input', { type: 'number', value: sk.dist, min: 0, step: 2000, style: 'width:88px', onchange: () => { sk.dist = +ds.value; layoutSkill(true); buildSkill(); } });
  const pb = ce('button', { id: 'placeBtn', className: P.on ? 'primary' : '', textContent: P.on ? T('배치 끝내기') : T('지도에서 끌어서 배치'), onclick: () => setPlace(!P.on) });
  d.append(ce('div', { className: 'row' }, T('시전자'), cs), ce('div', { className: 'row' }, T('레벨'), lv), posRow(sk.me, 0), ce('div', { className: 'dim', textContent: T('스킬2는 3레벨, 궁은 5레벨부터 효과가 납니다') }),
    ce('h2', { textContent: T('위치') }), ce('div', { className: 'row' }, pb), ce('div', { className: 'row' }, T('기본 거리'), ds, ce('button', { textContent: T('기본 배치로'), title: T('시전자는 지도 가운데, 허수아비는 기본 거리만큼 옆에 세웁니다'), onclick: () => { layoutSkill(true); buildSkill(); } })),
    ce('div', { className: 'dim', textContent: T('좌표는 0~960000, 스킬 사거리(range)와 같은 단위입니다') }), ce('h2', { textContent: T('허수아비(AI 없음)') }));
  sk.dummies.forEach((q, i) => { const c2 = champSel(q.champ); c2.onchange = () => { q.champ = c2.value; placeArt(); buildSkill(); }; const tm = ce('select', { onchange: () => { q.team = +tm.value; buildSkill(); } }); tm.append(ce('option', { value: 1, textContent: T('적'), selected: q.team === 1 }), ce('option', { value: 0, textContent: T('아군'), selected: q.team === 0 })); const hp = ce('input', { type: 'number', value: q.hp, min: 0, title: T('0 = 가득'), oninput: () => q.hp = +hp.value }); const l2 = ce('input', { type: 'number', value: q.level, min: 1, max: 18, oninput: () => q.level = +l2.value }); d.append(ce('div', { className: 'card' }, ce('div', { className: 'row' }, ce('b', { textContent: NO(i + 1) }), tm, c2), ce('div', { className: 'row' }, T('체력'), hp, T('레벨'), l2, ce('button', { textContent: T('빼기'), onclick: () => { sk.dummies.splice(i, 1); for (const st of sk.steps) if ((st.tg || 1) > i + 1) st.tg--; buildSkill(); } })), posRow(q, i + 1))); });
  d.append(ce('button', { textContent: T('허수아비 추가'), onclick: () => { if (sk.dummies.length >= 9) return; sk.dummies.push({ team: 1, champ: 'knight', hp: 0, level: 1, n: 1 }); placeArt(); buildSkill(); } }), ce('h2', { textContent: T('시전 순서') }));
  sk.steps.forEach((q, i) => { const t = ce('input', { type: 'number', value: q.t, oninput: () => q.t = +t.value }); const s2 = ce('select', { onchange: () => q.slot = +s2.value }); [T('평타'), T('스킬1'), T('스킬2'), T('궁')].forEach((n, k) => s2.append(ce('option', { value: k, textContent: n, selected: q.slot === k }))); const tg = ce('select', { title: T('대상 허수아비'), style: 'max-width:110px', onchange: () => q.tg = +tg.value }); sk.dummies.forEach((w, k) => tg.append(ce('option', { value: k + 1, textContent: NO(k + 1) + ' ' + (w.team ? T('적 ') : T('아군 ')) + (S.names[w.champ] || w.champ), selected: (q.tg || 1) === k + 1 }))); const at = ce('select', { title: T('자동 = 스킬 방식에 맞춰 · 대상 = 그 허수아비 · 좌표 = 그 허수아비가 서 있는 자리 · 방향 = 그 허수아비 쪽'), onchange: () => q.how = at.value }); [['auto', T('자동')], ['target', T('대상')], ['pos', T('좌표')], ['dir', T('방향')]].forEach(([v, n]) => at.append(ce('option', { value: v, textContent: n, selected: (q.how || 'auto') === v }))); d.append(ce('div', { className: 'row' }, T('틱'), t, s2, '→', tg, at, ce('button', { textContent: '×', onclick: () => { sk.steps.splice(i, 1); buildSkill(); } }))); });
  const en = ce('input', { type: 'number', value: sk.end, oninput: () => sk.end = +en.value }); d.append(ce('div', { className: 'row' }, ce('button', { textContent: T('시전 추가'), onclick: () => { sk.steps.push({ t: (sk.steps.at(-1)?.t || 0) + 160, slot: 1 }); buildSkill(); } }), T('끝 틱'), en), ce('label', { className: 'row' }, ce('input', { type: 'checkbox', checked: !!sk.minion, onchange: e => { sk.minion = e.target.checked; } }), T('미니언 나오기')), ce('div', { className: 'row' }, ce('button', { className: 'primary', textContent: T('실행'), onclick: runSkill })));
  placeInfo();
}
// ── 위치 배치: 숫자로 적거나 지도에서 끌어 놓는다 ──
const P = { on: false, sel: 0, hold: null }, HOME = 480000, MAPMAX = 960000, inMap = v => Math.max(0, Math.min(MAPMAX, Math.round(v) || 0));
// 5대5 맵의 타워·넥서스 자리(경기 프레임에서 읽은 값) [x, y, 팀, 넥서스]
const TOWERS = [[48000, 272000, 0], [272000, 48000, 1], [48000, 528000, 0], [528000, 48000, 1], [368000, 592000, 0], [592000, 368000, 1], [240000, 720000, 0], [720000, 240000, 1], [688000, 912000, 0], [912000, 688000, 1], [432000, 912000, 0], [912000, 432000, 1], [110992, 784000, 0], [176000, 848000, 0], [784000, 110992, 1], [848000, 176000, 1], [96000, 864000, 0, 1], [864000, 96000, 1, 1]];
const towerRange = () => R.base.gs?.tower?.attack?.range || 75000;
const inTower = (o, team) => TOWERS.some(t => !t[3] && t[2] !== team && Math.hypot(t[0] - o.x, t[1] - o.y) <= towerRange());
// 위치가 없는 것만(force = 전부) 기본 자리로
function layoutSkill(force) {
  if (!sk.me || force) sk.me = { x: HOME, y: HOME };
  const c = [0, 0];
  for (const q of sk.dummies) { const k = c[q.team]++; if (q.x === undefined || force) { const off = (k % 2 ? -1 : 1) * Math.ceil(k / 2) * 9000; q.x = inMap(sk.me.x + (q.team === 0 ? -sk.dist : sk.dist)); q.y = inMap(sk.me.y + off); } }
}
const placeUnits = () => [{ o: sk.me, champ: sk.champ, team: 0, name: T('시전자') }, ...sk.dummies.map((q, i) => ({ o: q, champ: q.champ, team: q.team, name: NO(i + 1) }))];
const distTo = o => Math.round(Math.hypot(o.x - sk.me.x, o.y - sk.me.y));
function posRow(o, n) {
  const mk = k => ce('input', { type: 'number', min: 0, max: MAPMAX, step: 1000, style: 'width:88px', value: o[k], className: 'pos' + k, onfocus: () => { P.sel = n; }, onchange: e => { o[k] = inMap(+e.target.value); placeInfo(); } });
  return ce('div', { className: 'row posrow' }, 'x', mk('x'), 'y', mk('y'), ce('span', { className: 'dim' }));
}
// 숫자 칸·거리 표시를 지금 위치로
function placeInfo() {
  if (!sk) return; const us = placeUnits();
  document.querySelectorAll('#tab-skill .posrow').forEach((r, i) => { const o = us[i]?.o; if (!o) return; for (const k of ['x', 'y']) { const e = r.querySelector('.pos' + k); if (document.activeElement !== e) e.value = o[k]; } r.lastChild.textContent = (i ? T('거리 ') + distTo(o) : '') + (inTower(o, us[i].team) ? T(' ⚠ 적 타워 사거리 안') : ''); });
}
async function placeArt() { if (P.on) await loadChampArt(placeUnits().map(u => u.champ)); }
async function setPlace(on) {
  P.on = on; P.hold = null; if (on) { await loadChampArt(placeUnits().map(u => u.champ)); V.play = false; $('#play').textContent = T('재생'); V.cx = mp(sk.me.x); V.cy = mp(sk.me.y); V.zoom = 2.2; $('#follow').checked = false; $('#msg').textContent = T('캐릭터를 끌어서 옮기세요 · 빈 곳 끌기 = 지도 이동 · 휠 = 확대'); } else $('#msg').textContent = '';
  buildSkill();
}
const scrToMap = e => { const r = canvas.getBoundingClientRect(); return [(e.clientX - r.left - canvas.clientWidth / 2) / V.zoom + V.cx, (e.clientY - r.top - canvas.clientHeight / 2) / V.zoom + V.cy]; };
// 누른 자리의 캐릭터(위에 그려진 것 먼저)
function placeHit(e) {
  const [mx, my] = scrToMap(e), us = placeUnits(); let b = -1;
  us.forEach((u, i) => { const X = mp(u.o.x), Y = mp(u.o.y); if (Math.abs(mx - X) < 15 && my > Y - 38 && my < Y + 8 && (b < 0 || u.o.y >= us[b].o.y)) b = i; });
  return b < 0 ? null : { i: b, dx: mp(us[b].o.x) - mx, dy: mp(us[b].o.y) - my };
}
function placeMove(e) { const [mx, my] = scrToMap(e), o = placeUnits()[P.hold.i]?.o; if (!o) return; o.x = inMap((mx + P.hold.dx - 160) * 1000); o.y = inMap((my + P.hold.dy - 160) * 1000); placeInfo(); }
// ── 사거리 원 ── (값은 계산기가 게임에서 읽은 실제 사거리: 레벨·버프 포함)
const RNC = ['#f2c14e', '#7ed492', '#7ab4ff', '#ff8fe0'], RNN = [T('평타'), T('스킬1'), T('스킬2'), T('궁')];
const rngOn = () => [...document.querySelectorAll('.rng')].filter(x => x.checked).map(x => +x.value);
// 닿는 거리 = 사거리 + (대상 지정이면 내 몸) + 상대 몸(챔피언 기준)
const reach = (rg, k, tr) => { const s = rg?.s[k]; return s ? s[0] + (s[1] === 0 ? rg.rad : 0) + tr : 0; };
function drawRange(rg, x, y) {
  if (!rg) return; const px = 1 / V.zoom;
  for (const k of rngOn()) { const s = rg.s[k]; if (!s || s[2] === 4) continue; const r = reach(rg, k, 10000) / 1000; if (r > 900) continue; cx.strokeStyle = RNC[k]; cx.fillStyle = RNC[k] + '12'; cx.lineWidth = 1.5 * px; cx.beginPath(); cx.arc(mp(x), mp(y), r, 0, 7); cx.fill(); cx.stroke(); cx.fillStyle = RNC[k]; cx.font = 12 * px + 'px "Malgun Gothic"'; cx.textAlign = 'center'; cx.fillText(RNN[k] + ' ' + s[0] + (s[1] === 3 ? T(' (대상 없음)') : ''), mp(x), mp(y) - r - 4 * px); }
}
// 배치 화면용: 시전자 챔피언·레벨·모드로 아주 짧은 경기를 돌려 사거리를 읽어 둔다
const RGP = { key: '', rg: null, wait: false };
async function probeRange() {
  if (!P.on || !sk || !S.ins || !rngOn().length) return; const key = [S.save, sk.champ, sk.level, JSON.stringify(S.sel), JSON.stringify(R.ch)].join('|'); if (RGP.key === key || RGP.wait) return; RGP.wait = true; RGP.rg = null;
  try {
    const body = ['save=' + S.save, ...modDlls(), 'every=1', ...await modSettingLines(), ...await ruleLines(), ...modLines(), 'key ' + S.ins.replays[0].key, 'swap 0 0 ' + sk.champ, 'only 0 0', 'until 32', 'at 0 freeze 0', 'at 2 level 0 ' + sk.level].join('\n'); const { id } = JSON.parse(await api('/api/run', { method: 'POST', body }));
    for (let i = 0; i < 160; i++) { await new Promise(r => setTimeout(r, 250)); const s = await J('/api/run/' + id); if (s.state === 'fail') throw new Error(s.log.split('\n').at(-1)); if (s.state === 'done') break; }
    for (const l of (await api('/api/run/' + id + '/run.ndjson')).split('\n')) if (l.startsWith('{"t":"rg"')) { const o = JSON.parse(l); if (o.i === 0) RGP.rg = o; }
    RGP.key = key;
  } catch (e) { console.warn(e); RGP.key = key; $('#msg').textContent = T('사거리를 읽지 못했습니다: ') + e.message; }
  RGP.wait = false;
}
document.querySelectorAll('.rng').forEach(x => x.addEventListener('change', () => probeRange()));
function drawPlace(now, W, H) {
  cx.translate(W / 2, H / 2); cx.scale(V.zoom, V.zoom); cx.translate(-V.cx, -V.cy);
  drawMap(0);
  const px = 1 / V.zoom; for (const [tx, ty, tm, nx] of TOWERS) { const X = mp(tx), Y = mp(ty), c = tm ? '255,107,107' : '90,162,255'; if (!nx) { cx.fillStyle = 'rgba(' + c + ',.07)'; cx.strokeStyle = 'rgba(' + c + ',.5)'; cx.lineWidth = px; cx.beginPath(); cx.arc(X, Y, towerRange() / 1000, 0, 7); cx.fill(); cx.stroke(); } drawAnim('ingame/' + (tm ? 'red' : 'blue') + (nx ? '_nexus' : '_tower'), ['idle'], now, X, Y, false); }
  probeRange(); drawRange(RGP.rg, sk.me.x, sk.me.y);
  const us = placeUnits(), M = us[0]; cx.font = Math.max(7, 12 * px) + 'px "Malgun Gothic"'; cx.textAlign = 'center';
  for (const u of us.slice(1)) { cx.strokeStyle = u.team ? '#ff6b6b88' : '#5aa2ff88'; cx.lineWidth = px; cx.setLineDash([4 * px, 4 * px]); cx.beginPath(); cx.moveTo(mp(M.o.x), mp(M.o.y)); cx.lineTo(mp(u.o.x), mp(u.o.y)); cx.stroke(); cx.setLineDash([]); }
  for (const u of [...us].sort((a, b) => a.o.y - b.o.y)) {
    const X = mp(u.o.x), Y = mp(u.o.y), i = us.indexOf(u), me = i === 0;
    cx.strokeStyle = i === P.sel ? '#f2c14e' : me ? '#8fe3a1' : u.team ? '#ff6b6b' : '#5aa2ff'; cx.lineWidth = (i === P.sel ? 2 : 1.2) * px; cx.beginPath(); cx.ellipse(X, Y, 11, 5, 0, 0, 7); cx.stroke();
    if (!drawAnim(S.champArt?.[u.champ] || 'champions/' + u.champ, ['idle'], now, X, Y - MID, !me && u.o.x > M.o.x, 1, { mid: 1 })) { cx.fillStyle = u.team ? '#ff6b6b' : '#5aa2ff'; cx.beginPath(); cx.arc(X, Y - 8, 7, 0, 7); cx.fill(); }
    cx.fillStyle = '#000b'; const t = u.name + ' ' + (S.names[u.champ] || u.champ) + (me ? '' : ' · ' + distTo(u.o)) + (inTower(u.o, u.team) ? T(' ⚠타워') : ''), w = cx.measureText(t).width; cx.fillRect(X - w / 2 - 2 * px, Y - 46 - 11 * px, w + 4 * px, 14 * px); cx.fillStyle = me ? '#8fe3a1' : u.team ? '#ffb1b1' : '#a9ceff'; cx.fillText(t, X, Y - 46);
  }
}
// 선수 번호: 블루 0번 = 시전자. 허수아비는 블루(아군)·레드(적) 선수로 채운다.
async function runSkill() {
  if (B.on) return;
  layoutSkill();
  const L = ['key ' + S.ins.replays[0].key]; const ally = sk.dummies.filter(x => x.team === 0), foe = sk.dummies.filter(x => x.team === 1);
  if (ally.length > 4 || foe.length > 5) { $('#msg').textContent = T('아군 4 · 적 5 까지'); return; }
  P.on = false; P.hold = null; buildSkill();
  L.push('swap 0 0 ' + sk.champ, 'only 0 0'); ally.forEach((q, i) => { L.push('swap 0 ' + (i + 1) + ' ' + q.champ, 'only 0 ' + (i + 1)); }); foe.forEach((q, i) => { L.push('swap 1 ' + i + ' ' + q.champ, 'only 1 ' + i); });
  const N = 1 + ally.length + foe.length; L.push('until ' + sk.end);
  for (let i = 0; i < N; i++) L.push('at 0 freeze ' + i); L.push('at 2 level 0 ' + sk.level); const X = sk.me.x, Y = sk.me.y; L.push('at 20 pos 0 ' + X + ' ' + Y);
  const order = [...ally.map((q, i) => ({ q, n: 1 + i })), ...foe.map((q, i) => ({ q, n: 1 + ally.length + i }))];
  order.forEach(({ q, n }) => { if (q.level > 1) L.push('at 2 level ' + n + ' ' + q.level); L.push('at 20 pos ' + n + ' ' + q.x + ' ' + q.y); if (q.hp > 0) L.push('at 25 hp ' + n + ' ' + q.hp); });
  for (const q of sk.steps) {
    const w = order.find(o => o.q === (sk.dummies[(q.tg || 1) - 1] || sk.dummies[0])); if (!w) continue; L.push('at ' + q.t + ' cd0 0'); const c = 'at ' + q.t + ' cast 0 ' + q.slot;
    if (!q.how || q.how === 'auto') L.push(c + ' auto ' + w.n); else if (q.how === 'target') L.push(c + ' target ' + w.n); else if (q.how === 'pos') L.push(c + ' pos ' + w.q.x + ' ' + w.q.y); else L.push(c + ' dir ' + ((w.q.x - X) || 1) + ' ' + (w.q.y - Y));
  }
  const run = await startRun(L, { every: 1, rules: sk.minion ? {} : { gs: { 'minion_wave_setting.start_tick': 900000000 } } }); if (!run) return; V.sel = 0; run.focus = [mp(X), mp(Y)]; V.cx = mp(X); V.cy = mp(Y); V.zoom = 2.2; fillPlayers();
  const nc = run.casts.filter(c => c.i === 0).length;
  // 요청한 자리에 섰는지(벽 속 등은 못 선다): 첫 시전 전 프레임과 비교
  const chk = Math.max(0, run.tick.findIndex(t => t >= 26)), f = frame(chk), bad = [];
  for (const { o, n, name } of [{ o: sk.me, n: 0, name: T('시전자') }, ...order.map(w => ({ o: w.q, n: w.n, name: NO(sk.dummies.indexOf(w.q) + 1) }))]) { const e = f.es.find(q => run.entPl[q.id] === n); if (e && Math.hypot(e.x - o.x, e.y - o.y) > 3000) bad.push(name); }
  $('#msg').innerHTML = (run.fails.length ? T('<span class="bad">시전 실패 ') + run.fails.length + T('건</span>') : '<span class="' + (nc < sk.steps.length ? 'bad' : 'ok') + T('">시전 ') + nc + '/' + sk.steps.length + T('건') + (nc < sk.steps.length ? T(' (안 나간 것은 사거리 밖이거나 시전 방식이 안 맞습니다 — 「자동」으로 해 보세요)') : '') + '</span>') + (bad.length ? T(' <span class="bad">· 그 자리에 못 섬: ') + bad.join(', ') + T(' (벽 등)</span>') : '');
}

// ── 설정을 바꾸는 모드(override_info) ─────────────────
// merge = JSON 키 단위 재귀 병합(배열은 통째 교체), override = 통째 교체. 고른 순서대로 쌓는다.
const SKIND = { item_setting: ['is', 'item_setting'], game_setting: ['gs', 'game_setting'], champion_info: ['ci', 'champion_info_sheet'] };
function jmerge(a, b) { if (a && b && typeof a === 'object' && typeof b === 'object' && !Array.isArray(a) && !Array.isArray(b)) { for (const [k, v] of Object.entries(b)) a[k] = k in a ? jmerge(a[k], v) : v; return a; } return b; }
function modSettings(m) {
  if (!m.override) return []; let o; try { o = JSON.parse(m.override.replace(/^\uFEFF/, '')); } catch { return [{ bad: true, key: T('mod.override_info 형식 오류') }]; }
  return Object.entries(o).filter(([k]) => k.startsWith('asset/base/setting/')).map(([k, v]) => {
    const name = k.slice('asset/base/setting/'.length), src = String(v.remapping || '').split('/').slice(2).join('/');
    const kind = SKIND[name] || (name.includes('/') || /map_setting|patch_setting|data_setting|other_setting|training_setting|names/.test(name) ? null : ['dc', 'data_champion']);
    return { key: k, name, type: v.type, kind, path: kind ? m.dir + '/' + src + '.' + kind[1] : '' };
  });
}
async function modSettingLines() {
  const acc = {}, L = [];
  for (const id of Object.keys(S.sel.sets)) {
    const m = S.mods.find(x => x.id === id); if (!m) continue;
    for (const st of modSettings(m)) {
      if (!st.kind) continue; const key = st.kind[0] === 'dc' ? 'dc/' + st.name : st.kind[0];
      const mod = JSON.parse((await api('/api/modsetting?path=' + encodeURIComponent(st.path))).replace(/^\uFEFF/, ''));
      acc[key] = st.type === 'merge' ? jmerge(acc[key] || structuredClone(await ruleBase(key)), mod) : mod;
    }
  }
  for (const [k, o] of Object.entries(acc)) { const kd = k.startsWith('dc/') ? 'dc' : k; const b = await api('/api/blob/' + kd, { method: 'POST', body: JSON.stringify(o) }); L.push(kd === 'dc' ? 'fileblob ' + b : 'setting_full ' + k + ' ' + b); }
  return L;
}
// 모드 아이템 이름·번호: 모드를 켜고 1틱만 돌려 얻는다(dll 조합이 바뀔 때 한 번)
async function probeItems() {
  const key = Object.keys(S.sel.dlls).join('|'); if (S.probeKey === key) return; S.probeKey = key; S.modItems = [];
  if (!key || !S.save || !S.ins) return;
  try {
    const body = ['save=' + S.save, ...modDlls(), 'key ' + S.ins.replays[0].key, 'until 1'].join('\n'); const { id } = JSON.parse(await api('/api/run', { method: 'POST', body }));
    for (let i = 0; i < 120; i++) { await new Promise(r => setTimeout(r, 400)); const s = await J('/api/run/' + id); if (s.state === 'fail') return; if (s.state === 'done') break; }
    for (const l of (await api('/api/run/' + id + '/run.ndjson')).split('\n')) if (l.startsWith('{"t":"moditems"')) S.modItems = JSON.parse(l).list;
    if (S.modItems.length) { $('#msg').textContent = T('모드 아이템 ') + S.modItems.length + T('개를 빌드 목록에 추가했습니다'); buildCustom(); }
  } catch (e) { console.warn(e); }
}

// ── 규칙 편집(게임 규칙·챔피언 수치·아이템) ─────────────
const R = { base: {}, ch: { gs: {}, ci: {}, is: {} }, kind: 'ci', q: '' };
const RK = { gs: T('게임 규칙'), ci: T('챔피언 수치'), is: T('아이템') };
async function ruleBase(k) { return R.base[k] ||= JSON.parse(await api('/api/setting/' + k)); }
function flat(o, p, out) { for (const [k, v] of Object.entries(o)) { const q = p ? p + '.' + k : k; if (typeof v === 'number') out.push([q, v]); else if (v && typeof v === 'object' && !Array.isArray(v)) flat(v, q, out); else if (Array.isArray(v) && v.every(x => typeof x === 'number')) v.forEach((x, i) => out.push([q + '.' + i, x])); } return out; }
async function ruleLines(extra = {}) {
  const L = [];
  for (const k of Object.keys(R.ch)) {
    const c = Object.entries({ ...R.ch[k], ...(extra[k] || {}) }); if (!c.length) continue; // extra = 이번 실행에만 덮는 값
    const o = structuredClone(await ruleBase(k));
    for (const [path, v] of c) { const ps = path.split('.'); let t = o; for (const x of ps.slice(0, -1)) t = t[x]; t[ps.at(-1)] = v; }
    L.push('setting ' + k + ' ' + await api('/api/blob/' + k, { method: 'POST', body: JSON.stringify(o) }));
  }
  return L;
}
async function buildRules() {
  const d = $('#tab-rules'); d.replaceChildren();
  const ks = ce('select', { onchange: () => { R.kind = ks.value; buildRules(); } }); for (const k of Object.keys(RK)) ks.append(ce('option', { value: k, textContent: RK[k], selected: R.kind === k }));
  const q = ce('input', { placeholder: T('이름 검색(예: archer, kill_gold)'), value: R.q, style: 'width:100%' }); const list = ce('div');
  const fill = async () => { const f = flat(await ruleBase(R.kind), '', []); list.replaceChildren(); const hit = f.filter(([n]) => R.q === '' ? n in R.ch[R.kind] : n.toLowerCase().includes(R.q.toLowerCase())).slice(0, 80); if (!hit.length) list.append(ce('div', { className: 'dim', textContent: R.q ? T('없음') : T('검색하면 숫자를 고칠 수 있습니다. 고친 값은 모든 실행에 적용됩니다.') })); for (const [n, v] of hit) { const i = ce('input', { type: 'number', value: n in R.ch[R.kind] ? R.ch[R.kind][n] : v, style: 'width:90px', oninput: () => { const x = +i.value; if (x === v) delete R.ch[R.kind][n]; else R.ch[R.kind][n] = x; row.classList.toggle('sel', n in R.ch[R.kind]); ruleCount(); } }); const row = ce('div', { className: 'row card' + (n in R.ch[R.kind] ? ' sel' : '') }, ce('span', { textContent: n, style: 'flex:1;word-break:break-all' }), i, ce('span', { className: 'dim', textContent: T('원래 ') + v })); list.append(row); } };
  q.oninput = () => { R.q = q.value; clearTimeout(q.t); q.t = setTimeout(fill, 200); };
  d.append(ce('div', { className: 'row' }, ks, ce('button', { textContent: T('이 종류 되돌리기'), onclick: () => { R.ch[R.kind] = {}; ruleCount(); buildRules(); } })), q, ce('div', { className: 'dim', textContent: T('세이브의 설정 위에 고친 숫자만 덮습니다. 목록 안의 항목 수를 늘리는 편집은 아직 안 됩니다.') }), list); await fill(); ruleCount();
}
function ruleCount() { const n = Object.values(R.ch).reduce((a, x) => a + Object.keys(x).length, 0); $('#ruleN').textContent = n; }

// ── 세이브·모드 ──────────────────────────────────────
async function loadSaves() {
  const l = await J('/api/saves'), s = $('#save'); s.replaceChildren(); for (const x of l) s.append(ce('option', { value: x.path, textContent: x.name + ' · ' + new Date(x.time * 1000).toLocaleString(LANG) + ' · ' + (x.size / 1e6).toFixed(0) + 'MB' })); if (l.length) { for (const x of l.slice(0, 6)) { s.value = x.path; await pickSave(x.path); if (S.ins) break; } } else $('#msg').textContent = T('세이브가 없습니다 — 게임에서 경기를 한 번 이상 하고 저장한 뒤 「새로 고침」을 누르세요');
}
async function pickSave(p) { S.save = p; S.ryear = null; S.rlg = '*'; S.rshow = 500; if (!S.lgn) S.lgn = await J('/asset/league_names.json').catch(() => ({})); $('#msg').textContent = T('세이브 읽는 중…'); busy(T('세이브를 읽는 중입니다')); try { S.ins = await J('/api/inspect?save=' + encodeURIComponent(p)); $('#msg').textContent = ''; if (!S.ins.replays.length) { S.ins = null; $('#msg').innerHTML = T('<span class="bad">이 세이브에는 재생할 수 있는 리플레이가 없습니다. 다른 세이브를 고르세요.</span>'); } } catch (e) { S.ins = null; $('#msg').innerHTML = '<span class="bad">' + SV(e.message).replace(/</g, '&lt;') + '</span>'; } idle(); buildReplay(); custom = null; buildCustom(); buildSkill(); }
// 고른 모드를 기억한다(presets/_mods.json). 다음에 켜면 그대로 고른 상태로 시작한다.
let selT = 0;
function saveSel() { clearTimeout(selT); selT = setTimeout(() => api('/api/preset/_mods', { method: 'POST', body: JSON.stringify(S.sel) }).catch(e => console.warn(e)), 200); }
async function restoreSel() {
  if (S.restored) return; S.restored = true; let o; try { o = await J('/api/preset/_mods'); } catch { return; }
  const lost = [], file = p => p.split('/').slice(-1)[0];
  for (const [id, m] of Object.entries(o.champs || {})) { if (S.mods.some(x => x.champions.some(c => c.id === id && c.path === m.path))) S.sel.champs[id] = { path: m.path, mode: m.mode === 'replace' ? 'replace' : 'fill' }; else lost.push(id); }
  for (const p of Object.keys(o.dlls || {})) { if (S.mods.some(x => x.dlls.some(d => d.path === p && d.stable))) S.sel.dlls[p] = 1; else lost.push(file(p)); }
  for (const id of Object.keys(o.sets || {})) { if (S.mods.some(x => x.id === id && modSettings(x).some(q => q.kind))) S.sel.sets[id] = 1; else lost.push(id); }
  S.lost = lost;
}
async function loadMods() {
  S.mods = await J('/api/mods'); await restoreSel(); const d = $('#modsList'); d.replaceChildren();
  for (const m of S.mods) { const c = ce('div', { className: 'card' }, ce('b', { textContent: m.id.replace('[개발]', T('[개발]')) }));
    for (const ch of m.champions) { const on = ce('input', { type: 'checkbox', checked: S.sel.champs[ch.id]?.path === ch.path, onchange: () => { if (on.checked) S.sel.champs[ch.id] = { path: ch.path, mode: md.value }; else delete S.sel.champs[ch.id]; modCount(); } }); const md = ce('select', { onchange: () => { if (S.sel.champs[ch.id]) { S.sel.champs[ch.id].mode = md.value; saveSel(); } } }); [['fill', T('추가/채움')], ['replace', T('교체')]].forEach(([v, n]) => md.append(ce('option', { value: v, textContent: n, selected: S.sel.champs[ch.id]?.mode === v }))); c.append(ce('div', { className: 'row' }, on, T('챔피언 ') + ch.id, md)); }
    const ms = modSettings(m);
    if (ms.length) { const on = ce('input', { type: 'checkbox', disabled: ms.every(x => !x.kind), checked: !!S.sel.sets[m.id], onchange: () => { if (on.checked) S.sel.sets[m.id] = 1; else delete S.sel.sets[m.id]; modCount(); } }); c.append(ce('div', { className: 'row' }, on, T('설정 교체: ') + ms.map(x => x.bad ? x.key : x.name + (x.type === 'merge' ? T('(병합)') : T('(통째)')) + (x.kind ? '' : T(' — 미지원'))).join(', '))); }
    for (const dl of m.dlls) { const on = ce('input', { type: 'checkbox', disabled: !dl.stable, checked: !!S.sel.dlls[dl.path], onchange: () => { if (on.checked) S.sel.dlls[dl.path] = 1; else delete S.sel.dlls[dl.path]; modCount(); } }); c.append(ce('div', { className: 'row' }, on, dl.path.split('/').pop(), ce('span', { className: 'tag' + (dl.stable ? '' : ' warn'), textContent: dl.stable ? T('공식 API') : T('이전 버전'), title: dl.stable ? '' : T('예전 방식(클래식 API) dll 이라 켤 수 없습니다') }), ...(dl.hooks ? [ce('span', { className: 'tag warn', textContent: T('게임 함수 후킹') })] : []))); }
    d.append(c); }
}
function modCount() { saveSel(); modShow(); }
function modShow() { clearTimeout(S.pt); S.pt = setTimeout(probeItems, 600); $('#modN').textContent = Object.keys(S.sel.champs).length + Object.keys(S.sel.dlls).length + Object.keys(S.sel.sets).length; buildCustom(); buildSkill(); }

// 게임 폴더 직접 지정(못 찾았거나 버전이 다를 때)
function gameForm(b, en) {
  const i = ce('input', { value: en.game || '', style: 'flex:1;min-width:260px', placeholder: T('예) D:\\SteamLibrary\\steamapps\\common\\Teamfight Manager2') }), m = ce('span');
  const go = ce('button', { textContent: T('이 폴더 쓰기'), onclick: async () => {
    m.textContent = T(' 확인 중…'); try { await api('/api/game', { method: 'POST', body: i.value }); } catch (e) { m.textContent = ' ' + SV(e.message); return; }
    m.textContent = T(' 다시 시작하는 중… 첫 실행이면 그림을 꺼내느라 몇 초 걸립니다'); for (let k = 0; k < 120; k++) { await new Promise(r => setTimeout(r, 500)); try { await J('/api/env'); location.reload(); return; } catch {} } m.textContent = T(' 다시 켜지지 않았습니다. 프로그램을 직접 다시 실행해 주세요');
  } });
  b.append(ce('div', { className: 'row' }, T('게임 폴더'), i, go, m));
}
// ── 시작 ─────────────────────────────────────────────
document.querySelectorAll('nav button').forEach(b => b.onclick = () => { document.querySelectorAll('nav button').forEach(x => x.classList.toggle('on', x === b)); for (const t of ['replay', 'custom', 'skill', 'rules']) $('#tab-' + t).hidden = t !== b.dataset.tab; if (b.dataset.tab !== 'skill' && P.on) setPlace(false); });
$('#save').onchange = e => pickSave(e.target.value); $('#reload').onclick = () => { SHV = Date.now(); for (const k of Object.keys(sheets)) delete sheets[k]; loadSaves(); loadMods(); }; $('#modsBtn').onclick = () => $('#modsDlg').showModal(); $('#modsOff').onclick = async () => { S.sel = { champs: {}, dlls: {}, sets: {} }; modCount(); await loadMods(); }; $('#modsClose').onclick = () => $('#modsDlg').close(); $('#modsDlg').onclick = e => { if (e.target === e.currentTarget) e.currentTarget.close(); };
$('#play').onclick = playToggle;
// ── 틱 단위로 넘기기 ──
function setPlay(on) { V.play = on; $('#play').textContent = on ? T('멈춤') : T('재생'); }
function playToggle() { if (!V.run) return; if (!V.play && V.t >= V.run.off.length - 1) V.t = 0; setPlay(!V.play); }
const curTick = () => V.run.tick[Math.min(Math.floor(V.t), V.run.off.length - 1)];
// 그 틱(없으면 바로 앞 기록)으로
function gotoTick(t) { if (!V.run?.off.length) return; setPlay(false); const T = V.run.tick; let a = 0, b = T.length; while (a < b) { const m = (a + b) >> 1; if (T[m] <= t) a = m + 1; else b = m; } V.t = Math.max(0, a - 1); $('#seek').value = V.t; tickShow(); }
function stepTick(n) { if (!V.run?.off.length || P.on || B.on) return; const c = curTick(); gotoTick(c + n); if (n > 0 && curTick() === c) { V.t = Math.min(V.run.off.length - 1, Math.floor(V.t) + 1); $('#seek').value = V.t; tickShow(); } }
function tickShow() { const e = $('#tickIn'); if (V.run?.off.length && document.activeElement !== e) e.value = curTick(); }
document.querySelectorAll('button.step').forEach(b => b.onclick = () => stepTick(+b.dataset.n));
$('#tickIn').onchange = e => gotoTick(+e.target.value || 0); $('#tickIn').onkeydown = e => { if (e.key === 'Enter') { gotoTick(+e.target.value || 0); e.target.blur(); } };
window.addEventListener('keydown', e => {
  const el = document.activeElement, tag = el?.tagName; if ($('#modsDlg').open || e.ctrlKey || e.altKey || e.metaKey || !V.run) return;
  if (tag === 'TEXTAREA' || tag === 'SELECT' || (tag === 'INPUT' && el.id !== 'seek' && el.type !== 'checkbox')) return;
  if (e.key === 'ArrowRight' || e.key === 'ArrowLeft') { e.preventDefault(); stepTick((e.key === 'ArrowRight' ? 1 : -1) * (e.shiftKey ? 60 : 1)); }
  else if (e.key === ' ') { if (tag === 'BUTTON') el.blur(); e.preventDefault(); playToggle(); }
  else if (e.key === 'Home') { e.preventDefault(); gotoTick(0); } else if (e.key === 'End') { e.preventDefault(); gotoTick(1e9); }
}); $('#seek').oninput = e => { V.t = +e.target.value; };
let drag = null; canvas.onmousedown = e => { P.hold = P.on ? placeHit(e) : null; if (P.hold) { P.sel = P.hold.i; e.preventDefault(); document.activeElement?.blur(); } else drag = [e.clientX, e.clientY]; }; window.onmouseup = () => { drag = null; P.hold = null; }; window.onmousemove = e => { if (P.hold) return placeMove(e); if (drag) { V.cx -= (e.clientX - drag[0]) / V.zoom; V.cy -= (e.clientY - drag[1]) / V.zoom; drag = [e.clientX, e.clientY]; $('#follow').checked = false; } }; canvas.onwheel = e => { e.preventDefault(); V.zoom = Math.max(0.2, Math.min(8, V.zoom * (e.deltaY < 0 ? 1.15 : 0.87))); };
let lt = performance.now(); (function loop(n) { const dt = Math.min(100, n - lt); lt = n; if (V.run && V.play) { const every = V.run.tick[1] - V.run.tick[0] || 1; V.t += dt / 1000 * 60 / every * +$('#speed').value; if (V.t >= V.run.off.length - 1) { V.t = V.run.off.length - 1; V.play = false; $('#play').textContent = T('재생'); } $('#seek').value = Math.floor(V.t); } tickShow(); try { draw(n); } catch (e) { console.error(e); } requestAnimationFrame(loop); })(lt);
i18nStatic();
(async () => { try { [S.art, S.names, S.nt] = await Promise.all([J('/asset/art.json'), J('/asset/champion_names_' + LANG + '.json'), J('/asset/names.json')]); } catch (e) { console.warn(e); } try { const en = await J('/api/env'); if (en.problems.length) { const b = ce('div', { id: 'warn' }); for (const x of en.problems) b.append(ce('div', { textContent: '⚠ ' + SV(x) })); gameForm(b, en); document.body.prepend(b); } } catch (e) { console.warn(e); } await loadMods(); await loadSaves(); await buildRules(); modShow(); const ns = Object.keys(S.sel.champs).length + Object.keys(S.sel.dlls).length + Object.keys(S.sel.sets).length; if (S.lost?.length) $('#msg').innerHTML = '<span class="bad"></span>', $('#msg').firstChild.textContent = T('지난번에 고른 모드 중 찾지 못한 것: ') + S.lost.join(', '); else if (ns) $('#msg').textContent = T('지난번에 고른 모드 ') + ns + T('개를 켠 상태입니다'); })();
