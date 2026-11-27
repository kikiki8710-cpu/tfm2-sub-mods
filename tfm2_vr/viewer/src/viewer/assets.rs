use anyhow::{Context,Result,bail};
use serde::Deserialize;
use std::{collections::{HashMap,BTreeMap,HashSet},path::{Path,PathBuf}};
use image::{RgbaImage,GenericImageView};
use glam::{Mat4,Quat,Vec3};
use crate::{source::{self,Manifest},geometry::{self,Vertex,Instance,Mesh}};
pub const ATLAS_SIZE:u32=2048;
#[derive(Clone,Copy,Debug)]pub struct Region {pub uv:[f32;4],pub page:u32}
#[derive(Clone,Copy)]pub struct Cel {pub duration:f64,pub region:Region,pub width:u32,pub height:u32,pub pivot_x:f32,pub pivot_y:f32}
pub struct Animation {pub cels:Vec<Cel>,pub duration:f64}
pub struct ProjectileArt{pub animation:Animation,pub source_key:String,pub tag:String}
pub fn projectile_key(kind:&str,name:&str,team:i32)->String{format!("{kind}:{name}:{team}")}
pub fn projectile_art_key(kind:&str,name:&str,team:i32,category:u8)->String{format!("{}:{category}",projectile_key(kind,name,team))}
/// 0 대기·1 이동·2 공격(추정용)·3 사망·4 평타·5 기술1·6 기술2·7 궁
pub struct Visual {pub head:f32,pub anims:[Animation;8],pub has_action:[bool;4]}
#[derive(Deserialize)]pub struct Rect{pub x:u32,pub y:u32,pub w:u32,pub h:u32}
#[derive(Deserialize)]struct SourceRect{x:f64,y:f64,w:f64,h:f64}
impl SourceRect{fn pixels(&self,w:u32,h:u32)->Rect{let normalized=self.w<1.0&&self.h<1.0;let(sx,sy)=if normalized{(w as f64,h as f64)}else{(1.0,1.0)};Rect{x:(self.x*sx).round()as u32,y:(self.y*sy).round()as u32,w:(self.w*sx).round()as u32,h:(self.h*sy).round()as u32}}}
#[derive(Deserialize)]struct SourceCel{duration:f64,data:SourceRect}
#[derive(Deserialize)]struct SourceAnim{frames:Vec<SourceCel>}
#[derive(Deserialize)]struct Sprite{sheet:String,anims:BTreeMap<String,SourceAnim>}
#[derive(Deserialize)]struct MapImage{url:String}
#[derive(Deserialize)]struct AssetManifest{sprites:HashMap<String,Sprite>,maps:HashMap<String,MapImage>}
#[derive(Deserialize)]#[serde(rename_all="camelCase")]struct TerrainManifest{wall_top:String,bush_top:String,wall_side:Option<String>,#[serde(default)]bush_side:Option<String>}
#[derive(Deserialize)]struct TerrainMesh{position:Vec<f32>,uv:Vec<f32>,color:Vec<f32>,shade:Vec<f32>,#[serde(default)]region:Vec<u16>}
#[derive(Deserialize)]struct Rock{x:f32,y:f32,radius:f32,height:f32,color:[f32;3],seed:f32}
#[derive(Deserialize)]struct Tree{x:f32,y:f32,height:f32,seed:f32}
#[derive(Deserialize)]#[serde(rename_all="camelCase")]struct Terrain{pixel_scale:f32,meshes:HashMap<String,TerrainMesh>,rocks:Vec<Rock>,trees:Vec<Tree>,#[serde(default)]bush_runs:Vec<[usize;4]>}
struct Packer{pages:Vec<RgbaImage>,counts:[u32;3]}
impl Packer{
 fn new()->Self{Self{pages:Vec::new(),counts:[0;3]}}
 fn put(&mut self,image:&RgbaImage)->Result<Region>{let(w,h)=image.dimensions();let group=crate::mips::BUCKETS.iter().position(|&s|w+2<=s&&h+2<=s).context("텍스처 셀이 최대 크기를 넘었습니다")?;let size=crate::mips::BUCKETS[group];let layer=self.counts[group];if layer>=crate::mips::PAGE_STRIDE{bail!("셀 슬라이스 용량 초과");}let mut tile=RgbaImage::new(size,size);for y in 0..size{for x in 0..size{tile.put_pixel(x,y,*image.get_pixel(x.saturating_sub(1).min(w-1),y.saturating_sub(1).min(h-1)));}}self.pages.push(tile);self.counts[group]+=1;Ok(Region{uv:[1.0/size as f32,1.0/size as f32,w as f32/size as f32,h as f32/size as f32],page:group as u32*crate::mips::PAGE_STRIDE+layer})}
}
/// 챔피언 프레임 중심에서 바닥까지(픽셀). 대기 동작 76명 중 53명이 정확히 11.5, 나머지도 같은 선을 기준으로 숙이거나 떠 있다.
const CHAMPION_GROUND:f32=11.5;
fn clone_animation(a:&Animation)->Animation{Animation{cels:a.cels.clone(),duration:a.duration}}
/// 이름 붙은 이펙트의 원본 그림(AnimationEffectViewSystem 등록). follow 면 대상을 따라간다.
pub struct EffectArt{pub animation:Animation,pub follow:bool,pub hold:f64}
/// 투사체 종류별 원본 그림(0.7.3 등록 표). hit 은 같은 이름으로 등록된 타격 효과.
pub struct KindArt{pub key:String,pub tags:Vec<String>,pub system:String,pub phases:Vec<Animation>,pub hit:Option<Animation>}
pub struct Assets{pub effect_art:Vec<Option<EffectArt>>,pub kind_art:Vec<Option<KindArt>>,pub effect_demo_art:HashMap<String,ProjectileArt>,pub npc_projectile_fallback:HashSet<String>,pub combat_cooldown_masks:[[u8;5];2],pub combat_item_sets:HashMap<u32,[u16;6]>,pub bush_cells:Vec<u16>,pub closed_terrain:bool,pub pages:Vec<RgbaImage>,pub visuals:Vec<Visual>,pub visual_ids:Vec<[usize;3]>,pub meshes:Vec<Mesh>,pub generated:bool,pub portraits:[Vec<RgbaImage>;2],pub foot_audit:Vec<crate::visual_assets::FootAudit>,pub projectile_art:HashMap<String,ProjectileArt>}
pub fn cache_path(cache:&Path,url:&str)->Result<PathBuf>{let rel=url.strip_prefix("/cache/").context("자산 경로 접두사 오류")?.replace("%23","#").replace("%20"," ");let p=Path::new(&rel);if p.components().any(|c|!matches!(c,std::path::Component::Normal(_))){bail!("캐시 밖 자산 경로 거절");}Ok(cache.join(p))}
fn sprite_key(kind:&str,name:&str,team:i32)->String{if kind=="champion"||name=="small_jiangshi"{format!("champions/{}",if name=="small_jiangshi"{"jiangshi"}else{name})}else if kind=="tower"||name=="nexus"{format!("ingame/{}_{}",if team==0{"blue"}else{"red"},name)}else if kind=="minion"{"UI_aseprite/minion".into()}else{format!("ingame/{}",name.replace("_monster",""))}}
impl Assets {
 pub fn load(cache:&Path,meta:&Manifest,generated:bool)->Result<Self>{
  let mut src:AssetManifest=source::read_json(&cache.join("assets/manifest.json"))?;if cache.join("effects/manifest.json").exists(){let extra:AssetManifest=source::read_json(&cache.join("effects/manifest.json"))?;src.sprites.extend(extra.sprites);}let mut pack=Packer::new();let mut images:HashMap<String,RgbaImage>=HashMap::new();let mut visual_map:HashMap<(String,i32),usize>=HashMap::new();let mut visuals=Vec::new();let mut foot_audit=Vec::new();let max=meta.catalog.keys().copied().max().unwrap_or(0)as usize;let mut visual_ids=vec![[0;3];max+1];
  for (&id,m)in &meta.catalog {for (slot,team)in [(0,0),(1,1),(2,-1)]{let key=sprite_key(&m.kind,&m.name,team);let map_key=(format!("{}|{}",key,if m.name=="small_jiangshi"{"mini"}else{"normal"}),if m.kind=="minion"{team}else{0});let index=if let Some(&i)=visual_map.get(&map_key){i}else{
   let sprite=src.sprites.get(&key).with_context(||format!("스프라이트 누락: {key}"))?;if !images.contains_key(&sprite.sheet){images.insert(sprite.sheet.clone(),image::open(cache_path(cache,&sprite.sheet)?)?.to_rgba8());}
   let ground=if m.kind=="champion"{CHAMPION_GROUND}else{let idle=if m.kind=="minion"{format!("{}_{}_idle",if m.name.starts_with("range"){"range"}else{"melee"},if team==0{"blue"}else{"red"})}else if m.name=="small_jiangshi"{"idle_mini".into()}else{"idle".to_string()};let image=&images[&sprite.sheet];let frames:Vec<RgbaImage>=sprite.anims.get(&idle).or_else(||sprite.anims.iter().find(|(k,_)|k.ends_with("idle")).map(|(_,a)|a)).map(|a|a.frames.iter().map(|f|{let r=f.data.pixels(image.width(),image.height());image.view(r.x,r.y,r.w.min(image.width()-r.x),r.h.min(image.height()-r.y)).to_image()}).collect()).unwrap_or_default();crate::visual_assets::idle_ground(&frames)};let mut animations=Vec::new();let mut has_action=[false;4];for(index,state)in ["idle","run","attack","dead","attack","skill","skill2","ult"].into_iter().enumerate(){let epic=key=="ingame/epic";if index>=4&&!(epic&&index==5){if m.kind!="champion"{animations.push(clone_animation(&animations[0]));continue;}let wanted=if state=="skill"&&!sprite.anims.contains_key("skill"){"skill1"}else{state};if !sprite.anims.contains_key(wanted){let base=if sprite.anims.contains_key("attack"){2}else{0};animations.push(clone_animation(&animations[base]));continue;}has_action[index-4]=true;if index==4{animations.push(clone_animation(&animations[2]));continue;}}let state=if epic&&index==2{"attack_left"}else if epic&&index==5{"attack_right"}else if index==5&&!sprite.anims.contains_key("skill"){"skill1"}else{state};let tag=if m.kind=="minion"{format!("{}_{}_{}",if m.name.starts_with("range"){ "range" }else{"melee"},if team==0{"blue"}else{"red"},state)}else if m.name=="small_jiangshi"{format!("{state}_mini")}else{state.to_string()};let anim=sprite.anims.get(&tag).or_else(||sprite.anims.iter().find(|(k,_)|k.ends_with("idle")).map(|(_,a)|a)).context("idle 애니메이션 없음")?;let mut raw=Vec::new();for f in &anim.frames{let image=&images[&sprite.sheet];let r=f.data.pixels(image.width(),image.height());if r.x+r.w>image.width()||r.y+r.h>image.height(){bail!("도트 프레임 범위 오류");}raw.push((f.duration,image.view(r.x,r.y,r.w,r.h).to_image()));}let registered=crate::visual_assets::register(raw,format!("{key}:{tag}"),ground);let mut cels=Vec::new();for f in registered.frames{let region=pack.put(&f.image)?;cels.push(Cel{duration:f.duration,region,width:f.image.width(),height:f.image.height(),pivot_x:registered.pivot[0],pivot_y:registered.pivot[1]});}foot_audit.push(registered.audit);let duration=cels.iter().map(|c|c.duration).sum();animations.push(Animation{cels,duration});}let head=animations[0].cels.first().map(|c|c.pivot_y).unwrap_or(0.0);let visual=Visual{head,has_action,anims:animations.try_into().map_err(|_|anyhow::anyhow!("애니메이션 상태 수 오류"))?};let i=visuals.len();visuals.push(visual);visual_map.insert(map_key,i);i};visual_ids[id as usize][slot]=index;}}
  let portraits=std::array::from_fn(|team|meta.roster[team].iter().map(|name|{let sprite=&src.sprites[&format!("champions/{name}")];let anim=sprite.anims.get("idle").unwrap();let image=&images[&sprite.sheet];let f=&anim.frames[0];let rect=f.data.pixels(image.width(),image.height());crate::visual_assets::portrait(&image.view(rect.x,rect.y,rect.w,rect.h).to_image())}).collect());let mut meshes=Vec::new();for (i,key)in ["background_5v5","wall_shadow_5v5","bush_shadow_5v5"].iter().enumerate(){let image=image::open(cache_path(cache,&src.maps[*key].url)?)?.to_rgba8();let image=image.view(0,96,1280,1088).to_image();let region=pack.put(&image)?;let model=Mat4::from_scale_rotation_translation(Vec3::new(2.4,2.04,1.0),Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2),Vec3::new(0.0,0.76+i as f32*0.0003,0.0));meshes.push(geometry::mesh(key,geometry::quad(),vec![Instance::new(model,region.uv,[1.0;4],[1.0,0.0,0.0,region.page as f32])],true,false));}
  let closed_terrain=!std::env::args().any(|a|a=="--legacy-terrain");let terrain_dir=if closed_terrain{"terrain_closed"}else{"terrain"};let tm:TerrainManifest=source::read_json(&cache.join(terrain_dir).join("manifest.json"))?;let mut t:Terrain=source::read_json(&cache.join(terrain_dir).join("geometry.json.gz"))?;let mut bush_cells=vec![0u16;1280*1280];for &[y,x0,x1,id]in &t.bush_runs{anyhow::ensure!(y<1280&&x0<x1&&x1<=1280&&id<256,"수풀 영역 범위 오류");bush_cells[y*1280+x0..y*1280+x1].fill(id as u16);}if closed_terrain{for kind in ["wall","bush"]{let bottom=t.meshes.remove(&format!("{kind}Bottom")).context("지형 바닥 누락")?;let top=t.meshes.get_mut(&format!("{kind}Top")).unwrap();top.position.extend(bottom.position);top.uv.extend(bottom.uv);top.color.extend(bottom.color);top.shade.extend(bottom.shade);top.region.extend(bottom.region);}}let bush_side=tm.bush_side.as_ref().map(|p|cache_path(cache,p)).transpose()?.map(|p|image::open(p).map(|i|i.to_rgba8())).transpose()?.map(|i|pack.put(&i)).transpose()?;let wall=pack.put(&image::open(cache_path(cache,&tm.wall_top)?)?.to_rgba8())?;let bush=pack.put(&image::open(cache_path(cache,&tm.bush_top)?)?.to_rgba8())?;
  let generated_side=if generated{tm.wall_side.as_ref().map(|p|cache_path(cache,p)).transpose()?.filter(|p|p.exists()).map(|p|image::open(p).map(|i|i.to_rgba8())).transpose()?.map(|i|pack.put(&i)).transpose()?}else{None};let used_generated=generated_side.is_some();let original=pack.put(&crate::visual_assets::original_wall(cache)?)?;let side=Some(generated_side.unwrap_or(original));
  for (name,image,opacity,blend)in [("wallTop",Some(wall),1.0,false),("wallSide",side,1.0,false),("bushTop",Some(bush),0.48,true),("bushSide",bush_side,0.30,true)]{let raw=&t.meshes[name];let mut vertices=Vec::with_capacity(raw.position.len()/3);for i in 0..raw.position.len()/3{let c=if image.is_some(){if name=="wallSide"{&raw.shade}else{&raw.color}}else{&raw.color};vertices.push(Vertex{position:[raw.position[i*3],raw.position[i*3+1],raw.position[i*3+2]],uv:[raw.uv[i*2],raw.uv[i*2+1]],color:if closed_terrain&&name.starts_with("bush"){[if name=="bushTop"{1.35}else{c[i*3]*1.35},if name=="bushTop"{1.35}else{c[i*3+1]*1.35},if name=="bushTop"{1.35}else{c[i*3+2]*1.35},raw.region[i]as f32]}else if name=="wallTop"||name=="bushTop"{[1.0;4]}else{[c[i*3],c[i*3+1],c[i*3+2],1.0]}});}let region=image.unwrap_or(Region{uv:[0.0;4],page:0});let mode=if closed_terrain&&name.starts_with("bush"){if name=="bushSide"{9.0}else{8.0}}else if image.is_some(){if name=="wallSide"{6.0}else{1.0}}else{0.0};let opacity=if closed_terrain&&name.starts_with("bush"){1.0}else{opacity};let inst=Instance::new(Mat4::from_translation(Vec3::new(0.0,0.761,0.0)),region.uv,[1.0,1.0,1.0,opacity],[mode,if blend{0.08}else{0.12},if name=="bushTop"{0.42}else{0.28},region.page as f32]);meshes.push(geometry::mesh(name,vertices,vec![inst],blend,!blend));}
  let rocks=t.rocks.iter().map(|r|geometry::solid_instance(Mat4::from_scale_rotation_translation(Vec3::new(r.radius*1.2,r.height*0.7,r.radius),Quat::from_rotation_y(r.seed),Vec3::new((r.x-640.0)*t.pixel_scale,0.76+r.height*0.38,(r.y-640.0)*t.pixel_scale)),geometry::rgb(r.color,1.0))).collect();meshes.push(geometry::mesh("edgeRocks",geometry::icosahedron(),rocks,false,true));
  let mut branches=Vec::new();for tree in &t.trees{let x=(tree.x-640.0)*t.pixel_scale;let z=(tree.y-640.0)*t.pixel_scale;let h=tree.height;let lean=if tree.seed as i32%2!=0{1.0}else{-1.0}*h*0.1;let base=Vec3::new(x,0.76,z);branches.push(geometry::solid_instance(geometry::branch(base,Vec3::new(x+lean,0.76+h,z),0.011,0.010),geometry::color_hex(0x4f4c3e,1.0)));for j in 0..7{let u=0.34+j as f32*0.085;let a=j as f32*2.4+tree.seed;let origin=Vec3::new(x+lean*u,0.76+h*u,z);let length=h*(0.42-j as f32*0.027);let end=origin+Vec3::new(a.cos()*length,length*0.45,a.sin()*length*0.6);branches.push(geometry::solid_instance(geometry::branch(origin,end,0.0045-j as f32*0.0004,0.004-j as f32*0.00035),geometry::color_hex(0x4f4c3e,1.0)));let twig=end+Vec3::new((a+0.8).cos()*length*0.35,length*0.35,(a+0.8).sin()*length*0.3);branches.push(geometry::solid_instance(geometry::branch(end,twig,0.0018,0.0018),geometry::color_hex(0x4f4c3e,1.0)));}}meshes.push(geometry::mesh("edgeTrees",geometry::cylinder(),branches,false,true));
  let boxes=[(Vec3::new(2.52,0.12,2.16),Vec3::new(0.0,0.695,0.0),0x334b4a),(Vec3::new(2.4,0.035,2.04),Vec3::new(0.0,0.735,0.0),0x354b42),(Vec3::new(0.62,0.66,0.62),Vec3::new(0.0,0.33,0.0),0x1a3035)].iter().map(|&(scale,p,c)|geometry::solid_instance(Mat4::from_scale_rotation_translation(scale,Quat::IDENTITY,p),geometry::color_hex(c,1.0))).collect();meshes.push(geometry::mesh("table",geometry::box_vertices(),boxes,false,true));
  let effect_demo_art=if std::env::args().any(|a|a=="--effect-demo-compare"){load_effect_demo_art(&src,&mut images,&mut pack,cache)?}else{HashMap::new()};
  let projectile_art=load_projectile_art(&src,&mut images,&mut pack,cache,meta)?;let kind_art=load_kind_art(&src,&mut images,&mut pack,cache,meta)?;let effect_art=load_effect_art(&src,&mut images,&mut pack,cache,meta)?;let mut npc_projectile_fallback=HashSet::new();for m in meta.catalog.values().filter(|m|m.kind=="minion"||m.kind=="tower"){for team in[0,1]{let key=sprite_key(&m.kind,&m.name,team);let Some(sprite)=src.sprites.get(&key)else{continue;};let wanted=if m.kind=="minion"{if team==0{"blue_projectile"}else{"red_projectile"}}else{"attack_projectile"};let count=sprite.anims.keys().filter(|tag|tag.contains("projectile")&&(m.kind!="minion"||tag.starts_with(if team==0{"blue_"}else{"red_"}))).count();if count==1&&sprite.anims.contains_key(wanted){npc_projectile_fallback.insert(projectile_key(&m.kind,&m.name,team));}}}
  let icon_file=cache.join("combat_ui/manifest.json");let combat_cooldown_masks=if icon_file.exists(){let data:serde_json::Value=source::read_json(&icon_file)?;data.get("cooldownMasks").map(|v|serde_json::from_value(v.clone())).transpose()?.unwrap_or([[0u8;5];2])}else{[[7u8;5];2]};let icons:HashMap<String,u16>=if icon_file.exists(){let data:serde_json::Value=source::read_json(&icon_file)?;serde_json::from_value(data["icons"].clone())?}else{HashMap::new()};let combat_item_sets=meta.item_sets.iter().map(|(&id,keys)|{let mut slots=[0;6];for(i,key)in keys.iter().take(6).enumerate(){slots[i]=icons.get(key).copied().unwrap_or(0);}(id,slots)}).collect();Ok(Self{effect_art,kind_art,effect_demo_art,npc_projectile_fallback,combat_cooldown_masks,combat_item_sets,bush_cells,closed_terrain,pages:pack.pages,visuals,visual_ids,meshes,generated:used_generated,portraits,foot_audit,projectile_art})
 }
 pub fn npc_fallback_allowed(&self,meta:&crate::source::EntityMeta,team:i32,category:u8)->bool{category==0&&self.npc_projectile_fallback.contains(&projectile_key(&meta.kind,&meta.name,team))}
 pub fn projectile_art_for(&self,meta:&crate::source::EntityMeta,team:i32,category:u8)->Option<&ProjectileArt>{let category=if self.npc_fallback_allowed(meta,team,category){1}else{category};self.projectile_art.get(&projectile_art_key(&meta.kind,&meta.name,team,category))}
 pub fn visual(&self,id:usize,team:i32)->&Visual{&self.visuals[self.visual_ids[id][if team==0{0}else if team==1{1}else{2}]]}
}

