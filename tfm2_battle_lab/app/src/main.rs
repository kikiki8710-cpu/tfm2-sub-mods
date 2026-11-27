//! TFM2 전투 실험실 앱 — 내 PC 안에서만(127.0.0.1) 열리는 작은 서버 + 브라우저 화면.
//! 계산은 별도 프로세스(lab_worker.exe)가 한다. 이 프로그램은 게임 코드를 올리지 않는다.
mod setup;
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

struct Cfg { root: PathBuf, game: PathBuf, saves: Vec<PathBuf>, worker: PathBuf, port: u16 }
struct Job { state: String, log: String }
type Jobs = Arc<Mutex<HashMap<String, Job>>>;

fn env_path(k: &str, d: PathBuf) -> PathBuf { std::env::var_os(k).map(PathBuf::from).unwrap_or(d) }

fn main() {
    let exe_dir = std::env::current_exe().ok().and_then(|p| p.parent().map(|x| x.to_path_buf())).unwrap_or_default();
    // root = exe 옆 폴더(개발 중 다른 곳의 exe 로 돌릴 때는 BL_ROOT 로 지정)
    let root = env_path("BL_ROOT", exe_dir.clone());
    let game = setup::find_game(&root);
    let appdata = std::env::var_os("APPDATA").map(PathBuf::from).unwrap_or_default();
    let saves = vec![appdata.join("TeamSamoyed/TeamfightManager2/data"), root.join("saves")];
    let worker = env_path("BL_WORKER", { let w = exe_dir.join("lab_worker.exe"); if w.exists() { w } else { root.join("lab_worker.exe") } });
    // 다시 시작할 때는 앞 프로세스가 포트를 놓을 때까지 잠깐 기다린다
    let want: u16 = std::env::var("BL_PORT").ok().and_then(|p| p.parse().ok()).unwrap_or(0);
    let mut bound = TcpListener::bind(("127.0.0.1", want));
    for _ in 0..40 { if bound.is_ok() || want == 0 { break; } std::thread::sleep(std::time::Duration::from_millis(150)); bound = TcpListener::bind(("127.0.0.1", want)); }
    let l = match bound { Ok(l) => l, Err(e) => { eprintln!("포트를 열 수 없습니다 / Cannot open the port: {e}"); return; } };
    let port = l.local_addr().unwrap().port();
    let cfg = Arc::new(Cfg { root, game, saves, worker, port });
    for d in ["runs", "cache", "presets", "saves", "devmods"] { let _ = std::fs::create_dir_all(cfg.root.join(d)); }
    println!("게임 폴더 / Game folder: {}", cfg.game.display());
    match setup::prepare(&cfg.root, &cfg.game, &TABLES) { Ok(Some(n)) => println!("그림 {n}개 준비 완료 / {n} graphics prepared"), Ok(None) => {} Err(e) => println!("⚠ {e}") }
    setup::prune_runs(&cfg.root, 10);
    for e in setup::problems(&cfg.root, &cfg.game, &cfg.worker) { println!("⚠ {e}"); }
    let jobs: Jobs = Default::default();
    println!("이 창을 닫으면 프로그램이 꺼집니다. / Close this window to quit.");
    println!("TFM2 전투 실험실 / Battle Lab: http://127.0.0.1:{port}/");
    if std::env::var_os("BL_NO_BROWSER").is_none() { let _ = std::process::Command::new("rundll32").args(["url.dll,FileProtocolHandler", &format!("http://127.0.0.1:{port}/")]).spawn(); }
    for s in l.incoming().flatten() { let (c, j) = (cfg.clone(), jobs.clone()); std::thread::spawn(move || { let _ = handle(s, &c, &j); }); }
}

/// 우리가 만든 이름표·그림 표(게임 파일 아님)
const TABLES: [(&str, &[u8]); 5] = [("league_names.json", include_bytes!("../../assets/league_names.json")), ("champion_names_en.json", include_bytes!("../../assets/champion_names_en.json")),("art.json", include_bytes!("../../assets/art.json")), ("names.json", include_bytes!("../../assets/names.json")), ("champion_names_ko.json", include_bytes!("../../assets/champion_names_ko.json"))];
struct Req { method: String, path: String, query: HashMap<String, String>, headers: HashMap<String, String>, body: Vec<u8> }

