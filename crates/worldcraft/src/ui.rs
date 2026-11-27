use crate::{
    session::*,
    storage::{self, Disk},
    streaming::Stream,
};
use bevy::{
    diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin},
    math::DVec3,
    prelude::*,
    tasks::{block_on, futures_lite::future, AsyncComputeTaskPool, Task},
};
use bevy_egui::{egui, EguiContexts};
use std::sync::Arc;
use worldcraft_core::{config::seed_from_text, *};

pub fn style(mut contexts: EguiContexts, mut ui: ResMut<UiState>, mut configured: Local<bool>) {
    if *configured {
        return;
    }
    *configured = true;
    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };
    let mut s = (*ctx.global_style()).clone();
    s.spacing.item_spacing = egui::vec2(10.0, 9.0);
    s.spacing.button_padding = egui::vec2(14.0, 8.0);
    s.visuals = egui::Visuals::dark();
    s.visuals.panel_fill = egui::Color32::from_rgb(18, 25, 29);
    s.visuals.window_fill = egui::Color32::from_rgb(22, 30, 35);
    s.visuals.selection.bg_fill = egui::Color32::from_rgb(52, 103, 79);
    ctx.set_visuals(s.visuals.clone());
    ctx.set_global_style(s);
    ui.recents = storage::recent();
}
#[allow(clippy::too_many_arguments)]
pub fn draw(
    mut contexts: EguiContexts,
    mut session: ResMut<Session>,
    mut player: ResMut<Player>,
    mut ui: ResMut<UiState>,
    mut controls: ResMut<Controls>,
    mut graphics: ResMut<Graphics>,
    stream: Res<Stream>,
    mut disk: ResMut<Disk>,
    diagnostics: Res<DiagnosticsStore>,
    mut preview: Local<MapCache>,
    rays: Res<crate::raytrace::RayState>,
    mut models: ResMut<crate::models::Models>,
    mut exporter: ResMut<crate::export::Exporter>,
    mut limits: ResMut<crate::performance::Performance>,
    render: crate::render_quality::UiInfo,
) {
    let caps = &render.caps;
    let target = &render.target;
    let reconstruction = &render.state;
    let fsr = &render.fsr;
    if std::env::args().any(|a| a == "--ray-smoke-test") {
        return;
    }
    let scene_texture =
        contexts.add_image(bevy_egui::EguiTextureHandle::Strong(target.image.clone()));
    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };
    if !ui.home && session.world.is_some() {
        ctx.layer_painter(egui::LayerId::background()).image(
            scene_texture,
            ctx.content_rect(),
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            egui::Color32::WHITE,
        );
    }
    // Keep the same native scene and camera; F2 only hides editor overlays.
    if ui.photo && !ui.home && session.world.is_some() {
        return;
    }
    if limits.calibrating {
        egui::Window::new("Checking this PC").collapsible(false).resizable(false).show(ctx,|p| {p.spinner();p.label("Choosing graphics settings with a short rendered scene…");p.small("You can start exploring immediately. Automatic quality follows your frame-rate target.");});
    }
    egui::Window::new("Render settings")
        .open(&mut ui.render_settings)
        .default_width(410.0)
        .default_height(620.0)
        .resizable(true)
        .show(ctx, |p| {
            egui::ScrollArea::vertical().show(p, |p| {
                crate::render_quality::controls(
                    p,
                    &mut limits.preferences.render,
                    &reconstruction,
                    &fsr,
                    &caps,
                    &target,
                    &render.surfaces,
                );
            });
        });
    let mut root = egui::Ui::new(
        ctx.clone(),
        "worldcraft-root".into(),
        egui::UiBuilder::new()
            .layer_id(egui::LayerId::background())
            .max_rect(ctx.viewport_rect()),
    );
    let busy = disk.busy();
    let editing = session.mode == Mode::Edit;
    egui::Panel::top("toolbar").show(&mut root, |bar| {
        bar.horizontal_wrapped(|bar| {
            bar.strong(
                egui::RichText::new("WORLDCRAFT").color(egui::Color32::from_rgb(159, 221, 175)),
            );
            if bar.button("Render settings").clicked() {
                ui.render_settings = !ui.render_settings;
            }
            if bar.button("Home").clicked() {
                ui.home = true;
                player.captured = false;
            }
            if bar
                .add_enabled(!session.dirty && !busy, egui::Button::new("New world"))
                .on_hover_text("Save current changes before replacing this world")
                .clicked()
            {
                ui.home = true;
                ui.create = true;
                player.captured = false;
            }
            if bar
                .add_enabled(!session.dirty && !busy, egui::Button::new("Open"))
                .clicked()
            {
                storage::open(&mut disk, None);
                player.captured = false;
            }
            if bar
                .add_enabled(
                    session.world.is_some() && !busy && controls.live.is_empty(),
                    egui::Button::new("Save  Ctrl+S"),
                )
                .clicked()
            {
                storage::save(&mut disk, &session, &player, false);
                player.captured = false;
            }
            bar.separator();
            if session.world.is_some() {
                for (mode, label) in [
                    (Mode::Walk, "Explore"),
                    (Mode::Fly, "Fly"),
                    (Mode::Edit, "Edit world"),
                ] {
                    if bar.selectable_label(session.mode == mode, label).clicked() {
                        session.mode = mode;
                        player.velocity = 0.0;
                        player.captured = false;
                        ui.home = false;
                    }
                }
                if bar.button("Map").clicked() {
                    graphics.map = !graphics.map;
                    player.captured = false;
                }
                if bar.button("Export").clicked() {
                    exporter.open = true;
                    player.captured = false;
                }
                if bar.button("Settings").clicked() {
                    ui.inspector = !ui.inspector;
                    player.captured = false;
                }
            }
            if bar.button("Performance").clicked() {
                ui.performance = !ui.performance;
                player.captured = false;
            }
            if bar.button("Help  F1").clicked() {
                ui.help = !ui.help;
                player.captured = false;
            }
            if bar
                .button("Photo  F2")
                .on_hover_text("Hide overlays. F2 or Esc restores them.")
                .clicked()
            {
                ui.photo = true;
            }
        });
    });
    egui::Panel::bottom("status").show(&mut root, |bar| {
        bar.horizontal_wrapped(|bar| {
            if let Some(w) = &session.world {
                bar.label(format!(
                    "{}{}",
                    w.name,
                    if session.dirty { " *" } else { "" }
                ));
                bar.separator();
                bar.label(format!(
                    "Chunks {} / {} · {} jobs · last generation {:.1} ms",
                    stream.loaded.len(),
                    stream.desired,
                    stream.queue(),
                    stream.last_ms
                ));
                if let Some(fps) = diagnostics
                    .get(&FrameTimeDiagnosticsPlugin::FPS)
                    .and_then(|d| d.smoothed())
                {
                    bar.label(format!("{fps:.0} FPS"));
                }
            }
            if busy {
                bar.spinner();
                if let Some(progress) = &disk.inference {
                    use std::sync::atomic::Ordering;
                    let step = progress.step.load(Ordering::Relaxed);
                    let total = progress.total_steps.load(Ordering::Relaxed).max(1);
                    bar.label(format!("{} · {step}/{total}", progress.stage_name()));
                    bar.add(
                        egui::ProgressBar::new((step as f32 / total as f32).clamp(0.0, 1.0))
                            .desired_width(110.0),
                    );
                    if bar.button("Cancel generation").clicked() {
                        progress.cancel.store(true, Ordering::Relaxed);
                    }
                } else {
                    bar.label("Reading / saving world…");
                }
            }
            if !ui.message.is_empty() {
                bar.label(&ui.message);
            }
        });
    });
    if ui.home {
        egui::CentralPanel::default().show(&mut root,|p|{
            let top_space=(p.available_height()*0.08).clamp(16.0,60.0);
            egui::ScrollArea::vertical().auto_shrink([false,false]).show(p,|p|{
            p.add_space(top_space);
            p.vertical_centered(|p|{
                p.set_max_width(590.0);
                p.heading(egui::RichText::new("A whole world.").size(44.0));
                p.heading(egui::RichText::new("Yours to explore.").size(44.0).color(egui::Color32::from_rgb(153,210,170)));
                p.add_space(12.0);p.label("Choose a seed. Find a wilderness. Make it your own.");
                p.add_space(24.0);
                if !ui.create{
                    if p.add_enabled(!session.dirty&&!busy,egui::Button::new("Create a new world").min_size(egui::vec2(300.0,44.0))).clicked(){ui.create=true;}
                    if p.add_enabled(!session.dirty&&!busy,egui::Button::new("Open an existing world").min_size(egui::vec2(300.0,40.0))).clicked(){storage::open(&mut disk,None);}
                    if session.world.is_some()&&p.button("Return to your world").clicked(){ui.home=false;}
                    if session.world.is_some()&&!session.dirty&&!busy&&p.button("Unload saved world to free memory").clicked(){session.world=None;session.generator=None;session.history=EditHistory::default();session.chunk_revisions.clear();session.epoch=session.epoch.wrapping_add(1);}
                    if p.add_enabled(!session.dirty&&!busy,egui::Button::new("Explore the detailed AI example")).clicked(){storage::detailed_example(&mut disk);}
                    if p.add_enabled(!session.dirty&&!busy,egui::Button::new("Explore the fast AI example")).clicked(){storage::example(&mut disk);}
                    if session.dirty{p.label("Save your current world before opening or creating another.");}
                    if !ui.recents.is_empty(){
                        p.add_space(20.0);p.strong("Recent worlds");
                        for path in ui.recents.clone(){
                            let label=path.file_name().unwrap_or_default().to_string_lossy().into_owned();
                            if p.add_enabled(!session.dirty&&!busy,egui::Button::new(label)).clicked(){storage::open(&mut disk,Some(path));}
                        }
                    }
                }else{
                    p.horizontal(|p|{p.label("World name");p.text_edit_singleline(&mut ui.name);});
                    p.horizontal(|p|{
                        p.label("World seed");p.text_edit_singleline(&mut ui.seed);
                        if p.button("Random").clicked(){
                            ui.seed=std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos().to_string();
                        }
                    });
                    let old=ui.settings.preset;
                    egui::ComboBox::from_id_salt("preset").selected_text(old.label()).show_ui(p,|p|{
                        for preset in Preset::ALL{p.selectable_value(&mut ui.settings.preset,preset,preset.label());}
                    });
                    if old!=ui.settings.preset{ui.settings=GeneratorSettings::for_preset(ui.settings.preset,seed_from_text(&ui.seed));}
                    p.add(egui::Slider::new(&mut ui.settings.mountain_strength,0.0..=4.0).text("Mountains"));
                    p.add(egui::Slider::new(&mut ui.settings.vegetation,0.0..=2.0).text("Vegetation"));
                    p.add(egui::Slider::new(&mut ui.settings.temperature,0.0..=1.0).text("Temperature"));
                    p.add(egui::Slider::new(&mut ui.settings.rainfall,0.0..=1.0).text("Rainfall"));
                    p.separator();
                    let empty=ui.settings.preset==Preset::Empty;
                    if empty{ui.learned=false;p.small("Empty creates a flat canvas.");}
                    p.add_enabled(!empty,egui::Checkbox::new(&mut ui.learned,"AI terrain (experimental)"));
                    if ui.learned {
                        p.horizontal_wrapped(|p|{
                            p.selectable_value(&mut ui.detailed,false,"Fast regional");
                            p.selectable_value(&mut ui.detailed,true,"Detailed landscape");
                        });
                        if ui.detailed {
                            p.small(format!("Fine relief across a {:.1} km central region, with broad AI terrain and classical surroundings.",9.57*ui.settings.terrain_scale));
                            p.small("Optional 2.28 GB model set. Needs 6 GiB available RAM. CPU generation can take a few minutes.");
                        }else{
                            p.small("Fast AI regional relief and climate, with classical small-scale detail.");
                        }
                        p.small("Saved terrain opens and explores offline without the AI models.");
                        if models.runtime.is_none(){p.colored_label(egui::Color32::YELLOW,"This build is missing its native AI runtime.");}
                        let ready=if ui.detailed{models.detailed_ready()}else{models.ready()};
                        if !ready&&!models.downloading()&&models.runtime.is_some()&&p.button(if ui.detailed{"Download detailed models · 2.28 GB"}else{"Download fast model · 22.5 MB"}).clicked(){models.download(ui.detailed);}
                        if models.downloading() {
                            use std::sync::atomic::Ordering;
                            let bytes=models.progress.bytes.load(Ordering::Relaxed);
                            let total=models.progress.total_bytes.load(Ordering::Relaxed);
                            p.small(models.progress.stage_name());
                            if total>0{p.add(egui::ProgressBar::new((bytes as f32/total as f32).clamp(0.0,1.0)).text(format!("{:.1} / {:.1} MB",bytes as f64/1e6,total as f64/1e6)));}else{p.spinner();}
                            if p.button("Cancel").clicked(){models.cancel();}
                        }
                        if !models.error.is_empty(){p.colored_label(egui::Color32::LIGHT_RED,&models.error);}
                        if ready{p.small("Models ready · checksums verified before generation");}
                    }
                    p.checkbox(&mut ui.advanced,"Advanced generation settings");
                    if ui.advanced{
                        p.add(egui::Slider::new(&mut ui.settings.sea_level,-150.0..=200.0).text("Sea level (m)"));
                        p.add(egui::Slider::new(&mut ui.settings.terrain_scale,0.2..=5.0).text("Landscape scale"));
                        p.add(egui::Slider::new(&mut ui.settings.vertical_scale,0.1..=5.0).text("Vertical scale"));
                        p.add(egui::Slider::new(&mut ui.settings.river_strength,0.0..=3.0).text("River valley depth"));
                        p.small("River valleys use continuous terrain masks. Watershed simulation is planned.");
                    }
                    p.add_space(10.0);
                    p.horizontal(|p|{
                        if p.add_enabled(!busy&&!session.dirty&&(!ui.learned||if ui.detailed{models.detailed_ready()}else{models.ready()}),egui::Button::new("Generate and explore")).clicked(){
                            ui.settings.seed=seed_from_text(&ui.seed);
                            if ui.name.trim().is_empty(){ui.message="Give your world a name first".into();}
                            else if let Err(e)=ui.settings.validate(){ui.message=e;}
                            else{
                                if ui.learned {
                                    if ui.detailed{
                                        if let Err(error) = storage::create_detailed(&mut disk,ui.name.trim().chars().take(64).collect(),ui.settings.clone(),models.model.clone().unwrap(),models.runtime.clone().unwrap(),limits.ai_progress()) { ui.message = error; }
                                    }else{
                                        storage::create_learned(&mut disk,ui.name.trim().chars().take(64).collect(),ui.settings.clone(),models.model.clone().unwrap(),models.runtime.clone().unwrap(),limits.ai_progress());
                                    }
                                } else {storage::create(&mut disk,ui.name.trim().chars().take(64).collect(),ui.settings.clone());}
                                if disk.busy() { ui.message="Preparing terrain and finding a safe starting place…".into(); }
                            }
                        }
                        if p.button("Cancel").clicked(){ui.create=false;}
                    });
                }
                p.add_space(28.0);p.small("Development build · Native worlds · Offline exploration");
            });
            });
        });
    } else {
        if ui.inspector && !player.captured {
            egui::Panel::right("inspector").default_size(285.0).resizable(true).show(&mut root,|p|{
                egui::ScrollArea::vertical().show(p,|p|{
                    if editing{
                        p.heading("Shape your world");
                        p.small("Left drag to paint. Right drag + WASD to move.");
                        egui::ComboBox::from_id_salt("brush").selected_text(controls.brush.kind.label()).show_ui(p,|p|{
                            for kind in BrushKind::ALL{p.selectable_value(&mut controls.brush.kind,kind,kind.label());}
                        });
                        p.add(egui::Slider::new(&mut controls.brush.radius,2.0..=256.0).text("Brush radius"));
                        p.add(egui::Slider::new(&mut controls.brush.strength,0.1..=30.0).text("Strength"));
                        if controls.brush.kind==BrushKind::Biome{
                            egui::ComboBox::from_id_salt("biome").selected_text(controls.brush.biome.label()).show_ui(p,|p|{
                                for b in Biome::ALL{p.selectable_value(&mut controls.brush.biome,b,b.label());}
                            });
                        }
                        if controls.brush.kind==BrushKind::Vegetation{
                            p.add(egui::Slider::new(&mut controls.brush.density,0.0..=2.0).text("Density"));
                        }
                        p.horizontal(|p|{
                            if p.add_enabled(controls.live.is_empty()&&!session.history.transactions.is_empty(),egui::Button::new("Undo")).clicked(){session.history.undo();session.rebuild(&[]);}
                            if p.add_enabled(controls.live.is_empty()&&!session.history.redo.is_empty(),egui::Button::new("Redo")).clicked(){session.history.redo();session.rebuild(&[]);}
                        });
                        p.label(format!("{} edit transactions",session.history.transactions.len()));
                        p.separator();
                    }
                    p.heading("Your surroundings");
                    if let Some(g)=&session.generator{
                        let s=g.sample(player.position.x,player.position.z);
                        p.label(s.biome.label());p.label(format!("Elevation {:.0} m",s.height));
                        p.label(format!("Position {:.0}, {:.0}",player.position.x,player.position.z));
                    }
                    p.separator();p.heading("Light and atmosphere");
                    p.add(egui::Slider::new(&mut graphics.hour,0.0..=23.99).text("Time of day"));
                    p.checkbox(&mut graphics.cycle,"Day / night cycle");
                    if graphics.cycle {
                        p.add(egui::Slider::new(&mut graphics.day_minutes,2.0..=60.0).text("Minutes per day"));
                    }
                    egui::ComboBox::new("weather-mode","Weather")
                        .selected_text(graphics.weather.map_or("Automatic",crate::weather::Kind::label))
                        .show_ui(p,|p|{
                            p.selectable_value(&mut graphics.weather,None,"Automatic");
                            for kind in crate::weather::Kind::ALL {
                                p.selectable_value(&mut graphics.weather,Some(kind),kind.label());
                            }
                        });
                    if graphics.weather.is_none() { p.small(render.weather.kind.label()); }
                    p.add(egui::Slider::new(&mut graphics.breeze,0.0..=1.0).text("Foliage breeze")).on_hover_text("Foliage movement pauses in experimental ray mode.");
                    p.add(egui::Slider::new(&mut graphics.fog,0.0..=3.0).text("Haze"));
                    p.checkbox(&mut graphics.volumetrics,"Volumetric light shafts (High)");
                    p.checkbox(&mut graphics.clouds,"Textured cloud volume (High)");
                    p.add(egui::Slider::new(&mut graphics.mist_density,0.0..=0.008).text("Ground mist density"));
                    p.add(egui::Slider::new(&mut graphics.mist_height,20.0..=1500.0).text("Mist layer top (m)"));
                    p.add(egui::Slider::new(&mut graphics.exposure,-3.0..=3.0).text("Exposure offset"));
                    p.separator();p.heading("Graphics");
                    p.label(&caps.gpu);p.small(format!("{} · {} CPU threads",caps.backend,caps.cpu_threads));
                    p.small(&caps.ray_reason);
                    if caps.software {p.small("Software graphics: shadow maps are limited to 1024 pixels to reduce memory use.");}
                    p.add_enabled(caps.hardware_rays && !caps.software,
                        egui::Checkbox::new(&mut graphics.ray_tracing,"Ray-traced lighting (experimental)"));
                    if rays.active {
                        p.small(format!("{} ray instances · {} preparing",rays.instances,rays.pending));
                        p.small("Direct and indirect rays. Secondary surfaces use averaged vertex colours.");
                        p.small("Grass self-shadowing is unavailable in this experimental mode.");
                    }
                    let automatic=limits.preferences.mode==crate::performance::Mode::Auto;
                    if automatic {p.small(format!("Automatic: {:?} · {} chunks · {:.0}% resolution",limits.quality(graphics.quality),limits.radius(graphics.radius),limits.render_scale()*100.0));}
                    p.add_enabled_ui(!automatic,|p| {
                    let old=graphics.quality;
                    egui::ComboBox::from_id_salt("quality").selected_text(format!("{:?}",graphics.quality)).show_ui(p,|p|{
                        for(q,label)in[(Quality::Low,"Low"),(Quality::Balanced,"Balanced"),(Quality::High,"High")]{
                            p.selectable_value(&mut graphics.quality,q,label);
                        }
                    });
                    if graphics.quality!=old{graphics.radius=match graphics.quality{Quality::Low=>3,Quality::Balanced=>6,Quality::High=>9};}
                    p.add(egui::Slider::new(&mut graphics.radius,2..=12).text("View distance (chunks)"));
                    });
                    if automatic {p.small("Choose a manual mode in Performance to set graphics and view distance yourself.");}
                    p.small("High adds ambient occlusion. Balanced uses atmosphere and shadows. Low reduces graphics cost.");
                    p.add(egui::Slider::new(&mut controls.speed,5.0..=160.0).text("Fly speed (m/s)"));
                    p.separator();p.small("E: edit · Tab: walk/fly · Esc: release mouse");
                    p.small("Ctrl+S: save · Ctrl+Z/Y: undo/redo");
                });
            });
        }
        if !player.captured {
            egui::Area::new(egui::Id::new("explore_hint")).anchor(egui::Align2::CENTER_TOP,[0.0,70.0]).show(ctx,|p|{
                egui::Frame::popup(p.style()).show(p,|p|{
                    p.label(if editing{"Paint the landscape, or right drag to move."}else{"Click the landscape to explore. WASD moves. Space jumps. Shift sprints."});
                });
            });
        } else {
            let c = ctx.content_rect().center();
            let painter = ctx.layer_painter(egui::LayerId::new(
                egui::Order::Foreground,
                egui::Id::new("crosshair"),
            ));
            painter.line_segment(
                [c - egui::vec2(4.0, 0.0), c + egui::vec2(4.0, 0.0)],
                egui::Stroke::new(1.0_f32, egui::Color32::WHITE),
            );
            painter.line_segment(
                [c - egui::vec2(0.0, 4.0), c + egui::vec2(0.0, 4.0)],
                egui::Stroke::new(1.0_f32, egui::Color32::WHITE),
            );
        }
    }
    if graphics.map && !ui.home {
        egui::Window::new("World map").default_width(540.0).show(ctx,|p|{
            if session.generator.as_ref().is_some_and(|g|g.version()>=2){p.small("Regional preview · small erosion details are omitted");}
            p.horizontal(|p|{
                for(i,label)in["Biome","Elevation","Temperature","Moisture"].iter().enumerate(){
                    p.selectable_value(&mut ui.map_layer,i,*label);
                }
            });
            p.add(egui::Slider::new(&mut ui.map_zoom,256.0..=8000.0).text("Map width (m)"));
            if let Some(g)=&session.generator{
                let coord=ChunkCoord::at(player.position.x,player.position.z);
                let o=coord.origin();let center=[o[0]+64.0,o[1]+64.0];
                let map_key=(session.epoch,session.revision,coord,ui.map_zoom.to_bits(),ui.map_layer);
                preview.update(ctx,Arc::clone(g),map_key);
                if let Some(texture)=&preview.texture{
                    let size=p.available_width().min(520.0).min((ctx.content_rect().height()-240.0).max(128.0));
                    let response=p.image((texture.id(),egui::vec2(size,size))).interact(egui::Sense::click());
                    if response.double_clicked()&&preview.key==Some(map_key){
                        if let Some(pos)=response.interact_pointer_pos(){
                            let uv=(pos-response.rect.min)/response.rect.size();
                            let x=center[0]+(f64::from(uv.x)-0.5)*ui.map_zoom;
                            let z=center[1]+(f64::from(uv.y)-0.5)*ui.map_zoom;
                            player.position=DVec3::new(x,g.height(x,z)+50.0,z);
                            player.velocity=0.0;session.mode=Mode::Fly;player.captured=false;
                        }
                    }
                }
                if preview.task.is_some(){p.spinner();p.label("Updating map…");}
                p.small("North is up. The center is your current terrain chunk. Double-click to fly to a location.");
            }
        });
    }
    exporter.show(
        ctx,
        session.generator.clone(),
        [player.position.x, player.position.z],
    );
    if ui.performance {
        egui::Window::new("Performance and power").default_width(380.0).show(ctx, |p| {
            p.label(format!("Active budget: {:?} · 3D resolution: {}%", limits.mode(), (limits.render_scale()*100.0).round()));
            p.label(&limits.calibration_status);
            if limits.preferences.mode == crate::performance::Mode::Auto {
                p.label(format!("Automatic graphics: {:?} · view radius: {} chunks",limits.quality(graphics.quality),limits.radius(graphics.radius)));
                if limits.controller.frame_ms > 0.0 { p.label(format!("Recent p90 {:.1} ms · {:.0} FPS target ({:.1} ms)",limits.controller.frame_ms,limits.effective_target,1000.0/limits.effective_target)); }
                if limits.controller.limited { p.colored_label(egui::Color32::YELLOW,"Minimum detail reached for the current frame-rate target."); }
            }
            egui::ComboBox::from_id_salt("power_mode").selected_text(format!("{:?}", limits.preferences.mode)).show_ui(p, |p| {
                for (mode, label) in [(crate::performance::Mode::Auto,"Automatic"), (crate::performance::Mode::Eco,"Eco · low-end PCs"), (crate::performance::Mode::Balanced,"Balanced"), (crate::performance::Mode::Performance,"Performance")] {
                    p.selectable_value(&mut limits.preferences.mode, mode, label);
                }
            });
            if limits.preferences.mode == crate::performance::Mode::Auto {
                p.add(egui::Slider::new(&mut limits.preferences.auto_target_fps, 15..=120).text("Automatic quality FPS target"));
            }
            p.add(egui::Slider::new(&mut limits.preferences.fps, 15..=120).text("Maximum FPS"));
            p.small("Eco: 25 near chunks, one paced terrain job, simpler trees, no grass or distant terrain, Low graphics. Distance haze softens the shorter view range. Manual Eco caps at 30 FPS.");
            p.small("Home pauses 3D rendering and terrain work. Unfocused windows pause 3D and use fewer updates.");
            p.small("AI thread changes apply to the next generation. Detailed AI needs 6 GiB available RAM; the offline examples do not run AI.");
            p.small("These are work limits, not a hard process RAM or CPU percentage cap.");
            if let Some(memory) = limits.memory {
                p.label(format!("System RAM: {:.1} GiB available / {:.1} GiB total", memory.available as f64 / 1073741824.0, memory.total as f64 / 1073741824.0));
            }
            if limits.pressure { p.colored_label(egui::Color32::YELLOW, "Low available RAM: Eco protection is active."); }
            if !limits.save_error.is_empty() { p.label(format!("Could not save preferences: {}", limits.save_error)); }
            p.small("Preferences are kept on this PC. Your world and terrain settings stay the same.");
            if p.button("Close").clicked() { ui.performance=false; }
        });
    }
    if ui.help {
        egui::Window::new("Welcome to Worldcraft").collapsible(false).show(ctx,|p|{
            p.label("Create a world, then click the landscape to capture your mouse.");
            egui::Grid::new("keys").striped(true).show(p,|p|{
                for(key,action)in[("W A S D","Move"),("Mouse","Look"),("Space","Jump / fly up"),
                    ("Shift","Sprint / fast flight"),("Ctrl","Fly down"),("Tab","Switch walking / flying"),
                    ("E","Toggle editing"),("F2","Photo view"),("Esc","Release mouse / leave photo view"),("Ctrl+S","Save"),("Ctrl+Z / Y","Undo / redo")]{
                    p.monospace(key);p.label(action);p.end_row();
                }
            });
            p.separator();p.label("In Edit mode, left drag paints one undoable stroke. Right drag moves the camera.");
            p.label("Recovery snapshots save every minute while you have changes. Open a recovery JSON through Open to restore it.");
            p.hyperlink_to("Project documentation","https://github.com/InduwaraLasindu/Worldcraft");
            if p.button("Back to the world").clicked(){ui.help=false;}
        });
    }
}
type MapKey = (u64, u64, ChunkCoord, u64, usize);
#[derive(Default)]
pub struct MapCache {
    task: Option<Task<(MapKey, Vec<u8>)>>,
    texture: Option<egui::TextureHandle>,
    key: Option<MapKey>,
}
impl MapCache {
    fn update(&mut self, ctx: &egui::Context, g: Arc<Generator>, key: MapKey) {
        if let Some(result) = self
            .task
            .as_mut()
            .and_then(|t| block_on(future::poll_once(t)))
        {
            self.task = None;
            self.key = Some(result.0);
            self.texture = Some(ctx.load_texture(
                "world_map",
                egui::ColorImage::from_rgb([128, 128], &result.1),
                egui::TextureOptions::LINEAR,
            ));
        }
        if self.key != Some(key) && self.task.is_none() {
            self.task = Some(AsyncComputeTaskPool::get().spawn(async move {
                let o = key.2.origin();
                let span = f64::from_bits(key.3);
                let mut pixels = Vec::with_capacity(128 * 128 * 3);
                for z in 0..128 {
                    for x in 0..128 {
                        let wx = o[0] + 64.0 + (f64::from(x) / 127.0 - 0.5) * span;
                        let wz = o[1] + 64.0 + (f64::from(z) / 127.0 - 0.5) * span;
                        let s = g.preview_sample(wx, wz);
                        let color = match key.4 {
                            0 => s.biome.color(),
                            1 => {
                                let h = ((s.height - g.settings().sea_level + 100.0) / 800.0)
                                    .clamp(0.0, 1.0) as f32;
                                [h, h, h]
                            }
                            2 => [s.temperature as f32, 0.2, 1.0 - s.temperature as f32],
                            _ => [0.1, s.moisture as f32, 0.3 + s.moisture as f32 * 0.7],
                        };
                        for c in color {
                            pixels.push((c.clamp(0.0, 1.0) * 255.0) as u8);
                        }
                    }
                }
                (key, pixels)
            }));
        }
    }
}
