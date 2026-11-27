use anyhow::{ensure,Context,Result};
use glam::{Mat4,Quat,Vec3};
use crate::{Prepared,log::{Log,root},gpu::Gpu,scene::{Scene,TABLE,PIXEL},hud::Hud};
fn update(scene:&mut Scene,gpu:&mut Gpu,hud:&mut Hud,head:Vec3,q:Quat)->Result<()>{scene.show_hud=true;for _ in 0..12{scene.update(gpu,head,q,0.1)?;}gpu.update_hud(hud.render(&scene.hud_content()))?;Ok(())}
pub fn run(log:&Log,prepared:Prepared)->Result<()>{
 let finish_mode=std::env::args().any(|a|a=="--ui-finish-compare");let layout_mode=finish_mode||std::env::args().any(|a|a=="--ui-layout-compare");let prefix=if finish_mode{"native_ui_finish"}else if layout_mode{"native_ui_layout"}else{"native_combat"};
 let mut gpu=Gpu::new(log,None,0xb000)?;let mut scene=Scene::new(prepared.source,prepared.assets,&mut gpu)?;
 scene.playing=false;scene.elapsed_seconds=12.0;scene.hud_scale=0.55;let mut hud=Hud::new(&scene.assets.portraits)?;
 let(texture,target)=gpu.offscreen(1800,1200)?;
 // 넓은 검사 카메라만 사용한다. 실제 OpenXR FOV는 변경하지 않는다.
 let projection=Mat4::perspective_rh(70f32.to_radians(),1.5,0.01,40.0);let mut views=Vec::new();
 for tick in[1800,24600,39600]{for(name,az)in[("front",0.0),("left60",-60.0),("right60",60.0)]{
  scene.seek(tick as f64);let(head,q,view)=crate::capture::camera(az,42.0,if layout_mode{5.4}else{3.7});update(&mut scene,&mut gpu,&mut hud,head,q)?;
  let audit=scene.combat_audit(head);ensure!(audit["panelInstances"]==2&&audit["mapInstances"]==1,"팀 패널/미니맵 누락");
  for a in audit["nominalPanelFontDegrees"].as_array().unwrap(){ensure!(a.as_f64().unwrap()>=1.0,"패널 기준 글자 각크기 미달");}
  gpu.mask(|_|true);gpu.render(&target,projection*view,scene.world(),true);gpu.save_png(&texture,&root().join(format!("_tmp/shots/{prefix}_t{tick}_{name}.png")))?;
  views.push(serde_json::json!({"tick":tick,"view":name,"FOVdegrees":70,"cameraDistance":if layout_mode{5.4}else{3.7},"audit":audit,"drawCalls":gpu.calls}));
  if name=="front"{
   let pose=Mat4::from_rotation_translation(scene.hud_pose().1,scene.hud_pose().0+scene.hud_pose().1*Vec3::new(0.0,0.17,0.52));
   gpu.mask(|n|n=="hud"||n=="killFeed");gpu.render(&target,Mat4::perspective_rh(55f32.to_radians(),1.5,0.01,40.0)*pose.inverse(),scene.world(),true);gpu.save_png(&texture,&root().join(format!("_tmp/shots/{prefix}_hud_t{tick}.png")))?;
   if tick==24600{for team in 0..2{
    let panel=scene.combat_panel_model(team).unwrap();let(s,q,p)=panel.to_scale_rotation_translation();let eye=p+q*Vec3::new(0.0,0.0,s.y*1.1);let vp=Mat4::perspective_rh(60f32.to_radians(),1.5,0.01,40.0)*Mat4::from_rotation_translation(q,eye).inverse();gpu.mask(|n|n=="playerPanels");gpu.render(&target,vp,scene.world(),true);gpu.save_png(&texture,&root().join(format!("_tmp/shots/{prefix}_panel_team{team}.png")))?;
   }
   let map=scene.combat_minimap_model().unwrap();let center=map.transform_point3(Vec3::ZERO);let view=Mat4::look_at_rh(center+Vec3::new(0.0,1.04,0.24),center,Vec3::Y);gpu.mask(|n|n=="minimap"||n=="miniDots");gpu.render(&target,Mat4::perspective_rh(50f32.to_radians(),1.5,0.01,40.0)*view,scene.world(),true);gpu.save_png(&texture,&root().join(format!("_tmp/shots/{prefix}_minimap_close.png")))?;
   }
  }
 }}
 // 실제 상태이상이 걸린 챔피언을 찾아 가까운 거리와 먼 거리 LOD를 대조한다.
 let cc_case=(0..=scene.source.manifest().result.ticks).find_map(|tick|{
  let f=scene.source.frame(tick);let ui=f.ui.as_ref()?;let cc=ui.cc.iter().find(|c|c.1<10&&scene.source.manifest().catalog.get(&c.0).is_some_and(|m|m.kind=="champion"))?;
  let row=f.entities.iter().find(|r|r.0==cc.0&&r.alive())?;Some((tick,*row,cc.1))
 }).context("실제 챔피언 CC 사례 없음")?;
 scene.seek(cc_case.0 as f64);let row=cc_case.1;let foot=Vec3::new(((row.1*0.001-480.0)*PIXEL as f64)as f32,TABLE+0.002,((row.2*0.001-480.0)*PIXEL as f64)as f32);
 let head=foot+Vec3::new(0.0,0.23,0.40);let view=Mat4::look_at_rh(head,foot+Vec3::Y*0.04,Vec3::Y);let q=view.inverse().to_scale_rotation_translation().1;update(&mut scene,&mut gpu,&mut hud,head,q)?;let near=scene.combat_audit(head);ensure!(near["labelInstances"].as_u64().unwrap()>0,"가까운 CC/레벨 라벨 없음");
 gpu.mask(|n|!["hud","killFeed","playerPanels","minimap","miniDots"].contains(&n));gpu.render(&target,Mat4::perspective_rh(55f32.to_radians(),1.5,0.01,40.0)*view,scene.world(),true);gpu.save_png(&texture,&root().join(format!("_tmp/shots/{prefix}_overhead_cc.png")))?;
 let shield_case=(0..=scene.source.manifest().result.ticks).find_map(|tick|scene.source.frame(tick).entities.iter().find(|e|e.alive()&&e.8>0.0&&scene.source.manifest().catalog[&e.0].kind=="champion").map(|row|(tick,*row)));
 let shield_audit=if let Some((tick,row))=shield_case{scene.seek(tick as f64);let foot=Vec3::new(((row.1*0.001-480.0)*PIXEL as f64)as f32,TABLE+0.002,((row.2*0.001-480.0)*PIXEL as f64)as f32);let h=foot+Vec3::new(0.0,0.23,0.40);let view=Mat4::look_at_rh(h,foot+Vec3::Y*0.04,Vec3::Y);let q=view.inverse().to_scale_rotation_translation().1;update(&mut scene,&mut gpu,&mut hud,h,q)?;gpu.mask(|n|!["hud","killFeed","playerPanels","minimap","miniDots"].contains(&n));gpu.render(&target,Mat4::perspective_rh(55f32.to_radians(),1.5,0.01,40.0)*view,scene.world(),true);gpu.save_png(&texture,&root().join(format!("_tmp/shots/{prefix}_shield.png")))?;serde_json::json!({"tick":tick,"entity":row.0,"hp":row.3,"maxHp":row.4,"shield":row.8,"관측":"공식 엔티티 보호막 실제 양수 사례"})}else{serde_json::json!({"관측":"경기에서 챔피언 보호막 양수 사례 없음 · 실제 양수 시각 미검증"})};
 let(far_head,far_q,_)=crate::capture::camera(0.0,42.0,5.0);scene.update(&mut gpu,far_head,far_q,0.1)?;let far=scene.combat_audit(far_head);ensure!(far["labelInstances"]==0&&gpu.batch_count("bars")>0,"먼 거리 LOD가 단순 HP를 유지하지 않음");
 // 실제 킬 직후에 요소별 토글을 검사한다. 수치나 전투 상태는 바꾸지 않는다.
 let kill_tick=(0..=scene.source.manifest().result.ticks).find(|t|!scene.source.recent_kills(*t).is_empty()).context("실제 킬 없음")?;
 let objective_tick=(0..=scene.source.manifest().result.ticks).find(|t|!scene.source.recent_objectives(*t).is_empty()).context("실제 목표물 소멸 없음")?;
 scene.seek(objective_tick as f64);let(h,q,_)=crate::capture::camera(0.0,42.0,3.7);update(&mut scene,&mut gpu,&mut hud,h,q)?;let objective_audit=scene.combat_audit(h);ensure!(scene.combat_feed_audit().iter().any(|e|e.tick>0&&e.kind!=0),"실제 목표물 사건 표시 누락");let pose=Mat4::from_rotation_translation(scene.hud_pose().1,scene.hud_pose().0+scene.hud_pose().1*Vec3::new(0.0,0.17,0.52));gpu.mask(|n|n=="hud"||n=="killFeed");gpu.render(&target,Mat4::perspective_rh(55f32.to_radians(),1.5,0.01,40.0)*pose.inverse(),scene.world(),true);gpu.save_png(&texture,&root().join(format!("_tmp/shots/{prefix}_objective_event.png")))?;
 scene.seek(kill_tick as f64);let(head,q,_)=crate::capture::camera(0.0,42.0,1.6);update(&mut scene,&mut gpu,&mut hud,head,q)?;let original=scene.ui;let mut toggles=Vec::new();
 for element in["ui_top","ui_kill_log","ui_player_panels","ui_overhead","ui_minimap","ui_events"]{
  scene.seek(if element=="ui_events"{objective_tick as f64}else{kill_tick as f64});scene.ui=original;match element{"ui_top"=>scene.ui.top=false,"ui_kill_log"=>scene.ui.kill_log=false,"ui_player_panels"=>scene.ui.player_panels=false,"ui_overhead"=>scene.ui.overhead=false,"ui_minimap"=>scene.ui.minimap=false,_=>scene.ui.events=false};scene.update(&mut gpu,head,q,0.0)?;
  match element{"ui_top"=>ensure!(gpu.batch_count("hud")==0,"상단 토글 실패"),"ui_kill_log"=>ensure!(scene.combat_feed_audit().iter().all(|e|e.tick==0||e.kind!=0),"킬 로그 토글 실패"),"ui_player_panels"=>ensure!(gpu.batch_count("playerPanels")==0,"선수 패널 토글 실패"),"ui_overhead"=>ensure!(gpu.batch_count("bars")==0&&gpu.batch_count("unitLabels")==0,"머리 위 토글 실패"),"ui_minimap"=>ensure!(gpu.batch_count("minimap")==0&&gpu.batch_count("miniDots")==0,"미니맵 토글 실패"),_=>ensure!(scene.combat_feed_audit().iter().all(|e|e.tick==0||e.kind==0),"목표 사건 토글 실패")}
  toggles.push(element);
 }
 scene.ui=original;
 // 테이블에 붙은 중심이 world 변환과 일치하는지 검사한다. 입력 규칙을 호출하지 않는다.
 scene.table_origin=Vec3::new(0.2,0.0,-0.3);scene.table_yaw=0.7;scene.table_scale=1.2;scene.update(&mut gpu,head,q,0.0)?;
 let moved=scene.combat_audit(head);for bounds in moved["panelClearanceMeters"].as_array().unwrap(){ensure!(bounds[0].as_f64().unwrap()>0.17&&bounds[1].as_f64().unwrap()>0.02,"변환 후 패널/테이블 교차");}
 let mut transformations=Vec::new();if layout_mode{for(yaw,scale)in[(0,0.5),(0,1.0),(0,2.0),(0,3.0),(90,1.0),(180,1.0),(270,1.0),(60,1.5)]{scene.table_yaw=(yaw as f32).to_radians();scene.table_scale=scale;scene.table_height=0.85;scene.table_origin=Vec3::new(0.20,0.0,-0.10);scene.seek(24600.0);let center=Vec3::new(0.20,0.85,-0.10);let distance=(scale*2.5f32).max(5.4);let h=center+Vec3::new(0.0,42f32.to_radians().sin(),42f32.to_radians().cos())*distance;let view=Mat4::look_at_rh(h,center,Vec3::Y);let q=view.inverse().to_scale_rotation_translation().1;update(&mut scene,&mut gpu,&mut hud,h,q)?;let audit=scene.combat_audit(h);for bounds in audit["panelClearanceMeters"].as_array().unwrap(){ensure!(bounds[0].as_f64().unwrap()>0.17&&bounds[1].as_f64().unwrap()>0.02,"회전/크기/이동 후 패널 교차");}ensure!(audit["minimapFrontClearanceMeters"].as_f64().unwrap()>0.07,"미니맵 테이블 안쪽 침범");gpu.mask(|_|true);gpu.render(&target,Mat4::perspective_rh(85f32.to_radians(),1.5,0.01,40.0)*view,scene.world(),true);let label=format!("yaw{yaw}_scale{}",(scale*100.0)as i32);gpu.save_png(&texture,&root().join(format!("_tmp/shots/{prefix}_{label}.png")))?;transformations.push(serde_json::json!({"yaw":yaw,"scale":scale,"audit":audit,"tableOrigin":[0.20,0.85,-0.10]}));}}
 gpu.check()?;let report=serde_json::json!({"검증":"지정9시점·패널각크기·CC거리LOD·6토글·테이블부착 검사 통과","views":views,"cc":{"tick":cc_case.0,"entity":row.0,"kind":cc_case.2,"near":near,"far":far},"shield":shield_audit,"objective":{"tick":objective_tick,"audit":objective_audit,"관측":"실제 존재→부재 전환. 처치자나 보상은 만들지 않음."},"toggles":toggles,"transformations":transformations,"transformedBounds":moved,"tableAttachment":"테이블 변환 뒤 보이는 좌우 바깥 지원면에 재배치. 양측판은 시선 평면과 평행하며 기울기를 포함해 경계를 보존.","captureHUDscale":0.55,"실제VR기본HUDscale":1.0,"실제VR상단24px글자기준각도":1.22,"원본수치":"저장경기 그대로. 마지막 부착 검사는 표시용 테이블 변환만 바꿈.","실기기":"미검증"});std::fs::write(root().join(if finish_mode{"_tmp/logs/native_ui_finish_capture.json"}else if layout_mode{"_tmp/logs/native_ui_layout_capture.json"}else{"_tmp/logs/native_combat_ui_capture.json"}),serde_json::to_vec_pretty(&report)?)?;log.line("전투 UI 자료·기하·토글·시점 검사 완료");Ok(())
}