fn load_projectile_art(src:&AssetManifest,images:&mut HashMap<String,RgbaImage>,pack:&mut Packer,cache:&Path,meta:&Manifest)->Result<HashMap<String,ProjectileArt>>{
 let mut result=HashMap::new();
 for m in meta.catalog.values(){for team in [0,1,-1]{for category in 1..=4{
  let output=projectile_art_key(&m.kind,&m.name,team,category);if result.contains_key(&output){continue;}
  let mut candidates:Vec<(String,String)>=Vec::new();let key=sprite_key(&m.kind,&m.name,team);
  if category==1{
   match m.name.as_str(){"soldier"=>candidates.push(("sprite/bullet".into(),"idle".into())),"gunner"=>candidates.push(("sprite/gunner_bullet".into(),"idle".into())),"archer"=>candidates.push(("sprite/arrow".into(),"idle".into())),_=>{}}
   if m.kind=="minion"{candidates.push(("UI_aseprite/minion".into(),if team==0{"blue_projectile"}else{"red_projectile"}.into()));}
   else if m.kind=="tower"{candidates.push((key.clone(),"attack_projectile".into()));}
   else{for tag in ["attack_projectile","projectile"]{candidates.push((key.clone(),tag.into()));}if m.name=="gambler"{candidates.push(("skill_effect/gambler_projectile".into(),"attack".into()));}}
  }else{
   // 종류가 추정된 경우에도 해당 종류의 태그만 쓴다. 다른 종류로 대체하지 않는다.
   let tag=match category{2=>"skill_projectile",3=>"skill2_projectile",4=>"ult_projectile",_=>unreachable!()};candidates.push((key.clone(),tag.into()));
   if m.name=="soldier"&&category==2{candidates.insert(0,("sprite/bullet".into(),"idle".into()));}
  }
  let Some((key,tag))=candidates.into_iter().find(|(k,t)|src.sprites.get(k).is_some_and(|s|s.anims.contains_key(t)))else{continue;};
  let sprite=&src.sprites[&key];if !images.contains_key(&sprite.sheet){images.insert(sprite.sheet.clone(),image::open(cache_path(cache,&sprite.sheet)?)?.to_rgba8());}let im=&images[&sprite.sheet];let mut cels=Vec::new();
  for f in &sprite.anims[&tag].frames{let rect=f.data.pixels(im.width(),im.height());let image=im.view(rect.x,rect.y,rect.w,rect.h).to_image();let region=pack.put(&image)?;cels.push(Cel{duration:f.duration,region,width:rect.w,height:rect.h,pivot_x:rect.w as f32/2.0,pivot_y:rect.h as f32/2.0});}
  let duration=cels.iter().map(|c|c.duration).sum();result.insert(output,ProjectileArt{animation:Animation{cels,duration},source_key:key,tag});
 }}}Ok(result)
}

