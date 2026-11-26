import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput;
import bevy_render::view::View;
@group(0) @binding(0) var scene_color:texture_2d<f32>;
@group(0) @binding(1) var scene_sampler:sampler;
@group(0) @binding(2) var scene_depth:texture_depth_2d;
@group(0) @binding(3) var cloud_noise:texture_3d<f32>;
@group(0) @binding(4) var cloud_sampler:sampler;
@group(0) @binding(5) var<uniform> view:View;
struct SkySettings {anchor:vec4<f32>,sun:vec4<f32>,weather:vec4<f32>,sky:vec4<f32>,sampling:vec4<f32>};
@group(0) @binding(6) var<uniform> sky:SkySettings;

fn weather_at(p:vec3<f32>)->f32 {
    let world=p.xz+sky.anchor.xz+vec2<f32>(sky.weather.x,sky.weather.w);
    let uv=world/32768.0+sky.weather.yz*0.25;
    return textureSampleLevel(cloud_noise,cloud_sampler,vec3<f32>(uv.x,0.28,uv.y),1.0).r;
}
// Convective columns grow from a common condensation level. Their tops follow
// local updrafts; the regional weather controls how tall those updrafts can get.
fn density(p:vec3<f32>,lod:f32,weather:f32)->f32 {
    let height=(p.y-sky.anchor.w)/900.0;
    if height<=0.02 || height>=1.0 {return 0.0;}
    let convection=smoothstep(0.28,0.73,weather);
    let world=p.xz+sky.anchor.xz+vec2<f32>(sky.weather.x,sky.weather.w);
    let uv=vec3<f32>(world.x/4096.0+sky.weather.y,height*0.26+0.14,world.y/4096.0+sky.weather.z);
    let field=textureSampleLevel(cloud_noise,cloud_sampler,uv,lod).rg;
    let column=smoothstep(0.32,0.72,field.x+0.32*(field.y-0.5));
    let base=0.035+0.07*(1.0-convection);
    let top=base+0.17+column*(0.30+0.42*convection);
    let envelope=smoothstep(base,base+0.10,height)*(1.0-smoothstep(top-0.18,top,height));
    if envelope<=0.0 {return 0.0;}
    let threshold=sky.sky.w-0.09+(0.5-weather)*0.39-(sky.sampling.y-0.5)*0.45;
    let mass=field.x+(field.y-0.46)*(0.40+0.26*height)-(1.0-envelope)*0.19-threshold;
    if mass<=0.0 {return 0.0;}
    // Cross the broad field on a different axis. Integer frequency ratios keep
    // the edge detail periodic across both wind and floating-origin wraps.
    let edge_uv=uv.zxy*4.0+vec3<f32>(0.31,0.63,0.17);
    let erosion=textureSampleLevel(cloud_noise,cloud_sampler,edge_uv,lod+1.0).g;
    return smoothstep(0.0,0.13,mass-0.13*(1.0-erosion))*envelope*0.006;
}
fn sunlight(p:vec3<f32>,lod:f32,weather:f32)->f32 {
    // Midpoints of two sunward intervals. Shorten both at the top of the
    // deck, so a sunlit crown sees sky while a thick underside stays shaded.
    let reach=clamp((sky.anchor.w+900.0-p.y)/max(sky.sun.y,0.05),0.0,990.0);
    let split=min(175.0,reach*0.45);
    let near=density(p+sky.sun.xyz*(0.5*split),lod+0.35,weather);
    let far=density(p+sky.sun.xyz*(0.5*(split+reach)),lod+1.15,weather);
    return exp(-near*split-far*(reach-split));
}
// Integrate finite height fog along the opaque-depth interval. Regional moisture
// changes both the height and extinction of the bank. Projected cloud openings
// change radiance along the ray, instead of brightening the whole horizon.
fn valley_air(origin:vec3<f32>,ray:vec3<f32>,far:f32)->vec4<f32> {
    let sea=sky.anchor.w-1600.0;
    let reach=min(far,6200.0);
    if sky.sampling.z<=0.0 || origin.y<sea || reach<3.0 {
        return vec4<f32>(0.0,0.0,0.0,1.0);
    }
    let height_scale=max(12.0,sky.sampling.w*0.33);
    if min(origin.y,origin.y+ray.y*reach)>sea+height_scale*9.0 {
        return vec4<f32>(0.0,0.0,0.0,1.0);
    }
    let daylight=smoothstep(-0.06,0.18,sky.sun.y);
    let low_sun=1.0-smoothstep(0.12,0.80,sky.sun.y);
    let facing=max(0.0,dot(ray,sky.sun.xyz));
    let phase=0.16+0.55*pow(facing,3.0)+0.95*pow(facing,14.0);
    let sun_tint=mix(vec3<f32>(1.0,0.95,0.83),vec3<f32>(1.0,0.68,0.38),low_sun);
    var total=vec3<f32>(0.0);
    var transmission=1.0;
    var previous=0.0;
    for(var i=1u;i<=6u;i+=1u) {
        let fraction=f32(i)/6.0;
        let end=reach*fraction*fraction;
        let p=origin+ray*((previous+end)*0.5);
        let world=p.xz+sky.anchor.xz+vec2<f32>(sky.weather.x,sky.weather.w)*0.25;
        let uv=world/2048.0+sky.weather.yz;
        let humidity=textureSampleLevel(cloud_noise,cloud_sampler,vec3<f32>(uv.x,0.43,uv.y),1.0).r;
        let bank=smoothstep(0.27,0.72,humidity);
        let altitude=max(0.0,p.y-sea);
        let vertical=exp(-altitude/(height_scale*(0.65+0.95*bank)));
        let sigma=min(sky.sampling.z,0.003)*vertical*(0.48+2.20*bank*bank)*(1.0+0.85*low_sun*daylight);
        let opacity=1.0-exp(-sigma*(end-previous));
        var lit=1.0;
        if sky.sky.z>0.5 && daylight>0.0 {
            let cloud=p+sky.sun.xyz*max(0.0,(sky.anchor.w+290.0-p.y)/max(0.07,sky.sun.y));
            lit=exp(-density(cloud,0.45,weather_at(cloud))*1050.0);
        }
        let ambient=vec3<f32>(0.54,0.72,0.95)*(0.06+daylight*7400.0);
        let radiance=ambient+sun_tint*sky.sun.w*phase*lit*daylight;
        total+=transmission*opacity*radiance*view.exposure;
        transmission*=1.0-opacity;
        previous=end;
    }
    return vec4<f32>(total,transmission);
}
@fragment
fn fragment(in:FullscreenVertexOutput)->@location(0) vec4<f32> {
    let original=textureSampleLevel(scene_color,scene_sampler,in.uv,0.0);
    let ndc=vec2<f32>(in.uv.x*2.0-1.0,1.0-in.uv.y*2.0);
    let near_point=view.world_from_clip*vec4<f32>(ndc,1.0,1.0);
    let origin=view.world_position;
    let ray=normalize(near_point.xyz/near_point.w-origin);
    // Temporal reconstruction keeps output HDR at presentation resolution while
    // the prepass is smaller. Address depth by UV, never output pixel indices.
    let depth_size=textureDimensions(scene_depth);
    let depth_pixel=min(vec2<u32>(in.uv*vec2<f32>(depth_size)),depth_size-vec2<u32>(1u));
    let depth=textureLoad(scene_depth,vec2<i32>(depth_pixel),0);
    var far=36000.0;
    if depth>0.0000001 {
        let point=view.world_from_clip*vec4<f32>(ndc,depth,1.0);
        far=min(far,length(point.xyz/point.w-origin));
    }
    let air=valley_air(origin,ray,far);
    let clear=vec4<f32>(original.rgb*air.a+air.rgb,original.a);
    if sky.sky.z<0.5 {return clear;}
    // Parallel rays inside the layer still see clouds when flying through it.
    var start=0.0;
    var end=far;
    if abs(ray.y)<0.000001 {
        if origin.y<sky.anchor.w || origin.y>sky.anchor.w+900.0 {return clear;}
    } else {
        let a=(sky.anchor.w-origin.y)/ray.y;
        let b=(sky.anchor.w+900.0-origin.y)/ray.y;
        start=max(min(a,b),0.0);
        end=min(max(a,b),far);
    }
    if end<=start {return clear;}
    let steps=u32(clamp(sky.sampling.x,16.0,48.0));
    let step_length=(end-start)/f32(steps);
    let lod=clamp(log2(max(step_length/100.0,1.0)),0.0,3.5);
    var transmittance=1.0;
    var accumulated=vec3<f32>(0.0);
    let jitter=0.36+0.28*fract(52.9829189*fract(dot(in.position.xy,vec2<f32>(0.06711056,0.00583715))));
    let daylight=max(sky.sky.x,0.0);
    let sun_color=vec3<f32>(1.0,0.72+daylight*0.28,0.51+daylight*0.49);
    let cos_angle=dot(ray,sky.sun.xyz);
    // Normalized Henyey-Greenstein lobes (4*pi folded into the radiance scale).
    let forward=0.5775/pow(1.4225-1.30*cos_angle,1.5);
    let back=0.96/pow(1.04+0.40*cos_angle,1.5);
    let phase=0.36*(0.72*forward+0.28*back);
    for(var i=0u;i<steps;i+=1u) {
        let travel=start+(f32(i)+jitter)*step_length;
        let p=origin+ray*travel;
        let weather=weather_at(p);
        let amount=density(p,lod,weather)*(1.0-smoothstep(14000.0,36000.0,travel));
        if amount>0.00001 {
            let direct=sunlight(p,lod,weather);
            let opacity=1.0-exp(-amount*step_length);
            // A bounded sky fill approximates multiple scattering. Every sample
            // gets two sun probes, avoiding alternating cached-light bands.
            let layer=clamp((p.y-sky.anchor.w)/900.0,0.0,1.0);
            let fill=vec3<f32>(0.67,0.80,1.0)*(0.02+(5800.0+9200.0*layer)*sqrt(daylight));
            let scattered=0.23*(1.0-direct)*sqrt(direct);
            let light=(fill+sun_color*sky.sun.w*(direct*phase+scattered))*view.exposure;
            accumulated+=transmittance*opacity*light;
            transmittance*=1.0-opacity;
            if transmittance<0.02 {break;}
        }
    }
    return vec4<f32>((original.rgb*transmittance+accumulated)*air.a+air.rgb,original.a);
}
