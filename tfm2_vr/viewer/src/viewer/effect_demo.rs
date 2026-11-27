//! 원본 규칙 대조용 통제 입력 시연. 저장 경기의 이펙트 발생으로 해석하지 않는다.
use anyhow::{ensure, Result};
use glam::{Mat4, Quat, Vec3};
use serde::Serialize;
use crate::{Prepared, assets::{Animation, Assets}, geometry::{self, Instance, Vertex}, gpu::Gpu, log::{Log, root}};

#[derive(Clone, Debug, Serialize)]
struct SpriteCommand { kind: &'static str, frame: usize, center: [f32; 2], cursor: f32 }
#[derive(Serialize)]
struct Commands { age: f32, radius: f32, sprites: Vec<SpriteCommand> }

// 0.7.3: RVA2d24b40 / 2789460. 기간 비교는 엄격한 >, 소진 시 마지막 셀.
fn sample(animation: &Animation, age: f32, repeat: bool) -> usize {
    let total = animation.cels.iter().fold(-0.0f32, |sum, c| sum + c.duration as f32);
    let mut cursor = if repeat { age % total } else { age };
    for (index, cel) in animation.cels.iter().enumerate() {
        if cel.duration as f32 > cursor { return index; }
        cursor -= cel.duration as f32;
    }
    animation.cels.len() - 1
}

fn commands(assets: &Assets, age: f32) -> Commands {
    let mut sprites = vec![
        SpriteCommand {kind:"projectile",frame:sample(&assets.effect_demo_art["projectile"].animation,age,true),center:[-280.0,0.0],cursor:age},
        SpriteCommand {kind:"skill",frame:sample(&assets.effect_demo_art["skill"].animation,age,false),center:[280.0,0.0],cursor:age},
    ];
    let radius = 100.0 * (age / 0.15f32).min(1.0);
    if age >= 0.15f32 {
        let anim = &assets.effect_demo_art["ground"].animation;
        let duration = anim.cels.iter().fold(-0.0f32, |s,c| s+c.duration as f32);
        let period = 1.2f32 * duration;
        for i in 0..5u32 {
            let elapsed = (i as f32 * period / 5.0f32) + (age + -0.15f32);
            let cursor = elapsed % period;
            if cursor >= duration { continue; }
            let cycle = (elapsed / period) as u32;
            let seed = (cycle.wrapping_mul(31).wrapping_add(i)).wrapping_mul(0x6d13);
            let angle = (seed.wrapping_add(0x79b1) as u16 as f32 / 65535.0f32) * std::f32::consts::TAU;
            let distance = (seed.wrapping_add(0xf362) as u16 as f32 / 65535.0f32).sqrt() * 100.0f32 * 0.85f32;
            sprites.push(SpriteCommand {kind:"ground",frame:sample(anim,cursor,false),center:[angle.sin()*distance,angle.cos()*distance],cursor});
        }
    }
    Commands {age,radius,sprites}
}

fn circle_vertices() -> Vec<Vertex> {
    let fill=geometry::rgb([0.8,24.0/255.0,24.0/255.0],50.0/255.0);
    let mut out=Vec::new();
    let point=|i:usize,r:f32| {let a=i as f32*std::f32::consts::TAU/64.0;[a.sin()*r,a.cos()*r,0.0]};
    let vertex=|p,color|Vertex{position:p,uv:[0.0;2],color};
    for i in 0..64 {
        for p in [[0.0;3],point(i,1.0),point(i+1,1.0)] {out.push(vertex(p,fill));}
    }
    out
}

fn ring_vertices()->Vec<Vertex>{
    let color=geometry::rgb([227.0/255.0,86.0/255.0,36.0/255.0],1.0);
    [(-1.0,-1.0),(1.0,-1.0),(1.0,1.0),(-1.0,-1.0),(1.0,1.0),(-1.0,1.0)].into_iter().map(|(side,outer)|Vertex{position:[side,outer,side*outer],uv:[0.0;2],color}).collect()
}

fn ring_instances(radius:f32,spatial:bool)->Vec<Instance>{
    if radius<=0.0{return Vec::new();}
    let half=std::f32::consts::PI/64.0;
    let projection=if spatial{Mat4::from_scale_rotation_translation(Vec3::splat(0.004),Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2),Vec3::new(0.0,0.002,0.0))}else{Mat4::IDENTITY};
    (0..64).map(|i|{let angle=(i as f32+0.5)*std::f32::consts::TAU/64.0;let radial=Vec3::new(angle.sin(),angle.cos(),0.0);let tangent=Vec3::new(angle.cos(),-angle.sin(),0.0);
        let model=Mat4::from_cols((tangent*(radius*half.sin())).extend(0.0),(radial*1.5).extend(0.0),(tangent*(1.5*half.tan())).extend(0.0),(radial*(radius*half.cos())).extend(1.0));
        geometry::solid_instance(projection*model,[1.0;4])
    }).collect()
}

