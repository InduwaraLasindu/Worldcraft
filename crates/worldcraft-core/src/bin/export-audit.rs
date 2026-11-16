use std::{io::BufReader,path::Path};
use worldcraft_core::*;
fn main()->Result<(),String>{
    let g=Generator::new(GeneratorSettings::for_preset(Preset::Alpine,20261116),vec![Stroke{center:[-128.0,256.0],brush:Brush::default()}]);
    let r=export::Region{center:[-128.0,256.0],width:512.0,cells:64};
    export::write_glb(Path::new("terrain-audit.glb"),&g,r,|_|Ok(()))?;
    export::write_obj(Path::new("terrain-audit.obj"),&g,r,|_|Ok(()))?;
    export::write_heightmap(Path::new("terrain-audit.png"),&g,r,|_|Ok(()))?;
    let decoder=png::Decoder::new(BufReader::new(std::fs::File::open("terrain-audit.png").map_err(|e|e.to_string())?));
    let mut reader=decoder.read_info().map_err(|e|e.to_string())?;
    assert_eq!(reader.info().bit_depth,png::BitDepth::Sixteen);
    let meta:serde_json::Value=serde_json::from_str(&reader.info().uncompressed_latin1_text.iter().find(|t|t.keyword=="Worldcraft").unwrap().text).unwrap();
    let mut data=vec![0;reader.output_buffer_size().unwrap()];
    let info=reader.next_frame(&mut data).map_err(|e|e.to_string())?;
    assert_eq!((info.width,info.height),(65,65));
    let min=meta["minimum_metres"].as_f64().unwrap();let max=meta["maximum_metres"].as_f64().unwrap();
    for z in 0..65 {for x in 0..65 {
        let index=(z*65+x)*2;let value=u16::from_be_bytes([data[index],data[index+1]]) as f64;
        let h=min+value/65535.0*(max-min).max(0.001);
        let actual=g.height(-384.0+x as f64*8.0,z as f64*8.0);
        assert!((actual-h).abs()<=(max-min).max(0.001)/65535.0*0.501+1e-8);
    }}
    println!("Terrain GLB/OBJ and calibrated 16-bit PNG passed; {} vertices; PNG error within half one quantization step",65*65);
    Ok(())
}