fn read_req(s: &mut TcpStream) -> Option<Req> {
    s.set_read_timeout(Some(std::time::Duration::from_secs(20))).ok()?;
    let mut buf = Vec::new();
    let mut tmp = [0u8; 8192];
    let hend = loop {
        let n = s.read(&mut tmp).ok()?;
        if n == 0 { return None; }
        buf.extend_from_slice(&tmp[..n]);
        if let Some(i) = buf.windows(4).position(|w| w == b"\r\n\r\n") { break i; }
        if buf.len() > 64 * 1024 { return None; }
    };
    let head = String::from_utf8_lossy(&buf[..hend]).into_owned();
    let mut lines = head.split("\r\n");
    let mut p = lines.next()?.split(' ');
    let (method, target) = (p.next()?.to_string(), p.next()?.to_string());
    let mut headers = HashMap::new();
    for l in lines { if let Some((k, v)) = l.split_once(':') { headers.insert(k.trim().to_ascii_lowercase(), v.trim().to_string()); } }
    let len: usize = headers.get("content-length").and_then(|v| v.parse().ok()).unwrap_or(0);
    if len > 4 << 20 { return None; }
    let mut body = buf[hend + 4..].to_vec();
    while body.len() < len { let n = s.read(&mut tmp).ok()?; if n == 0 { break; } body.extend_from_slice(&tmp[..n]); }
    body.truncate(len);
    let (path, q) = target.split_once('?').map(|(a, b)| (a.to_string(), b.to_string())).unwrap_or((target.clone(), String::new()));
    let mut query = HashMap::new();
    for kv in q.split('&').filter(|x| !x.is_empty()) { let (k, v) = kv.split_once('=').unwrap_or((kv, "")); query.insert(dec(k), dec(v)); }
    Some(Req { method, path: dec(&path), query, headers, body })
}
fn dec(s: &str) -> String {
    let b = s.as_bytes(); let mut o = Vec::new(); let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() { if let Ok(v) = u8::from_str_radix(&s[i + 1..i + 3], 16) { o.push(v); i += 3; continue; } }
        o.push(if b[i] == b'+' { b' ' } else { b[i] }); i += 1;
    }
    String::from_utf8_lossy(&o).into_owned()
}
fn js(s: &str) -> String { let mut o = String::from("\""); for c in s.chars() { match c { '"' => o.push_str("\\\""), '\\' => o.push_str("\\\\"), '\n' => o.push_str("\\n"), '\r' => o.push_str("\\r"), '\t' => o.push_str("\\t"), c if (c as u32) < 32 => {} c => o.push(c) } } o.push('"'); o }
fn send(s: &mut TcpStream, code: u16, ct: &str, body: &[u8]) {
    let _ = write!(s, "HTTP/1.1 {code} X\r\nContent-Type: {ct}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nX-Content-Type-Options: nosniff\r\nConnection: close\r\n\r\n", body.len());
    let _ = s.write_all(body);
}
fn txt(s: &mut TcpStream, code: u16, m: &str) { send(s, code, "text/plain; charset=utf-8", m.as_bytes()) }
fn json(s: &mut TcpStream, m: &str) { send(s, 200, "application/json; charset=utf-8", m.as_bytes()) }