fn instances(assets:&Assets,commands:&Commands,spatial:bool) -> Vec<Instance> {
    commands.sprites.iter().map(|cmd|{
        let cel=&assets.effect_demo_art[cmd.kind].animation.cels[cmd.frame];
        let(w,h)=(cel.width as f32,cel.height as f32);
        let model=if spatial {
            // 2D 원본의 층·y를 3D 높이로 단정하지 않는다. 수직 카드와 발 위치는 시연용 투영이다.
            let y=if cmd.kind=="projectile"{0.3}else{h*0.002};
            Mat4::from_scale_rotation_translation(Vec3::new(w*0.004,h*0.004,1.0),Quat::IDENTITY,Vec3::new(cmd.center[0]*0.004,y,cmd.center[1]*0.004))
        }else{Mat4::from_scale_rotation_translation(Vec3::new(w,h,1.0),Quat::IDENTITY,Vec3::new(cmd.center[0],-cmd.center[1],0.0))};
        Instance::new(model,cel.region.uv,[1.0;4],[1.0,0.0,0.0,cel.region.page as f32])
    }).collect()
}

fn update(gpu:&mut Gpu,assets:&Assets,commands:&Commands,spatial:bool,sprite_batch:usize,circle_batch:usize,ring_batch:usize,floor_batch:usize)->Result<()> {
    gpu.update(sprite_batch,&instances(assets,commands,spatial))?;
    let model=if spatial{Mat4::from_scale_rotation_translation(Vec3::splat(commands.radius*0.004),Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2),Vec3::new(0.0,0.001,0.0))}else{Mat4::from_scale(Vec3::splat(commands.radius))};
    gpu.update(circle_batch,&[geometry::solid_instance(model,[1.0;4])])?;
    gpu.update(ring_batch,&ring_instances(commands.radius,spatial))?;
    let floor=if spatial{vec![geometry::solid_instance(Mat4::from_scale_rotation_translation(Vec3::new(3.6,1.8,1.0),Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2),Vec3::new(0.0,-0.005,0.0)),geometry::rgb([0.12,0.18,0.20],1.0))]}else{Vec::new()};
    gpu.update(floor_batch,&floor)?;Ok(())
}

fn verify_original_cells(assets:&Assets)->Result<usize>{
    let cache=root().join("_cache");
    let base:serde_json::Value=crate::source::read_json(&cache.join("assets/manifest.json"))?;
    let effects:serde_json::Value=crate::source::read_json(&cache.join("effects/manifest.json"))?;
    let mut checked=0;
    for art in assets.effect_demo_art.values(){
        let source=effects["sprites"].get(&art.source_key).or_else(||base["sprites"].get(&art.source_key)).ok_or_else(||anyhow::anyhow!("원본 셀 대조 자산 없음"))?;
        let sheet=image::open(crate::assets::cache_path(&cache,source["sheet"].as_str().unwrap())?)?.to_rgba8();
        let frames=source["anims"][&art.tag]["frames"].as_array().unwrap();ensure!(frames.len()==art.animation.cels.len(),"원본 셀 수 불일치");
        for (f,cel)in frames.iter().zip(&art.animation.cels){
            let d=&f["data"];let normalized=d["w"].as_f64().unwrap()<1.0&&d["h"].as_f64().unwrap()<1.0;
            let val=|key:&str,axis:u32|(d[key].as_f64().unwrap()*if normalized{axis as f64}else{1.0}).round()as u32;
            let(x,y,w,h)=(val("x",sheet.width()),val("y",sheet.height()),val("w",sheet.width()),val("h",sheet.height()));
            ensure!((w,h)==(cel.width,cel.height),"원본 셀 크기 불일치");
            let group=(cel.region.page/2048)as usize;let layer=(cel.region.page%2048)as usize;let size=crate::mips::BUCKETS[group];
            let atlas=assets.pages.iter().filter(|p|p.width()==size).nth(layer).ok_or_else(||anyhow::anyhow!("시연 아틀라스 셀 없음"))?;
            let(ax,ay)=((cel.region.uv[0]*size as f32).round()as u32,(cel.region.uv[1]*size as f32).round()as u32);
            for cy in 0..h{for cx in 0..w{ensure!(sheet.get_pixel(x+cx,y+cy)==atlas.get_pixel(ax+cx,ay+cy),"원본 RGBA와 GPU 업로드 직전 셀 불일치");}}
            checked+=1;
        }
    }Ok(checked)
}

