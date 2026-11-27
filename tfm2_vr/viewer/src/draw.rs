use anyhow::{bail,Context,Result};
use glam::{Mat4,Vec3};
use std::{mem::size_of,fs::File,io::BufWriter,slice};
use windows::{core::{s,PCSTR},Win32::{Foundation::{HMODULE,LUID},Graphics::{Direct3D::*,Direct3D::Fxc::{D3DCompile,D3DCOMPILE_ENABLE_STRICTNESS},Direct3D11::*,Dxgi::*,Dxgi::Common::*}}};
use crate::log::{Log,root};

#[repr(C)] #[derive(Clone,Copy)] struct Vertex { p:[f32;3],c:[f32;3] }
pub struct Target { pub color:ID3D11RenderTargetView, depth:ID3D11DepthStencilView, width:u32,height:u32 }
pub struct Draw { pub device:ID3D11Device, context:ID3D11DeviceContext, vs:ID3D11VertexShader,ps:ID3D11PixelShader,layout:ID3D11InputLayout,vertices:ID3D11Buffer,constants:ID3D11Buffer,raster:ID3D11RasterizerState,depth_state:ID3D11DepthStencilState,count:u32,log:Log }
const SHADER:&str=r#"
cbuffer Camera : register(b0) { column_major float4x4 mvp; };
struct Input { float3 position:POSITION; float3 color:COLOR; };
struct Output { float4 position:SV_POSITION; float3 color:COLOR; };
Output vs_main(Input i) { Output o;o.position=mul(mvp,float4(i.position,1));o.color=i.color;return o; }
float4 ps_main(Output i):SV_TARGET { return float4(i.color,1); }
"#;
fn vertices()->Vec<Vertex> {
 let mut v=Vec::new();
 fn quad(v:&mut Vec<Vertex>,p:[[f32;3];4],c:[f32;3]){for i in [0,1,2,0,2,3]{v.push(Vertex{p:p[i],c});}}
 quad(&mut v,[[-12.,0.,-12.],[-12.,0.,12.],[12.,0.,12.],[12.,0.,-12.]],[0.49,0.63,0.65]);
 for i in -24..=24 {let q=i as f32*0.5;let d=0.0025;let c=if i%2==0{[0.24,0.40,0.46]}else{[0.38,0.54,0.57]};quad(&mut v,[[q-d,0.002,-12.],[q+d,0.002,-12.],[q+d,0.002,12.],[q-d,0.002,12.]],c);quad(&mut v,[[-12.,0.002,q-d],[12.,0.002,q-d],[12.,0.002,q+d],[-12.,0.002,q+d]],c);}
 for (x,c) in [(-0.48,[0.94,0.12,0.09]),(0.,[0.10,0.73,0.31]),(0.48,[0.11,0.35,0.94])] {
  let p=|dx:f32,dy:f32,dz:f32|[x+dx*0.14,1.35+dy*0.14,-1.4+dz*0.14];
  for (face,shade) in [([p(-1.,-1.,1.),p(1.,-1.,1.),p(1.,1.,1.),p(-1.,1.,1.)],1.),([p(1.,-1.,-1.),p(-1.,-1.,-1.),p(-1.,1.,-1.),p(1.,1.,-1.)],0.7),([p(-1.,-1.,-1.),p(-1.,-1.,1.),p(-1.,1.,1.),p(-1.,1.,-1.)],0.78),([p(1.,-1.,1.),p(1.,-1.,-1.),p(1.,1.,-1.),p(1.,1.,1.)],0.65),([p(-1.,1.,1.),p(1.,1.,1.),p(1.,1.,-1.),p(-1.,1.,-1.)],0.9),([p(-1.,-1.,-1.),p(1.,-1.,-1.),p(1.,-1.,1.),p(-1.,-1.,1.)],0.5)] {quad(&mut v,face,c.map(|a|a*shade));}
 }
 v
}
fn compile(log:&Log,entry:PCSTR,target:PCSTR)->Result<ID3DBlob> {let(mut code,mut error)=(None,None);let result=log.dx("D3DCompile",||unsafe{D3DCompile(SHADER.as_ptr().cast(),SHADER.len(),s!("cube.hlsl"),None,None,entry,target,D3DCOMPILE_ENABLE_STRICTNESS,0,&mut code,Some(&mut error))});if let Some(e)=error {let text=unsafe{slice::from_raw_parts(e.GetBufferPointer().cast::<u8>(),e.GetBufferSize())};log.line(String::from_utf8_lossy(text));}result?;code.context("셰이더 바이트코드 없음")}
impl Draw {
 pub fn new(log:&Log,luid:Option<LUID>,min_level:i32)->Result<Self>{unsafe{
  let factory:IDXGIFactory1=log.dx("CreateDXGIFactory1",||CreateDXGIFactory1())?;
  let mut selected=None;
  for i in 0..32 {let adapter=match factory.EnumAdapters1(i){Ok(a)=>a,Err(e) if e.code()==DXGI_ERROR_NOT_FOUND=>break,Err(e)=>return Err(e.into())};let desc=log.dx("IDXGIAdapter1::GetDesc1",||adapter.GetDesc1())?;let end=desc.Description.iter().position(|&c|c==0).unwrap_or(desc.Description.len());log.line(format!("DXGI 어댑터{i} {} LUID={},{}",String::from_utf16_lossy(&desc.Description[..end]),desc.AdapterLuid.HighPart,desc.AdapterLuid.LowPart));if luid.map(|l|l.LowPart==desc.AdapterLuid.LowPart&&l.HighPart==desc.AdapterLuid.HighPart).unwrap_or(i==0){selected=Some(adapter);}}
  let adapter=selected.context("OpenXR가 요구한 LUID의 DXGI 어댑터가 없습니다. 다른 어댑터로 대체하지 않습니다.")?;
  let levels:Vec<_>=[D3D_FEATURE_LEVEL_12_1,D3D_FEATURE_LEVEL_12_0,D3D_FEATURE_LEVEL_11_1,D3D_FEATURE_LEVEL_11_0].into_iter().filter(|l|l.0>=min_level).collect();if levels.is_empty(){bail!("OpenXR 최소 D3D 기능수준을 지원하지 않습니다");}
  let(mut device,mut context)=(None,None);let mut level=D3D_FEATURE_LEVEL::default();log.dx("D3D11CreateDevice",||D3D11CreateDevice(&adapter,D3D_DRIVER_TYPE_UNKNOWN,HMODULE::default(),D3D11_CREATE_DEVICE_BGRA_SUPPORT,Some(&levels),D3D11_SDK_VERSION,Some(&mut device),Some(&mut level),Some(&mut context)))?;let device=device.context("D3D 장치 없음")?;let context=context.context("D3D 컨텍스트 없음")?;log.line(format!("D3D11 선택 기능수준=0x{:X}",level.0));
  let vbcode=compile(log,s!("vs_main"),s!("vs_5_0"))?;let pbcode=compile(log,s!("ps_main"),s!("ps_5_0"))?;let vb=slice::from_raw_parts(vbcode.GetBufferPointer().cast::<u8>(),vbcode.GetBufferSize());let pb=slice::from_raw_parts(pbcode.GetBufferPointer().cast::<u8>(),pbcode.GetBufferSize());
  let(mut vs,mut ps,mut layout)=(None,None,None);log.dx("CreateVertexShader",||device.CreateVertexShader(vb,None,Some(&mut vs)))?;log.dx("CreatePixelShader",||device.CreatePixelShader(pb,None,Some(&mut ps)))?;
  let elements=[D3D11_INPUT_ELEMENT_DESC{SemanticName:s!("POSITION"),Format:DXGI_FORMAT_R32G32B32_FLOAT,InputSlotClass:D3D11_INPUT_PER_VERTEX_DATA,..Default::default()},D3D11_INPUT_ELEMENT_DESC{SemanticName:s!("COLOR"),Format:DXGI_FORMAT_R32G32B32_FLOAT,AlignedByteOffset:12,InputSlotClass:D3D11_INPUT_PER_VERTEX_DATA,..Default::default()}];log.dx("CreateInputLayout",||device.CreateInputLayout(&elements,vb,Some(&mut layout)))?;
  let data=vertices();let desc=D3D11_BUFFER_DESC{ByteWidth:(data.len()*size_of::<Vertex>()) as u32,Usage:D3D11_USAGE_IMMUTABLE,BindFlags:D3D11_BIND_VERTEX_BUFFER.0 as u32,..Default::default()};let init=D3D11_SUBRESOURCE_DATA{pSysMem:data.as_ptr().cast(),..Default::default()};let mut vertices=None;log.dx("CreateBuffer(정점)",||device.CreateBuffer(&desc,Some(&init),Some(&mut vertices)))?;
  let mut constants=None;let desc=D3D11_BUFFER_DESC{ByteWidth:64,Usage:D3D11_USAGE_DEFAULT,BindFlags:D3D11_BIND_CONSTANT_BUFFER.0 as u32,..Default::default()};log.dx("CreateBuffer(행렬)",||device.CreateBuffer(&desc,None,Some(&mut constants)))?;
  let mut raster=None;let desc=D3D11_RASTERIZER_DESC{FillMode:D3D11_FILL_SOLID,CullMode:D3D11_CULL_NONE,DepthClipEnable:true.into(),..Default::default()};log.dx("CreateRasterizerState",||device.CreateRasterizerState(&desc,Some(&mut raster)))?;
  let mut depth_state=None;let desc=D3D11_DEPTH_STENCIL_DESC{DepthEnable:true.into(),DepthWriteMask:D3D11_DEPTH_WRITE_MASK_ALL,DepthFunc:D3D11_COMPARISON_LESS_EQUAL,..Default::default()};log.dx("CreateDepthStencilState",||device.CreateDepthStencilState(&desc,Some(&mut depth_state)))?;
  Ok(Self{device,context,vs:vs.unwrap(),ps:ps.unwrap(),layout:layout.unwrap(),vertices:vertices.unwrap(),constants:constants.unwrap(),raster:raster.unwrap(),depth_state:depth_state.unwrap(),count:data.len() as u32,log:log.clone()})
 }}
 pub fn target(&self,color:&ID3D11Texture2D,width:u32,height:u32,format:DXGI_FORMAT)->Result<Target>{unsafe{let mut view=None;let rtv_desc=D3D11_RENDER_TARGET_VIEW_DESC{Format:format,ViewDimension:D3D11_RTV_DIMENSION_TEXTURE2D,Anonymous:D3D11_RENDER_TARGET_VIEW_DESC_0{Texture2D:D3D11_TEX2D_RTV{MipSlice:0}}};self.log.dx("CreateRenderTargetView",||self.device.CreateRenderTargetView(color,Some(&rtv_desc),Some(&mut view)))?;let desc=D3D11_TEXTURE2D_DESC{Width:width,Height:height,MipLevels:1,ArraySize:1,Format:DXGI_FORMAT_D32_FLOAT,SampleDesc:DXGI_SAMPLE_DESC{Count:1,Quality:0},Usage:D3D11_USAGE_DEFAULT,BindFlags:D3D11_BIND_DEPTH_STENCIL.0 as u32,..Default::default()};let mut texture=None;self.log.dx("CreateTexture2D(깊이)",||self.device.CreateTexture2D(&desc,None,Some(&mut texture)))?;let mut depth=None;self.log.dx("CreateDepthStencilView",||self.device.CreateDepthStencilView(texture.as_ref().unwrap(),None,Some(&mut depth)))?;Ok(Target{color:view.unwrap(),depth:depth.unwrap(),width,height})}}
 pub fn render(&self,target:&Target,mvp:Mat4){unsafe{let c=&self.context;let matrix=mvp.to_cols_array();c.UpdateSubresource(&self.constants,0,None,matrix.as_ptr().cast(),0,0);c.OMSetRenderTargets(Some(&[Some(target.color.clone())]),&target.depth);c.ClearRenderTargetView(&target.color,&[0.73,0.86,0.94,1.]);c.ClearDepthStencilView(&target.depth,D3D11_CLEAR_DEPTH.0,1.,0);c.RSSetViewports(Some(&[D3D11_VIEWPORT{Width:target.width as f32,Height:target.height as f32,MinDepth:0.,MaxDepth:1.,..Default::default()}]));c.RSSetState(&self.raster);c.OMSetDepthStencilState(&self.depth_state,0);c.IASetInputLayout(&self.layout);c.IASetPrimitiveTopology(D3D_PRIMITIVE_TOPOLOGY_TRIANGLELIST);let stride=size_of::<Vertex>() as u32;let offset=0;let buffer=Some(self.vertices.clone());c.IASetVertexBuffers(0,1,Some(&buffer),Some(&stride),Some(&offset));c.VSSetShader(&self.vs,None);c.PSSetShader(&self.ps,None);c.VSSetConstantBuffers(0,Some(&[Some(self.constants.clone())]));c.Draw(self.count,0);c.OMSetRenderTargets(None,None);}}
 pub fn check(&self)->Result<()> {self.log.dx("GetDeviceRemovedReason",||unsafe{self.device.GetDeviceRemovedReason()})}
 pub fn flush(&self){unsafe{self.context.Flush();}}
 pub fn offscreen(&self)->Result<()> {unsafe{let(w,h)=(1280,900);let mut texture=None;let desc=D3D11_TEXTURE2D_DESC{Width:w,Height:h,MipLevels:1,ArraySize:1,Format:DXGI_FORMAT_R8G8B8A8_TYPELESS,SampleDesc:DXGI_SAMPLE_DESC{Count:1,Quality:0},Usage:D3D11_USAGE_DEFAULT,BindFlags:D3D11_BIND_RENDER_TARGET.0 as u32,..Default::default()};self.log.dx("CreateTexture2D(오프스크린)",||self.device.CreateTexture2D(&desc,None,Some(&mut texture)))?;let texture=texture.unwrap();let target=self.target(&texture,w,h,DXGI_FORMAT_R8G8B8A8_UNORM)?;let view=Mat4::look_at_rh(Vec3::new(0.,1.6,0.),Vec3::new(0.,1.45,-1.4),Vec3::Y);self.render(&target,Mat4::perspective_rh(70f32.to_radians(),w as f32/h as f32,0.05,50.)*view);self.flush();self.check()?;
  let mut staging=None;let desc=D3D11_TEXTURE2D_DESC{Usage:D3D11_USAGE_STAGING,BindFlags:0,CPUAccessFlags:D3D11_CPU_ACCESS_READ.0 as u32,..desc};self.log.dx("CreateTexture2D(읽기)",||self.device.CreateTexture2D(&desc,None,Some(&mut staging)))?;let staging=staging.unwrap();self.context.CopyResource(&staging,&texture);let mut map=D3D11_MAPPED_SUBRESOURCE::default();self.log.dx("Map(오프스크린)",||self.context.Map(&staging,0,D3D11_MAP_READ,0,Some(&mut map)))?;let mut pixels=vec![0u8;(w*h*4)as usize];for y in 0..h as usize {let row=slice::from_raw_parts((map.pData as *const u8).add(y*map.RowPitch as usize),w as usize*4);pixels[y*w as usize*4..(y+1)*w as usize*4].copy_from_slice(row);}self.context.Unmap(&staging,0);
  let path=root().join("_tmp/shots/native_cube_offscreen.png");std::fs::create_dir_all(path.parent().unwrap())?;let mut encoder=png::Encoder::new(BufWriter::new(File::create(&path)?),w,h);encoder.set_color(png::ColorType::Rgba);encoder.set_depth(png::BitDepth::Eight);encoder.write_header()?.write_image_data(&pixels)?;self.log.line(format!("오프스크린 PNG 저장={} 크기={w}x{h} 정점={}",path.display(),self.count));Ok(())
 }}
}
pub fn view_projection(view:&openxr::View)->Mat4 {let p=view.pose;let world=Mat4::from_rotation_translation(glam::Quat::from_xyzw(p.orientation.x,p.orientation.y,p.orientation.z,p.orientation.w),Vec3::new(p.position.x,p.position.y,p.position.z));let(l,r,d,u)=(view.fov.angle_left.tan(),view.fov.angle_right.tan(),view.fov.angle_down.tan(),view.fov.angle_up.tan());let(n,f)=(0.05,50.);let projection=Mat4::from_cols_array(&[2./(r-l),0.,0.,0.,0.,2./(u-d),0.,0.,(r+l)/(r-l),(u+d)/(u-d),-f/(f-n),-1.,0.,0.,-f*n/(f-n),0.]);projection*world.inverse()}

