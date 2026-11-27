import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput;
import bevy_render::view::View;
@group(0) @binding(0) var scene_color:texture_2d<f32>;
@group(0) @binding(1) var scene_sampler:sampler;
@group(0) @binding(2) var scene_depth:texture_depth_2d;
@group(0) @binding(3) var cloud_noise:texture_3d<f32>;
@group(0) @binding(4) var cloud_sampler:sampler;
@group(0) @binding(5) var<uniform> view:View;
struct SkySettings {anchor:vec4<f32>,sun:vec4<f32>,weather:vec4<f32>,sky:vec4<f32>,sampling:vec4<f32>,moon:vec4<f32>,storm:vec4<f32>};
@group(0) @binding(6) var<uniform> sky:SkySettings;

const CLOUD_BOTTOM:f32=0.035;
// The tallest updraft tops out at 0.925 of the layer. Leave rounding headroom.
const CLOUD_TOP:f32=0.93;

fn weather_at(p:vec3<f32>)->vec2<f32> {
    let world=p.xz+sky.anchor.xz+vec2<f32>(sky.weather.x,sky.weather.w);
    let uv=(world+vec2<f32>(world.y,-world.x)*0.5)/65536.0+sky.weather.yz*0.25;
    return textureSampleLevel(cloud_noise,cloud_sampler,vec3<f32>(uv.x,0.28,uv.y),1.0).rg;
}
// Convective columns grow from a common condensation level. Their tops follow
// local updrafts; the regional weather controls how tall those updrafts can get.
fn density(p:vec3<f32>,lod:f32,weather:vec2<f32>)->f32 {
    let height=(p.y-sky.anchor.w)/900.0;
    if height<=CLOUD_BOTTOM || height>=CLOUD_TOP {return 0.0;}
    let convection=smoothstep(0.28,0.73,weather.x);
    let world=p.xz+sky.anchor.xz+vec2<f32>(sky.weather.x,sky.weather.w);
    // Regional weather bends the smaller, repeating billow field. Both
    // coordinates use integer phase shifts over the full wrapped cloud origin.
    let warp=(weather-vec2<f32>(0.5))*vec2<f32>(0.49,0.63);
    let shear=height*vec2<f32>(0.082,-0.052)*(0.5+0.5*convection);
    let horizontal=world/4096.0+vec2<f32>(world.y,-world.x)/16384.0
        +sky.weather.yz+warp+shear;
    let uv=vec3<f32>(horizontal.x,height*0.26+0.14,horizontal.y);
    let field=textureSampleLevel(cloud_noise,cloud_sampler,uv,lod).rg;
    let column=smoothstep(0.32,0.72,field.x+0.32*(field.y-0.5));
    let base=0.035+0.07*(1.0-convection);
    let top=base+0.17+column*(0.30+0.42*convection);
    let envelope=smoothstep(base,base+0.10,height)*(1.0-smoothstep(top-0.18,top,height));
    if envelope<=0.0 {return 0.0;}
    let bank=smoothstep(0.38,0.64,weather.x+0.18*(weather.y-0.5));
    let threshold=sky.sky.w-0.09+(0.5-weather.x)*0.39-(sky.sampling.y-0.5)*0.45
        +0.12*(0.52-bank);
    let mass=field.x+(field.y-0.46)*(0.40+0.26*height)-(1.0-envelope)*0.19-threshold;
    if mass<=0.0 {return 0.0;}
    // Even maximal erosion cannot change a saturated interior sample.
    if mass>=0.26 {return envelope*0.006;}
    // Cross the broad field on a different axis. Integer frequency ratios keep
    // the edge detail periodic across both wind and floating-origin wraps.
    let edge_uv=uv.zxy*4.0+vec3<f32>(0.31,0.63,0.17);
    let erosion=textureSampleLevel(cloud_noise,cloud_sampler,edge_uv,lod+1.0).g;
    return smoothstep(0.0,0.13,mass-0.13*(1.0-erosion))*envelope*0.006;
}
fn sunlight(p:vec3<f32>,lod:f32,weather:vec2<f32>)->f32 {
    // Midpoints of two sunward intervals. Shorten both at the top of the
    // deck, so a sunlit crown sees sky while a thick underside stays shaded.
    let reach=clamp((sky.anchor.w+900.0-p.y)/max(sky.sun.y,0.05),0.0,990.0);
    let split=min(175.0,reach*0.45);
    let near=density(p+sky.sun.xyz*(0.5*split),lod+0.35,weather);
    let far=density(p+sky.sun.xyz*(0.5*(split+reach)),lod+1.15,weather);
    return exp(-near*split-far*(reach-split));
}
fn mist_visible(origin:vec3<f32>,ray:vec3<f32>,start:f32,end:f32)->bool {
    let sea=sky.anchor.w-1600.0;
    let height_scale=max(12.0,sky.sampling.w*0.33);
    return sky.sampling.z>0.0 && origin.y>=sea && end>=3.0 && end>start
        && min(origin.y+ray.y*start,origin.y+ray.y*end)<=sea+height_scale*9.0;
}

