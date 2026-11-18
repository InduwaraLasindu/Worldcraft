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

fn density(p:vec3<f32>,lod:f32)->f32 {
    let height=(p.y-sky.anchor.w)/900.0;
    if height<=0.0 || height>=1.0 {return 0.0;}
    let world=p.xz+sky.anchor.xz+vec2<f32>(sky.weather.x,sky.weather.w);
    let offset=sky.weather.yz;
    // A broad two-dimensional weather field sets cloud base and top. The
    // volume uses comparable horizontal/vertical feature sizes for cumulus.
    let deck_uv=world/32768.0+offset*0.25;
    let weather=textureSampleLevel(cloud_noise,cloud_sampler,vec3<f32>(deck_uv.x,0.28,deck_uv.y),1.0).r;
    let convection=smoothstep(0.29,0.72,weather);
    let bottom=0.035+0.09*(1.0-convection);
    let top=0.32+0.65*convection;
    let envelope=smoothstep(bottom,bottom+0.12,height)*(1.0-smoothstep(top-0.28,top,height));
    if envelope<=0.0 {return 0.0;}
    let uv=vec3<f32>(world.x/4096.0+offset.x,height*0.24+0.14,world.y/4096.0+offset.y);
    let mass=textureSampleLevel(cloud_noise,cloud_sampler,uv,lod).r;
    let erosion_uv=vec3<f32>(-uv.z,uv.y,uv.x)*3.0+vec3<f32>(0.17,0.29,0.43);
    let erosion=textureSampleLevel(cloud_noise,cloud_sampler,erosion_uv,lod+0.4).r;
    let threshold=sky.sky.w-0.08+(0.5-weather)*0.33-(sky.sampling.y-0.5)*0.45;
    let shape=smoothstep(threshold,threshold+0.22,mass+(erosion-0.55)*0.24-(1.0-envelope)*0.23);
    return shape*envelope*0.0048;
}
fn sunlight(p:vec3<f32>,lod:f32)->f32 {
    var optical=0.0;
    for(var j=0u;j<2u;j+=1u) {
        optical+=density(p+sky.sun.xyz*(f32(j)+0.5)*380.0,lod+1.0)*380.0;
    }
    return exp(-optical);
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
    if abs(ray.y)<0.001 {return original;}
    let a=(sky.anchor.w-origin.y)/ray.y;
    let b=(sky.anchor.w+900.0-origin.y)/ray.y;
    let start=max(min(a,b),0.0);
    let end=min(max(a,b),far);
    if end<=start {return original;}
    let steps=u32(clamp(sky.sampling.x,16.0,48.0));
    let step_length=(end-start)/f32(steps);
    let lod=clamp(log2(max(step_length/100.0,1.0)),0.0,3.5);
    var transmittance=1.0;
    var accumulated=vec3<f32>(0.0);
    let jitter=0.36+0.28*fract(52.9829189*fract(dot(in.position.xy,vec2<f32>(0.06711056,0.00583715))));
    let daylight=max(sky.sky.x,0.0);
    let sun_color=vec3<f32>(1.0,0.72+daylight*0.28,0.51+daylight*0.49);
    let facing=pow(max(dot(ray,sky.sun.xyz),0.0),8.0);
    for(var i=0u;i<steps;i+=1u) {
        let travel=start+(f32(i)+jitter)*step_length;
        let p=origin+ray*travel;
        let amount=density(p,lod)*(1.0-smoothstep(14000.0,36000.0,travel));
        if amount>0.00001 {
            let direct=sunlight(p,lod);
            let opacity=1.0-exp(-amount*step_length);
            // A bounded sky fill approximates multiple scattering. Every sample
            // gets two sun probes, avoiding alternating cached-light bands.
            let layer=clamp((p.y-sky.anchor.w)/900.0,0.0,1.0);
            let fill=vec3<f32>(0.66,0.79,1.0)*(0.02+(8000.0+6500.0*layer)*sqrt(daylight));
            let light=(fill+sun_color*sky.sun.w*direct*(0.38+facing*0.25))*view.exposure;
            accumulated+=transmittance*opacity*light;
            transmittance*=1.0-opacity;
            if transmittance<0.02 {break;}
        }
    }
    return vec4<f32>(original.rgb*transmittance+accumulated,original.a);
}
