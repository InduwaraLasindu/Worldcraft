//! Hosted software-rendered integration check. This is not a hardware benchmark.
use bevy::{app::AppExit,math::DVec3,prelude::*,render::view::screenshot::{save_to_disk,Screenshot}};
use worldcraft_core::*;
use crate::{session::*,streaming::Stream};

#[derive(Resource,Default)]
pub struct Smoke { frames:u32, stage:u8, capture_frame:u32 }
pub fn start(mut session:ResMut<Session>,mut player:ResMut<Player>,mut ui:ResMut<UiState>,mut graphics:ResMut<Graphics>){
    session.create("Cloud smoke test".into(),GeneratorSettings::for_preset(Preset::Temperate,20261116),&mut player);
    player.position+=DVec3::new(0.0,45.0,70.0);player.pitch=-0.35;
    session.mode=Mode::Fly;ui.home=false;ui.inspector=false;
    graphics.radius=2;graphics.quality=Quality::Low;
}
pub fn check(mut commands:Commands,mut state:ResMut<Smoke>,mut session:ResMut<Session>,
    player:Res<Player>,stream:Res<Stream>,mut graphics:ResMut<Graphics>,mut exit:EventWriter<AppExit>){
    state.frames+=1;
    assert!(state.frames<5000,"Smoke test timed out before terrain finished streaming");
    if state.stage==0&&stream.loaded.len()>=9&&state.frames>100{
        let x=player.position.x;let z=player.position.z;
        let before=session.generator.as_ref().unwrap().height(x,z);
        session.history.push(vec![Stroke{center:[x,z],brush:Brush::default()}]).unwrap();
        session.rebuild(&[]);
        assert!((session.generator.as_ref().unwrap().height(x,z)-before-5.0).abs()<1e-6);
        assert!(session.history.undo());session.rebuild(&[]);
        assert!((session.generator.as_ref().unwrap().height(x,z)-before).abs()<1e-6);
        assert!(session.history.redo());session.rebuild(&[]);
        let world=session.snapshot(&player).unwrap();
        let path=std::env::temp_dir().join("worldcraft-smoke.worldcraft.json");
        save_world(&path,&world).unwrap();assert_eq!(load_world(&path).unwrap(),world);
        state.stage=1;
        info!("SMOKE: streaming, brush, undo/redo and save/load passed");
    }
    if state.stage==1&&stream.loaded.len()==stream.desired&&stream.queue()==0
        &&stream.loaded.iter().all(|(coord,l)|l.key.revision==*session.chunk_revisions.get(coord).unwrap_or(&0)){
        commands.spawn(Screenshot::primary_window()).observe(save_to_disk("worldcraft-smoke.png"));
        state.stage=2;state.capture_frame=state.frames;
    }
    if state.stage==2&&state.frames>state.capture_frame+20&&std::path::Path::new("worldcraft-smoke.png").exists(){
        graphics.quality=Quality::Balanced;state.stage=3;state.capture_frame=state.frames;
        info!("SMOKE: switched to Balanced atmosphere");
    }
    if state.stage==3&&state.frames>state.capture_frame+60{
        commands.spawn(Screenshot::primary_window()).observe(save_to_disk("worldcraft-balanced.png"));
        state.stage=4;state.capture_frame=state.frames;
    }
    if state.stage==4&&state.frames>state.capture_frame+20&&std::path::Path::new("worldcraft-balanced.png").exists(){
        graphics.quality=Quality::High;state.stage=5;state.capture_frame=state.frames;
        info!("SMOKE: switched to High ambient occlusion");
    }
    if state.stage==5&&state.frames>state.capture_frame+60{
        commands.spawn(Screenshot::primary_window()).observe(save_to_disk("worldcraft-high.png"));
        state.stage=6;state.capture_frame=state.frames;
    }
    if state.stage==6&&state.frames>state.capture_frame+20&&std::path::Path::new("worldcraft-high.png").exists(){
        info!("SMOKE: Low/Balanced/High captured; native application exited cleanly");
        exit.write(AppExit::Success);
    }
}
