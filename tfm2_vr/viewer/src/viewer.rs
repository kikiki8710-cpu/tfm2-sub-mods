#[allow(dead_code)] #[path="log.rs"] mod log;
#[path="viewer/source.rs"] mod source;
#[path="viewer/geometry.rs"] mod geometry;
#[path="viewer/assets.rs"] mod assets;
#[path="viewer/facing.rs"] mod facing;
#[path="viewer/gpu.rs"] mod gpu;
#[path="viewer/scene.rs"] mod scene;
#[path="viewer/hud.rs"] mod hud;
#[path="viewer/capture.rs"] mod capture;
#[path="viewer/perf.rs"] mod perf;
#[path="viewer/window.rs"] mod window;
#[path="viewer/desktop.rs"] mod desktop;
#[path="viewer/input.rs"] mod input;
#[path="viewer/xrflow.rs"] mod xrflow;
#[path="viewer/validation.rs"] mod validation;
#[path="viewer/visual_assets.rs"] mod visual_assets;
#[path="viewer/visual_validation.rs"] mod visual_validation;
#[path="viewer/settings.rs"] mod settings;
#[path="viewer/mips.rs"] mod mips;
#[path="viewer/manipulation.rs"] mod manipulation;
#[path="viewer/quality_capture.rs"] mod quality_capture;
#[path="viewer/projectiles.rs"] mod projectiles;
#[path="viewer/projectile_capture.rs"] mod projectile_capture;
#[path="viewer/hud_stream.rs"] mod hud_stream;
#[path="viewer/profile.rs"] mod profile;
#[path="viewer/hud_pipeline_validation.rs"] mod hud_pipeline_validation;
#[path="viewer/terrain_capture.rs"] mod terrain_capture;
#[path="viewer/bullet_capture.rs"] mod bullet_capture;
#[path="viewer/combat_capture.rs"] mod combat_capture;
#[path="viewer/npc_capture.rs"] mod npc_capture;
#[path="viewer/effect_demo.rs"] mod effect_demo;
#[path="viewer/kind_capture.rs"] mod kind_capture;
#[path="viewer/event_capture.rs"] mod event_capture;
#[path="viewer/pivot_capture.rs"] mod pivot_capture;
#[path="viewer/live/mod.rs"] mod live;
#[path="viewer/live_capture.rs"] mod live_capture;
#[path="viewer/play/mod.rs"] mod play;
#[path="viewer/lang.rs"] mod lang;
use anyhow::Result;
use source::FrameSource;
pub struct Prepared{pub source:std::sync::Arc<dyn source::FrameSource>,pub assets:assets::Assets}
fn load_async(generated:bool)->std::sync::mpsc::Receiver<Result<Prepared>>{let legacy=std::env::args().any(|a|a=="--legacy-data");let(tx,rx)=std::sync::mpsc::sync_channel(1);std::thread::spawn(move||{let result=(||{let cache=log::root().join("_cache");let source=std::sync::Arc::new(if legacy{source::SavedMatch::load_named(&cache,"replay")?}else{source::SavedMatch::load(&cache)?});let assets=assets::Assets::load(&cache,source.manifest(),generated)?;Ok(Prepared{source,assets})})();let _=tx.send(result);});rx}
fn main(){
 let log=match log::Log::named("native_viewer"){Ok(l)=>l,Err(e)=>{eprintln!("뷰어 로그 생성 실패: {e:#}");std::process::exit(1)}};
 let result:Result<()>=(||{
  let args:Vec<String>=std::env::args().collect();let mode=args.get(1).map(String::as_str).unwrap_or("");
  if mode=="--desktop"{return desktop::run(&log,&args);}
  lang::init(&log);
  if mode=="--play"{return play::xr::run(&log,&args);}
  if mode=="--play-desktop"{return play::desktop::run(&log,&args);}
  if mode=="--ui-compare"{return play::desktop::compare(&log,&args);}
  if mode=="--ui-jitter"{return play::desktop::jitter(&log,&args);}
  if mode=="--ui-hover"{return play::desktop::hover(&log,&args);}
  if mode=="--ui-play"{return play::desktop::play(&log,&args);}
  if mode=="--live-compare"{return live_capture::run(&log,&args);}
  if mode=="--session"{return xrflow::run(&log,&args);}
  if !matches!(mode,"--offscreen"|"--kind-compare"|"--event-compare"|"--pivot-compare"|"--quality-compare"|"--projectile-compare"|"--hud-validate"|"--validate-data"|"--terrain-compare"|"--bullet-art-compare"|"--combat-ui-compare"|"--ui-layout-compare"|"--npc-art-compare"|"--layout-hud-validate"|"--effect-demo-compare"|"--ui-finish-compare"|"--ui-finish-hud-validate"){println!("사용법: tfm2_vr_viewer.exe --desktop | --offscreen | --quality-compare | --projectile-compare | --hud-validate | --terrain-compare | --session | --validate-data");return Ok(());}let prepared=load_async(args.iter().any(|a|a=="--generated-textures")).recv()??;
  if mode=="--pivot-compare"{return pivot_capture::run(&log,prepared);}
  if mode=="--event-compare"{return event_capture::run(&log,prepared);}
  if mode=="--kind-compare"{return kind_capture::run(&log,prepared);}
  if mode=="--effect-demo-compare"{return effect_demo::run(&log,prepared);}
  if mode=="--npc-art-compare"{return npc_capture::run(&log,prepared);}
  if mode=="--ui-finish-compare"{return combat_capture::run(&log,prepared);}
  if mode=="--ui-layout-compare"{return combat_capture::run(&log,prepared);}
  if mode=="--combat-ui-compare"{return combat_capture::run(&log,prepared);}
  if mode=="--bullet-art-compare"{return bullet_capture::run(&log,prepared);}
  if mode=="--terrain-compare"{return terrain_capture::run(&log,prepared);}
  if mode=="--ui-finish-hud-validate"{return hud_pipeline_validation::run(&log,prepared);}
  if mode=="--layout-hud-validate"{return hud_pipeline_validation::run(&log,prepared);}
  if mode=="--hud-validate"{return hud_pipeline_validation::run(&log,prepared);}
  if mode=="--projectile-compare"{return projectile_capture::run(&log,prepared);}
  if mode=="--quality-compare"{return quality_capture::run(&log,prepared);}
  if mode=="--offscreen"{return capture::offscreen(&log,prepared);}
  let snapshots:Vec<_>=[1800,24600,40987].into_iter().map(|t|{let f=prepared.source.frame(t);serde_json::json!({"tick":t,"kills":f.kills,"units":f.entities.len()})}).collect();
  let report=serde_json::json!({"검증":"자료준비 통과","버전":prepared.source.manifest().game_version,"프레임":prepared.source.manifest().result.frames,"메타":prepared.source.manifest().catalog.len(),"형식":prepared.source.manifest().version,"최대투사체":prepared.source.max_projectiles(),"아틀라스페이지":prepared.assets.pages.len(),"원본전용":!prepared.assets.generated,"시점":snapshots});
  std::fs::write(log::root().join("_tmp/logs/native_viewer_data.json"),serde_json::to_vec_pretty(&report)?)?;println!("{report}");Ok(())
 })();match result{Ok(())=>log.line("네이티브 뷰어 실행 완료"),Err(e)=>{log.line(format!("네이티브 뷰어 오류: {e:#}"));std::process::exit(1);}}
}
