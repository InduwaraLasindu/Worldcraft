use bevy::{prelude::*,tasks::{block_on,futures_lite::future,IoTaskPool,Task}};
use bevy_egui::egui;
use std::sync::{Arc,atomic::{AtomicBool,AtomicUsize,Ordering}};
use worldcraft_core::{Generator,export::Region};
#[derive(Clone,Copy,PartialEq,Eq,Default)]
enum Format {#[default]Glb,Obj,Heightmap}
#[derive(Resource)]
pub struct Exporter {
    pub open:bool,center:[f64;2],width:f64,cells:usize,format:Format,
    task:Option<Task<Result<String,String>>>,progress:Arc<AtomicUsize>,cancel:Arc<AtomicBool>,message:String,
}
impl Default for Exporter {
    fn default()->Self {Self{open:false,center:[0.0,0.0],width:2048.0,cells:256,format:Format::Glb,task:None,progress:Arc::new(AtomicUsize::new(0)),cancel:Arc::new(AtomicBool::new(false)),message:String::new()}}
}
pub fn poll(mut export:ResMut<Exporter>) {
    if let Some(result)=export.task.as_mut().and_then(|task|block_on(future::poll_once(task))){
        export.task=None;export.message=match result {Ok(message)=>message,Err(error)=>error};
    }
}
impl Exporter {
    pub fn show(&mut self,ctx:&egui::Context,generator:Option<Arc<Generator>>,position:[f64;2]) {
        if !self.open{return;}let mut visible=true;
        egui::Window::new("Export terrain").open(&mut visible).default_width(380.0).show(ctx,|p|{
            p.label("Export the edited terrain around a location.");
            p.small("Terrain only. Trees, water and application shaders are not included.");
            let busy=self.task.is_some();
            p.add_enabled_ui(!busy,|p|{
                if p.button("Use current position").clicked(){self.center=position;}
                p.horizontal(|p|{
                    p.add(egui::DragValue::new(&mut self.center[0]).speed(10.0).prefix("X: "));
                    p.add(egui::DragValue::new(&mut self.center[1]).speed(10.0).prefix("Z: "));
                });
                p.add(egui::Slider::new(&mut self.width,128.0..=8192.0).text("Width (m)"));
                egui::ComboBox::from_id_salt("export-res").selected_text(format!("{} × {} cells",self.cells,self.cells)).show_ui(p,|p|{
                    for cells in [32,64,128,256,512]{p.selectable_value(&mut self.cells,cells,format!("{cells} × {cells} cells"));}
                });
                p.small(format!("Sample spacing {:.2} m · {} vertices",self.width/self.cells as f64,(self.cells+1).pow(2)));
                for (format,label) in [(Format::Glb,"GLB · colour and normals"),(Format::Obj,"OBJ · geometry and normals"),(Format::Heightmap,"16-bit PNG · heightmap")] {
                    p.radio_value(&mut self.format,format,label);
                }
                if self.format==Format::Heightmap{p.small("PNG text metadata contains the origin, spacing and elevation range.");}
                if p.add_enabled(generator.is_some(),egui::Button::new("Choose location and export")).clicked() {
                    let g=generator.clone().unwrap();let region=Region{center:self.center,width:self.width,cells:self.cells};let format=self.format;
                    self.progress=Arc::new(AtomicUsize::new(0));self.cancel=Arc::new(AtomicBool::new(false));self.message.clear();
                    let progress=Arc::clone(&self.progress);let cancel=Arc::clone(&self.cancel);
                    self.task=Some(IoTaskPool::get().spawn(async move{
                        region.validate()?;
                        let ext=match format{Format::Glb=>"glb",Format::Obj=>"obj",Format::Heightmap=>"png"};
                        let Some(path)=rfd::FileDialog::new().set_title("Export Worldcraft terrain").set_file_name(format!("Worldcraft-terrain.{ext}")).add_filter("Terrain export",&[ext]).save_file() else{return Ok("Export cancelled".into());};
                        let check=|value|{
                            if cancel.load(Ordering::Relaxed){return Err("Export cancelled".into());}
                            progress.store(value,Ordering::Relaxed);Ok(())
                        };
                        match format{
                            Format::Glb=>worldcraft_core::export::write_glb(&path,&g,region,check)?,
                            Format::Obj=>worldcraft_core::export::write_obj(&path,&g,region,check)?,
                            Format::Heightmap=>worldcraft_core::export::write_heightmap(&path,&g,region,check)?,
                        }
                        Ok(format!("Exported {}",path.display()))
                    }));
                }
            });
            if busy{
                p.add(egui::ProgressBar::new(self.progress.load(Ordering::Relaxed) as f32/100.0).show_percentage());
                if p.button("Cancel export").clicked(){self.cancel.store(true,Ordering::Relaxed);}
            }
            if !self.message.is_empty(){p.label(&self.message);}
        });self.open=visible;
    }
}
