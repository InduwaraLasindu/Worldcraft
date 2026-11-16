use std::{path::{Path,PathBuf},sync::{Arc,atomic::{AtomicBool,Ordering}},thread,time::{Duration,Instant}};
use worldcraft_ml::{assets::{self,ModelPaths},Progress,MODEL_BYTES};
fn cancelled_transfer(folder:&Path,detailed:bool,threshold:u64)->Result<(),String>{
    let progress=Arc::new(Progress::default());let done=Arc::new(AtomicBool::new(false));
    let p=Arc::clone(&progress);let finished=Arc::clone(&done);
    let watcher=thread::spawn(move||{
        let until=Instant::now()+Duration::from_secs(120);
        loop{
            let bytes=p.bytes.load(Ordering::Relaxed);
            if bytes>=threshold{p.cancel.store(true,Ordering::Relaxed);return bytes;}
            if finished.load(Ordering::Relaxed)||Instant::now()>until{p.cancel.store(true,Ordering::Relaxed);return bytes;}
            thread::yield_now();
        }
    });
    let result=assets::download_models(folder,&progress,detailed);
    done.store(true,Ordering::Relaxed);
    let interrupted_at=watcher.join().map_err(|_|"Download cancellation monitor failed")?;
    if interrupted_at<threshold{return Err(format!("Transfer did not reach its cancellation threshold: {interrupted_at}; result={result:?}"));}
    if result.err().as_deref()!=Some("Cancelled"){return Err("Native transfer did not stop after cancellation".into());}
    let model=ModelPaths::in_folder(folder);
    let parent=model.coarse.parent().ok_or("Missing model parent")?;
    if parent.exists()&&std::fs::read_dir(parent).map_err(|e|e.to_string())?.filter_map(Result::ok).any(|e|e.path().extension().is_some_and(|e|e=="part")){
        return Err("Cancellation left a partial download".into());
    }Ok(())
}
fn main()->Result<(),String>{
    let root=PathBuf::from(std::env::args().nth(1).ok_or("Hosted temporary folder required")?);
    let fresh=root.join("cancel-and-retry");
    cancelled_transfer(&fresh,false,65536)?;
    let paths=ModelPaths::in_folder(&fresh);
    assert!(!paths.coarse.exists(),"Cancelled download became a final model");
    let progress=Progress::default();
    let downloaded=assets::download_models(&fresh,&progress,false)?;
    assets::verify_asset(&downloaded.coarse,assets::COARSE,&Progress::default())?;
    assert_eq!(progress.bytes.load(Ordering::Relaxed),MODEL_BYTES);
    cancelled_transfer(&fresh,true,MODEL_BYTES+65536)?;
    assert!(!paths.base.exists()&&!paths.decoder.exists(),"Cancelled detailed download left final large models");
    assets::verify_asset(&paths.coarse,assets::COARSE,&Progress::default())?;
    assets::download_models(&fresh,&Progress::default(),false)?;
    let source=std::env::var("GITHUB_SHA").map_err(|e|e.to_string())?;
    let report=format!(r#"{{"status":"passed","source":"{source}","native_rust":true,"cancelled_fast_transfer":true,"verified_retry_bytes":{MODEL_BYTES},"cancelled_live_base_transfer":true,"fast_model_preserved":true,"partial_files_removed":true}}"#);
    std::fs::write("native-download-audit.json",&report).map_err(|e|e.to_string())?;
    println!("{report}");Ok(())
}