pub fn run(log:&Log,mut prepared:Prepared)->Result<()> {
    ensure!(prepared.assets.effect_demo_art.len()==3,"시연 전용 자산3개 필요");
    let checked_cells=verify_original_cells(&prepared.assets)?;
    let mut gpu=Gpu::new(log,None,0xb000)?;gpu.upload_atlas(&mut prepared.assets)?;
    let floor_batch=gpu.add_mesh(&geometry::mesh("effectDemoFloor",geometry::quad(),Vec::new(),false,true),1)?;
    let circle=geometry::mesh("effectDemoCircle",circle_vertices(),Vec::new(),true,false);
    let circle_batch=gpu.add_mesh(&circle,1)?;
    let ring_batch=gpu.add_mesh(&geometry::mesh("effectDemoRing",ring_vertices(),Vec::new(),true,false),64)?;
    let mut mesh=geometry::mesh("effectDemoSprites",geometry::quad(),Vec::new(),true,false);mesh.depth_test=false;
    let sprite_batch=gpu.add_mesh(&mesh,7)?;
    let assets=&prepared.assets;let(texture,target)=gpu.offscreen_samples(1800,1000,1)?;
    let flat=Mat4::orthographic_rh(-450.0,450.0,-250.0,250.0,0.01,20.0)*Mat4::look_at_rh(Vec3::new(0.0,0.0,5.0),Vec3::ZERO,Vec3::Y);
    let mut records=Vec::new();
    for (index,age) in [0.0f32,0.075,0.15,0.225,0.3,0.45,0.6,1.1,1.8].into_iter().enumerate(){
        let c=commands(assets,age);gpu.quality=false;update(&mut gpu,assets,&c,false,sprite_batch,circle_batch,ring_batch,floor_batch)?;
        gpu.render(&target,flat,Mat4::IDENTITY,true);gpu.save_png(&texture,&root().join(format!("_tmp/shots/effect_demo_flat_{index:02}.png")))?;
        records.push(serde_json::json!({"index":index,"commands":c}));
    }
    let projection=Mat4::perspective_rh(48f32.to_radians(),1.8,0.01,30.0);
    for index in 0..60 {
        let age=index as f32/30.0;let c=commands(assets,age);gpu.quality=true;
        update(&mut gpu,assets,&c,true,sprite_batch,circle_batch,ring_batch,floor_batch)?;
        let view=Mat4::look_at_rh(Vec3::new(0.0,1.6,2.6),Vec3::new(0.0,0.1,0.0),Vec3::Y);
        gpu.render(&target,projection*view,Mat4::IDENTITY,true);gpu.save_png(&texture,&root().join(format!("_tmp/shots/effect_demo_3d_{index:03}.png")))?;
    }
    for (name,az) in [("left60",-60f32),("right60",60f32)] {
        let c=commands(assets,0.3);update(&mut gpu,assets,&c,true,sprite_batch,circle_batch,ring_batch,floor_batch)?;
        let a=az.to_radians();let view=Mat4::look_at_rh(Vec3::new(a.sin()*2.6,1.6,a.cos()*2.6),Vec3::new(0.0,0.1,0.0),Vec3::Y);
        gpu.render(&target,projection*view,Mat4::IDENTITY,true);gpu.save_png(&texture,&root().join(format!("_tmp/shots/effect_demo_3d_{name}.png")))?;
    }
    gpu.check()?;
    let selected:Vec<_>=["projectile","ground","skill"].into_iter().map(|kind|{let a=&assets.effect_demo_art[kind];serde_json::json!({"kind":kind,"key":a.source_key,"tag":a.tag,"frames":a.animation.cels.iter().map(|c|serde_json::json!({"duration":c.duration,"width":c.width,"height":c.height})).collect::<Vec<_>>()})}).collect();
    let report=serde_json::json!({"검증":"원본 자산·정적 시간식의 통제 입력 시연 생성","원본셀RGBA완전일치수":checked_cells,"입력":"직접 지정한 커서·위치·반경100. 저장 경기의 발생/생성시각을 추출한 것이 아님.","selected":selected,"flat":records,"animationFrames":60,"실기기":"미검증","원본뷰실행":"없음","조작":"변경 없음","원형선폭":3.0,"한계":"3D 수직 카드/높이/중립 바닥/투영은 시연용. 원형선은65점/폭3의 miter 기하이며 Skia 래스터 전체 일치를 주장하지 않음. 원본 전체 명령/Skia 화면의 동적 대조는 미검증."});
    std::fs::write(root().join("_tmp/logs/effect_demo_native_capture.json"),serde_json::to_vec_pretty(&report)?)?;
    log.line("이펙트3종 통제 입력 시연 생성 완료 · 실제 경기 이펙트 연결 아님");Ok(())
}

#[cfg(test)]mod tests{
    use super::*;use crate::assets::{Cel,Region};
    fn animation()->Animation{Animation{duration:0.6,cels:[0.1,0.2,0.3].into_iter().map(|duration|Cel{duration,region:Region{uv:[0.0;4],page:0},width:1,height:1,pivot_x:0.0,pivot_y:0.0}).collect()}}
    #[test]fn strict_boundary_and_clamped_final_frame(){let a=animation();assert_eq!(sample(&a,0.0,false),0);assert_eq!(sample(&a,0.1,false),1);assert_eq!(sample(&a,10.0,false),2);assert_eq!(sample(&a,0.61,true),0);}
    #[test]fn ground_outline_width_is_constant_during_growth(){for radius in [0.5,50.0,100.0]{for spatial in [false,true]{let v=ring_instances(radius,spatial);let m=Mat4::from_cols_array_2d(&v[0].model);let a=m.transform_point3(Vec3::new(0.0,-1.0,0.0));let b=m.transform_point3(Vec3::new(0.0,1.0,0.0));assert!((a.distance(b)-if spatial{0.012}else{3.0}).abs()<1e-5);}}}
}
