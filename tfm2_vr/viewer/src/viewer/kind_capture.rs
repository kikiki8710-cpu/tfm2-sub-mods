// 투사체 종류별(v5) 실제 발생 틱 근접 캡처. 종류마다 수명 중간 프레임 하나와 투사체만 남긴 기여 픽셀 수를 남긴다.
use anyhow::{ensure,Result};
use glam::{Mat4,Vec3};
use crate::{Prepared,log::{Log,root},gpu::Gpu,scene::{Scene,TABLE,PIXEL}};
pub fn run(log:&Log,prepared:Prepared)->Result<()>{
 let mut gpu=Gpu::new(log,None,0xb000)?;let mut scene=Scene::new(prepared.source,prepared.assets,&mut gpu)?;
 scene.show_hud=false;scene.elapsed_seconds=12.0;scene.playing=false;
 let(texture,target)=gpu.offscreen(480,360)?;let projection=Mat4::perspective_rh(42f32.to_radians(),480.0/360.0,0.01,40.0);
 let kinds=scene.source.manifest().projectile_kinds.clone();ensure!(!kinds.is_empty(),"v5 종류 자료 필요");let mut cases=Vec::new();
 let mut sheet=image::RgbaImage::new(480*4,360*((kinds.len()as u32).div_ceil(4)));
 for(index,name)in kinds.iter().enumerate(){
  // 벽에 가리지 않는 예를 고르기 위해 앞의 몇 개를 시도하고 기여 픽셀이 가장 큰 것을 남긴다.
  let mut tries=Vec::new();let mut seen=std::collections::HashSet::new();
  for tick in 0..=scene.source.manifest().result.ticks{let f=scene.source.frame(tick);for(i,p)in f.pk.iter().enumerate(){let mid=if p.3==4{30}else{3};if p.1 as usize==index&&tick>=p.2+mid&&seen.insert((p.0,p.2)){tries.push((tick,f.projectiles[i],p.0,p.2));}}if tries.len()>=12{break;}}
  let mut best:Option<(usize,image::RgbaImage,serde_json::Value)>=None;
  for(tick,row,id,spawn)in &tries{scene.seek(*tick as f64+0.25);
   let point=Vec3::new(((row.1*0.001-480.0)*PIXEL as f64)as f32,TABLE+0.03,((row.2*0.001-480.0)*PIXEL as f64)as f32);
   let head=point+Vec3::new(0.0,0.26,0.36);let view=Mat4::look_at_rh(head,point,Vec3::Y);let q=view.inverse().to_scale_rotation_translation().1;
   for _ in 0..8{scene.update(&mut gpu,head,q,0.1)?;}
   gpu.mask(|n|n=="projectiles");gpu.render(&target,projection*view,scene.world(),true);let(_,_,only)=gpu.pixels(&texture)?;let bg=[only[0],only[1],only[2],only[3]];let visible=only.chunks_exact(4).filter(|p|**p!=bg).count();
   gpu.mask(|_|true);gpu.render(&target,projection*view,scene.world(),true);let(w,h,full)=gpu.pixels(&texture)?;
   let flight=scene.projectile_snapshot().into_iter().find(|v|v["kind"]==serde_json::json!(name)).unwrap_or_default();
   if best.as_ref().is_none_or(|b|visible>b.0){best=Some((visible,image::RgbaImage::from_raw(w,h,full).unwrap(),serde_json::json!({"tick":tick,"id":id,"spawn":spawn,"flight":flight})));}}
  if let Some((visible,im,detail))=best{image::imageops::replace(&mut sheet,&im,(index as i64%4)*480,(index as i64/4)*360);cases.push(serde_json::json!({"index":index,"kind":name,"visibleProjectilePixels":visible,"detail":detail}));}else{cases.push(serde_json::json!({"index":index,"kind":name,"관측":"이번 경기에 없음"}));}
 }
 sheet.save(root().join("_tmp/shots/kind_contact.png"))?;gpu.check()?;
 std::fs::write(root().join("_tmp/logs/kind_capture.json"),serde_json::to_vec_pretty(&serde_json::json!({"cases":cases,"실기기":"미검증"}))?)?;
 for c in &cases{println!("{} {} 픽셀 {} tick {} {} {}",c["index"],c["kind"],c["visibleProjectilePixels"],c["detail"]["tick"],c["detail"]["flight"]["kindKey"],c["detail"]["flight"]["kindTags"]);}
 log.line("투사체 종류별 캡처 완료");Ok(())
}
