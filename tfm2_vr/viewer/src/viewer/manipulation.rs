use glam::{Vec3,Quat};
#[derive(Clone,Copy,Debug)]pub struct Table{pub center:Vec3,pub scale:f32,pub yaw:f32}
impl Table{pub fn constrain(&mut self,head:Vec3,floor:f32){self.scale=self.scale.clamp(0.5,3.0);self.center.y=self.center.y.clamp(floor+0.3,floor+1.5);let mut d=self.center-head;d.y=0.0;let n=d.length();if n>3.5{self.center.x=head.x+d.x*3.5/n;self.center.z=head.z+d.z*3.5/n;}else if n<0.35{if n<0.0001{d=Vec3::NEG_Z;}else{d/=n;}self.center.x=head.x+d.x*0.35;self.center.z=head.z+d.z*0.35;}self.yaw=(self.yaw+std::f32::consts::PI).rem_euclid(std::f32::consts::TAU)-std::f32::consts::PI;}}
#[derive(Clone,Copy,Default)]struct Press{down:bool,blocked:bool,used:bool,start:Vec3,age:f32}
#[derive(Clone,Copy)]struct Anchor{mask:u8,table:Table,mid:Vec3,vector:Vec3}
#[derive(Default)]pub struct Grab{press:[Press;2],anchor:Option<Anchor>}
#[derive(Default,Debug)]pub struct ResultFrame{pub toggle:bool,pub haptic:[bool;2],pub points:[Option<Vec3>;2],pub grabbing:[bool;2]}
fn midpoint(mask:u8,hands:[Option<Vec3>;2])->Vec3{if mask==3{(hands[0].unwrap()+hands[1].unwrap())*0.5}else{hands[if mask==1{0}else{1}].unwrap()}}
impl Grab{
 pub fn cancel(&mut self){self.anchor=None;for p in &mut self.press{*p=Press{blocked:true,..Default::default()};}}
 pub fn busy(&self)->bool{self.press.iter().any(|p|p.down)}
 pub fn step(&mut self,down:[bool;2],hands:[Option<Vec3>;2],dt:f32,table:&mut Table,head:Vec3,floor:f32)->ResultFrame{
  let mut result=ResultFrame::default();let mut mask=0u8;
  for i in 0..2{let p=&mut self.press[i];if p.down{p.age+=if dt.is_finite(){dt.max(0.0)}else{0.0};}if !down[i]{if p.down&&!p.used&&!p.blocked&&p.age<0.25&&hands[i].is_some_and(|h|h.is_finite()&&p.start.distance(h)<0.02){result.toggle=!result.toggle;}*p=Press::default();continue;}if p.blocked{continue;}let Some(position)=hands[i].filter(|v|v.is_finite())else{*p=Press{blocked:true,..Default::default()};continue;};if !p.down{*p=Press{down:true,start:position,..Default::default()};}result.points[i]=Some(position);if !p.used&&(p.age>=0.25||p.start.distance(position)>=0.02){p.used=true;result.haptic[i]=true;}if p.used{mask|=1<<i;}}
  if self.press.iter().all(|p|p.down&&!p.blocked)&&hands.iter().all(Option::is_some){mask=3;for i in 0..2{if !self.press[i].used{result.haptic[i]=true;}self.press[i].used=true;}}
  if mask==0{self.anchor=None;return result;}
  let mid=midpoint(mask,hands);let vector=if mask==3{hands[1].unwrap()-hands[0].unwrap()}else{Vec3::ZERO};
  if self.anchor.map(|a|a.mask)!=Some(mask){
   // 한 손에서 잡기 판정 직후에는 처음 누른 위치부터의 이동을 반영한다.
   let first=self.anchor.is_none();let initial_mid=if first&&mask!=3{self.press[if mask==1{0}else{1}].start}else{mid};
   self.anchor=Some(Anchor{mask,table:*table,mid:initial_mid,vector});
  }
  let a=self.anchor.unwrap();let mut next=a.table;
  if mask==3{let scale=if a.vector.length()>0.04{(a.table.scale*vector.length()/a.vector.length()).clamp(0.5,3.0)}else{a.table.scale};let horizontal=Vec3::new(vector.x,0.0,vector.z);let old=Vec3::new(a.vector.x,0.0,a.vector.z);let angle=if horizontal.length()>0.04&&old.length()>0.04{let d=horizontal.x.atan2(horizontal.z)-old.x.atan2(old.z);(d+std::f32::consts::PI).rem_euclid(std::f32::consts::TAU)-std::f32::consts::PI}else{0.0};next.scale=scale;next.yaw=a.table.yaw+angle;next.center=mid+Quat::from_rotation_y(angle)*(a.table.center-a.mid)*(scale/a.table.scale);
  }else{next.center=a.table.center+mid-a.mid;}
  next.constrain(head,floor);*table=next;for i in 0..2{result.grabbing[i]=mask&(1<<i)!=0;}result
 }
}
pub fn view_facing(motion:Vec3,to_eye:Vec3,previous:f32)->f32{let right=Vec3::new(to_eye.z,0.0,-to_eye.x).normalize_or_zero();let projected=motion.dot(right);if projected.abs()<0.01{previous}else{projected.signum()}}
#[cfg(test)]mod tests{
 use super::*;fn table()->Table{Table{center:Vec3::new(0.0,0.76,-1.4),scale:1.0,yaw:0.0}}fn p(x:f32,y:f32,z:f32)->Option<Vec3>{Some(Vec3::new(x,y,z))}fn step(g:&mut Grab,t:&mut Table,d:[bool;2],h:[Option<Vec3>;2],dt:f32)->ResultFrame{g.step(d,h,dt,t,Vec3::new(0.0,1.6,0.0),0.0)}
 #[test]fn short_click_and_hold_are_distinct(){let(mut g,mut t)=(Grab::default(),table());step(&mut g,&mut t,[true,false],[p(0.0,1.0,-0.5),None],0.01);assert!(step(&mut g,&mut t,[false,false],[p(0.0,1.0,-0.5),None],0.1).toggle);step(&mut g,&mut t,[true,false],[p(0.0,1.0,-0.5),None],0.01);for _ in 0..3{step(&mut g,&mut t,[true,false],[p(0.0,1.0,-0.5),None],0.1);}assert!(!step(&mut g,&mut t,[false,false],[p(0.0,1.0,-0.5),None],0.1).toggle);}
 #[test]fn one_hand_translates_without_scale_or_rotation(){let(mut g,mut t)=(Grab::default(),table());step(&mut g,&mut t,[true,false],[p(0.0,1.0,-0.5),None],0.01);let r=step(&mut g,&mut t,[true,false],[p(0.1,1.2,-0.3),None],0.1);assert!(r.haptic[0]);assert!((t.center-Vec3::new(0.1,0.96,-1.2)).length()<1e-5);assert_eq!(t.scale,1.0);assert_eq!(t.yaw,0.0);assert!(!step(&mut g,&mut t,[false,false],[p(0.1,1.2,-0.3),None],0.01).toggle);}
 #[test]fn two_hands_scale_rotate_about_midpoint_and_release_without_jump(){let(mut g,mut t)=(Grab::default(),table());step(&mut g,&mut t,[true,true],[p(-0.2,1.0,-0.5),p(0.2,1.0,-0.5)],0.01);let a=t;step(&mut g,&mut t,[true,true],[p(0.1,1.1,-0.9),p(0.1,1.1,-0.1)],0.1);assert!((t.scale-2.0).abs()<1e-5);assert!((t.yaw.abs()-std::f32::consts::FRAC_PI_2).abs()<1e-5);let expected=Vec3::new(0.1,1.1,-0.5)+Quat::from_rotation_y(t.yaw)*(a.center-Vec3::new(0.0,1.0,-0.5))*2.0;assert!((t.center-expected).length()<1e-5);let before=t.center;let r=step(&mut g,&mut t,[false,true],[None,p(0.1,1.1,-0.1)],0.1);assert!(!r.toggle);assert!((t.center-before).length()<1e-5);}
 #[test]fn lost_tracking_blocks_until_release(){let(mut g,mut t)=(Grab::default(),table());step(&mut g,&mut t,[true,false],[p(0.0,1.0,-0.5),None],0.01);step(&mut g,&mut t,[true,false],[None,None],0.1);let before=t.center;let r=step(&mut g,&mut t,[true,false],[p(1.0,1.0,-0.5),None],0.1);assert!(!r.grabbing[0]&&!r.toggle);assert_eq!(t.center,before);assert!(!step(&mut g,&mut t,[false,false],[p(1.0,1.0,-0.5),None],0.1).toggle);}
 #[test]fn session_pause_cancels_without_click_or_jump(){let(mut g,mut t)=(Grab::default(),table());step(&mut g,&mut t,[true,false],[p(0.0,1.0,-0.5),None],0.01);step(&mut g,&mut t,[true,false],[p(0.1,1.0,-0.5),None],0.1);g.cancel();let before=t.center;step(&mut g,&mut t,[true,false],[p(1.0,1.0,-0.5),None],0.1);assert_eq!(before,t.center);assert!(!step(&mut g,&mut t,[false,false],[p(1.0,1.0,-0.5),None],0.1).toggle);}
 #[test]fn release_frame_counts_toward_threshold(){let(mut g,mut t)=(Grab::default(),table());step(&mut g,&mut t,[true,false],[p(0.0,1.0,-0.5),None],0.01);for _ in 0..2{step(&mut g,&mut t,[true,false],[p(0.0,1.0,-0.5),None],0.1);}assert!(!step(&mut g,&mut t,[false,false],[p(0.0,1.0,-0.5),None],0.06).toggle);step(&mut g,&mut t,[true,false],[p(0.0,1.0,-0.5),None],0.01);assert!(!step(&mut g,&mut t,[false,false],[p(0.03,1.0,-0.5),None],0.01).toggle);}
 #[test]fn safety_and_view_direction(){let mut t=table();t.center=Vec3::new(50.0,20.0,0.0);t.scale=9.0;t.constrain(Vec3::new(0.0,1.6,0.0),0.0);assert_eq!(t.scale,3.0);assert_eq!(t.center.y,1.5);assert!(t.center.x<=3.5);assert_eq!(view_facing(Vec3::X,Vec3::Z,1.0),1.0);assert_eq!(view_facing(Vec3::X,-Vec3::Z,1.0),-1.0);}
 #[test]fn head_motion_without_grab_does_not_move_table(){let(mut g,mut t)=(Grab::default(),table());let before=t.center;g.step([false;2],[None;2],0.1,&mut t,Vec3::new(0.0,1.6,-1.39),0.0);assert_eq!(before,t.center);}
 #[test]fn long_frame_still_counts_as_hold(){let(mut g,mut t)=(Grab::default(),table());step(&mut g,&mut t,[true,false],[p(0.0,1.0,-0.5),None],0.01);assert!(step(&mut g,&mut t,[true,false],[p(0.0,1.0,-0.5),None],0.3).grabbing[0]);}
 #[test]fn facing_matches_four_view_directions(){let m=Vec3::new(1.0,0.0,0.5);for(eye,expected)in[(Vec3::Z,1.0),(Vec3::X,-1.0),(-Vec3::Z,-1.0),(-Vec3::X,1.0)]{assert_eq!(view_facing(m,eye,1.0),expected);}}
}
