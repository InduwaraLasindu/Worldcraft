// Shared observer for forward, depth and shadow passes, including floating-origin rebases.
import bevy_pbr::render::{mesh_functions,view_transformations::position_world_to_clip};
@if(PREPASS_PIPELINE)
import bevy_pbr::prepass::io::VertexOutput;
@else
import bevy_pbr::render::forward_io::VertexOutput;
@group(constants::MATERIAL_BIND_GROUP) @binding(100) var<uniform> surface_kind:vec4<f32>;
struct SurfaceFrame {origin:vec4<f32>,observer:vec4<f32>,animation:vec4<f32>,previous_observer:vec4<f32>,previous_wind:vec4<f32>};
@group(constants::MATERIAL_BIND_GROUP) @binding(103) var<storage,read> frame:SurfaceFrame;
struct TerrainVertex {
    @builtin(instance_index) instance_index:u32,
    @location(0) position:vec3<f32>,
    @if(VERTEX_NORMALS)
    @location(1) normal:vec3<f32>,
    @if(VERTEX_UVS_A)
    @location(2) uv:vec2<f32>,
    @if(VERTEX_TANGENTS)
    @location(4) tangent:vec4<f32>,
    @if(BOTANICAL_CENTER)
    @location(11) branch_center:vec3<f32>,
    @if(VERTEX_COLORS)
    @location(5) color:vec4<f32>,
    @if(TERRAIN_MORPH)
    @location(8) parent:vec4<f32>,
    @if(TERRAIN_MORPH)
    @location(9) parent_normal:vec3<f32>,
    @if(TERRAIN_MORPH)
    @location(10) parent_color:vec4<f32>,
};
@if(TERRAIN_MORPH)
fn morph_amount(v:TerrainVertex,model:mat4x4<f32>,observer:vec4<f32>)->f32 {
    let world=mesh_functions::mesh_position_local_to_world(model,vec4<f32>(v.position,1.0));
    let delta=abs(world.xz-observer.xz);
    var distance=max(delta.x,delta.y);
    var end=v.parent.w;
    var start=end-192.0;
    if end==1.0 {
        end=observer.w;start=end-64.0;
        distance=length(model[3].xyz-observer.xyz);
    }
    if end>0.0 {return smoothstep(start,end,distance)*v.parent_color.a;}
    return 0.0;
}
// Shrink each leaf cluster about its own centre and each branch about its
// centreline. The last frame contains zero-area triangles in all raster passes.
@if(BOTANICAL_CENTER)
fn outer_position(position:vec3<f32>,center:vec3<f32>,root:vec3<f32>,eye:vec3<f32>,end:f32)->vec3<f32> {
    if end<=0.0 {return position;}
    let fraction=smoothstep(end-96.0,end,distance(root,eye));
    return mix(position,center,fraction);
}
fn wind_position(base:vec3<f32>,uv_height:f32,model:mat4x4<f32>,anchor:vec2<f32>,seconds:f32,breeze:f32)->vec3<f32> {
    var position=base;
    @if(SURFACE_FOLIAGE || SURFACE_GROUND)
    if breeze>0.0 {
        var root=model[3].xz+anchor;
        @if(SURFACE_GROUND)
        root=mesh_functions::mesh_position_local_to_world(model,vec4<f32>(position,1.0)).xz+anchor;
        let phase=dot(root,vec2<f32>(0.0981747704,0.0490873852));
        var flexibility=clamp(position.y/12.0,0.0,1.0);
        @if(SURFACE_GROUND)
        flexibility=clamp(uv_height,0.0,1.0);
        let bend=flexibility*flexibility*breeze;
        let gust=sin(seconds*1.2+phase)*0.16+sin(seconds*2.7+phase*2.0+position.y)*0.08;
        position.x+=gust*bend;
        position.z+=sin(seconds*1.5+phase+1.1)*0.12*bend;
    }
    return position;
}
@vertex
fn vertex(v:TerrainVertex)->VertexOutput {
    let detail=vec4<f32>(surface_kind.x,frame.animation.x,frame.animation.z,frame.animation.y);
    let anchor=frame.origin;
    let observer=frame.observer;
    var out:VertexOutput;
    let model=mesh_functions::get_world_from_local(v.instance_index);
    var position=v.position;
    @if(VERTEX_UVS_A)
    var uv=v.uv;
    @if(VERTEX_NORMALS)
    var normal=v.normal;
    @if(VERTEX_COLORS)
    var color=v.color;
    @if(TERRAIN_MORPH) {
        let weight=morph_amount(v,model,observer);
        @if(SURFACE_TERRAIN && VERTEX_UVS_A)
        uv.x=mix(v.uv.x,v.uv.y,weight);
        position=mix(position,v.parent.xyz,weight);
        @if(VERTEX_NORMALS)
        normal=normalize(mix(normal,v.parent_normal,weight));
        @if(VERTEX_COLORS)
        color=vec4<f32>(mix(color.rgb,v.parent_color.rgb,weight),color.a);
    }
    var uv_height=0.0;
    @if(VERTEX_UVS_A)
    uv_height=v.uv.y;
    @if(BOTANICAL_CENTER)
    position=outer_position(position,v.branch_center,model[3].xyz,observer.xyz,frame.origin.w);
    position=wind_position(position,uv_height,model,anchor.xz,detail.y,detail.w);
    out.world_position=mesh_functions::mesh_position_local_to_world(model,vec4<f32>(position,1.0));
    out.position=position_world_to_clip(out.world_position.xyz);
    @if(UNCLIPPED_DEPTH_ORTHO_EMULATION) {
        out.unclipped_depth=out.position.z;
        out.position.z=min(out.position.z,1.0);
    }
    @if(!PREPASS_PIPELINE || NORMAL_PREPASS_OR_DEFERRED_PREPASS) {
        @if(VERTEX_NORMALS)
        out.world_normal=mesh_functions::mesh_normal_local_to_world(normal,v.instance_index);
        @if(VERTEX_TANGENTS)
        out.world_tangent=mesh_functions::mesh_tangent_local_to_world(model,v.tangent,v.instance_index);
    }
    @if(VERTEX_UVS_A)
    out.uv=uv;
    @if(VERTEX_COLORS)
    out.color=color;
    @if(VERTEX_OUTPUT_INSTANCE_INDEX)
    out.instance_index=v.instance_index;
    @if(PREPASS_PIPELINE && MOTION_VECTOR_PREPASS) {
        let previous=mesh_functions::get_previous_world_from_local(v.instance_index);
        var previous_position=v.position;
        @if(TERRAIN_MORPH)
        previous_position=mix(previous_position,v.parent.xyz,morph_amount(v,previous,frame.previous_observer));
        @if(BOTANICAL_CENTER)
        previous_position=outer_position(previous_position,v.branch_center,previous[3].xyz,frame.previous_observer.xyz,frame.origin.y);
        previous_position=wind_position(previous_position,uv_height,previous,frame.previous_wind.zw,frame.previous_wind.x,frame.previous_wind.y);
        out.previous_world_position=mesh_functions::mesh_position_local_to_world(previous,vec4<f32>(previous_position,1.0));
    }
    @if(VISIBILITY_RANGE_DITHER)
    out.visibility_range_dither=mesh_functions::get_visibility_range_dither_level(v.instance_index,model[3]);
    return out;
}
