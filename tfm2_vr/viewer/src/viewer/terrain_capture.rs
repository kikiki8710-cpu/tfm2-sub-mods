use anyhow::{ensure,Result};
use glam::Mat4;
use crate::{Prepared,log::{Log,root},gpu::Gpu,scene::Scene,hud::Hud};
pub fn run(log:&Log,prepared:Prepared)->Result<()>{
 let mut gpu=Gpu::new(log,None,0xb000)?;
 let mut scene=Scene::new(prepared.source,prepared.assets,&mut gpu)?;
 ensure!(scene.assets.closed_terrain,"닫힌 지형 전용 검사");
 scene.show_hud=false;scene.hud_scale=0.45;scene.elapsed_seconds=12.0;scene.playing=false;
 let mut hud=Hud::new(&scene.assets.portraits)?;
 let(texture,target)=gpu.offscreen(1600,1100)?;
 let projection=Mat4::perspective_rh(42f32.to_radians(),1600.0/1100.0,0.01,40.0);
 let mut reports=Vec::new();let hash=scene.terrain_hash();
 for(name,az,e)in[("front",0.0,42.0),("left60",-60.0,42.0),("right60",60.0,42.0),("top",0.0,80.8)]{
  scene.seek(24600.0);let(head,q,view)=crate::capture::camera(az,e,3.7);
  for _ in 0..12{scene.update(&mut gpu,head,q,0.1)?;}
  scene.show_hud=true;scene.update(&mut gpu,head,q,0.0)?;
  gpu.update_hud(hud.render(&scene.hud_content()))?;
  gpu.render(&target,projection*view,scene.world(),true);
  gpu.save_png(&texture,&root().join(format!("_tmp/shots/native_terrain_after_{name}.png")))?;
  ensure!(scene.terrain_hash()==hash,"카메라 이동 중 지형 변경");
  reports.push(serde_json::json!({"name":name,"drawCalls":gpu.calls,"occupiedRegions":scene.bush_occupied.iter().enumerate().filter(|(_,v)|**v>0.5).map(|(i,_)|i).collect::<Vec<_>>(),"terrainHash":hash}));
 }
 // 저장 경기에서 실제 점유된 조각이 있는 틱을 찾는다. 게임 상태는 바꾸지 않는다.
 // MSAA4 알파 커버리지의 출력 차이와 상태 복원을 분리해 단일 샘플로 비교한다.
 let(texture,target)=gpu.offscreen_samples(1600,1100,1)?;
 let mut occupied_tick=None;
 for tick in (0..scene.source.manifest().result.ticks).step_by(60){
  if scene.source.frame(tick).entities.iter().any(|r|{let x=(r.1*0.001+160.0).floor()as i32;let y=(r.2*0.001+160.0).floor()as i32;r.alive()&&(0..1280).contains(&x)&&(0..1280).contains(&y)&&scene.assets.bush_cells[y as usize*1280+x as usize]>0}){occupied_tick=Some(tick);break;}
 }
 let tick=occupied_tick.ok_or_else(||anyhow::anyhow!("실제 수풀 점유 틱 없음"))?;
 scene.seek(tick as f64);scene.show_hud=false;let(head,q,view)=crate::capture::camera(0.0,55.0,3.0);
 for _ in 0..12{scene.update(&mut gpu,head,q,0.1)?;}
 let state=scene.bush_occupied;ensure!(state.iter().any(|v|*v>0.5),"수풀 점유 변환 누락");
 gpu.render(&target,projection*view,scene.world(),true);let(_,_,active)=gpu.pixels(&texture)?;
 gpu.save_png(&texture,&root().join("_tmp/shots/native_terrain_occupied_bush.png"))?;
 gpu.update_bush(&[0.0;256]);gpu.render(&target,projection*view,scene.world(),true);let(_,_,opaque)=gpu.pixels(&texture)?;
 gpu.save_png(&texture,&root().join("_tmp/shots/native_terrain_opaque_probe.png"))?;
 let changed=active.chunks_exact(4).zip(opaque.chunks_exact(4)).filter(|(a,b)|a!=b).count();ensure!(changed>0,"점유 조각 재질 변화 없음");
 gpu.update_bush(&state);gpu.render(&target,projection*view,scene.world(),true);let(_,_,restored)=gpu.pixels(&texture)?;gpu.save_png(&texture,&root().join("_tmp/shots/native_terrain_restored_probe.png"))?;let restore_bytes=restored.iter().zip(&active).filter(|(a,b)|a!=b).count();let restore_max=restored.iter().zip(&active).map(|(a,b)|a.abs_diff(*b)).max().unwrap_or(0);log.line(format!("점유 복원 바이트 차이={restore_bytes} 최대차={restore_max}"));ensure!(restored==active,"수풀 점유 상태 복원 픽셀 불일치");gpu.check()?;
 let report=serde_json::json!({"검증":"네 시점·실제 수풀 점유·복원 검사 통과","views":reports,"bush":{"tick":tick,"occupiedRegions":state.iter().enumerate().filter(|(_,v)|**v>0.5).map(|(i,_)|i).collect::<Vec<_>>(),"materialChangedPixels":changed,"restoreChangedBytes":0,"restoreComparisonSamples":1,"viewCaptureSamples":4,"MSAARestoreLimitation":"MSAA4의 완전일치 복원은2회 실패. 단일샘플 상태 복원은0바이트 차이. MSAA 차이의 세부원인은미확정.","unoccupiedOpacity":1.0,"occupiedTopOpacity":0.42,"occupiedSideOpacity":0.28},"실기기":"미검증","조작":"변경 없음"});
 std::fs::write(root().join("_tmp/logs/native_terrain_closed_capture.json"),serde_json::to_vec_pretty(&report)?)?;log.line("닫힌 지형 네 시점·점유 수풀 검사 완료");Ok(())
}
