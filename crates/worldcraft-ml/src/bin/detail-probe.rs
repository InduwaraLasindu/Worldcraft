use ort::{session::{Session,SessionInputValue},value::Tensor};
use std::{path::PathBuf,time::Instant};
fn main()->Result<(),String>{
    let args:Vec<String>=std::env::args().collect();
    let folder=PathBuf::from(args.get(1).ok_or("model folder required")?);
    let runtime=PathBuf::from(args.get(2).ok_or("runtime required")?);
    let env=ort::init_from(&runtime).map_err(|e|e.to_string())?.with_name("worldcraft-detail-probe").with_telemetry(false).build().map_err(|e|e.to_string())?;
    let mut results=Vec::new();
    for (name,side,channels,condition) in [("base_model.onnx",64usize,5usize,true),("decoder_model.onnx",256,1,false)]{
        let start=Instant::now();
        println!("Loading {name} sequentially on the CPU");
        let mut session=Session::builder(&env).map_err(|e|e.to_string())?
            .with_intra_threads(2).map_err(|e|e.to_string())?
            .with_inter_threads(1).map_err(|e|e.to_string())?
            .with_parallel_execution(false).map_err(|e|e.to_string())?
            .with_deterministic_compute(true).map_err(|e|e.to_string())?
            .commit_from_file(folder.join(name)).map_err(|e|format!("{name}: {e}"))?;
        let input_names:Vec<String>=session.inputs().iter().map(|i|i.name().to_owned()).collect();
        println!("Loaded {name}: inputs={input_names:?}");
        let mut x=vec![0.0_f32;5*side*side];
        let noise=worldcraft_ml::normal_noise(20261116,channels*side*side);
        x[..noise.len()].copy_from_slice(&noise);
        let mut inputs:Vec<(String,SessionInputValue<'_>)>=vec![
            ("x".into(),Tensor::from_array(([1usize,5,side,side],x)).map_err(|e|e.to_string())?.into()),
            ("noise_labels".into(),Tensor::from_array(([1usize],vec![160_f32.atan()])).map_err(|e|e.to_string())?.into())];
        if condition{inputs.push(("cond_0".into(),Tensor::from_array(([1usize,58],vec![0.0_f32;58])).map_err(|e|e.to_string())?.into()));}
        let output=session.run(inputs).map_err(|e|format!("{name} inference: {e}"))?;
        let (shape,data)=output[0].try_extract_tensor::<f32>().map_err(|e|e.to_string())?;
        if **shape!=[1,channels as i64,side as i64,side as i64]||!data.iter().all(|v|v.is_finite()){return Err(format!("Invalid {name} result"));}
        let result=serde_json::json!({"model":name,"inputs":input_names,"output_shape":format!("{shape:?}"),"finite":true,"min":data.iter().copied().fold(f32::INFINITY,f32::min),"max":data.iter().copied().fold(f32::NEG_INFINITY,f32::max),"seconds":start.elapsed().as_secs_f64(),"memory":std::fs::read_to_string("/proc/self/status").unwrap_or_default().lines().filter(|s|s.starts_with("VmHWM")||s.starts_with("VmRSS")).collect::<Vec<_>>()});
        println!("{result}");results.push(result);
    }
    std::fs::write("detail-probe.json",serde_json::to_vec_pretty(&serde_json::json!({"status":"passed","native_rust":true,"provider":"ONNX Runtime CPU","contract_probe_only":true,"models":results})).map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;
    Ok(())
}
