import bevy_pbr::render::{
    pbr_fragment::pbr_input_from_standard_material,
    pbr_functions::alpha_discard,
};
@if(VISIBILITY_RANGE_DITHER)
import bevy_pbr::render::pbr_functions::visibility_range_dither;
@if(DISTANCE_FOG)
import bevy_pbr::render::{fog::linear_fog,mesh_view_bindings::fog,pbr_types::STANDARD_MATERIAL_FLAGS_FOG_ENABLED_BIT};
@if(PREPASS_PIPELINE)
import bevy_pbr::{prepass::io::{VertexOutput,FragmentOutput},deferred::functions::deferred_output};
@else
import bevy_pbr::render::{forward_io::{VertexOutput,FragmentOutput},pbr_functions::{apply_pbr_lighting,main_pass_post_lighting_processing}};
@group(constants::MATERIAL_BIND_GROUP) @binding(100) var<uniform> surface_kind:vec4<f32>;
struct SurfaceFrame {origin:vec4<f32>,observer:vec4<f32>,animation:vec4<f32>,previous_observer:vec4<f32>,previous_wind:vec4<f32>};
@group(constants::MATERIAL_BIND_GROUP) @binding(103) var<storage,read> frame:SurfaceFrame;
@group(constants::MATERIAL_BIND_GROUP) @binding(110) var terrain_color:texture_2d_array<f32>;
@group(constants::MATERIAL_BIND_GROUP) @binding(111) var terrain_sampler:sampler;
@group(constants::MATERIAL_BIND_GROUP) @binding(112) var terrain_normal:texture_2d_array<f32>;
@if(SURFACE_TERRAIN || SURFACE_ROCK)
struct SurfaceSample {color:vec3<f32>, bump:vec3<f32>, roughness:f32};
@if(SURFACE_TERRAIN)
fn ground_sample(p:vec3<f32>,n:vec3<f32>,variation:f32,tint:vec3<f32>)->SurfaceSample {
    // Both warp periods divide the floating origin's 4096 m wrap. This breaks
    // short texture repeats without a spatial discontinuity on origin rebases.
    let uv=p.xz/2.0+vec2<f32>(sin(p.z*6.28318530718/128.0),sin(p.x*6.28318530718/256.0))*0.20;
    let floor_color=textureSample(terrain_color,terrain_sampler,uv,0);
    let floor_normal=textureSample(terrain_normal,terrain_sampler,uv,0);
    let grass_color=textureSample(terrain_color,terrain_sampler,uv,2);
    let grass_normal=textureSample(terrain_normal,terrain_sampler,uv,2);
    let woodland=1.0-smoothstep(0.19,0.30,tint.g);
    let floor_weight=smoothstep(0.36,0.74,variation)*mix(0.25,0.80,woodland);
    let color=mix(grass_color,floor_color,floor_weight);
    let normal=mix(grass_normal,floor_normal,floor_weight);
    let bump=vec3<f32>(normal.x*2.0-1.0,0.0,normal.y*2.0-1.0);
    // Preserve scanned luminance/roughness while coupling pigment to the actual
    // biome. The original dry scan otherwise turned every forest floor tan.
    let luma=dot(color.rgb,vec3<f32>(0.2126,0.7152,0.0722));
    let pigment=tint/max(dot(tint,vec3<f32>(0.2126,0.7152,0.0722)),0.02);
    let recolored=mix(color.rgb,luma*pigment,0.92-floor_weight*0.40);
    return SurfaceSample(recolored,bump,normal.a);
}
@if(SURFACE_TERRAIN || SURFACE_ROCK)
fn rock_sample(p:vec3<f32>,n:vec3<f32>,dx:vec3<f32>,dy:vec3<f32>)->SurfaceSample {
    // Triplanar projection avoids stretched cliff UVs. Derivatives select actual
    // texture mips; one shared normal/roughness map bounds texture traffic.
    var w=pow(abs(n),vec3<f32>(4.0));w=w/max(w.x+w.y+w.z,0.001);
    let cx=textureSampleGrad(terrain_color,terrain_sampler,p.zy/4.0,1,dx.zy/4.0,dy.zy/4.0).rgb;
    let cy=textureSampleGrad(terrain_color,terrain_sampler,p.xz/4.0,1,dx.xz/4.0,dy.xz/4.0).rgb;
    let cz=textureSampleGrad(terrain_color,terrain_sampler,p.xy/4.0,1,dx.xy/4.0,dy.xy/4.0).rgb;
    let nx=textureSampleGrad(terrain_normal,terrain_sampler,p.zy/4.0,1,dx.zy/4.0,dy.zy/4.0);
    let ny=textureSampleGrad(terrain_normal,terrain_sampler,p.xz/4.0,1,dx.xz/4.0,dy.xz/4.0);
    let nz=textureSampleGrad(terrain_normal,terrain_sampler,p.xy/4.0,1,dx.xy/4.0,dy.xy/4.0);
    let bx=vec3<f32>(0.0,nx.y*2.0-1.0,nx.x*2.0-1.0);
    let by=vec3<f32>(ny.x*2.0-1.0,0.0,ny.y*2.0-1.0);
    let bz=vec3<f32>(nz.x*2.0-1.0,nz.y*2.0-1.0,0.0);
    return SurfaceSample(cx*w.x+cy*w.y+cz*w.z,bx*w.x+by*w.y+bz*w.z,nx.a*w.x+ny.a*w.y+nz.a*w.z);
}

