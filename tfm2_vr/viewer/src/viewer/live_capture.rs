// 저장한 실시간 명령 폴더를 여러 각도로 그려 확인한다: --live-compare <폴더> [이름]
use anyhow::{Context,Result};
use glam::{Mat4,Vec3};
use std::{path::Path,time::{Duration,Instant}};
use crate::{log::{Log,root},gpu::Gpu,geometry,live::{Live,feed::Feed},scene::TABLE};
pub fn run(log:&Log,args:&[String])->Result<()>{
 let folder=args.get(2).context("폴더")?;let label=args.get(3).map(String::as_str).unwrap_or("live");
 let mut gpu=Gpu::new(log,None,0xb000)?;let table=[(Vec3::new(2.52,0.12,2.16),Vec3::new(0.0,0.695,0.0),0x334b4au32),(Vec3::new(2.4,0.035,2.04),Vec3::new(0.0,0.735,0.0),0x354b42)].iter().map(|&(s,p,c)|geometry::solid_instance(Mat4::from_scale_rotation_translation(s,glam::Quat::IDENTITY,p),geometry::color_hex(c,1.0))).collect();gpu.add_mesh(&geometry::mesh("table",geometry::box_vertices(),table,false,true),2)?;
 let mut live=Live::new(&mut gpu,Feed::folder(Path::new(folder))?,log)?;gpu.live=true;
 let(texture,target)=gpu.offscreen(1600,1000)?;let projection=Mat4::perspective_rh(42f32.to_radians(),1.6,0.01,40.0);
 let views=[("front",0.0f32,42.0f32,3.3f32,Vec3::new(0.0,TABLE,0.0)),("close",0.0,34.0,1.15,Vec3::new(0.1,TABLE,0.0)),("side",-70.0,30.0,1.5,Vec3::new(0.0,TABLE,0.0)),("back",180.0,40.0,2.4,Vec3::new(0.0,TABLE,0.0)),("top",0.0,78.0,3.0,Vec3::new(0.0,TABLE,0.0))];
 let start=Instant::now();let mut worst=0.0f64;let mut n=0;
 let warm=args.get(4).and_then(|v|v.parse::<f64>().ok()).unwrap_or(4.0);while start.elapsed().as_secs_f64()<warm{let t=Instant::now();live.update(&mut gpu,Vec3::new(0.0,2.4,2.4),glam::Quat::IDENTITY,0.45,0.52)?;worst=worst.max(t.elapsed().as_secs_f64()*1000.0);n+=1;std::thread::sleep(Duration::from_millis(8));}
 log.line(format!("실시간 준비 반복 {n} 최악 {worst:.2}ms 상태 {:?}",live.stats));
 for(name,az,el,d,center)in views{let(a,e)=(az.to_radians(),el.to_radians());let head=center+Vec3::new(a.sin()*e.cos(),e.sin(),a.cos()*e.cos())*d;let view=Mat4::look_at_rh(head,center,Vec3::Y);for _ in 0..25{live.update(&mut gpu,head,view.inverse().to_scale_rotation_translation().1,0.45,0.52)?;std::thread::sleep(Duration::from_millis(10));}gpu.render(&target,projection*view,Mat4::IDENTITY,true);gpu.save_png(&texture,&root().join(format!("_tmp/shots/{label}_{name}.png")))?;log.line(format!("{name} {:?} 칸 대기 {}",live.stats,live.pending()));}
 gpu.check()?;Ok(())
}
