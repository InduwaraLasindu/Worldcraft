//! Optional model management, with streamed downloads, explicit size and cancellation.
use bevy::{prelude::*,tasks::{block_on,futures_lite::future,IoTaskPool,Task}};
use std::{path::PathBuf,sync::{Arc,atomic::Ordering}};
use worldcraft_ml::Progress;
#[derive(Resource)]
pub struct Models {
    pub model:Option<PathBuf>,
    pub runtime:Option<PathBuf>,
    pub progress:Arc<Progress>,
    pub error:String,
    task:Option<Task<Result<PathBuf,String>>>,
    checked:bool,
}
pub fn runtime_path()->Option<PathBuf> {
    if let Some(path)=std::env::var_os("WORLDCRAFT_ORT_DYLIB").map(PathBuf::from).filter(|p|p.is_file()) {return Some(path);}
    let root=std::env::current_exe().ok()?.parent()?.to_owned();
    let name=if cfg!(windows) {"onnxruntime.dll"} else {"libonnxruntime.so"};
    [root.join(name),root.join("lib").join(name)].into_iter().find(|p|p.is_file())
}
impl Default for Models {
    fn default()->Self {
        let file=worldcraft_ml::model_path(&crate::storage::data_dir().join("models"));
        let model=file.metadata().ok().filter(|m|m.len()==worldcraft_ml::MODEL_BYTES).map(|_|file);
        Self {model,runtime:runtime_path(),progress:Arc::new(Progress::default()),error:String::new(),task:None,checked:false}
    }
}
impl Models {
    pub fn downloading(&self)->bool {self.task.is_some()}
    pub fn ready(&self)->bool {self.checked&&self.model.is_some()&&self.runtime.is_some()&&!self.downloading()}
    pub fn download(&mut self) {
        if self.downloading(){return;}
        self.error.clear();self.progress=Arc::new(Progress::default());
        let progress=Arc::clone(&self.progress);
        self.task=Some(IoTaskPool::get().spawn(async move{
            worldcraft_ml::download(&crate::storage::data_dir().join("models"),&progress)
        }));
    }
    pub fn cancel(&self) {self.progress.cancel.store(true,Ordering::Relaxed);}
}
pub fn poll(mut models:ResMut<Models>) {
    if !models.checked && models.task.is_none() {
        models.checked=true;
        if let Some(path)=models.model.clone() {
            models.task=Some(IoTaskPool::get().spawn(async move{worldcraft_ml::verify_model(&path)?;Ok(path)}));
        }
    }
    if let Some(result)=models.task.as_mut().and_then(|task|block_on(future::poll_once(task))) {
        models.task=None;
        match result {Ok(path)=>{models.model=Some(path);models.error.clear();},Err(error)=>{models.model=None;models.error=error}}
    }
}
