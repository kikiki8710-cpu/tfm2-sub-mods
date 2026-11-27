// v5 이펙트 이름별·챔피언 행동별 실제 발생 틱 근접 캡처와 행동 상태 검사.
use anyhow::{ensure,Result};
use glam::{Mat4,Vec3};
use crate::{Prepared,log::{Log,root},gpu::Gpu,scene::{Scene,TABLE,PIXEL}};
fn shot(scene:&mut Scene,gpu:&mut Gpu,target:&crate::gpu::Target,texture:&windows::Win32::Graphics::Direct3D11::ID3D11Texture2D,projection:Mat4,time:f64,x:f64,y:f64)->Result<image::RgbaImage>{
 scene.seek(time);let point=Vec3::new(((x*0.001-480.0)*PIXEL as f64)as f32,TABLE+0.03,((y*0.001-480.0)*PIXEL as f64)as f32);let head=point+Vec3::new(0.0,0.24,0.32);let view=Mat4::look_at_rh(head,point,Vec3::Y);let q=view.inverse().to_scale_rotation_translation().1;
 for _ in 0..6{scene.update(gpu,head,q,0.1)?;}gpu.mask(|_|true);gpu.render(target,projection*view,scene.world(),true);let(w,h,p)=gpu.pixels(texture)?;Ok(image::RgbaImage::from_raw(w,h,p).unwrap())}
pub fn run(log:&Log,prepared:Prepared)->Result<()>{
 let mut gpu=Gpu::new(log,None,0xb000)?;let mut scene=Scene::new(prepared.source,prepared.assets,&mut gpu)?;scene.show_hud=false;scene.elapsed_seconds=12.0;scene.playing=false;
 let(texture,target)=gpu.offscreen(400,300)?;let projection=Mat4::perspective_rh(42f32.to_radians(),400.0/300.0,0.01,40.0);
 let names=scene.source.manifest().effect_names.clone();let effects=scene.source.manifest().effects.clone();let mut report=Vec::new();let mut shots=Vec::new();
 for(i,name)in names.iter().enumerate(){let Some(e)=effects.iter().filter(|e|e.1 as usize==i).nth(1).or_else(||effects.iter().find(|e|e.1 as usize==i))else{continue;};let has=scene.assets.effect_art.get(i).is_some_and(Option::is_some);
  if has{let im=shot(&mut scene,&mut gpu,&target,&texture,projection,e.0 as f64+6.0,e.4,e.5)?;shots.push(im);}report.push(serde_json::json!({"effect":name,"tick":e.0,"art":has,"sheetIndex":if has{Some(shots.len()-1)}else{None}}));}
 let cols=5u32;let mut sheet=image::RgbaImage::new(400*cols,300*(shots.len()as u32).div_ceil(cols));for(i,im)in shots.iter().enumerate(){image::imageops::replace(&mut sheet,im,(i as i64%cols as i64)*400,(i as i64/cols as i64)*300);}sheet.save(root().join("_tmp/shots/effect_events_contact.png"))?;
 // 행동: 명단 10명 × 4종. 시작 뒤 중간 프레임에서 상태가 행동 상태인지 검사한다.
 let mut acts=Vec::new();let mut sheet=image::RgbaImage::new(400*4,300*10);let roster:Vec<String>=scene.source.manifest().roster.iter().flatten().cloned().collect();
 for(r,champion)in roster.iter().enumerate(){for slot in 1..=4u8{let found=scene.source.manifest().actions.iter().find(|a|a.2==slot&&scene.source.manifest().catalog[&a.1].name==*champion).copied();
  let Some(a)=found else{acts.push(serde_json::json!({"champion":champion,"slot":slot,"관측":"없음"}));continue;};let team=scene.source.frame(a.0).entities.iter().find(|e|e.0==a.1).map(|e|e.5).unwrap_or(0);let length=scene.assets.visual(a.1 as usize,team).anims[3+slot as usize].duration;let t=a.0+((length*60.0*0.45)as u32).clamp(1,12);let frame=scene.source.frame(t);let Some(row)=frame.entities.iter().find(|e|e.0==a.1).copied()else{continue;};
  let im=shot(&mut scene,&mut gpu,&target,&texture,projection,t as f64,row.1,row.2)?;image::imageops::replace(&mut sheet,&im,(slot as i64-1)*400,r as i64*300);let state=scene.avatar_state(a.1);acts.push(serde_json::json!({"champion":champion,"slot":slot,"tick":a.0,"state":state,"hasTag":scene.assets.visual(a.1 as usize,row.5).has_action[slot as usize-1]}));}}
 sheet.save(root().join("_tmp/shots/action_contact.png"))?;
 for a in &acts{if a["hasTag"]==true{ensure!(a["state"].as_u64().is_some_and(|s|s>=4),"행동 상태 불일치: {a}");}}
 gpu.check()?;std::fs::write(root().join("_tmp/logs/event_capture.json"),serde_json::to_vec_pretty(&serde_json::json!({"effects":report,"actions":acts,"실기기":"미검증"}))?)?;
 for e in &report{println!("{e}");}for a in &acts{println!("{a}");}log.line("이펙트·행동 캡처 완료");Ok(())
}
