// SPDX-License-Identifier: Apache-2.0
// Copyright 2024 TSAIL Team and The HuggingFace Team.
// Rust adaptation of the EDM DPM-Solver++ midpoint scheduler used by
// xandergos/terrain-diffusion-mc. See THIRD_PARTY_NOTICES.md.
pub struct Scheduler {pub sigmas:Vec<f32>,previous:Option<Vec<f32>>,index:usize}
impl Scheduler {
    pub fn new(steps:usize)->Self {
        assert!(steps>=2);
        let lo=0.002_f32.powf(1.0/7.0);let hi=80_f32.powf(1.0/7.0);
        let mut sigmas:Vec<f32>=(0..steps).map(|i|(hi+i as f32/(steps-1) as f32*(lo-hi)).powi(7)).collect();
        sigmas.push(0.0);Self {sigmas,previous:None,index:0}
    }
    pub fn step(&mut self,model:&[f32],sample:&[f32])->Vec<f32> {
        let s=self.sigmas[self.index];let t=self.sigmas[self.index+1];
        let skip=0.25/(s*s+0.25);let out=s*0.5/(s*s+0.25).sqrt();
        let denoised:Vec<f32>=sample.iter().zip(model).map(|(x,m)|skip*x+out*m).collect();
        let ratio=t/s;
        let result=if self.index>0 && t>0.0 {
            let prev=self.previous.as_ref().unwrap();
            let h=-(t as f64).ln()+(s as f64).ln();
            let h0=-(s as f64).ln()+(self.sigmas[self.index-1] as f64).ln();
            let r=(h0/h) as f32;
            sample.iter().zip(&denoised).zip(prev).map(|((x,d),p)|
                ratio*x-(ratio-1.0)*d-0.5*(ratio-1.0)*(d-p)/r).collect()
        } else {sample.iter().zip(&denoised).map(|(x,d)|ratio*x-(ratio-1.0)*d).collect()};
        self.previous=Some(denoised);self.index+=1;result
    }
}
