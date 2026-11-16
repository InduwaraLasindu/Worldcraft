//! Bounded terrain export from an immutable edited world snapshot.
use crate::{Generator,MeshData};
use serde_json::json;
use std::{io::Write,path::Path};

#[derive(Clone,Copy,Debug)]
pub struct Region {pub center:[f64;2],pub width:f64,pub cells:usize}
impl Region {
    pub fn validate(&self)->Result<(),String> {
        if !self.center.iter().all(|v|v.is_finite()&&v.abs()<crate::config::MAX_WORLD_COORD-20000.0)
            || !self.width.is_finite() || !(128.0..=16384.0).contains(&self.width)
            || ![32,64,128,256,512].contains(&self.cells) {return Err("Export bounds or resolution are out of range".into());}Ok(())
    }
}
pub fn terrain(g:&Generator,r:Region,mut progress:impl FnMut(usize)->Result<(),String>)->Result<MeshData,String> {
    r.validate()?;let mut mesh=MeshData::default();let row=r.cells+1;let step=r.width/r.cells as f64;
    for z in 0..row {progress(z*100/row)?;for x in 0..row {
        let lx=x as f64*step-r.width*0.5;let lz=z as f64*step-r.width*0.5;
        let (wx,wz)=(r.center[0]+lx,r.center[1]+lz);
        mesh.vertex([lx as f32,g.height(wx,wz) as f32,lz as f32],g.normal(wx,wz),g.color(wx,wz),[x as f32/r.cells as f32,z as f32/r.cells as f32]);
    }}
    for z in 0..r.cells {for x in 0..r.cells {
        let a=(z*row+x) as u32;let b=a+1;let c=a+row as u32;let d=c+1;
        mesh.indices.extend_from_slice(&[a,c,b,b,c,d]);
    }}progress(100)?;Ok(mesh)
}
fn atomic(path:&Path,bytes:&[u8])->Result<(),String> {
    let stamp=std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos();
    let temporary=path.with_extension(format!("{}.{}.exporting",std::process::id(),stamp));
    let result=(||{
        let mut file=std::fs::OpenOptions::new().create_new(true).write(true).open(&temporary).map_err(|e|e.to_string())?;
        file.write_all(bytes).map_err(|e|e.to_string())?;file.sync_all().map_err(|e|e.to_string())?;drop(file);
        // The file picker authorizes replacement. Keep a backup until replacement succeeds.
        let backup=path.with_extension(format!("{}.{stamp}.export-backup",std::process::id()));
        if path.exists(){std::fs::copy(path,&backup).map_err(|e|e.to_string())?;}
        match std::fs::rename(&temporary,path) {
            Ok(())=>{let _=std::fs::remove_file(&backup);Ok(())},
            Err(error)=>Err(format!("Cannot finalize export: {error}")),
        }
    })();
    if result.is_err(){let _=std::fs::remove_file(&temporary);}result
}
pub fn glb(mesh:&MeshData,r:Region)->Result<Vec<u8>,String> {
    r.validate()?;let n=mesh.positions.len();
    if n==0 || n>513*513 || mesh.normals.len()!=n || mesh.colors.len()!=n || mesh.uv.len()!=n
        || mesh.indices.len()>512*512*6 || mesh.indices.iter().any(|i|*i as usize>=n)
        || !mesh.positions.iter().flatten().all(|v|v.is_finite()) {return Err("Invalid export mesh".into());}
    let mut binary=Vec::with_capacity(n*48+mesh.indices.len()*4);
    let mut low=[f32::INFINITY;3];let mut high=[f32::NEG_INFINITY;3];
    for i in 0..n {
        for axis in 0..3 {low[axis]=low[axis].min(mesh.positions[i][axis]);high[axis]=high[axis].max(mesh.positions[i][axis]);}
        for v in mesh.positions[i].iter().chain(&mesh.normals[i]).chain(&mesh.colors[i]).chain(&mesh.uv[i]) {binary.extend_from_slice(&v.to_le_bytes());}
    }
    let index_offset=binary.len();
    for i in &mesh.indices {binary.extend_from_slice(&i.to_le_bytes());}
    let document=json!({
        "asset":{"version":"2.0","generator":"Worldcraft","extras":{"worldcraft_origin_metres":[r.center[0],0.0,r.center[1]],"terrain_only":true}},
        "scene":0,"scenes":[{"nodes":[0]}],"nodes":[{"mesh":0,"name":"Edited terrain"}],
        "meshes":[{"primitives":[{"attributes":{"POSITION":0,"NORMAL":1,"COLOR_0":2,"TEXCOORD_0":3},"indices":4,"material":0,"mode":4}]}],
        "materials":[{"name":"Terrain vertex colour","pbrMetallicRoughness":{"baseColorFactor":[1,1,1,1],"metallicFactor":0,"roughnessFactor":0.92}}],
        "buffers":[{"byteLength":binary.len()}],
        "bufferViews":[{"buffer":0,"byteOffset":0,"byteLength":index_offset,"byteStride":48,"target":34962},{"buffer":0,"byteOffset":index_offset,"byteLength":mesh.indices.len()*4,"target":34963}],
        "accessors":[
            {"bufferView":0,"byteOffset":0,"componentType":5126,"count":n,"type":"VEC3","min":low,"max":high},
            {"bufferView":0,"byteOffset":12,"componentType":5126,"count":n,"type":"VEC3"},
            {"bufferView":0,"byteOffset":24,"componentType":5126,"count":n,"type":"VEC4"},
            {"bufferView":0,"byteOffset":40,"componentType":5126,"count":n,"type":"VEC2"},
            {"bufferView":1,"byteOffset":0,"componentType":5125,"count":mesh.indices.len(),"type":"SCALAR"}
        ]
    });
    let mut encoded=serde_json::to_vec(&document).map_err(|e|e.to_string())?;
    while encoded.len()%4!=0{encoded.push(b' ');}while binary.len()%4!=0{binary.push(0);}
    let length=12+8+encoded.len()+8+binary.len();
    let mut output=Vec::with_capacity(length);output.extend_from_slice(b"glTF");output.extend_from_slice(&2u32.to_le_bytes());output.extend_from_slice(&(length as u32).to_le_bytes());
    output.extend_from_slice(&(encoded.len() as u32).to_le_bytes());output.extend_from_slice(b"JSON");output.extend(encoded);
    output.extend_from_slice(&(binary.len() as u32).to_le_bytes());output.extend_from_slice(b"BIN\0");output.extend(binary);Ok(output)
}
pub fn write_glb(path:&Path,g:&Generator,r:Region,progress:impl FnMut(usize)->Result<(),String>)->Result<(),String> {
    let mesh=terrain(g,r,progress)?;atomic(path,&glb(&mesh,r)?)
}
pub fn write_obj(path:&Path,g:&Generator,r:Region,progress:impl FnMut(usize)->Result<(),String>)->Result<(),String> {
    use std::fmt::Write as _;
    let mesh=terrain(g,r,progress)?;let mut text=format!("# Worldcraft terrain; metres; Y up\n# Origin X={} Z={}\n",r.center[0],r.center[1]);
    for p in &mesh.positions {writeln!(text,"v {} {} {}",p[0],p[1],p[2]).unwrap();}
    for uv in &mesh.uv {writeln!(text,"vt {} {}",uv[0],1.0-uv[1]).unwrap();}
    for n in &mesh.normals {writeln!(text,"vn {} {} {}",n[0],n[1],n[2]).unwrap();}
    for [a,b,c] in mesh.indices.as_chunks::<3>().0 {writeln!(text,"f {0}/{0}/{0} {1}/{1}/{1} {2}/{2}/{2}",a+1,b+1,c+1).unwrap();}
    atomic(path,text.as_bytes())
}
pub fn write_heightmap(path:&Path,g:&Generator,r:Region,mut progress:impl FnMut(usize)->Result<(),String>)->Result<(),String> {
    r.validate()?;let row=r.cells+1;let mut heights=Vec::with_capacity(row*row);
    for z in 0..row {progress(z*100/row)?;for x in 0..row {
        let wx=r.center[0]+(x as f64/r.cells as f64-0.5)*r.width;
        let wz=r.center[1]+(z as f64/r.cells as f64-0.5)*r.width;
        heights.push(g.height(wx,wz));
    }}
    let min=heights.iter().copied().fold(f64::INFINITY,f64::min);
    let max=heights.iter().copied().fold(f64::NEG_INFINITY,f64::max);
    let span=(max-min).max(0.001);
    let mut samples=Vec::with_capacity(heights.len()*2);
    for h in heights {samples.extend_from_slice(&(((h-min)/span*65535.0).round() as u16).to_be_bytes());}
    let metadata=json!({"worldcraft_heightmap":1,"origin_x_metres":r.center[0]-r.width*0.5,"origin_z_metres":r.center[1]-r.width*0.5,"spacing_metres":r.width/r.cells as f64,"minimum_metres":min,"maximum_metres":max,"encoding":"height = minimum + unsigned_16bit/65535 * max(maximum-minimum,0.001)","rows":"north to south","columns":"west to east"});
    let mut bytes=Vec::new();
    {let mut encoder=png::Encoder::new(&mut bytes,row as u32,row as u32);encoder.set_color(png::ColorType::Grayscale);encoder.set_depth(png::BitDepth::Sixteen);
    encoder.add_text_chunk("Worldcraft".into(),metadata.to_string()).map_err(|e|e.to_string())?;
    let mut writer=encoder.write_header().map_err(|e|e.to_string())?;writer.write_image_data(&samples).map_err(|e|e.to_string())?;writer.finish().map_err(|e|e.to_string())?;}
    progress(100)?;atomic(path,&bytes)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exported_mesh_uses_saved_edits_and_exact_world_coordinates() {
        let g=Generator::new(crate::GeneratorSettings::default(),vec![crate::Stroke{center:[-128.0,256.0],brush:crate::Brush::default()}]);
        let r=Region{center:[-128.0,256.0],width:128.0,cells:32};
        let mesh=terrain(&g,r,|_|Ok(())).unwrap();let middle=16*33+16;
        assert_eq!(mesh.positions[middle][1],g.height(-128.0,256.0) as f32);
        assert_eq!(mesh.indices.len(),32*32*6);
        let data=glb(&mesh,r).unwrap();assert_eq!(&data[..4],b"glTF");
        assert_eq!(u32::from_le_bytes(data[8..12].try_into().unwrap()) as usize,data.len());
    }
    #[test]
    fn export_cancellation_and_excessive_resolution_are_rejected() {
        let g=Generator::new(crate::GeneratorSettings::default(),vec![]);
        let r=Region{center:[0.0,0.0],width:1024.0,cells:256};
        assert!(terrain(&g,r,|_|Err("cancelled".into())).is_err());
        assert!(Region{cells:8192,..r}.validate().is_err());
    }
}
