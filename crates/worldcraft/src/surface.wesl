import bevy_pbr::render::{
    pbr_fragment::pbr_input_from_standard_material,
    pbr_functions::alpha_discard,
};
@if(PREPASS_PIPELINE)
import bevy_pbr::{prepass::io::{VertexOutput,FragmentOutput},deferred::functions::deferred_output};
@else
import bevy_pbr::render::{forward_io::{VertexOutput,FragmentOutput},pbr_functions::{apply_pbr_lighting,main_pass_post_lighting_processing}};
@group(constants::MATERIAL_BIND_GROUP) @binding(100) var<uniform> detail:vec4<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(101) var<uniform> anchor:vec4<f32>;

fn hash_cell(cell:vec3<f32>,period:f32)->f32{
    let p=cell-floor(cell/period)*period;
    return fract(sin(dot(p,vec3<f32>(127.1,311.7,74.7)))*43758.5453);
}
// Periodic volume noise plus analytic world-space derivatives. This avoids UV
// stretching on cliffs and keeps the normal detail independent of chunk LOD.
fn noise_gradient(position:vec3<f32>,scale:f32)->vec4<f32>{
    let p=position*scale;
    let cell=floor(p);let f=fract(p);
    let u=f*f*(vec3<f32>(3.0)-2.0*f);
    let du=6.0*f*(vec3<f32>(1.0)-f);
    let period=4096.0*scale;
    let a=hash_cell(cell,period);
    let b=hash_cell(cell+vec3<f32>(1.0,0.0,0.0),period);
    let c=hash_cell(cell+vec3<f32>(0.0,1.0,0.0),period);
    let d=hash_cell(cell+vec3<f32>(1.0,1.0,0.0),period);
    let e=hash_cell(cell+vec3<f32>(0.0,0.0,1.0),period);
    let f1=hash_cell(cell+vec3<f32>(1.0,0.0,1.0),period);
    let g=hash_cell(cell+vec3<f32>(0.0,1.0,1.0),period);
    let h=hash_cell(cell+vec3<f32>(1.0,1.0,1.0),period);
    let low=mix(mix(a,b,u.x),mix(c,d,u.x),u.y);
    let high=mix(mix(e,f1,u.x),mix(g,h,u.x),u.y);
    let dx=mix(mix(b-a,d-c,u.y),mix(f1-e,h-g,u.y),u.z)*du.x*scale;
    let dy=mix(mix(c-a,d-b,u.x),mix(g-e,h-f1,u.x),u.z)*du.y*scale;
    let dz=(high-low)*du.z*scale;
    return vec4<f32>(mix(low,high,u.z),dx,dy,dz);
}
@fragment
fn fragment(in:VertexOutput,@builtin(front_facing) is_front:bool)->FragmentOutput{
    var pbr=pbr_input_from_standard_material(in,is_front);
    if detail.z>0.5 {
        let p=pbr.world_position.xyz+anchor.xyz;
        if detail.x<0.5 {
            let footprint=max(length(dpdx(p)),length(dpdy(p)));
            let coarse_filter=1.0-smoothstep(1.0,8.0,footprint);
            let grain_filter=1.0-smoothstep(0.08,0.5,footprint);
            var coarse=vec4<f32>(0.5,0.0,0.0,0.0);
            var grain=vec4<f32>(0.5,0.0,0.0,0.0);
            if coarse_filter>0.0 {coarse=noise_gradient(p,0.25);}
            if grain_filter>0.0 {grain=noise_gradient(p,4.0);}
            let variation=mix(1.0,mix(0.78,1.16,coarse.x),coarse_filter)*mix(1.0,mix(0.92,1.07,grain.x),grain_filter);
            pbr.material.base_color=vec4<f32>(pbr.material.base_color.rgb*variation,pbr.material.base_color.a);
            let gradient=coarse.yzw*0.50*coarse_filter+grain.yzw*0.025*grain_filter;
            let tangent_gradient=gradient-pbr.N*dot(gradient,pbr.N);
            pbr.N=normalize(pbr.N-tangent_gradient);
            pbr.material.perceptual_roughness=clamp(0.86+grain.x*0.12,0.8,0.99);
        } else {
            let tau=6.28318530718;
            let q=dot(p.xz,vec2<f32>(1.0,2.0))*tau/64.0+detail.y*0.7;
            let r=dot(p.xz,vec2<f32>(2.0,-1.0))*tau/16.0-detail.y*1.1;
            let filter_q=1.0-smoothstep(0.3,1.5,fwidth(q));
            let filter_r=1.0-smoothstep(0.3,1.5,fwidth(r));
            let wave=vec3<f32>(0.09*cos(q)*filter_q+0.06*cos(r)*filter_r,0.0,0.18*cos(q)*filter_q-0.03*cos(r)*filter_r);
            pbr.N=normalize(pbr.N-wave);
            let fresnel=pow(1.0-clamp(dot(pbr.N,pbr.V),0.0,1.0),5.0);
            pbr.material.base_color.a=clamp(0.62+fresnel*0.30,0.0,1.0);
            pbr.material.perceptual_roughness=mix(0.35,0.18,filter_r);
        }
    }
    pbr.material.base_color=alpha_discard(pbr.material.flags,pbr.material.alpha_cutoff,pbr.material.base_color);
    var out:FragmentOutput;
    @if(PREPASS_PIPELINE) {out=deferred_output(in,pbr);} @else {
        out.color=apply_pbr_lighting(pbr);
        out.color=main_pass_post_lighting_processing(pbr,out.color);
    }
    return out;
}
