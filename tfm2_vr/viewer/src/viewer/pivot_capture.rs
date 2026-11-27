// 중심 흔들림 검사: 대상의 게임 좌표를 화면 가로 중앙에 고정하고 연속 틱을 찍는다.
use anyhow::Result;
use glam::{Mat4,Vec3};
use crate::{Prepared,log::{Log,root},gpu::Gpu,scene::{Scene,TABLE,PIXEL}};
pub fn run(log:&Log,prepared:Prepared)->Result<()>{
 let mut gpu=Gpu::new(log,None,0xb000)?;let mut scene=Scene::new(prepared.source,prepared.assets,&mut gpu)?;scene.show_hud=false;scene.elapsed_seconds=12.0;scene.playing=false;scene.projectiles_enabled=false;
 let(texture,target)=gpu.offscreen(220,220)?;let projection=Mat4::perspective_rh(38f32.to_radians(),1.0,0.01,40.0);let total=scene.source.manifest().result.ticks;
 let catalog:std::collections::HashMap<u32,String>=scene.source.manifest().catalog.iter().map(|(k,m)|(*k,m.name.clone())).collect();let knight=catalog.iter().find(|(_,m)|m.as_str()=="knight").map(|(id,_)|*id);let mut rows:Vec<(String,u32,Vec<u32>)>=Vec::new();
 if let Some(id)=knight{for slot in 1..=4u8{if let Some(a)=scene.source.manifest().actions.iter().find(|a|a.1==id&&a.2==slot).copied(){rows.push((format!("knight {slot}"),id,(0..12).map(|i|a.0+i*3).collect()));}}
  if let Some(t)=(600..total.saturating_sub(120)).step_by(20).find(|&t|{let g=|k:u32|scene.source.frame(k).entities.iter().find(|e|e.0==id).copied();match(g(t),g(t+100)){(Some(a),Some(b))=>a.6!=0&&b.6!=0&&(a.1-b.1).hypot(a.2-b.2)<1.0,_=>false}}){rows.push(("knight idle".into(),id,(0..12).map(|i|t+i*8).collect()));}}
 for (prefix,nth) in [("serpen",2usize),("rhino",1),("bee",1),("mushroom",1),("stump",1),("epic",1)]{if let Some(a)=scene.source.manifest().actions.iter().filter(|a|catalog[&a.1].starts_with(prefix)).nth(nth).copied(){rows.push((prefix.to_string(),a.1,(0..12).map(|i|a.0.saturating_sub(6)+i*3).collect()));}}
 if let Some(a)=scene.source.manifest().actions.iter().filter(|a|catalog[&a.1].starts_with("serpen")).nth(2).copied(){rows.push(("serpen long".into(),a.1,(0..12).map(|i|a.0.saturating_sub(6)+36+i*12).collect()));}
 let mut sheet=image::RgbaImage::new(220*12,220*rows.len()as u32);for(r,(name,id,ticks))in rows.iter().enumerate(){let mut states=Vec::new();for(i,&t)in ticks.iter().enumerate(){let frame=scene.source.frame(t);let Some(e)=frame.entities.iter().find(|e|e.0==*id).copied()else{continue;};
   if i==0{scene.seek(t as f64);}else{scene.time=t as f64;}let point=Vec3::new(((e.1*0.001-480.0)*PIXEL as f64)as f32,TABLE+0.05,((e.2*0.001-480.0)*PIXEL as f64)as f32);let size=if name.starts_with("serpen"){1.6}else{1.0};let head=point+Vec3::new(0.0,0.24,0.32)*size;let view=Mat4::look_at_rh(head,point,Vec3::Y);let q=view.inverse().to_scale_rotation_translation().1;
   if i==0{for _ in 0..6{scene.update(&mut gpu,head,q,0.1)?;}}scene.update(&mut gpu,head,q,0.0)?;gpu.mask(|_|true);gpu.render(&target,projection*view,scene.world(),true);let(w,h,p)=gpu.pixels(&texture)?;let mut im=image::RgbaImage::from_raw(w,h,p).unwrap();for y in 0..h{let px=im.get_pixel_mut(w/2,y);*px=image::Rgba([255,60,60,255]);}image::imageops::replace(&mut sheet,&im,(i*220)as i64,(r*220)as i64);states.push(scene.avatar_state(*id));}
  println!("{name} id {id} 시작 {} 상태 {states:?}",ticks[0]);}
 sheet.save(root().join("_tmp/shots/pivot_contact.png"))?;gpu.check()?;
 for p in ["knight:","serpen:"]{for a in scene.assets.foot_audit.iter().filter(|a|a.name.contains(p)){println!("{} 프레임 {} 캔버스 {:?} 기준 {:?} 오차 {}",a.name,a.frames,a.canvas,a.pivot,a.max_foot_residual_px);}}
 let bad:Vec<_>=scene.assets.foot_audit.iter().filter(|a|a.max_foot_residual_px>0.5).map(|a|a.name.clone()).collect();println!("오차 0.5 초과 {bad:?} / 전체 {}",scene.assets.foot_audit.len());log.line("중심 검사 캡처 완료");Ok(())
}