// Regional moisture changes the height and extinction of the bank.
// Projected cloud openings change its radiance along the ray.
fn valley_air(origin:vec3<f32>,ray:vec3<f32>,start:f32,end:f32,samples:u32)->vec4<f32> {
    if samples==0u {return vec4<f32>(0.0,0.0,0.0,1.0);}
    let sea=sky.anchor.w-1600.0;
    let height_scale=max(12.0,sky.sampling.w*0.33);
    let daylight=smoothstep(-0.06,0.18,sky.sun.y);
    let low_sun=1.0-smoothstep(0.12,0.80,sky.sun.y);
    let facing=max(0.0,dot(ray,sky.sun.xyz));
    let phase=0.16+0.55*pow(facing,3.0)+0.95*pow(facing,14.0);
    let sun_tint=mix(vec3<f32>(1.0,0.95,0.83),vec3<f32>(1.0,0.68,0.38),low_sun);
    var total=vec3<f32>(0.0);
    var transmission=1.0;
    var previous=start;
    for(var i=1u;i<=samples;i+=1u) {
        let fraction=f32(i)/f32(samples);
        let next=start+(end-start)*fraction*fraction;
        let p=origin+ray*((previous+next)*0.5);
        let world=p.xz+sky.anchor.xz+vec2<f32>(sky.weather.x,sky.weather.w)*0.25;
        let uv=world/2048.0+sky.weather.yz;
        let humidity=textureSampleLevel(cloud_noise,cloud_sampler,vec3<f32>(uv.x,0.43,uv.y),1.0).r;
        let bank=smoothstep(0.27,0.72,humidity);
        let altitude=max(0.0,p.y-sea);
        let vertical=exp(-altitude/(height_scale*(0.65+0.95*bank)));
        let sigma=min(sky.sampling.z,0.003)*vertical*(0.48+2.20*bank*bank)*(1.0+0.85*low_sun*daylight);
        let opacity=1.0-exp(-sigma*(next-previous));
        var lit=1.0;
        if sky.sky.z>0.5 && daylight>0.0 {
            let cloud=p+sky.sun.xyz*max(0.0,(sky.anchor.w+290.0-p.y)/max(0.07,sky.sun.y));
            lit=exp(-density(cloud,0.45,weather_at(cloud))*1050.0);
        }
        let ambient=vec3<f32>(0.54,0.72,0.95)*(0.06+daylight*7400.0);
        let radiance=ambient+sun_tint*sky.sun.w*phase*lit*daylight;
        total+=transmission*opacity*radiance*view.exposure;
        transmission*=1.0-opacity;
        previous=next;
    }
    return vec4<f32>(total,transmission);
}

fn through_air(color:vec4<f32>,air:vec4<f32>)->vec4<f32> {
    return vec4<f32>(color.rgb*air.a+air.rgb,color.a);
}

fn composite_mist(color:vec4<f32>,origin:vec3<f32>,ray:vec3<f32>,reach:f32)->vec4<f32> {
    let samples=select(0u,6u,mist_visible(origin,ray,0.0,reach));
    return through_air(color,valley_air(origin,ray,0.0,reach,samples));
}

fn cloud_interval(origin:vec3<f32>,ray:vec3<f32>,far:f32)->vec2<f32> {
    if sky.sky.z<0.5 {return vec2<f32>(0.0);}
    let base=sky.anchor.w;
    let top=base+900.0;
    // Parallel rays inside the layer still see clouds when flying through it.
    if abs(ray.y)<0.000001 {
        if origin.y<base || origin.y>top {return vec2<f32>(0.0);}
        return vec2<f32>(0.0,far);
    }
    let a=(base-origin.y)/ray.y;
    let b=(top-origin.y)/ray.y;
    return vec2<f32>(max(min(a,b),0.0),min(max(a,b),far));
}
fn stars_hash(cell:vec2<i32>)->vec3<f32> {
    let p=bitcast<vec2<u32>>(cell);
    var h=(p.x*0x9e3779b1u)^(p.y*0x85ebca77u);
    h=(h^(h>>16u))*0x7feb352du;
    h=(h^(h>>15u))*0x846ca68bu;
    return vec3<f32>(vec3<u32>(h,h>>8u,h>>16u)&vec3<u32>(255u))/255.0;
}