#[derive(Deserialize)]struct EffectArtRow{key:String,tag:String,follow:bool,#[serde(default)]hold:f64}
#[derive(Deserialize)]struct EffectArtFile{table:HashMap<String,Option<EffectArtRow>>}
fn load_effect_art(src:&AssetManifest,images:&mut HashMap<String,RgbaImage>,pack:&mut Packer,cache:&Path,meta:&Manifest)->Result<Vec<Option<EffectArt>>>{
 let path=cache.join("effects/effect_art.json");if meta.effect_names.is_empty()||!path.exists(){return Ok(Vec::new());}
 let file:EffectArtFile=source::read_json(&path)?;let mut out=Vec::new();
 for name in &meta.effect_names{out.push(match file.table.get(name).and_then(Option::as_ref){Some(r)=>pack_animation(src,images,pack,cache,&r.key,&r.tag)?.map(|animation|EffectArt{animation,follow:r.follow,hold:r.hold}),None=>None});}
 Ok(out)
}
#[derive(Deserialize)]struct KindArtRow{system:Option<String>,key:Option<String>,tags:Vec<String>,hit:Option<KindHitRow>}
#[derive(Deserialize)]struct KindHitRow{key:String,tag:String}
#[derive(Deserialize)]struct KindArtFile{table:HashMap<String,KindArtRow>}
fn pack_animation(src:&AssetManifest,images:&mut HashMap<String,RgbaImage>,pack:&mut Packer,cache:&Path,key:&str,tag:&str)->Result<Option<Animation>>{
 let Some(sprite)=src.sprites.get(key)else{return Ok(None);};let Some(anim)=sprite.anims.get(tag)else{return Ok(None);};
 if !images.contains_key(&sprite.sheet){images.insert(sprite.sheet.clone(),image::open(cache_path(cache,&sprite.sheet)?)?.to_rgba8());}let im=&images[&sprite.sheet];let mut cels=Vec::new();
 for f in &anim.frames{let rect=f.data.pixels(im.width(),im.height());let image=im.view(rect.x,rect.y,rect.w,rect.h).to_image();let region=pack.put(&image)?;cels.push(Cel{duration:f.duration,region,width:rect.w,height:rect.h,pivot_x:rect.w as f32/2.0,pivot_y:rect.h as f32/2.0});}
 let duration=cels.iter().map(|c|c.duration).sum();Ok(Some(Animation{cels,duration}))
}
fn load_kind_art(src:&AssetManifest,images:&mut HashMap<String,RgbaImage>,pack:&mut Packer,cache:&Path,meta:&Manifest)->Result<Vec<Option<KindArt>>>{
 let path=cache.join("effects/kind_art.json");if meta.projectile_kinds.is_empty()||!path.exists(){return Ok(Vec::new());}
 let file:KindArtFile=source::read_json(&path)?;let mut out=Vec::new();
 for name in &meta.projectile_kinds{let Some(row)=file.table.get(name)else{out.push(None);continue;};
  let mut phases=Vec::new();if let Some(key)=&row.key{for tag in &row.tags{if let Some(a)=pack_animation(src,images,pack,cache,key,tag)?{phases.push(a);}}}
  let hit=match &row.hit{Some(h)=>pack_animation(src,images,pack,cache,&h.key,&h.tag)?,None=>None};
  if phases.is_empty()&&hit.is_none(){out.push(None);continue;}
  out.push(Some(KindArt{key:row.key.clone().unwrap_or_default(),tags:row.tags.clone(),system:row.system.clone().unwrap_or_default(),phases,hit}));}
 Ok(out)
}