/// 경로 조각에 상위 이동·드라이브가 없는지
fn safe_rel(p: &str) -> bool { !p.is_empty() && !p.contains("..") && !p.contains(':') && !p.contains(char::from(92)) && !p.starts_with('/') }
fn is_hash(s: &str) -> bool { !s.is_empty() && s.len() < 80 && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-') }

fn handle(mut s: TcpStream, cfg: &Cfg, jobs: &Jobs) -> Option<()> {
    let r = read_req(&mut s)?;
    // 이 서버 자신의 화면에서 온 요청만 받는다(다른 사이트가 로컬 서버를 부르지 못하게)
    let host_ok = r.headers.get("host").map_or(false, |h| h == &format!("127.0.0.1:{}", cfg.port) || h == &format!("localhost:{}", cfg.port));
    if !host_ok { txt(&mut s, 403, "bad host"); return None; }
    if r.method == "POST" {
        let o = r.headers.get("origin").cloned().unwrap_or_default();
        if o != format!("http://127.0.0.1:{}", cfg.port) && o != format!("http://localhost:{}", cfg.port) { txt(&mut s, 403, "bad origin"); return None; }
    }
    match (r.method.as_str(), r.path.as_str()) {
        ("GET", "/") => send(&mut s, 200, "text/html; charset=utf-8", &std::fs::read(cfg.root.join("app/ui/index.html")).unwrap_or_else(|_| include_bytes!("../ui/index.html").to_vec())),
        ("GET", p) if p.starts_with("/ui/") => {
            let n = &p[4..];
            if !safe_rel(n) { txt(&mut s, 400, "bad path"); return None; }
            let ct = ctype(n);
            match std::fs::read(cfg.root.join("app/ui").join(n)) { Ok(b) => send(&mut s, 200, ct, &b), Err(_) => match n { "app.js" => send(&mut s, 200, ct, include_bytes!("../ui/app.js")), "style.css" => send(&mut s, 200, ct, include_bytes!("../ui/style.css")), _ => txt(&mut s, 404, "없음") } }
        }
        ("GET", "/api/modsetting") => {
            // 모드 폴더 안의 설정 JSON 하나(확장자 허용 목록)
            let f = r.query.get("path").cloned().unwrap_or_default();
            let ok = ["item_setting", "game_setting", "champion_info_sheet", "data_champion"].iter().any(|x| mod_file(cfg, &f, x));
            if !ok { txt(&mut s, 400, "허용되지 않은 파일"); return None; }
            match std::fs::read(&f) { Ok(b) => send(&mut s, 200, "application/json; charset=utf-8", &b), Err(e) => txt(&mut s, 404, &e.to_string()) }
        }
        ("GET", p) if p.starts_with("/api/setting/") => { match setting_src(cfg, &p[13..]).and_then(|f| std::fs::read(f).ok()) { Some(b) => send(&mut s, 200, "application/json; charset=utf-8", &b), None => txt(&mut s, 404, "없음") } }
        ("GET", p) if p.starts_with("/asset/") => asset(&mut s, cfg, &p[7..]),
        ("GET", "/api/env") => { let v: Vec<String> = setup::problems(&cfg.root, &cfg.game, &cfg.worker).iter().map(|x| js(x)).collect(); json(&mut s, &format!("{{\"game\":{},\"ver\":{},\"problems\":[{}]}}", js(&cfg.game.to_string_lossy()), js(setup::GAME_VER), v.join(","))); }
        ("POST", "/api/game") => {
            // 게임 폴더를 적어 두고 프로그램을 다시 시작한다(같은 포트)
            let r = setup::clean_path(&String::from_utf8_lossy(&r.body)).ok_or_else(|| "경로가 비어 있습니다".to_string()).and_then(|p| setup::save_game(&cfg.root, &p));
            match r {
                Err(e) => txt(&mut s, 400, &e),
                Ok(()) => {
                    txt(&mut s, 200, "ok");
                    let _ = s.shutdown(std::net::Shutdown::Both);
                    if let Ok(me) = std::env::current_exe() { if std::process::Command::new(me).env("BL_PORT", cfg.port.to_string()).env("BL_NO_BROWSER", "1").env("BL_ROOT", &cfg.root).spawn().is_ok() { std::process::exit(0); } }
                }
            }
        }
        ("GET", "/api/saves") => saves(&mut s, cfg),
        ("GET", "/api/mods") => mods(&mut s, cfg),
        ("GET", "/api/inspect") => inspect(&mut s, cfg, r.query.get("save").map(|x| x.as_str()).unwrap_or("")),
        ("GET", "/api/presets") => {
            let v: Vec<String> = std::fs::read_dir(cfg.root.join("presets")).map(|d| d.flatten().filter_map(|e| e.file_name().to_str().and_then(|n| n.strip_suffix(".json").map(|x| js(x)))).collect()).unwrap_or_default();
            json(&mut s, &format!("[{}]", v.join(",")));
        }
        ("GET", p) if p.starts_with("/api/preset/") => { let n = &p[12..]; if !preset_ok(n) { txt(&mut s, 400, "이름"); return None; } match std::fs::read(cfg.root.join("presets").join(format!("{n}.json"))) { Ok(b) => send(&mut s, 200, "application/json; charset=utf-8", &b), Err(_) => txt(&mut s, 404, "없음") } }
        ("POST", p) if p.starts_with("/api/preset/") => { let n = &p[12..]; if !preset_ok(n) { txt(&mut s, 400, "이름"); return None; } match std::fs::write(cfg.root.join("presets").join(format!("{n}.json")), &r.body) { Ok(_) => txt(&mut s, 200, "ok"), Err(e) => txt(&mut s, 500, &e.to_string()) } }
        ("POST", p) if p.starts_with("/api/blob/") => {
            // 고친 설정 JSON 을 내용 해시 이름으로 저장
            let k = &p[10..];
            if (setting_src(cfg, k).is_none() && k != "dc") || r.body.len() > 4 << 20 { txt(&mut s, 400, "종류"); return None; }
            let mut h: u64 = 0xcbf29ce484222325;
            for b in &r.body { h = (h ^ *b as u64).wrapping_mul(0x100000001b3); }
            let n = format!("blob_{k}_{h:016x}.json");
            match std::fs::write(cfg.root.join("cache").join(&n), &r.body) { Ok(_) => txt(&mut s, 200, &n), Err(e) => txt(&mut s, 500, &e.to_string()) }
        }
        ("POST", "/api/run") => run(&mut s, cfg, jobs, &String::from_utf8_lossy(&r.body)),
        ("GET", p) if p.starts_with("/api/run/") => run_get(&mut s, cfg, jobs, &p[9..]),
        _ => txt(&mut s, 404, "없음"),
    }
    Some(())
}
fn preset_ok(n: &str) -> bool { safe_rel(n) && n.chars().count() <= 40 && !n.contains('/') }
fn ctype(n: &str) -> &'static str { match n.rsplit('.').next().unwrap_or("") { "html" => "text/html; charset=utf-8", "js" => "text/javascript; charset=utf-8", "css" => "text/css; charset=utf-8", "png" => "image/png", "json" | "fanim" => "application/json; charset=utf-8", _ => "application/octet-stream" } }

/// 폴더 이름이 모드 id 와 다른 경우(창작마당 번호 폴더): mod.mod_info 의 mod_id 로 찾는다
fn by_mod_id(cfg: &Cfg, rel: &str) -> Option<PathBuf> {
    static MAP: Mutex<Option<HashMap<String, PathBuf>>> = Mutex::new(None);
    let (id, rest) = rel.split_once('/')?;
    let mut g = MAP.lock().unwrap_or_else(|e| e.into_inner());
    if !g.as_ref().map_or(false, |m| m.contains_key(id)) {
        let mut m = HashMap::new();
        for r in mod_roots(cfg) { if let Ok(rd) = std::fs::read_dir(&r) { for d in rd.flatten().map(|e| e.path()).filter(|p| p.is_dir()) {
            let t = std::fs::read_to_string(d.join("mod.mod_info")).unwrap_or_default();
            if let Some(v) = top_str(&t, "mod_id") { m.entry(v).or_insert(d); }
        } } }
        *g = Some(m);
    }
    let p = g.as_ref()?.get(id)?.join(rest);
    p.is_file().then_some(p)
}
/// 그림: /asset/<키>.png|.fanim|.json — 키 앞부분이 mods/<id>/ 면 게임 mods 폴더, 아니면 앱 자산 폴더
fn asset(s: &mut TcpStream, cfg: &Cfg, rel: &str) {
    if !safe_rel(rel) || !(rel.ends_with(".png") || rel.ends_with(".fanim") || rel.ends_with(".json") || rel.ends_with(".aseprite")) { txt(s, 400, "bad path"); return; }
    let p = if let Some(x) = rel.strip_prefix("mods/") { mod_roots(cfg).iter().map(|r| r.join(x)).find(|p| p.is_file()).or_else(|| by_mod_id(cfg, x)).unwrap_or_else(|| cfg.game.join("mods").join(x)) } else { cfg.root.join("assets").join(rel) };
    // 이름표(TABLES)는 자산 폴더에 없으면(옛 준비본) 내장본을 준다
    match std::fs::read(&p) { Ok(b) => send(s, 200, ctype(rel), &b), Err(_) => match TABLES.iter().find(|(n, _)| *n == rel) { Some((_, b)) => send(s, 200, ctype(rel), b), None => txt(s, 404, "없음") } }
}

fn list_saves(cfg: &Cfg) -> Vec<(PathBuf, u64, u64)> {
    let mut v = Vec::new();
    for d in &cfg.saves { if let Ok(rd) = std::fs::read_dir(d) { for e in rd.flatten() { let p = e.path(); if p.extension().map_or(false, |x| x == "data") { if let Ok(m) = e.metadata() { let t = m.modified().ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_secs()).unwrap_or(0); v.push((p, m.len(), t)); } } } } }
    v.sort_by_key(|x| std::cmp::Reverse(x.2));
    v
}
fn find_save(cfg: &Cfg, want: &str) -> Option<PathBuf> { list_saves(cfg).into_iter().map(|x| x.0).find(|p| p.to_string_lossy().replace(char::from(92), "/") == want) }
fn saves(s: &mut TcpStream, cfg: &Cfg) {
    let v: Vec<String> = list_saves(cfg).iter().map(|(p, n, t)| format!("{{\"path\":{},\"name\":{},\"size\":{n},\"time\":{t}}}", js(&p.to_string_lossy().replace(char::from(92), "/")), js(&p.file_name().unwrap_or_default().to_string_lossy()))).collect();
    json(s, &format!("[{}]", v.join(",")));
}
fn walk(dir: &Path, depth: usize, out: &mut Vec<PathBuf>) { if depth > 4 { return; } if let Ok(rd) = std::fs::read_dir(dir) { for e in rd.flatten() { let Ok(ft) = e.file_type() else { continue }; let p = e.path(); if ft.is_dir() { walk(&p, depth + 1, out) } else if ft.is_file() { out.push(p) } } } }
/// data_champion 파일 안의 맨 바깥 "id" (없으면 파일 이름)
fn champ_id(p: &Path) -> String { top_str(&std::fs::read_to_string(p).unwrap_or_default(), "id").unwrap_or_else(|| p.file_stem().unwrap_or_default().to_string_lossy().into_owned()) }
/// JSON 글에서 맨 바깥 객체의 문자열 값 하나(안쪽 객체의 같은 이름은 보지 않는다)
fn top_str(t: &str, name: &str) -> Option<String> {
    let (mut depth, mut i, b) = (0i32, 0usize, t.as_bytes());
    while i < b.len() {
        match b[i] {
            b'{' | b'[' => depth += 1,
            b'}' | b']' => depth -= 1,
            b'"' => {
                let st = i + 1; let mut j = st;
                while j < b.len() && b[j] != b'"' { if b[j] == 92 { j += 1; } j += 1; }
                if depth == 1 && t.get(st..j) == Some(name) {
                    let r = t[j + 1..].trim_start();
                    if let Some(v) = r.strip_prefix(':').map(|x| x.trim_start()).and_then(|x| x.strip_prefix('"')) { if let Some(e) = v.find('"') { return Some(v[..e].to_string()); } }
                }
                i = j;
            }
            _ => {}
        }
        i += 1;
    }
    None
}
fn mods(s: &mut TcpStream, cfg: &Cfg) {
    let mut rows = Vec::new();
    for root in mod_roots(cfg) { if let Ok(rd) = std::fs::read_dir(&root) {
        let mut ds: Vec<PathBuf> = rd.flatten().map(|e| e.path()).filter(|p| p.is_dir()).collect();
        ds.sort();
        for d in ds {
            let mut fs = Vec::new();
            walk(&d, 0, &mut fs);
            let name = d.file_name().unwrap_or_default().to_string_lossy().into_owned();
            let ch: Vec<String> = fs.iter().filter(|p| p.extension().map_or(false, |x| x == "data_champion")).map(|p| format!("{{\"path\":{},\"id\":{}}}", js(&p.to_string_lossy().replace(char::from(92), "/")), js(&champ_id(p)))).collect();
            let mut dlls = Vec::new();
            for p in fs.iter().filter(|p| p.extension().map_or(false, |x| x == "dll")) {
                let b = std::fs::read(p).unwrap_or_default();
                let has = |t: &[u8]| b.windows(t.len()).any(|w| w == t);
                dlls.push(format!("{{\"path\":{},\"stable\":{},\"hooks\":{}}}", js(&p.to_string_lossy().replace(char::from(92), "/")), has(b"tfm2_mod_entry_stable"), has(b"VirtualProtect")));
            }
            // 설정(asset/base/setting/…)을 바꾸는 override_info 만 화면에 넘긴다(병합은 화면이 JSON 으로 한다)
            let ov = std::fs::read_to_string(d.join("mod.override_info")).unwrap_or_default();
            let ov = if ov.contains("asset/base/setting/") { ov } else { String::new() };
            if ch.is_empty() && dlls.is_empty() && ov.is_empty() { continue; }
            // 창작마당은 폴더 이름이 번호라 mod_info 의 이름을 보여 준다
            let title = fs.iter().find(|p| p.file_name().map_or(false, |n| n == "mod.mod_info")).and_then(|p| std::fs::read_to_string(p).ok()).and_then(|t| top_str(&t, "name")).unwrap_or_default();
            let dev = root.ends_with("devmods");
            let shown = if title.is_empty() || title == name { name.clone() } else { format!("{title} ({name})") };
            let shown = if dev { format!("[개발] {shown}") } else { shown };
            rows.push(format!("{{\"id\":{},\"dir\":{},\"override\":{},\"champions\":[{}],\"dlls\":[{}]}}", js(&shown), js(&d.to_string_lossy().replace(char::from(92), "/")), js(&ov), ch.join(","), dlls.join(",")));
        }
    } }
    json(s, &format!("[{}]", rows.join(",")));
}
/// 모드가 「게임 폴더」로 보게 될 사본: exe + steam_api 만(약 95MB, 한 번)
fn game_copy(cfg: &Cfg) -> std::io::Result<PathBuf> {
    let dst = cfg.root.join("modgame");
    std::fs::create_dir_all(dst.join("mods"))?;
    for n in ["TeamfightManager2.exe", "steam_api64.dll", "steam_api.dll"] {
        let (a, b) = (cfg.game.join(n), dst.join(n));
        let Ok(m) = std::fs::metadata(&a) else { continue };
        let same = std::fs::metadata(&b).map_or(false, |t| t.len() == m.len() && t.modified().ok() >= m.modified().ok());
        if !same { std::fs::copy(&a, &b)?; }
    }
    Ok(dst.join("TeamfightManager2.exe"))
}
/// 모드 폴더(게임 mods/<id>)에서 dll·설정만 modbox/<id> 로 복사. 정션·링크는 따라가지 않는다.
fn sandbox_mod(cfg: &Cfg, dll: &str) -> std::io::Result<String> {
    let dll = std::fs::canonicalize(dll)?;
    let mods = mod_roots(cfg).iter().filter_map(|r| std::fs::canonicalize(r).ok()).find(|r| dll.starts_with(r)).ok_or_else(|| std::io::Error::other("모드 폴더 밖"))?;
    let rel = dll.strip_prefix(&mods).map_err(|_| std::io::Error::other("경로"))?.to_path_buf();
    let id = rel.components().next().ok_or_else(|| std::io::Error::other("경로"))?.as_os_str().to_owned();
    let dst = cfg.root.join("modgame/mods").join(&id);
    copy_mod(&mods.join(&id), &dst, 0)?;
    Ok(cfg.root.join("modgame/mods").join(&rel).to_string_lossy().replace(char::from(92), "/"))
}
fn copy_mod(src: &Path, dst: &Path, depth: usize) -> std::io::Result<()> {
    use std::os::windows::fs::MetadataExt;
    if depth > 4 { return Ok(()); }
    std::fs::create_dir_all(dst)?;
    for e in std::fs::read_dir(src)?.flatten() {
        let m = match std::fs::symlink_metadata(e.path()) { Ok(m) => m, Err(_) => continue };
        if m.file_attributes() & 0x400 != 0 { continue; } // 재분석 지점(정션·링크)
        let n = e.file_name(); let name = n.to_string_lossy().to_ascii_lowercase();
        if m.is_dir() { if !name.starts_with('_') && !["aseprite_resources", "sprite", "sound", "s"].contains(&name.as_str()) { copy_mod(&e.path(), &dst.join(&n), depth + 1)?; } continue; }
        let ext = name.rsplit('.').next().unwrap_or("");
        if !["dll", "cfg", "json", "txt", "toml", "ini", "mod_info", "i18n"].contains(&ext) || m.len() > 64_000_000 { continue; }
        let to = dst.join(&n);
        let same = std::fs::metadata(&to).map_or(false, |t| t.len() == m.len() && t.modified().ok() >= m.modified().ok());
        if !same { std::fs::copy(e.path(), &to)?; }
    }
    Ok(())
}
/// 설정 종류 → 지금 게임 번들의 원본 JSON(re/tools/update_assets.py)
fn setting_src(cfg: &Cfg, k: &str) -> Option<PathBuf> {
    if let Some(id) = k.strip_prefix("dc/") { if !is_hash(id) { return None; } return Some(cfg.root.join("assets/setting_src").join(format!("setting__{id}.data_champion"))); }
    let n = match k { "gs" => "setting__game_setting.game_setting", "ci" => "setting__champion_info.champion_info_sheet", "is" => "setting__item_setting.item_setting", _ => return None };
    Some(cfg.root.join("assets/setting_src").join(n))
}
/// 모드가 있는 폴더들: 게임 mods + 창작마당(…/steamapps/workshop/content/3009300)
fn mod_roots(cfg: &Cfg) -> Vec<PathBuf> {
    let mut v = vec![cfg.root.join("devmods"), cfg.game.join("mods")]; // devmods = 만들고 있는 모드(게임 폴더에 안 넣고 시험)
    if let Some(sa) = cfg.game.parent().and_then(|p| p.parent()) { v.push(sa.join("workshop/content/3009300")); }
    v.into_iter().filter(|p| p.is_dir()).collect()
}
/// 게임 mods 폴더 안에 실제로 있는 파일만 허용
fn mod_file(cfg: &Cfg, p: &str, ext: &str) -> bool {
    let Ok(c) = std::fs::canonicalize(p) else { return false };
    mod_roots(cfg).iter().filter_map(|r| std::fs::canonicalize(r).ok()).any(|g| c.starts_with(&g)) && c.extension().map_or(false, |x| x == ext)
}
fn inspect(s: &mut TcpStream, cfg: &Cfg, save: &str) {
    let Some(p) = find_save(cfg, save) else { txt(s, 400, "세이브를 찾을 수 없음"); return };
    let m = std::fs::metadata(&p).ok();
    let key = format!("{:x}_{:x}", m.as_ref().map(|m| m.len()).unwrap_or(0), m.and_then(|m| m.modified().ok()).and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_secs()).unwrap_or(0));
    // _2 = 리그(lg)·팀 이름(teams)이 든 형식(0.1.2~) — 옛 캐시는 다시 읽는다
    let out = cfg.root.join("cache").join(format!("{key}_2.json"));
    if !out.exists() {
        let r = std::process::Command::new(&cfg.worker).arg(&p).arg(cfg.game.join("TeamfightManager2.exe")).env("LAB_INSPECT", &out).output();
        match r { Ok(o) if o.status.success() && out.exists() => {} Ok(o) => { let mut l: Vec<String> = String::from_utf8_lossy(&o.stdout).lines().rev().take(3).map(String::from).collect(); if let Some(w) = exit_why(&o) { l.insert(0, w); } txt(s, 500, &format!("세이브 읽기 실패: {}", l.join(" / "))); return; } Err(e) => { txt(s, 500, &format!("계산기 실행 실패: {e}")); return; } }
    }
    match std::fs::read(&out) { Ok(b) => send(s, 200, "application/json; charset=utf-8", &b), Err(e) => txt(s, 500, &e.to_string()) }
}
/// 계산기(lab_worker)가 비정상으로 끝났을 때의 이유 한 줄(정상 실패 = 종료 코드 1 은 None)
fn exit_why(o: &std::process::Output) -> Option<String> {
    if o.status.success() { return None; }
    let Some(c) = o.status.code() else { return Some("LAB 계산기가 강제로 끝남 / simulator was killed".into()) };
    let c = c as u32;
    if c == 1 { return None; }
    let oom = String::from_utf8_lossy(&o.stderr).contains("memory allocation");
    let why = match c {
        3 => "1200초 안에 끝나지 않아 멈춤 / did not finish in 1200 s",
        _ if oom || c == 0xC000_0017 || c == 0xC000_012D => "메모리 부족 — 게임·브라우저 탭 등 다른 프로그램을 닫고 다시 해 보세요 / out of memory — close the game and other programs, then retry",
        0xC000_0005 => "게임 코드가 읽을 수 없는 데이터를 만남 / access violation in game code",
        0xC000_00FD => "스택 넘침 / stack overflow",
        0xC000_0409 => "게임 코드가 스스로 중단(메모리 부족이거나 내부 오류) / game code aborted (out of memory or internal error)",
        _ => "",
    };
    Some(format!("LAB 계산기 비정상 종료 / simulator crashed 0x{c:08X} {why}"))
}
const OK_WORDS: [&str; 14] = ["setting", "setting_full", "fileblob", "key", "seed", "swap", "stat", "items", "only", "mode", "until", "sample", "at", "file"];
fn run(s: &mut TcpStream, cfg: &Cfg, jobs: &Jobs, body: &str) {
    let mut save = None;
    let (mut mod_dlls, mut lines, mut trace, mut every) = (vec![], vec![], false, 3usize);
    for l in body.lines() {
        let l = l.trim();
        if l.is_empty() { continue; }
        if let Some(v) = l.strip_prefix("save=") { save = find_save(cfg, v); continue; }
        if let Some(v) = l.strip_prefix("dll=") { if !mod_file(cfg, v, "dll") { txt(s, 400, "허용되지 않은 dll 경로"); return; } mod_dlls.push(v.to_string()); continue; }
        if let Some(v) = l.strip_prefix("every=") { every = v.parse().unwrap_or(3).clamp(1, 60); continue; }
        if l == "trace=1" { trace = true; continue; }
        let w = l.split_whitespace().next().unwrap_or("");
        if !OK_WORDS.contains(&w) { txt(s, 400, &format!("허용되지 않은 줄: {w}")); return; }
        if w == "file" { let rest = l[4..].trim(); let path = rest.strip_suffix(" replace").or(rest.strip_suffix(" fill")).unwrap_or(rest).trim(); if !mod_file(cfg, path, "data_champion") && !Path::new(path).starts_with(cfg.root.join("saves")) { txt(s, 400, "허용되지 않은 챔피언 파일 경로"); return; } }
        if let Some(rest) = l.strip_prefix("setting_full ") {
            let Some((k, n)) = rest.split_once(' ') else { txt(s, 400, "setting_full"); return };
            let n = n.trim();
            if setting_src(cfg, k).is_none() || !is_hash(n.trim_end_matches(".json")) || !n.starts_with(&format!("blob_{k}_")) { txt(s, 400, "setting_full 파일"); return; }
            lines.push(format!("setting_full {k} {}", cfg.root.join("cache").join(n).to_string_lossy().replace(char::from(92), "/")));
            continue;
        }
        if let Some(n) = l.strip_prefix("fileblob ") {
            // 병합한 기본 데이터 챔피언(화면이 만든 JSON)
            let n = n.trim();
            if !is_hash(n.trim_end_matches(".json")) || !n.starts_with("blob_dc_") { txt(s, 400, "fileblob"); return; }
            lines.push(format!("file {} replace", cfg.root.join("cache").join(n).to_string_lossy().replace(char::from(92), "/")));
            continue;
        }
        if let Some(rest) = l.strip_prefix("setting ") {
            let Some((k, n)) = rest.split_once(' ') else { txt(s, 400, "setting"); return };
            let Some(src) = setting_src(cfg, k) else { txt(s, 400, "setting 종류"); return };
            if !is_hash(n.trim().trim_end_matches(".json")) || !n.trim().starts_with(&format!("blob_{k}_")) { txt(s, 400, "setting 파일"); return; }
            lines.push(format!("setting {k} {}|{}", src.to_string_lossy().replace(char::from(92), "/"), cfg.root.join("cache").join(n.trim()).to_string_lossy().replace(char::from(92), "/")));
            continue;
        }
        lines.push(l.to_string());
    }
    let Some(save) = save else { txt(s, 400, "세이브를 찾을 수 없음"); return };
    // 모드는 앱 폴더로 복사해 올린다(모드가 쓰는 로그·설정이 게임 폴더를 건드리지 않게)
    let mut boxed = vec![];
    for d in &mod_dlls { match sandbox_mod(cfg, d) { Ok(p) => boxed.push(p), Err(e) => { txt(s, 500, &format!("모드 복사 실패 {d}: {e}")); return; } } }
    let mod_dlls = boxed;
    let exe = if mod_dlls.is_empty() { cfg.game.join("TeamfightManager2.exe") } else { match game_copy(cfg) { Ok(p) => p, Err(e) => { txt(s, 500, &format!("게임 사본 준비 실패: {e}")); return; } } };
    let id = format!("{:x}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis()).unwrap_or(0));
    let dir = cfg.root.join("runs").join(&id);
    let _ = std::fs::create_dir_all(&dir);
    let f = |n: &str| dir.join(n).to_string_lossy().replace(char::from(92), "/");
    lines.push(format!("frames {every} {}", f("frames.bin")));
    lines.push(format!("out {}", f("run.ndjson")));
    if std::fs::write(dir.join("scenario.txt"), lines.join("\n") + "\n").is_err() { txt(s, 500, "시나리오 쓰기 실패"); return; }
    jobs.lock().unwrap().insert(id.clone(), Job { state: "running".into(), log: String::new() });
    let (jobs2, id2, worker, sc, dll, pg) = (jobs.clone(), id.clone(), cfg.worker.clone(), f("scenario.txt"), mod_dlls.join("|"), f("progress.txt"));
    std::thread::spawn(move || {
        let mut c = std::process::Command::new(worker);
        c.arg(&save).arg(&exe).env("LAB_SCENARIO", &sc).env("LAB_MODS", &dll).env("LAB_PROGRESS", &pg);
        if trace { c.env("LAB_TRACE", "1").env("LAB_TRACE_TXT", "1").env("LAB_TRACE_TO", "999999999"); }
        let r = c.output();
        let _ = std::fs::remove_file(&pg);
        let (st, log) = match r {
            Ok(o) => { let mut t = String::from_utf8_lossy(&o.stdout).into_owned() + &String::from_utf8_lossy(&o.stderr); if let Some(w) = exit_why(&o) { t.push('\n'); t.push_str(&w); } (if o.status.success() && t.contains("LAB OK") { "done" } else { "fail" }, t) }
            Err(e) => ("fail", e.to_string()),
        };
        let keep: String = log.lines().filter(|l| l.contains("LAB") || l.contains("VEH") || l.contains("panick") || l.contains("memory allocation")).collect::<Vec<_>>().join("\n");
        if let Some(j) = jobs2.lock().unwrap().get_mut(&id2) { j.state = st.into(); j.log = keep.lines().filter(|l| !l.starts_with("LAB 변경")).collect::<Vec<_>>().join("
").chars().take(6000).collect(); }
    });
    json(s, &format!("{{\"id\":{}}}", js(&id)));
}
fn run_get(s: &mut TcpStream, cfg: &Cfg, jobs: &Jobs, rest: &str) {
    let (id, file) = rest.split_once('/').unwrap_or((rest, ""));
    if !is_hash(id) { txt(s, 400, "id"); return; }
    if file.is_empty() {
        match jobs.lock().unwrap().get(id) { Some(j) => { let t: i64 = std::fs::read_to_string(cfg.root.join("runs").join(id).join("progress.txt")).ok().and_then(|x| x.trim().parse().ok()).unwrap_or(-1); json(s, &format!("{{\"state\":{},\"tick\":{t},\"log\":{}}}", js(&j.state), js(&j.log))) } None => json(s, "{\"state\":\"unknown\",\"log\":\"\"}") }
        return;
    }
    if !["run.ndjson", "frames.bin", "scenario.txt"].contains(&file) { txt(s, 404, "없음"); return; }
    match std::fs::read(cfg.root.join("runs").join(id).join(file)) { Ok(b) => send(s, 200, "application/octet-stream", &b), Err(_) => txt(s, 404, "없음") }
}