fn night_sky(ray:vec3<f32>)->vec3<f32> {
    let angle=sky.sky.y;
    let shell=vec2<f32>(dot(ray.xz,vec2<f32>(cos(angle),sin(angle))),
                        dot(ray.xz,vec2<f32>(-sin(angle),cos(angle))));
    let p=shell*(230.0/(0.9+ray.y));
    let star=stars_hash(vec2<i32>(floor(p)));
    let offset=star.xy*0.64+vec2<f32>(0.18);
    let radius=mix(0.085,0.28,star.z);
    let speck=(1.0-smoothstep(radius*0.24,radius,length(fract(p)-offset)))
        *smoothstep(0.957,0.998,star.z);
    let star_color=mix(vec3<f32>(0.70,0.82,1.0),vec3<f32>(1.0,0.86,0.68),star.y);
    let moon=dot(ray,sky.moon.xyz);
    let rim=smoothstep(0.99978,0.99983,moon);
    let moonlight=vec3<f32>(0.68,0.80,1.0);
    let glow=pow(max(0.0,moon),180.0)*0.11;
    let above=smoothstep(-0.025,0.18,ray.y);
    return ((speck*star_color*2.7+moonlight*(3.8*rim+glow))*above
        +vec3<f32>(0.017,0.031,0.063)*(0.35+0.65*above))*sky.moon.w;
}

fn lightning(ray:vec3<f32>)->f32 {
    let direction=vec2<f32>(cos(sky.storm.w),sin(sky.storm.w));
    let facing=dot(ray.xz,direction);
    if facing<0.65 || ray.y<0.038 || ray.y>0.30 {return 0.0;}
    let p=vec2<f32>(dot(ray.xz,vec2<f32>(-direction.y,direction.x)),ray.y);
    let rows=(p.y-0.038)*28.0;
    let row=i32(floor(rows));
    let salt=i32(sky.storm.w*999.0);
    let a=stars_hash(vec2<i32>(row,salt)).x;
    let b=stars_hash(vec2<i32>(row+1,salt)).x;
    let x=(mix(a,b,fract(rows))-0.5)*0.055+p.y*0.085;
    let width=abs(p.x-x);
    let rim=smoothstep(0.038,0.065,p.y)*(1.0-smoothstep(0.24,0.30,p.y));
    return rim*(5.0*(1.0-smoothstep(0.00045,0.0028,width))+0.65*(1.0-smoothstep(0.0,0.011,width)));
}

