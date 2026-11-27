use glam::{Mat4,Vec3,Quat};
#[repr(C)] #[derive(Clone,Copy,Debug)]
pub struct Vertex {pub position:[f32;3],pub uv:[f32;2],pub color:[f32;4]}
#[repr(C)] #[derive(Clone,Copy,Debug)]
pub struct Instance {pub model:[[f32;4];4],pub uv:[f32;4],pub color:[f32;4],pub params:[f32;4]}
impl Instance {pub fn new(model:Mat4,uv:[f32;4],color:[f32;4],params:[f32;4])->Self{Self{model:model.to_cols_array_2d(),uv,color,params}}}
pub struct Mesh {pub name:String,pub vertices:Vec<Vertex>,pub instances:Vec<Instance>,pub blend:bool,pub depth_write:bool,pub depth_test:bool,pub dynamic:bool}
pub fn linear(v:f32)->f32{if v<=0.04045{v/12.92}else{((v+0.055)/1.055).powf(2.4)}}
pub fn rgb(c:[f32;3],a:f32)->[f32;4]{[linear(c[0]),linear(c[1]),linear(c[2]),a]}
pub fn color_hex(c:u32,a:f32)->[f32;4]{rgb([((c>>16)&255)as f32/255.0,((c>>8)&255)as f32/255.0,(c&255)as f32/255.0],a)}
pub fn quad()->Vec<Vertex>{let points=[([-0.5,-0.5,0.0],[0.0,1.0]),([0.5,-0.5,0.0],[1.0,1.0]),([0.5,0.5,0.0],[1.0,0.0]),([-0.5,0.5,0.0],[0.0,0.0])];[0,1,2,0,2,3].iter().map(|&i|Vertex{position:points[i].0,uv:points[i].1,color:[1.0;4]}).collect()}
pub fn mesh(name:&str,vertices:Vec<Vertex>,instances:Vec<Instance>,blend:bool,depth_write:bool)->Mesh{Mesh{name:name.into(),vertices,instances,blend,depth_write,depth_test:true,dynamic:false}}
pub fn solid_instance(m:Mat4,color:[f32;4])->Instance{Instance::new(m,[0.0;4],color,[0.0,0.0,0.0,0.0])}
pub fn box_vertices()->Vec<Vertex>{let p=|x,y,z|[x,y,z];let mut out=Vec::new();for (q,shade) in [([p(-0.5,-0.5,0.5),p(0.5,-0.5,0.5),p(0.5,0.5,0.5),p(-0.5,0.5,0.5)],1.0),([p(0.5,-0.5,-0.5),p(-0.5,-0.5,-0.5),p(-0.5,0.5,-0.5),p(0.5,0.5,-0.5)],0.7),([p(-0.5,-0.5,-0.5),p(-0.5,-0.5,0.5),p(-0.5,0.5,0.5),p(-0.5,0.5,-0.5)],0.8),([p(0.5,-0.5,0.5),p(0.5,-0.5,-0.5),p(0.5,0.5,-0.5),p(0.5,0.5,0.5)],0.65),([p(-0.5,0.5,0.5),p(0.5,0.5,0.5),p(0.5,0.5,-0.5),p(-0.5,0.5,-0.5)],1.1),([p(-0.5,-0.5,-0.5),p(0.5,-0.5,-0.5),p(0.5,-0.5,0.5),p(-0.5,-0.5,0.5)],0.6)] {for i in [0,1,2,0,2,3]{out.push(Vertex{position:q[i],uv:[0.0;2],color:[shade,shade,shade,1.0]});}}out}
pub fn icosahedron()->Vec<Vertex>{let t=(1.0+5.0f32.sqrt())/2.0;let points=[[-1.0,t,0.0],[1.0,t,0.0],[-1.0,-t,0.0],[1.0,-t,0.0],[0.0,-1.0,t],[0.0,1.0,t],[0.0,-1.0,-t],[0.0,1.0,-t],[t,0.0,-1.0],[t,0.0,1.0],[-t,0.0,-1.0],[-t,0.0,1.0]];let faces=[[0,11,5],[0,5,1],[0,1,7],[0,7,10],[0,10,11],[1,5,9],[5,11,4],[11,10,2],[10,7,6],[7,1,8],[3,9,4],[3,4,2],[3,2,6],[3,6,8],[3,8,9],[4,9,5],[2,4,11],[6,2,10],[8,6,7],[9,8,1]];faces.iter().flat_map(|f|f.iter().map(|&i|Vertex{position:Vec3::from_array(points[i]).normalize().to_array(),uv:[0.0;2],color:[1.0;4]})).collect()}
pub fn cylinder()->Vec<Vertex>{let mut out=Vec::new();let p=|a:f32,y:f32,r:f32|[a.cos()*r,y,a.sin()*r];for i in 0..5{let a=i as f32*std::f32::consts::TAU/5.0;let b=(i+1)as f32*std::f32::consts::TAU/5.0;for point in [p(a,-0.5,1.0),p(b,-0.5,1.0),p(b,0.5,0.3),p(a,-0.5,1.0),p(b,0.5,0.3),p(a,0.5,0.3),[0.0,0.5,0.0],p(a,0.5,0.3),p(b,0.5,0.3),[0.0,-0.5,0.0],p(b,-0.5,1.0),p(a,-0.5,1.0)]{out.push(Vertex{position:point,uv:[0.0;2],color:[1.0;4]});}}out}
pub fn branch(a:Vec3,b:Vec3,r1:f32,r2:f32)->Mat4{let d=b-a;Mat4::from_scale_rotation_translation(Vec3::new(r1,d.length(),r2),Quat::from_rotation_arc(Vec3::Y,d.normalize()),(a+b)*0.5)}
