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
    caps: Res<crate::capabilities::Capabilities>,
    mut disk: ResMut<Disk>,
    diagnostics: Res<DiagnosticsStore>,
    mut preview: Local<MapCache>,
    rays: Res<crate::raytrace::RayState>,
    mut models: ResMut<crate::models::Models>,
    mut exporter: ResMut<crate::export::Exporter>,
) {
    if std::env::args().any(|a| a == "--ray-smoke-test") {
        return;
    }
    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };
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
            if bar.button("Help  F1").clicked() {
                ui.help = !ui.help;
                player.captured = false;
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
                    bar.label(format!("Terrain Diffusion · step {step}/20"));
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
                    if p.add_enabled(!session.dirty&&!busy,egui::Button::new("Explore the AI example")).clicked(){storage::example(&mut disk);}
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
                    p.checkbox(&mut ui.learned,"Terrain Diffusion regional hybrid (experimental)");
                    if ui.learned {
                        p.small("AI regional relief and climate, with classical fine detail and surrounding terrain.");
                        p.small("Generated relief is saved in your world. Exploration works offline.");
                        if models.runtime.is_none(){p.colored_label(egui::Color32::YELLOW,"This build is missing its native AI runtime.");}
                        if models.model.is_none() && !models.downloading() && p.button("Download model · 22.5 MB").clicked(){models.download();}
                        if models.downloading() {
                            use std::sync::atomic::Ordering;
                            let bytes=models.progress.bytes.load(Ordering::Relaxed);
                            p.add(egui::ProgressBar::new(bytes as f32/worldcraft_ml::MODEL_BYTES as f32).text(format!("{:.1} / 22.5 MB",bytes as f64/1e6)));
                            if p.button("Cancel download").clicked(){models.cancel();}
                        }
                        if !models.error.is_empty(){p.colored_label(egui::Color32::LIGHT_RED,&models.error);}
                        if models.ready(){p.small("Model ready · checksum verified before generation");}
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
                        if p.add_enabled(!busy&&!session.dirty&&(!ui.learned||models.ready()),egui::Button::new("Generate and explore")).clicked(){
                            ui.settings.seed=seed_from_text(&ui.seed);
                            if ui.name.trim().is_empty(){ui.message="Give your world a name first".into();}
                            else if let Err(e)=ui.settings.validate(){ui.message=e;}
                            else{
                                if ui.learned {
                                    storage::create_learned(&mut disk,ui.name.trim().chars().take(64).collect(),ui.settings.clone(),
                                        models.model.clone().unwrap(),models.runtime.clone().unwrap());
                                } else {storage::create(&mut disk,ui.name.trim().chars().take(64).collect(),ui.settings.clone());}
                                ui.message="Preparing terrain and finding a safe starting place…".into();
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
                    p.add(egui::Slider::new(&mut graphics.fog,0.0..=3.0).text("Haze"));
                    p.checkbox(&mut graphics.volumetrics,"Volumetric light shafts (High)");
                    p.add(egui::Slider::new(&mut graphics.mist_density,0.0..=0.008).text("Ground mist density"));
                    p.add(egui::Slider::new(&mut graphics.mist_height,20.0..=1500.0).text("Mist layer top (m)"));
                    p.add(egui::Slider::new(&mut graphics.exposure,-3.0..=3.0).text("Exposure offset"));
                    p.separator();p.heading("Graphics");
                    p.label(&caps.gpu);p.small(format!("{} · {} CPU threads",caps.backend,caps.cpu_threads));
                    p.small(&caps.ray_reason);
                    p.add_enabled(caps.hardware_rays && !caps.software,
                        egui::Checkbox::new(&mut graphics.ray_tracing,"Ray-traced lighting (experimental)"));
                    if rays.active {
                        p.small(format!("{} ray instances · {} preparing",rays.instances,rays.pending));
                        p.small("Direct and indirect rays. Secondary surfaces use averaged vertex colours.");
                    }
                    let old=graphics.quality;
                    egui::ComboBox::from_id_salt("quality").selected_text(format!("{:?}",graphics.quality)).show_ui(p,|p|{
                        for(q,label)in[(Quality::Low,"Low"),(Quality::Balanced,"Balanced"),(Quality::High,"High")]{
                            p.selectable_value(&mut graphics.quality,q,label);
                        }
                    });
                    if graphics.quality!=old{graphics.radius=match graphics.quality{Quality::Low=>3,Quality::Balanced=>6,Quality::High=>9};}
                    p.add(egui::Slider::new(&mut graphics.radius,2..=12).text("View distance (chunks)"));
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
    if ui.help {
        egui::Window::new("Welcome to Worldcraft").collapsible(false).show(ctx,|p|{
            p.label("Create a world, then click the landscape to capture your mouse.");
            egui::Grid::new("keys").striped(true).show(p,|p|{
                for(key,action)in[("W A S D","Move"),("Mouse","Look"),("Space","Jump / fly up"),
                    ("Shift","Sprint / fast flight"),("Ctrl","Fly down"),("Tab","Switch walking / flying"),
                    ("E","Toggle editing"),("Esc","Release mouse"),("Ctrl+S","Save"),("Ctrl+Z / Y","Undo / redo")]{
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
