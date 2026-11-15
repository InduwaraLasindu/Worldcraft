use bevy::{input::mouse::AccumulatedMouseMotion,math::DVec3,prelude::*,window::{CursorGrabMode,PrimaryWindow}};
use bevy_egui::EguiContexts;
use worldcraft_core::{*,config::MAX_WORLD_COORD};
use crate::{session::*,scene::MainCamera,storage::{self,Disk},streaming::Stream};

/// Match the exact two-triangle LOD0 surface the player sees, rather than the analytic field between vertices.
pub fn ground(g:&Generator,x:f64,z:f64)->f64{
    let ax=(x/4.0).floor()*4.0;let az=(z/4.0).floor()*4.0;
    let u=(x-ax)/4.0;let v=(z-az)/4.0;
    let a=g.height(ax,az);let b=g.height(ax+4.0,az);
    let c=g.height(ax,az+4.0);let d=g.height(ax+4.0,az+4.0);
    if u+v<=1.0{a+(b-a)*u+(c-a)*v}else{d+(c-d)*(1.0-u)+(b-d)*(1.0-v)}
}
#[allow(clippy::too_many_arguments)]
pub fn input(
    mut session:ResMut<Session>,mut player:ResMut<Player>,mut controls:ResMut<Controls>,
    mut ui:ResMut<UiState>,keys:Res<ButtonInput<KeyCode>>,mouse:Res<ButtonInput<MouseButton>>,
    motion:Res<AccumulatedMouseMotion>,time:Res<Time>,stream:Res<Stream>,
    mut windows:Query<&mut Window,With<PrimaryWindow>>,
    camera:Query<(&Camera,&GlobalTransform),With<MainCamera>>,
    mut contexts:EguiContexts,mut gizmos:Gizmos,mut disk:ResMut<Disk>,
){
    let Ok(mut window)=windows.single_mut()else{return};
    let ctx=contexts.ctx_mut();
    let over_ui=ctx.wants_pointer_input();let typing=ctx.wants_keyboard_input();
    if keys.just_pressed(KeyCode::Escape)||!window.focused||ui.home{player.captured=false;}
    if keys.just_pressed(KeyCode::F1){ui.help=!ui.help;player.captured=false;}
    if !typing&&!ui.home&&session.world.is_some(){
        if keys.just_pressed(KeyCode::KeyE){
            session.mode=if session.mode==Mode::Edit{Mode::Walk}else{Mode::Edit};
            player.captured=false;player.velocity=0.0;
        }
        if keys.just_pressed(KeyCode::Tab){
            session.mode=if session.mode==Mode::Fly{Mode::Walk}else{Mode::Fly};
            player.velocity=0.0;
        }
        let ctrl=keys.any_pressed([KeyCode::ControlLeft,KeyCode::ControlRight]);
        if ctrl&&keys.just_pressed(KeyCode::KeyS)&&controls.live.is_empty(){storage::save(&mut disk,&session,&player,false);}
        if ctrl&&keys.just_pressed(KeyCode::KeyZ)&&controls.live.is_empty(){
            if session.history.undo(){session.rebuild(&[]);}
        }
        if ctrl&&keys.just_pressed(KeyCode::KeyY)&&controls.live.is_empty(){
            if session.history.redo(){session.rebuild(&[]);}
        }
    }
    if !ui.home&&!over_ui&&window.focused{
        if session.mode==Mode::Edit{
            player.captured=mouse.pressed(MouseButton::Right);
        }else if mouse.just_pressed(MouseButton::Left){player.captured=true;}
    }
    window.cursor_options.grab_mode=if player.captured{CursorGrabMode::Locked}else{CursorGrabMode::None};
    window.cursor_options.visible=!player.captured;
    if player.captured&&!ui.home{
        player.yaw-=motion.delta.x*0.002;
        player.pitch=(player.pitch-motion.delta.y*0.002).clamp(-1.54,1.54);
    }
    let Some(g)=session.generator.clone()else{return};
    if !ui.home&&!typing&&player.captured{
        let dt=f64::from(time.delta_secs()).min(0.05);
        let axis=|positive:KeyCode,negative:KeyCode|f64::from(keys.pressed(positive)as u8)-f64::from(keys.pressed(negative)as u8);
        let forward=axis(KeyCode::KeyW,KeyCode::KeyS);
        let side=axis(KeyCode::KeyD,KeyCode::KeyA);
        let sprint=keys.any_pressed([KeyCode::ShiftLeft,KeyCode::ShiftRight]);
        let flying=session.mode!=Mode::Walk;
        let rotation=if flying{player.rotation()}else{Quat::from_rotation_y(player.yaw)};
        let mut movement=(rotation*Vec3::NEG_Z).as_dvec3()*forward+(rotation*Vec3::X).as_dvec3()*side;
        if flying{movement.y+=axis(KeyCode::Space,KeyCode::ControlLeft);}
        movement=movement.normalize_or_zero();
        if flying{
            player.position+=movement*controls.speed*if sprint{4.0}else{1.0}*dt;
            player.velocity=0.0;player.grounded=false;
        }else{
            if keys.just_pressed(KeyCode::Space)&&player.grounded{player.velocity=7.0;player.grounded=false;}
            let steps=(dt*120.0).ceil().max(1.0)as u32;
            for _ in 0..steps{
                let step=dt/f64::from(steps);
                let next=player.position+movement*if sprint{8.5}else{4.0}*step;
                let old_floor=ground(&g,player.position.x,player.position.z);
                let new_floor=ground(&g,next.x,next.z);
                let can_step=!player.grounded||new_floor-old_floor<=0.45;
                if can_step&&!stream.blocked(next.x,next.z,0.3)&&stream.loaded.contains_key(&ChunkCoord::at(next.x,next.z)){
                    player.position.x=next.x;player.position.z=next.z;
                }
                player.velocity-=20.0*step;
                let velocity=player.velocity;player.position.y+=velocity*step;
                let floor=ground(&g,player.position.x,player.position.z)+1.75;
                if player.position.y<=floor{
                    player.position.y=floor;player.velocity=0.0;player.grounded=true;
                }else{player.grounded=false;}
            }
        }
        player.position.x=player.position.x.clamp(-MAX_WORLD_COORD+1024.0,MAX_WORLD_COORD-1024.0);
        player.position.z=player.position.z.clamp(-MAX_WORLD_COORD+1024.0,MAX_WORLD_COORD-1024.0);
        player.position.y=player.position.y.clamp(-100_000.0,100_000.0);
    }
    controls.hit=None;
    if session.mode==Mode::Edit&&!ui.home&&!over_ui&&!player.captured&&window.focused{
        if let(Some(cursor),Ok((camera,transform)))=(window.cursor_position(),camera.single()){
            if let Ok(ray)=camera.viewport_to_world(transform,cursor){
                let start=ray.origin.as_dvec3()+session.origin;
                let direction=ray.direction.as_vec3().as_dvec3();
                controls.hit=pick(&g,start,direction);
            }
        }
    }
    if mouse.just_pressed(MouseButton::Left){
        controls.painting=controls.hit.is_some()&&session.mode==Mode::Edit&&!over_ui&&!ui.home;
    }
    if !mouse.pressed(MouseButton::Left)||session.mode!=Mode::Edit||ui.home||!window.focused{
        controls.painting=false;
    }
    if let Some(hit)=controls.hit{
        let radius=controls.brush.radius;
        for i in 0..48{
            let point=|j:usize|{
                let a=j as f64/48.0*std::f64::consts::TAU;
                let x=hit[0]+a.cos()*radius;let z=hit[2]+a.sin()*radius;
                (DVec3::new(x,g.height(x,z)+0.35,z)-session.origin).as_vec3()
            };
            gizmos.line(point(i),point(i+1),Color::srgb(0.6,1.0,0.55));
        }
        if mouse.pressed(MouseButton::Left)&&controls.painting{
            let now=time.elapsed_secs_f64();
            let spaced=controls.last_dab.is_none_or(|p|(p[0]-hit[0]).hypot(p[1]-hit[2])>radius*0.15);
            if (spaced||now-controls.last_time>=0.12)&&controls.live.len()<2048{
                let mut brush=controls.brush.clone();
                if matches!(brush.kind,BrushKind::Flatten|BrushKind::Smooth){
                    if let Some(first)=controls.live.first(){brush.target_height=first.brush.target_height;}
                    else if brush.kind==BrushKind::Smooth{
                        brush.target_height=(g.height(hit[0]+radius*0.3,hit[2])+g.height(hit[0]-radius*0.3,hit[2])
                            +g.height(hit[0],hit[2]+radius*0.3)+g.height(hit[0],hit[2]-radius*0.3))*0.25;
                    }else{brush.target_height=hit[1];}
                }
                controls.live.push(Stroke{center:[hit[0],hit[2]],brush});
                controls.last_dab=Some([hit[0],hit[2]]);controls.last_time=now;
                session.rebuild(&controls.live);
            }
        }
    }
    if (!mouse.pressed(MouseButton::Left)||session.mode!=Mode::Edit||ui.home)&&!controls.live.is_empty(){
        let live=std::mem::take(&mut controls.live);
        if let Err(e)=session.history.push(live){ui.message=e;}
        controls.last_dab=None;session.rebuild(&[]);
    }
}
fn pick(g:&Generator,start:DVec3,direction:DVec3)->Option<[f64;3]>{
    let mut previous=0.0;
    for i in 1..=600{
        let distance=f64::from(i)*3.0;
        let p=start+direction*distance;
        if p.y<=g.height(p.x,p.z){
            let(mut a,mut b)=(previous,distance);
            for _ in 0..14{
                let mid=(a+b)*0.5;let q=start+direction*mid;
                if q.y>g.height(q.x,q.z){a=mid}else{b=mid}
            }
            return Some((start+direction*((a+b)*0.5)).to_array());
        }
        previous=distance;
    }
    None
}
#[cfg(test)]mod tests{
    use super::*;
    #[test]fn collision_matches_mesh_vertices(){
        let g=Generator::new(GeneratorSettings::default(),Vec::new());
        for x in -20..20{assert!((ground(&g,f64::from(x)*4.0,-8.0)-g.height(f64::from(x)*4.0,-8.0)).abs()<1e-10);}
    }
    #[test]fn ray_picks_terrain(){
        let g=Generator::new(GeneratorSettings::default(),Vec::new());
        let h=g.height(0.0,0.0);
        let p=pick(&g,DVec3::new(0.0,h+100.0,0.0),DVec3::NEG_Y).unwrap();
        assert!((p[1]-h).abs()<0.001);
    }
}
