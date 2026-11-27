use std::sync::atomic::{AtomicU64,Ordering};
static CHUNK_DECODES:AtomicU64=AtomicU64::new(0);
pub fn chunk_decodes()->u64{CHUNK_DECODES.load(Ordering::Relaxed)}
// 파일 형식 v1의 읽기 전용 프레임 공급자. 화면/API/게임 실행과 독립적이다.
use anyhow::{Context,Result,bail};
use serde::{Deserialize,Serialize};
use std::{collections::HashMap,fs::File,io::{BufReader,Read},path::Path,sync::{Arc,mpsc},thread};

#[derive(Clone,Copy,Debug,Deserialize,Serialize)]
pub struct EntityRow(pub u32,pub f64,pub f64,pub f32,pub f32,pub i32,pub u8,pub u32,pub f32);
#[derive(Clone,Copy,Debug,Deserialize,Serialize)]pub struct ProjectileRow(pub u64,pub f64,pub f64,pub u32,pub i32,pub u8);
#[derive(Clone,Copy,Debug,Deserialize,Serialize)]pub struct ProjectileKind(pub u64,pub u32,pub u8,pub Option<u32>);
/// v5: 내부 ID, 종류 번호(manifest.projectileKinds), 생성 틱, 이동 종류(2 직선·4 지속·6 추적). projectiles 와 같은 순서.
#[derive(Clone,Copy,Debug,Deserialize,Serialize)]pub struct ProjectileInternal(pub u32,pub u16,pub u32,pub u8);
/// 추적 투사체가 사라진 첫 틱과 마지막 위치(목표에 닿은 것으로 본 추정).
/// v5: 챔피언 행동 시작(틱, 핸들, 1 평타·2 기술1·3 기술2·4 궁).
#[derive(Clone,Copy,Debug,Deserialize,Serialize)]pub struct ActionRow(pub u32,pub u32,pub u8);
/// v5: 이름 붙은 이펙트(틱, 이름 번호, 대상 핸들|0, 시전자 핸들|0, x, y).
#[derive(Clone,Copy,Debug,Deserialize,Serialize)]pub struct EffectRow(pub u32,pub u16,pub u32,pub u32,pub f64,pub f64);
#[derive(Clone,Copy,Debug)]pub struct Hit{pub tick:u32,pub kind:u16,pub x:f64,pub y:f64,pub team:i32}
impl EntityRow {pub fn id(self)->usize{self.0 as usize} pub fn alive(self)->bool{self.6!=0&&self.3>0.0}}
#[derive(Debug,Deserialize,Serialize)]
pub struct Frame {#[serde(default)]pub ui:Option<CombatFrame>,pub tick:u32,pub kills:[u32;2],pub entities:Vec<EntityRow>,pub players:Vec<(u32,f64)>,#[serde(default)]pub projectiles:Vec<ProjectileRow>,#[serde(default,rename="projectileCount")]pub projectile_count:u32,#[serde(default)]pub cooldowns:Vec<(u32,u32,u32,u32,u32)>,#[serde(default,rename="projectileKinds")]pub projectile_kinds:Vec<ProjectileKind>,#[serde(default)]pub pk:Vec<ProjectileInternal>}

#[derive(Clone,Debug,Deserialize,Serialize)]pub struct PlayerUi(pub u32,pub u32,pub u32,pub u32,pub u32,pub u32,pub u8,pub u32,pub u32,pub u32,pub u32,pub u32,pub u32,pub u32,pub u32,pub u32);
#[derive(Clone,Debug,Deserialize,Serialize)]pub struct CcRow(pub u32,pub u32,pub u64,pub String,pub i64,pub i64,pub u64,pub u64);
#[derive(Clone,Debug,Deserialize,Serialize)]pub struct KillRow(pub u32,pub u32,pub u32,pub u32,pub Vec<u32>);
#[derive(Clone,Debug,Deserialize,Serialize)]pub struct ObjectiveRow(pub u32,pub u32,pub String,pub i32,pub u8);
#[derive(Debug,Default,Deserialize,Serialize)]pub struct CombatFrame{#[serde(rename="scoreDiff")]pub score_diff:[i32;2],pub players:Vec<PlayerUi>,pub cc:Vec<CcRow>,#[serde(rename="killLogNew")]pub kill_log_new:Vec<KillRow>,#[serde(default,rename="objectivesNew")]pub objectives_new:Vec<ObjectiveRow>}

#[derive(Debug,Deserialize,Serialize)]
pub struct EntityMeta {pub id:u32,pub kind:String,pub name:String}
#[derive(Debug,Deserialize,Serialize)]
pub struct MatchResult {pub ticks:u32,#[serde(rename="blueWin")]pub blue_win:bool,#[serde(rename="playerKills")]pub player_kills:[u32;2],pub frames:u32}
#[derive(Debug,Deserialize,Serialize)]
pub struct Manifest {#[serde(default,rename="projectileKinds")]pub projectile_kinds:Vec<String>,#[serde(default)]pub actions:Vec<ActionRow>,#[serde(default)]pub effects:Vec<EffectRow>,#[serde(default,rename="effectNames")]pub effect_names:Vec<String>,#[serde(default,rename="itemSets")]pub item_sets:HashMap<u32,Vec<String>>,pub version:u32,#[serde(rename="gameVersion")]pub game_version:String,pub roster:[Vec<String>;2],pub seed:String,pub result:MatchResult,pub fps:u32,#[serde(rename="chunkCount")]pub chunk_count:usize,pub catalog:HashMap<u32,EntityMeta>}
pub struct FramePair<'a>{pub a:&'a Frame,pub b:&'a Frame,pub alpha:f64}
pub trait FrameSource:Send+Sync {
 fn manifest(&self)->&Manifest;
 fn pair(&self,time:f64)->FramePair<'_>;
 fn frame(&self,tick:u32)->&Frame;
 fn max_projectiles(&self)->usize;
 fn recent_hits(&self,_tick:u32)->&[Hit]{&[]}
 /// 그 틱까지 시작한 가장 최근 행동(시작 틱, 종류).
 fn last_action(&self,_handle:u32,_tick:u32)->Option<(u32,u8)>{None}
 fn recent_effects(&self,tick:u32)->&[EffectRow]{let e=&self.manifest().effects;let end=e.partition_point(|v|v.0<=tick);let start=e[..end].partition_point(|v|v.0<tick.saturating_sub(180));&e[start..end]}
 fn recent_kills(&self,_tick:u32)->&[KillRow]{&[]}
 fn recent_objectives(&self,_tick:u32)->&[ObjectiveRow]{&[]}
 fn max_id(&self)->usize{self.manifest().catalog.keys().copied().max().unwrap_or(0)as usize}
}
pub struct SavedMatch {actions:HashMap<u32,Vec<(u32,u8)>>,hits:Vec<Hit>,pub meta:Manifest,frames:Vec<Frame>,max_projectiles:usize,kill_logs:Vec<KillRow>,kill_observed:Vec<u32>,objectives:Vec<ObjectiveRow>}
impl SavedMatch {
 pub fn load(cache:&Path)->Result<Self>{
  Self::load_named(cache,if cache.join("replay_v5/manifest.json").exists()&&!std::env::args().any(|a|a=="--replay-v4"){"replay_v5"}else if cache.join("replay_v4/manifest.json").exists(){"replay_v4"}else if cache.join("replay_v3/manifest.json").exists(){"replay_v3"}else if cache.join("replay_v2/manifest.json").exists(){"replay_v2"}else{"replay"})
 }
 pub fn load_named(cache:&Path,name:&str)->Result<Self>{
  if !["replay","replay_v2","replay_v3","replay_v4","replay_v5"].contains(&name){bail!("허용하지 않은 로컬 경기 경로");}let directory=cache.join(name);
  let meta:Manifest=serde_json::from_reader(BufReader::new(File::open(directory.join("manifest.json"))?))?;
  if ![1,2,3,4,5].contains(&meta.version)||meta.game_version!="0.7.3"||meta.fps!=60{bail!("지원하지 않는 경기 형식/버전");}
  let mut frames=Vec::with_capacity(meta.result.frames as usize);
  for index in 0..meta.chunk_count {
   let path=directory.join(format!("chunks/{index:04}.json.gz"));
   let reader=flate2::read::GzDecoder::new(BufReader::new(File::open(&path).with_context(||format!("청크 열기: {}",path.display()))?));
   let mut chunk:Vec<Frame>=serde_json::from_reader(reader)?;CHUNK_DECODES.fetch_add(1,Ordering::Relaxed);frames.append(&mut chunk);
  }
  if frames.len()!=meta.result.frames as usize||frames.last().map(|f|f.tick)!=Some(meta.result.ticks){bail!("경기 프레임 수/끝 틱 불일치");}
  for(i,f)in frames.iter().enumerate(){if f.tick as usize!=i{bail!("불연속 경기 틱: {i}");}}
  if frames.last().unwrap().kills!=meta.result.player_kills{bail!("끝 점수 불일치");}
  let max_projectiles=frames.iter().map(|f|f.projectiles.len()).max().unwrap_or(0);if max_projectiles>10000{bail!("투사체 수 상한 초과");}let kill_observed=frames.iter().flat_map(|f|f.ui.iter().flat_map(move|u|u.kill_log_new.iter().map(move |_|f.tick))).collect();let kill_logs=frames.iter().filter_map(|f|f.ui.as_ref()).flat_map(|u|u.kill_log_new.iter().cloned()).collect();let objectives=frames.iter().filter_map(|f|f.ui.as_ref()).flat_map(|u|u.objectives_new.iter().cloned()).collect();let hits=if meta.effects.is_empty(){find_hits(&frames)}else{Vec::new()};let mut actions:HashMap<u32,Vec<(u32,u8)>>=HashMap::new();for a in &meta.actions{if !(1..=4).contains(&a.2){bail!("행동 종류 범위 오류");}actions.entry(a.1).or_default().push((a.0,a.2));}for v in actions.values_mut(){v.sort_unstable();}if meta.effects.iter().any(|e|e.1 as usize>=meta.effect_names.len()){bail!("이펙트 이름 번호 범위 오류");}for f in &frames{if !f.pk.is_empty()&&(f.pk.len()!=f.projectiles.len()||f.pk.iter().any(|p|p.1 as usize>=meta.projectile_kinds.len())){bail!("투사체 종류 자료 불일치: {}",f.tick);}}Ok(Self{actions,hits,meta,frames,max_projectiles,kill_logs,kill_observed,objectives})
 }
 pub fn start_loading(cache:std::path::PathBuf)->mpsc::Receiver<Result<Arc<Self>>>{let(tx,rx)=mpsc::sync_channel(1);thread::spawn(move||{let _=tx.send(Self::load(&cache).map(Arc::new));});rx}
 pub fn max_id(&self)->usize{self.meta.catalog.keys().copied().max().unwrap_or(0)as usize}
}
impl FrameSource for SavedMatch {
 fn manifest(&self)->&Manifest{&self.meta}
 fn pair(&self,time:f64)->FramePair<'_>{let t=time.clamp(0.0,self.meta.result.ticks as f64);let n=t.floor()as usize;FramePair{a:&self.frames[n],b:&self.frames[(n+1).min(self.frames.len()-1)],alpha:t-n as f64}}
 fn frame(&self,tick:u32)->&Frame{&self.frames[(tick as usize).min(self.frames.len()-1)]}
 fn max_projectiles(&self)->usize{self.max_projectiles}
 fn last_action(&self,handle:u32,tick:u32)->Option<(u32,u8)>{let v=self.actions.get(&handle)?;let end=v.partition_point(|a|a.0<=tick);end.checked_sub(1).map(|i|v[i])}
 fn recent_hits(&self,tick:u32)->&[Hit]{let end=self.hits.partition_point(|v|v.tick<=tick);let start=self.hits[..end].partition_point(|v|v.tick<tick.saturating_sub(90));&self.hits[start..end]}
 fn recent_kills(&self,tick:u32)->&[KillRow]{let end=self.kill_observed.partition_point(|v|*v<=tick);let start=self.kill_observed[..end].partition_point(|v|*v<tick.saturating_sub(480));&self.kill_logs[start..end]}
 fn recent_objectives(&self,tick:u32)->&[ObjectiveRow]{let end=self.objectives.partition_point(|v|v.0<=tick);let start=self.objectives[..end].partition_point(|v|v.0<tick.saturating_sub(480));&self.objectives[start..end]}
}
fn find_hits(frames:&[Frame])->Vec<Hit>{
 let mut out=Vec::new();
 for w in frames.windows(2){let(a,b)=(&w[0],&w[1]);if a.pk.len()!=a.projectiles.len(){continue;}
  for(p,row)in a.pk.iter().zip(&a.projectiles){if p.3==6&&!b.pk.iter().any(|n|n.0==p.0&&n.2==p.2){out.push(Hit{tick:b.tick,kind:p.1,x:row.1,y:row.2,team:row.4});}}}
 out
}
pub fn read_json<T:serde::de::DeserializeOwned>(path:&Path)->Result<T>{
 let mut bytes=Vec::new();if path.extension().is_some_and(|s|s=="gz"){flate2::read::GzDecoder::new(File::open(path)?).read_to_end(&mut bytes)?;}else{File::open(path)?.read_to_end(&mut bytes)?;}serde_json::from_slice(&bytes).with_context(||format!("JSON 읽기: {}",path.display()))
}
#[cfg(test)] mod tests {use super::*;#[test]fn row_round_trip(){let r:EntityRow=serde_json::from_str("[19,480001,640020,852,900,-1,1,3,12]").unwrap();assert_eq!(r.1,480001.0);assert_eq!(r.5,-1);assert!(r.alive());}}

#[cfg(test)]mod timeline_tests{use super::*;#[test]fn kill_log_is_visible_only_after_observed_frame(){let meta=Manifest{projectile_kinds:Vec::new(),actions:vec![],effects:vec![],effect_names:vec![],item_sets:HashMap::new(),version:4,game_version:"0.7.3".into(),roster:[vec![],vec![]],seed:"0".into(),result:MatchResult{ticks:500,blue_win:false,player_kills:[0;2],frames:501},fps:60,chunk_count:0,catalog:HashMap::new()};let source=SavedMatch{actions:HashMap::new(),hits:vec![],meta,frames:vec![],max_projectiles:0,kill_logs:vec![KillRow(10,0,1,2,vec![])],kill_observed:vec![11],objectives:vec![]};assert!(source.recent_kills(10).is_empty());assert_eq!(source.recent_kills(11).len(),1);assert_eq!(source.recent_kills(491).len(),1);assert!(source.recent_kills(492).is_empty());}}