// 별도 시연 모드에서만 읽는다. 경기용 자산 목록·애니메이션 선택은 유지한다.
fn load_effect_demo_art(src:&AssetManifest,images:&mut HashMap<String,RgbaImage>,pack:&mut Packer,cache:&Path)->Result<HashMap<String,ProjectileArt>>{
 let mut out=HashMap::new();
 for(id,key,tag)in[("projectile","champions/gambler","skill2_effect"),("ground","skill_effect/pyromancer_skill2_effect","skill2_loop"),("skill","champions/gambler","skill2")]{
  let sprite=src.sprites.get(key).with_context(||format!("시연 원본 자산 없음 {key}"))?;
  if !images.contains_key(&sprite.sheet){images.insert(sprite.sheet.clone(),image::open(cache_path(cache,&sprite.sheet)?)?.to_rgba8());}
  let im=&images[&sprite.sheet];let anim=sprite.anims.get(tag).context("시연 원본 태그 없음")?;let mut cels=Vec::new();
  for f in &anim.frames{let rect=f.data.pixels(im.width(),im.height());anyhow::ensure!(rect.x+rect.w<=im.width()&&rect.y+rect.h<=im.height(),"시연 셀 범위 초과");let image=im.view(rect.x,rect.y,rect.w,rect.h).to_image();let region=pack.put(&image)?;cels.push(Cel{duration:f.duration,region,width:rect.w,height:rect.h,pivot_x:rect.w as f32/2.0,pivot_y:rect.h as f32/2.0});}
  let duration=cels.iter().map(|c|c.duration).sum();out.insert(id.into(),ProjectileArt{animation:Animation{cels,duration},source_key:key.into(),tag:tag.into()});
 }Ok(out)
}