@if(SURFACE_TERRAIN)
fn hash_cell(cell:vec3<f32>,period:f32)->f32{
    let p=cell-floor(cell/period)*period;
    return fract(sin(dot(p,vec3<f32>(127.1,311.7,74.7)))*43758.5453);
}
// Periodic volume noise plus analytic world-space derivatives. This avoids UV
// stretching on cliffs and keeps the normal detail independent of chunk LOD.
@if(SURFACE_TERRAIN)
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
    let detail=vec4<f32>(surface_kind.x,frame.animation.x,frame.animation.z,frame.animation.y);
    let anchor=frame.origin;
    @if(VISIBILITY_RANGE_DITHER)
    visibility_range_dither(in.position,in.visibility_range_dither);
    var pbr=pbr_input_from_standard_material(in,is_front);
    @if(SURFACE_TERRAIN || SURFACE_WATER || SURFACE_ROCK)
    if detail.z>0.5 {
        let p=pbr.world_position.xyz+anchor.xyz;
        @if(SURFACE_TERRAIN) {
            let dx=dpdx(p);let dy=dpdy(p);
            let p0=pbr.material.base_color.rgb;
            let slope=smoothstep(0.30,0.72,1.0-pbr.N.y);
            let maximum=max(max(p0.r,p0.g),p0.b);
            let minimum=min(min(p0.r,p0.g),p0.b);
            let saturation=(maximum-minimum)/max(maximum,0.001);
            let green=smoothstep(0.85,1.25,p0.g/max(p0.r,0.001))
                *(1.0-smoothstep(0.38,0.52,p0.r))*smoothstep(0.22,0.45,saturation);
            let forest=green*(1.0-slope);
            let macro_variation=noise_gradient(p,0.0625).x;
            let ground=ground_sample(p,pbr.N,macro_variation,p0);
            // The vertex palette preserves painted biomes; bright snow and warm
            // sand keep their procedural appearance. Rock follows actual slope.
            var color=mix(p0,ground.color*(0.78+p0.g*0.8),forest*0.86);
            var bump=ground.bump*forest;
            var rough=mix(0.94,ground.roughness,forest);
            if slope>0.01 {
                let stone=rock_sample(p,pbr.N,dx,dy);
                color=mix(color,stone.color,slope);
                bump=mix(bump,stone.bump,slope);
                rough=mix(rough,stone.roughness,slope);
            }
            pbr.material.base_color=vec4<f32>(color*mix(0.88,1.10,macro_variation),pbr.material.base_color.a);
            let tangent=bump-pbr.N*dot(bump,pbr.N);
            pbr.N=normalize(pbr.N+tangent*0.65);
            pbr.material.perceptual_roughness=clamp(rough,0.45,0.99);
        }
        @if(SURFACE_WATER) {
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
        @if(SURFACE_ROCK) {
            let stone=rock_sample(p,pbr.N,dpdx(p),dpdy(p));
            pbr.material.base_color=vec4<f32>(stone.color,1.0);
            let tangent=stone.bump-pbr.N*dot(stone.bump,pbr.N);
            pbr.N=normalize(pbr.N+tangent*0.65);
            pbr.material.perceptual_roughness=clamp(stone.roughness,0.45,0.99);
        }
    }
    pbr.material.base_color=alpha_discard(pbr.material.flags,pbr.material.alpha_cutoff,pbr.material.base_color);
    var out:FragmentOutput;
    @if(PREPASS_PIPELINE) {out=deferred_output(in,pbr);} @else {
        out.color=apply_pbr_lighting(pbr);
        @if(DISTANCE_FOG) {
            if frame.animation.w>0.5 {
                // Eco's boundary follows resident tiles, not camera altitude.
                // Reuse Bevy's fog color and formula, then skip its radial fog.
                let distance=length(pbr.world_position.xz-frame.observer.xz);
                out.color=linear_fog(fog,out.color,distance,vec3<f32>(0.0));
                pbr.material.flags=pbr.material.flags & ~STANDARD_MATERIAL_FLAGS_FOG_ENABLED_BIT;
            }
        }
        out.color=main_pass_post_lighting_processing(pbr,out.color);
    }
    return out;
}