pub fn anchor(views:&[openxr::View],stage:bool)->Mat4 {
 let a=views[0].pose;let b=views[1].pose;
 let center=Vec3::new((a.position.x+b.position.x)*0.5,(a.position.y+b.position.y)*0.5,(a.position.z+b.position.z)*0.5);
 let q=glam::Quat::from_xyzw(a.orientation.x,a.orientation.y,a.orientation.z,a.orientation.w);
 let mut forward=q*Vec3::NEG_Z;forward.y=0.0;if forward.length_squared()<0.0001{forward=Vec3::NEG_Z;}forward=forward.normalize();
 let yaw=(-forward.x).atan2(-forward.z);
 Mat4::from_rotation_translation(glam::Quat::from_rotation_y(yaw),Vec3::new(center.x,if stage{0.0}else{center.y-1.6},center.z))
}
#[cfg(test)] mod tests {
 use super::*;
 #[test] fn asymmetric_d3d_frustum_and_depth(){
  let view=openxr::View{pose:openxr::Posef::IDENTITY,fov:openxr::Fovf{angle_left:(-0.5f32).atan(),angle_right:0.9f32.atan(),angle_down:(-0.6f32).atan(),angle_up:0.8f32.atan()}};
  let m=view_projection(&view);let n=0.05;
  assert!((m.project_point3(Vec3::new(-0.5*n,0.0,-n)).x+1.0).abs()<1e-5);
  assert!((m.project_point3(Vec3::new(0.9*n,0.0,-n)).x-1.0).abs()<1e-5);
  assert!((m.project_point3(Vec3::new(0.0,-0.6*n,-n)).y+1.0).abs()<1e-5);
  assert!((m.project_point3(Vec3::new(0.0,0.8*n,-n)).y-1.0).abs()<1e-5);
  assert!(m.project_point3(Vec3::new(0.0,0.0,-n)).z.abs()<1e-5);
  assert!((m.project_point3(Vec3::new(0.0,0.0,-50.0)).z-1.0).abs()<1e-5);
 }
 #[test] fn eye_pose_is_inverted(){
  let mut pose=openxr::Posef::IDENTITY;pose.position=openxr::Vector3f{x:0.032,y:1.6,z:0.0};let a=45f32.to_radians();let view=openxr::View{pose,fov:openxr::Fovf{angle_left:-a,angle_right:a,angle_down:-a,angle_up:a}};let center=view_projection(&view).project_point3(Vec3::new(0.032,1.6,-1.0));assert!(center.x.abs()<1e-5&&center.y.abs()<1e-5);
 }
 #[test] fn cubes_start_in_front_of_first_head_pose(){
  let mut pose=openxr::Posef::IDENTITY;pose.position=openxr::Vector3f{x:2.0,y:1.6,z:3.0};let q=glam::Quat::from_rotation_y(std::f32::consts::FRAC_PI_2);pose.orientation=openxr::Quaternionf{x:q.x,y:q.y,z:q.z,w:q.w};let v=openxr::View{pose,fov:openxr::Fovf{angle_left:-0.7,angle_right:0.7,angle_up:0.7,angle_down:-0.7}};let p=anchor(&[v,v],true).transform_point3(Vec3::new(0.0,1.35,-1.4));assert!((p.x-0.6).abs()<1e-5&&(p.y-1.35).abs()<1e-5&&(p.z-3.0).abs()<1e-5);
 }
}
