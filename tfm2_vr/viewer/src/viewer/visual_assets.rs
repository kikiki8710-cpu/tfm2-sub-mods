use anyhow::Result;
use image::RgbaImage;
use serde::Serialize;
use std::path::Path;
#[derive(Clone,Serialize)]pub struct FootAudit{pub name:String,pub frames:usize,pub canvas:[u32;2],pub pivot:[f32;2],pub old_bottom_padding_range:[f32;2],pub max_foot_residual_px:f32}
pub struct RegisteredCel{pub duration:f64,pub image:RgbaImage}
pub struct Registered{pub frames:Vec<RegisteredCel>,pub pivot:[f32;2],pub audit:FootAudit}
fn bounds(image:&RgbaImage,threshold:u8)->Option<[u32;4]>{let(mut x0,mut y0,mut x1,mut y1)=(image.width(),image.height(),0,0);let mut any=false;for(x,y,p)in image.enumerate_pixels(){if p[3]>=threshold{x0=x0.min(x);y0=y0.min(y);x1=x1.max(x+1);y1=y1.max(y+1);any=true;}}if any{Some([x0,y0,x1,y1])}else{None}}
/// 프레임 캔버스 중심에서 실제 바닥선까지의 거리(대기 동작에서 가장 아래). 프레임마다 다시 재면 칼끝·물보라에 끌려간다.
pub fn idle_ground(frames:&[RgbaImage])->f32{frames.iter().filter_map(|im|bounds(im,128).map(|b|b[3]as f32-im.height()as f32/2.0)).reduce(f32::max).unwrap_or(0.0)}
/// 원본 프레임은 캔버스 중심이 기준점이다(중심 대칭으로 잘린 홀수 크기). 모든 프레임을 중심에 맞추고 바닥선은 스프라이트당 하나만 쓴다.
pub fn register(frames:Vec<(f64,RgbaImage)>,name:String,ground:f32)->Registered{
 let info:Vec<_>=frames.iter().map(|(_,im)|{let b=bounds(im,39).unwrap_or([0,0,1,1]);(b,(im.width()/2)as i32,(im.height()/2)as i32)}).collect();let(mut left,mut right,mut up,mut down)=(1i32,1i32,1i32,1i32);let(mut low,mut high)=(f32::INFINITY,0.0f32);
 for((_,im),(b,cx,cy))in frames.iter().zip(&info){if bounds(im,39).is_none(){continue;}left=left.max(*cx-b[0]as i32);right=right.max(b[2]as i32-*cx);up=up.max(*cy-b[1]as i32);down=down.max(b[3]as i32-*cy);let margin=im.height()as f32-b[3]as f32;low=low.min(margin);high=high.max(margin);}
 let(w,h)=((left+right+4)as u32,(up+down+4)as u32);let(px0,py0)=(left+2,up+2);let pivot=[px0 as f32+0.5,py0 as f32+0.5+ground];let count=frames.len();let mut registered=Vec::new();let mut residual=0.0f32;
 for((duration,im),(b,cx,cy))in frames.into_iter().zip(info){residual=residual.max((px0 as f32-cx as f32+im.width()as f32/2.0-pivot[0]).abs());let mut image=RgbaImage::new(w,h);if bounds(&im,39).is_some(){for y in b[1]..b[3]{for x in b[0]..b[2]{let(px,py)=(x as i32+px0-cx,y as i32+py0-cy);if px>=0&&py>=0&&px<w as i32&&py<h as i32{image.put_pixel(px as u32,py as u32,*im.get_pixel(x,y));}}}}registered.push(RegisteredCel{duration,image});}
 if !low.is_finite(){low=0.0;}Registered{frames:registered,pivot,audit:FootAudit{name,frames:count,canvas:[w,h],pivot,old_bottom_padding_range:[low,high],max_foot_residual_px:residual}}
}
pub fn original_wall(cache:&Path)->Result<RgbaImage>{let source=image::open(cache.join("terrain/wall_front_reference.png"))?.to_rgba8();let mut body=image::imageops::crop_imm(&source,2,6,28,10).to_image();for p in body.pixels_mut(){if p[3]<39{*p=image::Rgba([35,54,61,255]);}else if (p[0]as u32+p[1]as u32+p[2]as u32)<90{*p=image::Rgba([24,37,43,255]);}}let mut tile=RgbaImage::new(56,20);for y in 0..20{for x in 0..56{tile.put_pixel(x,y,*body.get_pixel(if x<28{x}else{55-x},if y<10{y}else{19-y}));}}Ok(tile)}
pub fn portrait(image:&RgbaImage)->RgbaImage{let b=bounds(image,128).unwrap_or([0,0,image.width(),image.height()]);let size=(((b[3]-b[1])as f32*0.60).round()as u32).clamp(12,32);let(mut sum,mut count)=(0u64,0u64);for y in b[1]..(b[1]+size).min(b[3]){for x in b[0]..b[2]{if image.get_pixel(x,y)[3]>=128{sum+=x as u64;count+=1;}}}let cx=if count>0{(sum/count)as i32}else{((b[0]+b[2])/2)as i32};let mut crop=RgbaImage::new(size,size);for y in 0..size{for x in 0..size{let sx=cx+x as i32-size as i32/2;let sy=b[1]+y;if sx>=0&&sx<image.width()as i32&&sy<image.height(){crop.put_pixel(x,y,*image.get_pixel(sx as u32,sy));}}}image::imageops::resize(&crop,64,64,image::imageops::FilterType::Nearest)}
#[cfg(test)]mod tests{use super::*;
 #[test]fn 중심기준_칼끝과_물보라에_끌리지_않는다(){let body=|w:u32,h:u32,extra:Option<(u32,u32)>|{let mut im=RgbaImage::new(w,h);let(cx,cy)=(w/2,h/2);for y in cy-10..=cy+10{for x in cx-4..=cx+4{im.put_pixel(x,y,image::Rgba([200,150,90,255]));}}if let Some((x,y))=extra{im.put_pixel(x,y,image::Rgba([9,9,9,255]));}im};
  let frames=vec![(0.1,body(31,31,None)),(0.1,body(61,41,Some((0,40)))),(0.1,body(101,121,Some((90,120))))];let ground=idle_ground(&[frames[0].1.clone()]);assert_eq!(ground,10.5);let r=register(frames,"test".into(),ground);
  for f in &r.frames{let mut sum=0.0;let mut n=0;let mut bottom=0;for(x,y,p)in f.image.enumerate_pixels(){if p[0]==200{sum+=x as f32+0.5;n+=1;bottom=bottom.max(y+1);}}assert!((sum/n as f32-r.pivot[0]).abs()<1e-3);assert_eq!(bottom as f32,r.pivot[1]);}assert_eq!(r.audit.max_foot_residual_px,0.0);}
}
