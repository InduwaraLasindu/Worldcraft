//! Bounded numeric helpers for a finite Terrain Diffusion bake.
use crate::{normal_noise,tile_seed};
pub fn noise_patch(seed:u64,y0:i32,x0:i32,height:usize,width:usize,channels:usize,tile:usize)->Vec<f32>{
    assert!(height<=512&&width<=512&&channels<=6&&tile>0);
    let mut out=vec![0.0;channels*height*width];
    let t=tile as i32;
    for ty in y0.div_euclid(t)..=(y0+height as i32-1).div_euclid(t){
        for tx in x0.div_euclid(t)..=(x0+width as i32-1).div_euclid(t){
            let data=normal_noise(tile_seed(seed,ty,tx),channels*tile*tile);
            let ya=y0.max(ty*t);let yb=(y0+height as i32).min((ty+1)*t);
            let xa=x0.max(tx*t);let xb=(x0+width as i32).min((tx+1)*t);
            for c in 0..channels {for y in ya..yb {for x in xa..xb {
                out[(c*height+(y-y0) as usize)*width+(x-x0) as usize]=data[(c*tile+(y-ty*t) as usize)*tile+(x-tx*t) as usize];
            }}}
        }
    }out
}
fn axis_weights(from:usize,to:usize,extrapolate:bool,antialias:bool)->Vec<Vec<(usize,f32)>>{
    let scale=from as f64/to as f64;
    (0..to).map(|i|{
        let mut at=(i as f64+0.5)*scale-0.5;
        if antialias&&scale>1.0 {
            let mut weights=Vec::new();let mut total=0.0_f64;
            let start=((at-scale).ceil() as isize).max(0);
            let end=((at+scale).floor() as isize).min(from as isize-1);
            for j in start..=end {let weight=(1.0-(j as f64-at).abs()/scale).max(0.0);total+=weight;weights.push((j as usize,weight));}
            weights.into_iter().map(|(j,w)|(j,(w/total) as f32)).collect()
        } else if from==1 {vec![(0,1.0)]} else {
            if !extrapolate {at=at.clamp(0.0,(from-1) as f64);}
            let a=(at.floor() as isize).clamp(0,from as isize-2) as usize;
            let t=(at-a as f64) as f32;vec![(a,1.0-t),(a+1,t)]
        }
    }).collect()
}
pub fn resize_square(source:&[f32],from:usize,to:usize,extrapolate:bool,antialias:bool)->Vec<f32>{
    assert_eq!(source.len(),from*from);assert!(from>0&&to>0&&from<=512&&to<=512);
    let weights=axis_weights(from,to,extrapolate,antialias);
    let mut horizontal=vec![0.0;from*to];
    for y in 0..from {for x in 0..to {horizontal[y*to+x]=weights[x].iter().map(|&(j,w)|source[y*from+j]*w).sum();}}
    let mut output=vec![0.0;to*to];
    for y in 0..to {for x in 0..to {output[y*to+x]=weights[y].iter().map(|&(j,w)|horizontal[j*to+x]*w).sum();}}
    output
}
fn reflect(mut i:isize,n:usize)->usize {
    while i<0||i>=n as isize {if i<0{i=-i;}else{i=2*n as isize-2-i;}}i as usize
}
fn blur(source:&[f32],side:usize)->Vec<f32>{
    let mut kernel:Vec<f32>=(-5..=5).map(|i|(-0.5*(i*i) as f32/25.0).exp()).collect();
    let sum: f32=kernel.iter().sum();for w in &mut kernel{*w/=sum;}
    let mut tmp=vec![0.0;source.len()];let mut out=tmp.clone();
    for y in 0..side {for x in 0..side {
        tmp[y*side+x]=kernel.iter().enumerate().map(|(k,w)|source[y*side+reflect(x as isize+k as isize-5,side)]*w).sum();
    }}
    for y in 0..side {for x in 0..side {
        out[y*side+x]=kernel.iter().enumerate().map(|(k,w)|tmp[reflect(y as isize+k as isize-5,side)*side+x]*w).sum();
    }}out
}
pub fn reconstruct(residual:&[f32],low:&[f32],low_side:usize)->Vec<f32>{
    let side=low_side*8;assert_eq!(residual.len(),side*side);assert!(low_side>=16);
    let mut decoded=resize_square(low,low_side,side,true,false);
    for (v,r) in decoded.iter_mut().zip(residual){*v+=r;}
    let down=resize_square(&decoded,side,low_side,false,true);
    let filtered=blur(&down,low_side);
    let up=resize_square(&filtered,low_side,side,false,false);
    up.iter().zip(residual).map(|(l,r)|{let root=l+r;root.signum()*root*root}).collect()
}
pub fn blend_weight(x:usize,y:usize,side:usize)->f32 {
    let axis=|i:usize|(i+1).min(side-i) as f32/(side as f32*0.5);
    axis(x)*axis(y)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn overlapping_negative_noise_requests_return_identical_samples(){
        let a=noise_patch(u64::MAX,-35,-67,64,64,5,64);
        let b=noise_patch(u64::MAX,-19,-43,32,32,5,64);
        for c in 0..5 {for y in 0..32 {for x in 0..32 {
            assert_eq!(a[(c*64+y+16)*64+x+24],b[(c*32+y)*32+x]);
        }}}
    }
    #[test]
    fn resize_extrapolation_preserves_an_affine_heightfield(){
        let src:Vec<f32>=(0..16).flat_map(|y|(0..16).map(move|x|5.0+2.0*x as f32+3.0*y as f32)).collect();
        let out=resize_square(&src,16,128,true,false);
        for y in 0..128 {for x in 0..128 {
            let expected=5.0+2.0*((x as f32+0.5)/8.0-0.5)+3.0*((y as f32+0.5)/8.0-0.5);
            assert!((out[y*128+x]-expected).abs()<0.0001);
        }}
    }
    #[test]
    fn laplacian_reconstruction_keeps_a_constant_and_anti_aliases_detail(){
        let low=vec![10.0;56*56];let result=reconstruct(&vec![0.0;448*448],&low,56);
        assert!(result.iter().all(|v|(*v-100.0).abs()<0.001));
        let checker:Vec<f32>=(0..128).flat_map(|y|(0..128).map(move|x|if(x+y)%2==0{1.0}else{-1.0})).collect();
        let down=resize_square(&checker,128,16,false,true);
        assert!(down[2*16+2..14*16].iter().all(|v|v.abs()<0.01));
    }
    #[test]
    fn decoder_overlap_weights_cover_every_cropped_pixel(){
        let mut weights=vec![0.0_f32;448*448];
        for oy in [0,192] {for ox in [0,192] {for y in 0..256 {for x in 0..256 {
            weights[(oy+y)*448+ox+x]+=blend_weight(x,y,256);
        }}}}
        assert!(weights.iter().all(|v|v.is_finite()&&*v>0.0));
    }
}
