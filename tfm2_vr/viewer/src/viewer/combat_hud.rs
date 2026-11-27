// hud.rs 안에 포함되는 전투 UI의 고정 크기 내용과 메모리 GDI 그리기.
pub const CANVAS_HEIGHT:u32=1280;
#[derive(Clone,Copy,Default,Hash,PartialEq,Eq,serde::Serialize)]
pub struct PlayerCard{pub cooldown_mask:u8,pub hp:u16,pub shield:u16,pub level:u16,pub alive:bool,pub respawn:u16,pub k:u16,pub d:u16,pub a:u16,pub cooldowns:[u16;3],pub items:[u16;6]}
#[derive(Clone,Copy,Default,Hash,PartialEq,Eq,serde::Serialize)]
pub struct FeedEntry{pub tick:u32,pub kind:u8,pub team:i8,pub killer:u8,pub victim:u8,pub assists:u8}
#[derive(Clone,Copy,Default,Hash,PartialEq,Eq,serde::Serialize)]
pub struct CombatContent{pub feed_width:u16,pub available:bool,pub flags:u8,pub kill_diff:i32,pub gold_diff:i32,pub towers:[u8;2],pub players:[[PlayerCard;5];2],pub feed:[FeedEntry;3]}
#[derive(Default)]struct CombatResources{names:[Vec<String>;2],items:Vec<RgbaImage>,labels:[String;3]}
fn default_cooldown_labels()->[String;3]{["기술 1".into(),"기술 2".into(),"궁극기".into()]}
#[derive(serde::Deserialize)]struct CombatManifest{files:Vec<String>,#[serde(rename="rosterNames")]names:[Vec<String>;2],#[serde(default="default_cooldown_labels",rename="cooldownLabels")]labels:[String;3]}
impl CombatResources{fn load()->Result<Self>{let root=crate::log::root().join("_cache/combat_ui");if !root.join("manifest.json").exists(){return Ok(Self::default());}let m:CombatManifest=crate::source::read_json(&root.join("manifest.json"))?;let mut items=vec![RgbaImage::new(1,1)];for file in m.files{let p=std::path::Path::new(&file);anyhow::ensure!(p.components().all(|c|matches!(c,std::path::Component::Normal(_))),"UI 아이콘 경로 범위 오류");items.push(image::open(root.join(p))?.to_rgba8());}Ok(Self{names:m.names,items,labels:m.labels})}}
pub fn atlas_uv(x:f32,y:f32,w:f32,h:f32)->[f32;4]{[x/1024.0,y/CANVAS_HEIGHT as f32,w/1024.0,h/CANVAS_HEIGHT as f32]}
fn sprite_scaled(data:&mut[u8],image:&RgbaImage,x:i32,y:i32,w:i32,h:i32,scale:i32,dim:f32){let stride=1024*scale;for yy in 0..h*scale{for xx in 0..w*scale{let p=image.get_pixel((xx as u32*image.width()/(w*scale)as u32).min(image.width()-1),(yy as u32*image.height()/(h*scale)as u32).min(image.height()-1));let k=(((y*scale+yy)*stride+x*scale+xx)*4)as usize;let alpha=p[3]as f32/255.0;for c in 0..3{data[k+c]=(p[2-c]as f32*alpha*dim+data[k+c]as f32*(1.0-alpha))as u8;}}}}
impl Hud{
 unsafe fn text_metric(&self,value:&str,rect:RECT,color:u32){let previous=SelectObject(self.dc,HGDIOBJ(self.metric.0));SetTextColor(self.dc,COLORREF(color));let mut wide:Vec<u16>=value.encode_utf16().collect();let s=self.scale as i32;let mut rect=RECT{left:rect.left*s,top:rect.top*s,right:rect.right*s,bottom:rect.bottom*s};DrawTextW(self.dc,&mut wide,&mut rect,DT_CENTER|DT_SINGLELINE|DT_VCENTER|DT_NOPREFIX);SelectObject(self.dc,previous);}
 unsafe fn text32(&self,value:&str,x:i32,y:i32,w:i32,h:i32,color:u32){let previous=SelectObject(self.dc,HGDIOBJ(self.medium.0));SetTextColor(self.dc,COLORREF(color));let mut value:Vec<u16>=value.encode_utf16().collect();let s=self.scale as i32;let mut rect=RECT{left:x*s,top:y*s,right:(x+w)*s,bottom:(y+h)*s};DrawTextW(self.dc,&mut value,&mut rect,DT_LEFT|DT_SINGLELINE|DT_VCENTER|DT_NOPREFIX);SelectObject(self.dc,previous);}
 unsafe fn render_combat(&self,data:&mut[u8],content:&CombatContent){
  let scale=self.scale as i32;let fill=|data:&mut[u8],x,y,w,h,color|fill_scaled(data,x,y,w,h,color,scale);
  // 정적 레벨·상태 라벨. 실제 표시 여부는 장면의 LOD와 값으로 정한다.
  fill(data,0,1152,840,48,[18,34,42,255]);fill(data,0,1216,960,48,[18,34,42,255]);
  for level in 0..21{self.text32(&level.to_string(),level*40+2,1152,38,48,0x00ffffff);}
  for (i,text)in ["공중","기절","속박","공격X","기술X","이동기X","강제","도발","공포","매혹"].iter().enumerate(){self.text(text,RECT{left:i as i32*96+2,top:1216,right:i as i32*96+94,bottom:1264},false,0x007fe8ff);}
  if !content.available{return;}
  if content.flags&1!=0{self.text_metric(&format!("킬 차 {:+} · 보유 골드 차 {:+} · 타워 {}:{}",content.kill_diff,content.gold_diff,content.towers[0],content.towers[1]),RECT{left:12,top:162,right:1012,bottom:218},0x00d5e0e3);}
  if content.flags&4!=0{for team in 0..2{let x=if team==0{0}else{544};let color=if team==0{[155,211,239,255]}else{[233,164,160,255]};fill(data,x,320,480,640,[18,34,42,255]);for lane in 0..5{
   let y=320+lane as i32*128;let p=content.players[team][lane];fill(data,x,y,480,2,color);
   if let Some(portrait)=self.portraits[team].get(lane){sprite_scaled(data,portrait,x+8,y+10,64,64,scale,if p.alive{1.0}else{0.34});}
   let name=self.resources.names[team].get(lane).map(String::as_str).unwrap_or("");self.text32(&format!("{name}  Lv.{}",p.level),x+88,y+1,388,36,0x00e7e9ef);
   self.text32(&format!("{}/{}/{}",p.k,p.d,p.a),x+88,y+34,170,34,0x00b6c9d0);if !p.alive{self.text32(&format!("{}초",p.respawn),x+364,y+34,112,34,0x0086d5ef);}
   for (i,id)in p.items.iter().enumerate(){if let Some(icon)=self.resources.items.get(*id as usize){if *id!=0{sprite_scaled(data,icon,x+268+i as i32*32,y+39,26,26,scale,1.0);}}}
   fill(data,x+88,y+72,380,9,[48,64,69,255]);let hp=(380*p.hp.min(1000)as i32)/1000;let shield=(380*p.shield.min(1000)as i32)/1000;fill(data,x+88,y+72,hp,9,color);fill(data,x+88+hp,y+72,shield.min(380-hp),9,[205,244,240,255]);
   let visible=p.cooldown_mask.count_ones().max(1)as i32;let width=(464-(visible-1)*8)/visible;let mut column=0;for slot in 0..3{if p.cooldown_mask&(1<<slot)==0{continue;}let left=x+8+column*(width+8);let ready=p.cooldowns[slot]==0;let border=if ready{color}else{[61,80,88,255]};fill(data,left,y+86,width,36,border);fill(data,left+2,y+88,width-4,32,if ready{[43,77,87,255]}else{[24,39,46,255]});let label=if ready{self.resources.labels[slot].clone()}else{format!("{} {}",self.resources.labels[slot],p.cooldowns[slot])};self.text(&label,RECT{left:left+3,top:y+87,right:left+width-3,bottom:y+121},false,if ready{0x00e4f1f2}else{0x0097c5e7});column+=1;}
  }}}
  if content.flags&34!=0{for(row,entry)in content.feed.iter().enumerate(){if entry.tick==0{continue;}let y=960+row as i32*64;fill(data,0,y,content.feed_width as i32,64,[18,34,42,255]);if entry.kind==0&&entry.team>=0&&entry.team<2{
   let team=entry.team as usize;if let Some(p)=self.portraits[team].get(entry.killer as usize){sprite_scaled(data,p,8,y+2,60,60,scale,1.0);}if let Some(p)=self.portraits[1-team].get(entry.victim as usize){sprite_scaled(data,p,122,y+2,60,60,scale,1.0);}self.text32("→",78,y+6,42,50,0x00ffffff);
   if entry.assists!=0{self.text32("도움",220,y+8,76,48,0x00c3d3d5);}let mut column=0;for lane in 0..5{if entry.assists&(1<<lane)!=0{if let Some(p)=self.portraits[team].get(lane){sprite_scaled(data,p,310+column*62,y+5,54,54,scale,1.0);column+=1;}}}
  }else{let text=match entry.kind{1=>"타워 파괴",2=>"넥서스 파괴",3=>"세르펜 제거",4=>"모르가드 제거",_=>"정글 목표 제거"};let team=match entry.team{0=>"파랑 ",1=>"빨강 ",_=>""};self.text32(&format!("{team}{text}"),20,y+7,content.feed_width as i32-32,50,0x00c9e6ed);}}}
 }
 fn combat_alpha(&self,x:usize,y:usize,c:&CombatContent)->bool{let x=x/self.scale;let y=y/self.scale;(y>=1152&&y<1200&&x<840)||(y>=1216&&y<1264&&x<960)||(c.available&&c.flags&4!=0&&y>=320&&y<960&&(x<480||x>=544))||(c.available&&c.flags&34!=0&&y>=960&&y<1152&&x<c.feed_width as usize&&c.feed[(y-960)/64].tick!=0)}
}
