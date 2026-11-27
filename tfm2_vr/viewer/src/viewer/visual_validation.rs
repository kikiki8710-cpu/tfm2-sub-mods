use anyhow::{Result,ensure};
use glam::{Mat4,Quat};
use crate::{scene::Scene,gpu::{Gpu,Target},hud::Hud,log::root};
use windows::Win32::Graphics::Direct3D11::ID3D11Texture2D;

pub fn run(scene:&mut Scene,gpu:&mut Gpu,hud:&mut Hud,texture:&ID3D11Texture2D,target:&Target)->Result<()> {
 let projection=Mat4::perspective_rh(42f32.to_radians(),target.width as f32/target.height as f32,0.01,40.0);
 scene.seek(24600.0);let mut bars=Vec::new();
 for (az,e,roll,yaw,scale) in [(0.0,42.0,0.0,0.0,1.0),(-60.0,42.0,0.0,0.0,1.0),(60.0,42.0,0.0,0.0,1.0),(0.0,60.0,0.0,0.0,1.0),(0.0,42.0,25.0,0.3,1.5)] {
  scene.table_yaw=yaw;scene.table_scale=scale;let (head,q,_)=crate::capture::camera(az,e,3.7);let q=q*Quat::from_rotation_z((roll as f32).to_radians());let view=Mat4::from_rotation_translation(q,head).inverse();
  for _ in 0..12{scene.update(gpu,head,q,0.1)?;}
  let max=scene.healthbar_screen_angles(projection*view).into_iter().map(f32::abs).fold(0.0f32,f32::max);ensure!(max<0.01,"체력바 화면 수평 오차: {max}");bars.push(serde_json::json!({"az":az,"elevation":e,"headRoll":roll,"tableYaw":yaw,"tableScale":scale,"maxScreenAngleDeg":max}));
 }
 scene.table_yaw=0.0;scene.table_scale=1.0;scene.elapsed_seconds=12.0;scene.adjustment_seconds=0.0;let (head,q,view)=crate::capture::camera(0.0,42.0,3.7);for _ in 0..12{scene.update(gpu,head,q,0.1)?;}
 let mut states=Vec::new();
 for (name,elapsed,adjustment) in [("default",12.0,0.0),("intro",0.0,0.0),("adjustment",12.0,2.0)]{
  scene.elapsed_seconds=elapsed;scene.adjustment_seconds=adjustment;let content=scene.hud_content();let (hw,hh)=(hud.width,hud.height);let pixels=hud.render(&content);let rows=pixels.chunks_exact(hw as usize*4).filter(|row|row.chunks_exact(4).any(|p|p[3]>0)).count()/ (hw as usize/1024);image::save_buffer(root().join(format!("_tmp/shots/native_visual_hud_{name}.png")),pixels,hw,hh,image::ColorType::Rgba8)?;gpu.update_hud(pixels)?;gpu.render(target,projection*view,scene.world(),true);gpu.save_png(texture,&root().join(format!("_tmp/shots/native_visual_{name}.png")))?;states.push(serde_json::json!({"name":name,"intro":content.show_intro,"adjustment":content.adjustment.is_some(),"opaqueRows":rows}));
 }
 scene.elapsed_seconds=9.95;scene.hud_dirty=false;scene.update(gpu,head,q,0.1)?;ensure!(!scene.hud_content().show_intro&&scene.hud_dirty,"HUD 10초 경과 접기 실패");scene.elapsed_seconds=12.0;scene.adjustment_seconds=2.0;for _ in 0..21{scene.update(gpu,head,q,0.1)?;}ensure!(!scene.hud_content().show_intro&&scene.hud_content().adjustment.is_none(),"HUD 조절 안내 접기 실패");
 let mut feet=Vec::new();for row in &scene.source.frame(24600).entities{if scene.source.manifest().catalog[&row.0].kind=="champion"{let value=scene.sprite_measure(row.0,projection*view,target.height).unwrap();ensure!(value["footError"].as_f64().unwrap()<0.00001,"챔피언 발 피벗 이동");feet.push(serde_json::json!({"id":row.0,"name":scene.source.manifest().catalog[&row.0].name,"measure":value}));}}
 ensure!(scene.assets.foot_audit.iter().all(|a|a.max_foot_residual_px<=0.5),"픽셀 발 중심 정렬 오차");ensure!(!scene.assets.generated,"생성 텍스처 사용 금지");
 let report=serde_json::json!({"검증":"통과","원본전용":true,"동작별고정피벗":scene.assets.foot_audit,"챔피언발":feet,"체력바화면수평":bars,"HUD":states,"안내10초후접기":true,"조절표시2초후접기":true,"맵크롭":[0,96,1280,1088],"맵크기m":[2.4,2.04],"테이블크기m":[2.52,2.16],"아래검은여백m":{"전":0.2025,"후":0.0225},"실기기":"미검증"});std::fs::write(root().join("_tmp/logs/native_visual_validation.json"),serde_json::to_vec_pretty(&report)?)?;Ok(())
}
