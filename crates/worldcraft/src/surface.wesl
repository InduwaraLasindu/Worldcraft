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
@group(constants::MATERIAL_BIND_GROUP) @binding(113) var terrain_height:texture_2d_array<f32>;
@if(SURFACE_TERRAIN || SURFACE_ROCK)
struct SurfaceSample {color:vec3<f32>, bump:vec3<f32>, roughness:f32};
@if(SURFACE_TERRAIN || SURFACE_ROCK)
struct Projection {color:vec3<f32>, slopes:vec2<f32>, roughness:f32};
@if(SURFACE_TERRAIN || SURFACE_ROCK)
struct ReliefView {slide:vec3<f32>, cosine:f32, near:f32};

// Intersect a downward ray with the sampled height field. Refine the *first*
// crossing, avoiding offsets into a deeper overlapping ledge. All reads in
// divergent control flow use an explicit footprint.
@if(SURFACE_TERRAIN || SURFACE_ROCK)
fn intersection(uv:vec2<f32>, travel:vec2<f32>, layer:i32, lod:f32, count:i32)->vec2<f32> {
    let stride=1.0/f32(count);
    var upper=0.0;
    var lower=0.0;
    for (var i=0; i<count; i++) {
        lower=f32(i+1)*stride;
        let depth=1.0-textureSampleLevel(terrain_height,terrain_sampler,uv-travel*lower,layer,lod).r;
        if lower>=depth {break;}
        upper=lower;
    }
    for (var i=0; i<3; i++) {
        let mid=0.5*(upper+lower);
        let depth=1.0-textureSampleLevel(terrain_height,terrain_sampler,uv-travel*mid,layer,lod).r;
        if mid<depth {upper=mid;} else {lower=mid;}
    }
    return uv-travel*0.5*(upper+lower);
}
@if(SURFACE_TERRAIN || SURFACE_ROCK)
fn relief_view(n:vec3<f32>,v:vec3<f32>,dist:f32)->ReliefView {
    let cosine=dot(n,v);
    return ReliefView((v-n*cosine)/max(cosine,0.20),cosine,1.0-smoothstep(16.0,42.0,dist));
}
@if(SURFACE_TERRAIN || SURFACE_ROCK)
fn projected(base:vec2<f32>,dx:vec2<f32>,dy:vec2<f32>,layer:i32,slide:vec2<f32>,rv:ReliefView,period:f32,amplitude:f32)->Projection {
    let span=max(max(length(dx),length(dy)),0.000001);
    let near=rv.near*(1.0-smoothstep(0.012,0.045,span*period));
    let lod=log2(max(1.0,span*surface_kind.y));
    var uv=base;
    if surface_kind.y>0.0 && near>0.001 && frame.animation.z>1.0 && rv.cosine<0.995 {
        let weight=near*smoothstep(0.04,0.22,rv.cosine);
        let count=i32(round(mix(6.0,frame.animation.z,1.0-clamp(rv.cosine,0.0,1.0))));
        if weight>0.001 {
            uv=intersection(base,slide*(amplitude/period)*weight,layer,lod,count);
        }
    }
    let pigment=textureSampleGrad(terrain_color,terrain_sampler,uv,layer,dx,dy);
    let macro_n=textureSampleGrad(terrain_normal,terrain_sampler,uv,layer,dx,dy);
    var slope=macro_n.xy*2.0-1.0;
    var rough=macro_n.a;
    if surface_kind.y>0.0 {
        rough=pigment.a;
        if near>0.001 {
            let e=max(1.0/surface_kind.y,span*0.72);
            let du=vec2<f32>(e,0.0);
            let dv=vec2<f32>(0.0,e);
            let a=textureSampleLevel(terrain_height,terrain_sampler,uv-du,layer,lod).r;
            let b=textureSampleLevel(terrain_height,terrain_sampler,uv+du,layer,lod).r;
            let c=textureSampleLevel(terrain_height,terrain_sampler,uv-dv,layer,lod).r;
            let d=textureSampleLevel(terrain_height,terrain_sampler,uv+dv,layer,lod).r;
            let grad=vec2<f32>(a-b,c-d)*(amplitude/(2.0*e*period));
            // Bound single-texel specular spikes while retaining oriented facets.
            slope=mix(slope,grad*inverseSqrt(1.0+0.36*dot(grad,grad)),near);
        }
    }
    return Projection(pigment.rgb,slope,rough);
}
@if(SURFACE_TERRAIN)
fn ground_sample(p:vec3<f32>,dx:vec3<f32>,dy:vec3<f32>,rv:ReliefView,variation:f32,tint:vec3<f32>,cover:f32)->SurfaceSample {
    let rate=vec2<f32>(6.28318530718/128.0,6.28318530718/256.0);
    let warp=sin(p.zx*rate)*0.20;
    let slope=cos(p.zx*rate)*rate*0.20;
    let duff=projected(vec2<f32>(-p.z,p.x)*0.25-warp,
        vec2<f32>(-dx.z,dx.x)*0.25-slope*dx.zx,vec2<f32>(-dy.z,dy.x)*0.25-slope*dy.zx,
        0,vec2<f32>(-rv.slide.z,rv.slide.x)-4.0*slope*rv.slide.zx,rv,4.0,0.056);
    let grass=projected(p.xz*0.5+warp,dx.xz*0.5+slope*dx.zx,dy.xz*0.5+slope*dy.zx,
        2,rv.slide.xz+2.0*slope*rv.slide.zx,rv,2.0,0.025);
    let litter=smoothstep(0.20,0.80,variation)*mix(0.20,0.94,smoothstep(0.03,0.45,cover));
    let luma=vec3<f32>(0.2126,0.7152,0.0722);
    let pigment=tint/max(dot(tint,luma),0.02);
    let turf=mix(grass.color,dot(grass.color,luma)*pigment,0.91);
    let leaves=mix(duff.color,dot(duff.color,luma)*pigment,0.28);
    let bump=mix(grass.slopes,vec2<f32>(duff.slopes.y,-duff.slopes.x),litter);
    return SurfaceSample(mix(turf,leaves,litter),vec3<f32>(bump.x,0.0,bump.y),mix(grass.roughness,duff.roughness,litter));
}
@if(SURFACE_TERRAIN || SURFACE_ROCK)
fn rock_sample(p:vec3<f32>,n:vec3<f32>,dx:vec3<f32>,dy:vec3<f32>,rv:ReliefView)->SurfaceSample {
    var w=max(pow(abs(n),vec3<f32>(4.0))-vec3<f32>(0.006),vec3<f32>(0.0));
    w=w/(w.x+w.y+w.z);
    var stone=SurfaceSample(vec3<f32>(0.0),vec3<f32>(0.0),0.0);
    if w.x>0.0 {
        let x=projected(p.zy/4.0,dx.zy/4.0,dy.zy/4.0,1,rv.slide.zy,rv,4.0,0.096);
        stone.color+=w.x*x.color;stone.roughness+=w.x*x.roughness;
        stone.bump+=w.x*vec3<f32>(0.0,x.slopes.y,x.slopes.x);
    }
    if w.y>0.0 {
        let y=projected(p.xz/4.0,dx.xz/4.0,dy.xz/4.0,1,rv.slide.xz,rv,4.0,0.096);
        stone.color+=w.y*y.color;stone.roughness+=w.y*y.roughness;
        stone.bump+=w.y*vec3<f32>(y.slopes.x,0.0,y.slopes.y);
    }
    if w.z>0.0 {
        let z=projected(p.xy/4.0,dx.xy/4.0,dy.xy/4.0,1,rv.slide.xy,rv,4.0,0.096);
        stone.color+=w.z*z.color;stone.roughness+=w.z*z.roughness;
        stone.bump+=w.z*vec3<f32>(z.slopes.x,z.slopes.y,0.0);
    }
    return stone;
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
@if(SURFACE_WATER)
fn sea_hash(cell:vec2<i32>,mask:u32)->f32 {
    let wrapped=vec2<u32>(cell)&vec2<u32>(mask);
    var h=(wrapped.x*0x9e3779b1u)^(wrapped.y*0x85ebca77u);
    h=(h^(h>>16u))*0x7feb352du;
    return f32((h^(h>>15u))>>8u)*(2.0/16777216.0)-1.0;
}
@if(SURFACE_WATER)
fn sea_gradient(q:vec2<f32>,period:u32)->vec2<f32> {
    let c=vec2<i32>(floor(q));let f=fract(q);let u=f*f*(3.0-2.0*f);
    let mask=period-1u;
    let a=sea_hash(c,mask);let b=sea_hash(c+vec2<i32>(1,0),mask);
    let d=sea_hash(c+vec2<i32>(0,1),mask);let e=sea_hash(c+vec2<i32>(1,1),mask);
    return 6.0*f*(1.0-f)*vec2<f32>(mix(b-a,e-d,u.y),mix(d-a,e-b,u.x));
}
@fragment
fn fragment(in:VertexOutput,@builtin(front_facing) is_front:bool)->FragmentOutput{
    let detail=vec4<f32>(surface_kind.x,frame.animation.x,frame.animation.z,frame.animation.y);
    let anchor=frame.origin;
    @if(VISIBILITY_RANGE_DITHER)
    visibility_range_dither(in.position,in.visibility_range_dither);
    var pbr=pbr_input_from_standard_material(in,is_front);
    @if(SURFACE_GROUND && VERTEX_UVS_A) {
        if in.uv.x<0.8 {
            // A card represents a fan of round blades. Its plane normal makes
            // an entire tussock black when viewed across the sun. Use a broad
            // upward lobe, retaining the side normal and all cast/received shadows.
            pbr.N=normalize(vec3<f32>(pbr.N.x*0.36,1.0,pbr.N.z*0.36));
        }
    }
    @if(SURFACE_TERRAIN || SURFACE_WATER || SURFACE_ROCK)
    if detail.z>0.5 {
        let p=pbr.world_position.xyz+vec3<f32>(anchor.x,0.0,anchor.z);
        @if(SURFACE_TERRAIN) {
            let dx=dpdx(p);let dy=dpdy(p);
            let p0=pbr.material.base_color.rgb;
            let slope=smoothstep(0.30,0.72,1.0-pbr.N.y);
            let maximum=max(max(p0.r,p0.g),p0.b);
            let minimum=min(min(p0.r,p0.g),p0.b);
            let saturation=(maximum-minimum)/max(maximum,0.001);
            let turf=smoothstep(0.62,0.85,p0.g/max(p0.r,0.001))
                *(1.0-smoothstep(0.35,0.50,p0.r))*smoothstep(0.16,0.42,saturation)*(1.0-slope);
            let stand_noise=noise_gradient(p,0.0625);
            let macro_variation=stand_noise.x;
            var cover=0.0;
            @if(VERTEX_UVS_A && HABITAT_TERRAIN)
            cover=clamp(in.uv.x,0.0,1.0);
            let rv=relief_view(pbr.N,pbr.V,distance(pbr.world_position.xyz,frame.observer.xyz));
            var ground=SurfaceSample(p0,vec3<f32>(0.0),0.94);
            if turf>0.001 {ground=ground_sample(p,dx,dy,rv,macro_variation,p0,cover);}
            var color=mix(p0,ground.color*(0.86+p0.g*0.60),turf*0.95);
            var bump=ground.bump*turf;
            var rough=mix(0.94,ground.roughness,turf);
            var bedrock=slope;
            var settled_snow=0.0;
            var strata=0.0;
            var fracture=vec3<f32>(0.0);
            var snow_bump=vec3<f32>(0.0);
            var alpine_rock=0.0;
            @if(SNOW_CLIMATE && VERTEX_UVS_A) {
                let climate=clamp(in.uv.y,0.0,1.0);
                if climate>0.001 {
                    // Elongated rock joints form vertical chutes. Interrupted
                    // strata and wind exposure leave dark faces between shelves.
                    let ledge=noise_gradient(p,0.03125);
                    let warped=p+vec3<f32>(ledge.x-0.5,stand_noise.x-0.5,0.5-ledge.x)*7.0;
                    let column=noise_gradient(vec3<f32>(warped.x*2.0,warped.y*0.25,warped.z*2.0),0.125);
                    let chips=noise_gradient(vec3<f32>(p.x*2.0,p.y*0.5,p.z*2.0),0.5);
                    let close=1.0-smoothstep(1.2,5.5,max(length(dx),length(dy)));
                    let field=column.x+close*(chips.x-0.5)*0.42+(ledge.x-0.5)*0.26;
                    let rib=1.0-smoothstep(0.018,0.17,abs(field-0.48));
                    let lee=dot(pbr.N.xz,vec2<f32>(0.09,-0.07));
                    let shelf=pbr.N.y+lee+(ledge.x-0.5)*0.24+(column.x-0.5)*0.28-0.27*rib;
                    settled_snow=climate*smoothstep(0.39,0.64,shelf);
                    bedrock=max(bedrock,climate*(1.0-settled_snow));
                    alpine_rock=climate;
                    strata=climate*(0.22+0.26*column.x+0.19*(chips.x-0.5)*close);
                    fracture=-vec3<f32>(column.y*2.0,column.z*0.25,column.w*2.0)*1.6
                             -vec3<f32>(chips.y,0.25*chips.z,chips.w)*0.41*close;
                    snow_bump=fracture*(0.10+0.24*(1.0-settled_snow));
                }
            }
            if bedrock>0.01 {
                let stone=rock_sample(p,pbr.N,dx,dy,rv);
                let slate=mix(stone.color,dot(stone.color,vec3<f32>(0.2126,0.7152,0.0722))*vec3<f32>(0.64,0.72,0.81),alpine_rock)*(1.0-strata);
                color=mix(color,slate,bedrock);
                bump=mix(bump,stone.bump+fracture,bedrock);
                rough=mix(rough,stone.roughness,bedrock);
            }
            if settled_snow>0.001 {
                let powder=vec3<f32>(0.77,0.84,0.91)*(0.93+0.12*macro_variation);
                color=mix(color,powder,settled_snow);
                bump=mix(bump,snow_bump-vec3<f32>(stand_noise.y,0.0,stand_noise.w)*0.46,settled_snow);
                rough=mix(rough,0.82+0.12*macro_variation,settled_snow);
            }
            @if(HABITAT_TERRAIN) {
                // Past the individual trees, retain the same seeded stand and clearing
                // distribution as filtered reflectance/relief on the near and far mesh.
                let d=distance(pbr.world_position.xyz,frame.observer.xyz);
                let transition=smoothstep(anchor.w-96.0,anchor.w,d);
                if cover>0.002 && transition>0.0 {
                    let fine=noise_gradient(p,0.25);
                    let fine_weight=1.0-smoothstep(1.0,6.0,max(length(dx),length(dy)));
                    let crown=clamp(cover*1.65+(macro_variation-0.5)*min(cover*4.0,1.0)*0.9,0.0,0.95);
                    let grain=(fine.x-0.5)*fine_weight;
                    let leaves=mix(vec3<f32>(0.033,0.065,0.022),p0*0.43,0.45)*(0.82+macro_variation*0.32+grain*0.48);
                    color=mix(color,leaves,crown*transition);
                    bump=mix(bump,-vec3<f32>(stand_noise.y+fine.y*fine_weight,0.0,stand_noise.w+fine.w*fine_weight)*1.8,crown*transition);
                }
            }
            pbr.material.base_color=vec4<f32>(color*mix(0.88,1.10,macro_variation),pbr.material.base_color.a);
            let tangent=bump-pbr.N*dot(bump,pbr.N);
            pbr.N=normalize(pbr.N+tangent*0.65);
            pbr.material.perceptual_roughness=clamp(rough,0.45,0.99);
        }
        @if(SURFACE_WATER) {
            // Nonparallel swell, wind chop and short capillary ripples. Integer wave
            // vectors keep the phase continuous across the 4096 m origin rebase.
            let wind=vec2<f32>(sin(dot(p.xz,vec2<f32>(3.0,1.0))*6.2831853/256.0+detail.y*0.14),
                               sin(dot(p.xz,vec2<f32>(-1.0,2.0))*6.2831853/128.0-detail.y*0.18));
            var tilt=vec2<f32>(0.0);
            var lost=0.0;
            let waves=array<vec2<f32>,6>(vec2<f32>(17.0,9.0),vec2<f32>(-11.0,27.0),
                vec2<f32>(61.0,28.0),vec2<f32>(-93.0,47.0),vec2<f32>(189.0,97.0),vec2<f32>(307.0,-209.0));
            let power=array<f32,6>(0.055,0.042,0.035,0.024,0.018,0.014);
            let tempo=array<f32,6>(0.80,-1.17,2.10,-2.47,3.33,4.18);
            let dx=dpdx(p.xz);let dy=dpdy(p.xz);
            for(var i=0;i<6;i++) {
                let k=waves[i]*(6.2831853/128.0);
                let dir=normalize(k);
                let phase=dot(p.xz,k)+detail.y*tempo[i]+dot(dir,wind)*(0.77+0.19*f32(i));
                let footprint=max(abs(dot(dx,k)),abs(dot(dy,k)));
                let resolved=1.0-smoothstep(0.55,2.6,footprint);
                tilt+=dir*power[i]*resolved*(cos(phase)+0.17*cos(2.0*phase)*resolved);
                lost+=power[i]*power[i]*(1.0-resolved*resolved);
            }
            // Local wind packets break long crests into independently moving glints.
            // Their analytic slopes avoid texture reads and feed the same PBR lobe.
            let footprint=max(length(dx),length(dy));
            let packet=sea_gradient(p.xz*0.125+detail.y*vec2<f32>(0.052,-0.031),512u);
            let crossed=vec2<f32>(p.x+p.z,p.z-p.x);
            let micro=1.0-smoothstep(0.08,0.45,footprint);
            let chop=1.0-smoothstep(0.35,1.4,footprint);
            let broad=1.0-smoothstep(1.0,6.0,footprint);
            if broad>0.0 {
                tilt+=0.095*broad*sea_gradient(p.xz*0.5+packet*0.57+detail.y*vec2<f32>(-0.23,0.11),2048u);
            }
            if chop>0.0 {
                let g=sea_gradient(crossed*1.0+packet*0.86+detail.y*vec2<f32>(0.38,-0.56),4096u);
                tilt+=0.070*chop*vec2<f32>(g.x-g.y,g.x+g.y);
            }
            if micro>0.0 {
                tilt+=0.054*micro*sea_gradient(p.xz*4.0+packet*1.2+detail.y*vec2<f32>(-1.12,0.71),16384u);
            }
            lost+=0.004*(1.0-micro)+0.006*(1.0-chop)+0.007*(1.0-broad);
            pbr.N=normalize(pbr.N-vec3<f32>(tilt.x,0.0,tilt.y));
            let fresnel=pow(1.0-clamp(dot(pbr.N,pbr.V),0.0,1.0),5.0);
            pbr.material.base_color.a=0.62+fresnel*0.30;
            pbr.material.perceptual_roughness=clamp(sqrt(0.012+1.7*sqrt(lost)),0.11,0.42);
            @if(VERTEX_COLORS && VERTEX_UVS_A) {
                // The estuary water lattice carries signed bed depth. Open ocean
                // and the outer ring have no shoreline influence.
                let shallow=in.uv.y*(1.0-smoothstep(80.0,128.0,distance(pbr.world_position.xz,frame.observer.xz)));
                if shallow>0.0 {
                    let depth=max(in.uv.x,0.0);
                    let transmission=exp(-depth*vec3<f32>(0.64,0.43,0.54));
                    let bed=vec3<f32>(0.13,0.15,0.082)*(0.90+0.10*wind.y);
                    let water=mix(pbr.material.base_color.rgb,bed,transmission*shallow*(1.0-fresnel));
                    let lap=0.19+0.11*sin(dot(p.xz,vec2<f32>(5.0,-9.0))*6.2831853/128.0+detail.y*1.4);
                    let lace=(1.0-smoothstep(0.035,0.22,abs(depth-lap)))*smoothstep(-0.15,0.75,packet.x+0.3*wind.x);
                    pbr.material.base_color=vec4<f32>(mix(water,vec3<f32>(0.56,0.63,0.58),0.48*shallow*lace),pbr.material.base_color.a);
                }
            }
        }
        @if(SURFACE_ROCK) {
            let rv=relief_view(pbr.N,pbr.V,distance(pbr.world_position.xyz,frame.observer.xyz));
            let stone=rock_sample(p,pbr.N,dpdx(p),dpdy(p),rv);
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
