use std::sync::atomic::{AtomicU64,Ordering};
static BUILDS:AtomicU64=AtomicU64::new(0);
pub fn builds()->u64{BUILDS.load(Ordering::Relaxed)}
use image::RgbaImage;
pub const BUCKETS:[u32;3]=[128,256,2048];
pub const PAGE_STRIDE:u32=2048;
pub fn decode(c:u8)->f32{let v=c as f32/255.0;if v<=0.04045{v/12.92}else{((v+0.055)/1.055).powf(2.4)}}
fn encode(v:f32)->u8{let v=v.clamp(0.0,1.0);let s=if v<=0.0031308{v*12.92}else{1.055*v.powf(1.0/2.4)-0.055};(s*255.0).round()as u8}
pub fn premultiply(source:&RgbaImage)->RgbaImage{let mut out=source.clone();for p in out.pixels_mut(){if p[3]==255{continue;}if p[3]==0{p.0=[0;4];continue;}let a=p[3]as f32/255.0;for k in 0..3{p[k]=encode(decode(p[k])*a);}}out}
/// 셀마다 독립된 슬라이스. 선형 RGB에 알파를 곱한 뒤 sRGB로 저장한다.
pub fn chain(source:&RgbaImage)->Vec<RgbaImage>{BUILDS.fetch_add(1,Ordering::Relaxed);let mut w=source.width();let mut h=source.height();let mut linear:Vec<[f32;4]>=source.pixels().map(|p|{let a=p[3]as f32/255.0;[decode(p[0])*a,decode(p[1])*a,decode(p[2])*a,a]}).collect();let mut levels=Vec::new();loop{let mut image=RgbaImage::new(w,h);for(p,c)in image.pixels_mut().zip(&linear){*p=image::Rgba([encode(c[0]),encode(c[1]),encode(c[2]),(c[3]*255.0).round()as u8]);}levels.push(image);if w==1&&h==1{break;}let(nw,nh)=((w/2).max(1),(h/2).max(1));let mut next=vec![[0.0f32;4];(nw*nh)as usize];for y in 0..nh{for x in 0..nw{for dy in 0..2{for dx in 0..2{let c=linear[(((y*2+dy).min(h-1))*w+(x*2+dx).min(w-1))as usize];for k in 0..4{next[(y*nw+x)as usize][k]+=c[k]*0.25;}}}}}linear=next;w=nw;h=nh;}levels}
#[cfg(test)]mod tests{use super::*;#[test]fn alpha_mips_do_not_import_invisible_color(){let mut im=RgbaImage::from_pixel(2,2,image::Rgba([0,255,0,0]));im.put_pixel(0,0,image::Rgba([255,0,0,255]));let m=chain(&im);let p=m[1].get_pixel(0,0);assert_eq!(p[1],0);assert_eq!(p[3],64);assert!((decode(p[0])/0.25-1.0).abs()<0.02);}}