@fragment
fn fragment(in:FullscreenVertexOutput)->@location(0) vec4<f32> {
    var original=textureSampleLevel(scene_color,scene_sampler,in.uv,0.0);
    let ndc=vec2<f32>(in.uv.x*2.0-1.0,1.0-in.uv.y*2.0);
    let near_point=view.world_from_clip*vec4<f32>(ndc,1.0,1.0);
    let origin=view.world_position;
    let ray=normalize(near_point.xyz/near_point.w-origin);
    // Temporal reconstruction keeps output HDR at presentation resolution while
    // the prepass is smaller. Address depth by UV, never output pixel indices.
    let depth_size=textureDimensions(scene_depth);
    let depth_pixel=min(vec2<u32>(in.uv*vec2<f32>(depth_size)),depth_size-vec2<u32>(1u));
    let depth=textureLoad(scene_depth,vec2<i32>(depth_pixel),0);
    let sky_pixel=depth<=0.0000001;
    if sky_pixel {
        var rgb=original.rgb;
        if ray.y>-0.05 && sky.moon.w>0.0 {
            rgb+=night_sky(ray)*(1.0-0.85*sky.storm.x-0.14*sky.storm.y);
        }
        let daylight=1.0-0.92*sky.moon.w;
        let storm=vec3<f32>(0.29,0.34,0.41)*daylight;
        let dust=vec3<f32>(0.36,0.18,0.072)*daylight;
        rgb=mix(mix(rgb,storm,sky.storm.x*0.60),dust,sky.storm.y*0.68);
        if sky.storm.z>0.01 {
            rgb+=vec3<f32>(0.36,0.48,0.70)*sky.storm.z
                +vec3<f32>(0.72,0.84,1.0)*sky.storm.z*lightning(ray);
        }
        original=vec4<f32>(rgb,original.a);
    }
    var far=36000.0;
    if !sky_pixel {
        let point=view.world_from_clip*vec4<f32>(ndc,depth,1.0);
        far=min(far,length(point.xyz/point.w-origin));
    }
    let interval=cloud_interval(origin,ray,far);
    let mist_reach=min(far,6200.0);
    let start=interval.x;
    let end=interval.y;
    if end<=start {
        return composite_mist(original,origin,ray,mist_reach);
    }
    let steps=u32(clamp(sky.sampling.x,16.0,48.0));
    let span=end-start;
    let inverse_steps=1.0/f32(steps);
    // Long, grazing rays need finer samples near the first cloud banks.
    // Short rays, including most in-cloud views, keep uniform spacing.
    let spacing_bias=0.62*smoothstep(2600.0,15000.0,span);
    var previous=start;
    var transmittance=1.0;
    var accumulated=vec3<f32>(0.0);
    let noise=fract(52.9829189*fract(dot(in.position.xy,vec2<f32>(0.06711056,0.00583715))));
    let daylight=max(sky.sky.x,0.0);
    let sun_color=vec3<f32>(1.0,0.72+daylight*0.28,0.51+daylight*0.49);
    let cos_angle=dot(ray,sky.sun.xyz);
    // Normalized Henyey-Greenstein lobes (4*pi folded into the radiance scale).
    let forward=0.5775/pow(1.4225-1.30*cos_angle,1.5);
    let back=0.96/pow(1.04+0.40*cos_angle,1.5);
    let phase=0.36*(0.72*forward+0.28*back);
    for(var i=0u;i<steps;i+=1u) {
        let fraction=f32(i+1u)*inverse_steps;
        let next=start+span*mix(fraction,fraction*fraction,spacing_bias);
        let step_length=next-previous;
        // Spread coarse samples across each segment to hide coherent slices.
        // Fine samples stay close to their midpoint to limit visible grain.
        let jitter=0.5+(noise-0.5)*mix(0.28,0.74,smoothstep(180.0,650.0,step_length));
        let travel=mix(previous,next,jitter);
        previous=next;
        let lod=clamp(log2(max(step_length/100.0,1.0)),0.0,3.5);
        let p=origin+ray*travel;
        let layer=(p.y-sky.anchor.w)/900.0;
        if layer<=CLOUD_BOTTOM || layer>=CLOUD_TOP {continue;}
        let weather=weather_at(p);
        let amount=density(p,lod,weather)*(1.0-smoothstep(14000.0,36000.0,travel));
        if amount>0.00001 {
            let direct=sunlight(p,lod,weather);
            let opacity=1.0-exp(-amount*step_length);
            // A bounded sky fill approximates multiple scattering. Every sample
            // gets two sun probes, avoiding alternating cached-light bands.
            let fill=vec3<f32>(0.67,0.80,1.0)*(0.02+(5800.0+9200.0*layer)*sqrt(daylight)
                +9.0*sky.moon.w+3200.0*sky.storm.z);
            let scattered=0.23*(1.0-direct)*sqrt(direct);
            var light=(fill+sun_color*sky.sun.w*(direct*phase+scattered))*view.exposure;
            if sky_pixel && travel>4000.0 {
                // Air between the camera and a distant bank reduces its
                // contrast toward the sky color already rendered on this ray.
                let air_transmission=exp(-0.000055*(travel-4000.0));
                light=mix(original.rgb,light,air_transmission);
            }
            accumulated+=transmittance*opacity*light;
            transmittance*=1.0-opacity;
            if transmittance<0.02 {break;}
        }
    }
    if transmittance==1.0 {
        return composite_mist(original,origin,ray,mist_reach);
    }
    // Split the low mist at the near cloud boundary. From above, rays through
    // clouds toward a valley must see that valley's mist through the clouds.
    let mist_split=min(start,mist_reach);
    let front_mist=mist_visible(origin,ray,0.0,mist_split);
    let back_mist=mist_visible(origin,ray,mist_split,mist_reach);
    var front_samples=select(0u,6u,front_mist);
    var back_samples=select(0u,6u,back_mist);
    if front_mist && back_mist {
        // Share the original six probes. Quadratic spacing keeps more of them
        // near the camera without increasing the worst-case texture reads.
        front_samples=u32(clamp(round(6.0*sqrt(mist_split/mist_reach)),1.0,5.0));
        back_samples=6u-front_samples;
    }
    let background=through_air(original,valley_air(origin,ray,mist_split,mist_reach,back_samples));
    let cloudy=vec4<f32>(background.rgb*transmittance+accumulated,original.a);
    return through_air(cloudy,valley_air(origin,ray,0.0,mist_split,front_samples));
}
