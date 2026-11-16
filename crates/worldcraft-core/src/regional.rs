//! Persisted learned regional relief. Exploring and reopening need no model.
//! This is the coarse-model hybrid, not the full latent/decoder cascade.
use serde::{Deserialize,Serialize};
#[derive(Clone,Debug,Serialize,Deserialize,PartialEq)]
#[serde(deny_unknown_fields)]
pub struct LearnedRegion {
    pub recipe: String,
    pub model_revision: String,
    pub model_sha256: String,
    pub side: usize,
    pub origin: [f64;2],
    pub spacing: f64,
    pub blend: f64,
    pub height: Vec<f32>,
    pub temperature: Vec<f32>,
    pub precipitation: Vec<f32>,
}
impl LearnedRegion {
    pub fn validate(&self)->Result<(),String> {
        if self.recipe!="terrain-diffusion-coarse-hybrid-v1" || self.model_revision.len()!=40
            || !self.model_revision.bytes().all(|c|c.is_ascii_hexdigit())
            || self.model_sha256.len()!=64 || !self.model_sha256.bytes().all(|c|c.is_ascii_hexdigit()) {
            return Err("Unsupported learned-region recipe or model identity".into());
        }
        if !(16..=512).contains(&self.side) || !self.spacing.is_finite() || !(64.0..=8192.0).contains(&self.spacing)
            || !self.origin.iter().all(|c|c.is_finite() && c.abs()<1e8)
            || !self.blend.is_finite() || self.blend<self.spacing || self.blend>self.extent()*0.5 {
            return Err("Invalid learned-region grid or transition band".into());
        }
        let n=self.side*self.side;
        if self.height.len()!=n || self.temperature.len()!=n || self.precipitation.len()!=n
            || !self.height.iter().all(|h|h.is_finite()&&h.abs()<=20000.0)
            || !self.temperature.iter().all(|h|h.is_finite()&&(-100.0..=80.0).contains(h))
            || !self.precipitation.iter().all(|h|h.is_finite()&&(0.0..=30000.0).contains(h)) {
            return Err("Invalid learned-region samples".into());
        }
        Ok(())
    }
    pub fn extent(&self)->f64 {(self.side-1) as f64*self.spacing}
    /// Height in metres, temperature in Celsius, precipitation in mm/year, blend weight.
    pub fn sample(&self,x:f64,z:f64)->Option<[f64;4]> {
        let u=(x-self.origin[0])/self.spacing;let v=(z-self.origin[1])/self.spacing;
        let limit=(self.side-1) as f64;
        if !u.is_finite()||!v.is_finite()||u<=0.0||v<=0.0||u>=limit||v>=limit {return None;}
        let ix=(u.floor() as usize).min(self.side-2);let iz=(v.floor() as usize).min(self.side-2);
        let (tx,tz)=(u-ix as f64,v-iz as f64);let a=iz*self.side+ix;
        let read=|data:&[f32]| {
            let top=f64::from(data[a])*(1.0-tx)+f64::from(data[a+1])*tx;
            let bottom=f64::from(data[a+self.side])*(1.0-tx)+f64::from(data[a+self.side+1])*tx;
            top*(1.0-tz)+bottom*tz
        };
        let d=u.min(v).min(limit-u).min(limit-v)*self.spacing;
        let t=(d/self.blend).clamp(0.0,1.0);
        Some([read(&self.height),read(&self.temperature),read(&self.precipitation),t*t*(3.0-2.0*t)])
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn patch()->LearnedRegion {LearnedRegion {recipe:"terrain-diffusion-coarse-hybrid-v1".into(),model_revision:"a".repeat(40),model_sha256:"b".repeat(64),side:16,origin:[-1000.0,-1000.0],spacing:128.0,blend:256.0,height:vec![300.0;256],temperature:vec![20.0;256],precipitation:vec![1200.0;256]}}
    #[test]
    fn saved_regional_relief_is_continuous_and_models_are_not_needed_to_reopen() {
        let region=patch();region.validate().unwrap();
        let mut world=crate::WorldFile::new("Learned".into(),crate::GeneratorSettings::default());
        world.learned=Some(region.clone());
        let decoded:crate::WorldFile=serde_json::from_str(&serde_json::to_string(&world).unwrap()).unwrap();
        let g=crate::Generator::for_world(&decoded).unwrap();
        let before=g.height(0.0,0.0);
        assert!((before-300.0).abs()<20.0);
        let mut stroke=crate::Stroke {center:[0.0,0.0],brush:crate::Brush::default()};
        stroke.brush.strength=5.0;
        let edited=g.with_edits(vec![stroke]).unwrap();
        assert!((edited.height(0.0,0.0)-before-5.0).abs()<1e-6);
        let plain=crate::Generator::new(world.settings.clone(),vec![]);
        let x=region.origin[0];
        assert_eq!(g.height(x,-500.0),plain.height(x,-500.0));
        assert!((g.height(x+0.001,-500.0)-g.height(x-0.001,-500.0)).abs()<0.01);
        assert_eq!(g.height(0.0,0.0),crate::Generator::for_world(&decoded).unwrap().height(0.0,0.0));
        assert_eq!(g.preview_sample(0.0,0.0).height,before);
    }
    #[test]
    fn malformed_grids_and_nonfinite_samples_are_rejected() {
        let mut p=patch();p.height.pop();assert!(p.validate().is_err());
        let mut p=patch();p.temperature[0]=f32::NAN;assert!(p.validate().is_err());
        let mut p=patch();p.side=usize::MAX;assert!(p.validate().is_err());
    }
}
