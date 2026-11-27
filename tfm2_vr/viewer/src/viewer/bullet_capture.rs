use anyhow::{ensure,Context,Result};
use glam::{Mat4,Vec3};
use crate::{Prepared,log::{Log,root},gpu::Gpu,scene::{Scene,TABLE,PIXEL}};
pub fn run(log:&Log,prepared:Prepared)->Result<()>{
 let mut gpu=Gpu::new(log,None,0xb000)?;let mut scene=Scene::new(prepared.source,prepared.assets,&mut gpu)?;
 scene.show_hud=false;scene.elapsed_seconds=12.0;scene.playing=false;
 let(texture,target)=gpu.offscreen(1200,900)?;let projection=Mat4::perspective_rh(42f32.to_radians(),1200.0/900.0,0.01,40.0);let mut cases=Vec::new();
 for(name,category,label)in[("soldier",1,"soldier_attack"),("soldier",4,"soldier_ult"),("gunner",1,"gunner_attack"),("archer",1,"archer_attack"),("soldier",0,"soldier_unknown")]{
  let mut selected=None;
  for tick in 0..=scene.source.manifest().result.ticks{
   let frame=scene.source.frame(tick);
   if let Some(e)=frame.projectile_kinds.iter().find(|e|e.2==category&&scene.source.manifest().catalog.get(&(e.1+1)).is_some_and(|m|m.name==name)){
    if let Some(p)=frame.projectiles.iter().find(|p|p.0==e.0&&p.3==e.1){selected=Some((tick,*p));break;}
   }
  }
  let(tick,row)=selected.with_context(||format!("투사체 검사 사례 없음 {label}"))?;scene.seek(tick as f64+0.5);
  let point=Vec3::new(((row.1*0.001-480.0)*PIXEL as f64)as f32,TABLE+0.035,((row.2*0.001-480.0)*PIXEL as f64)as f32);
  let head=point+Vec3::new(0.0,0.20,0.32);let view=Mat4::look_at_rh(head,point,Vec3::Y);let q=view.inverse().to_scale_rotation_translation().1;
  for _ in 0..12{scene.update(&mut gpu,head,q,0.1)?;}
  let snapshots=scene.projectile_snapshot();let flight=snapshots.iter().find(|v|v["handle"].as_u64()==Some(row.0)).context("검사 투사체 보간 결과 없음")?;
  let source=flight["sourceKey"].as_str();match label{"soldier_attack"=>ensure!(source==Some("sprite/bullet"),"소총수 일반 총알 그림 불일치"),"gunner_attack"=>ensure!(source==Some("sprite/gunner_bullet"),"총잡이 일반 총알 그림 불일치"),"archer_attack"=>ensure!(source==Some("sprite/arrow"),"궁수 일반 화살 그림 불일치"),"soldier_ult"=>ensure!(flight["tag"]=="ult_projectile","소총수 궁극기 태그 불일치"),_=>ensure!(source.is_none()&&flight["fallbackGlow"]==true,"알 수 없는 종류가 스킬/궁 그림으로 대체됨")}
  gpu.mask(|_|true);gpu.render(&target,projection*view,scene.world(),true);gpu.save_png(&texture,&root().join(format!("_tmp/shots/native_bullet_{label}.png")))?;
  // 실제 장면에 가려져도 원본 선택 그림의 화면 기여를 별도 확인한다.
  gpu.mask(|n|n=="projectiles");gpu.render(&target,projection*view,scene.world(),true);let(_,_,pixels)=gpu.pixels(&texture)?;gpu.save_png(&texture,&root().join(format!("_tmp/shots/native_bullet_{label}_only.png")))?;let bg=&pixels[..4];let visible=pixels.chunks_exact(4).filter(|p|*p!=bg).count();ensure!(visible>0,"투사체 화면 기여 픽셀 없음");
  cases.push(serde_json::json!({"label":label,"tick":tick,"category":category,"flight":flight,"visibleProjectilePixels":visible}));gpu.mask(|_|true);
 }
 gpu.check()?;let report=serde_json::json!({"검증":"원본 일반3종·소총수 궁·미확정 팀색 빛 검사 통과","cases":cases,"실기기":"미검증","종류":"공식 쿨다운 기반 추정, 원본 타입 아님"});std::fs::write(root().join("_tmp/logs/native_bullet_art_capture.json"),serde_json::to_vec_pretty(&report)?)?;log.line("투사체 그림 구분·화면 기여 검사 완료");Ok(())
}
