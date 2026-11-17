// Shared observer for forward, depth and shadow passes, including floating-origin rebases.
import bevy_pbr::render::{mesh_functions,view_transformations::position_world_to_clip};
@if(PREPASS_PIPELINE)
import bevy_pbr::prepass::io::VertexOutput;
@else
import bevy_pbr::render::forward_io::VertexOutput;
@group(constants::MATERIAL_BIND_GROUP) @binding(100) var<uniform> surface_kind:vec4<f32>;
struct SurfaceFrame {origin:vec4<f32>,observer:vec4<f32>,animation:vec4<f32>};
@group(constants::MATERIAL_BIND_GROUP) @binding(103) var<storage,read> frame:SurfaceFrame;
struct TerrainVertex {
    @builtin(instance_index) instance_index:u32,
    @location(0) position:vec3<f32>,
    @if(VERTEX_NORMALS)
    @location(1) normal:vec3<f32>,
    @if(VERTEX_UVS_A)
    @location(2) uv:vec2<f32>,
    @if(VERTEX_COLORS)
    @location(5) color:vec4<f32>,
    @if(TERRAIN_MORPH)
    @location(8) parent:vec4<f32>,
    @if(TERRAIN_MORPH)
    @location(9) parent_normal:vec3<f32>,
    @if(TERRAIN_MORPH)
    @location(10) parent_color:vec4<f32>,
};
@vertex
fn vertex(v:TerrainVertex)->VertexOutput {
    let detail=vec4<f32>(surface_kind.x,frame.animation.x,frame.animation.z,frame.animation.y);
    let anchor=frame.origin;
    let observer=frame.observer;
    var out:VertexOutput;
    let model=mesh_functions::get_world_from_local(v.instance_index);
    var position=v.position;
    @if(VERTEX_NORMALS)
    var normal=v.normal;
    @if(VERTEX_COLORS)
    var color=v.color;
    @if(TERRAIN_MORPH) {
        let world=mesh_functions::mesh_position_local_to_world(model,vec4<f32>(position,1.0));
        let delta=abs(world.xz-observer.xz);
        var distance=max(delta.x,delta.y);
        var end=v.parent.w;
        var start=end-192.0;
        if end==1.0 {
            // Tree visibility uses Euclidean distance from the shared instance
            // origin, including altitude. Shadows use the same observer.
            end=observer.w;
            start=end-64.0;
            distance=length(model[3].xyz-observer.xyz);
        }
        var weight=0.0;
        if end>0.0 {weight=smoothstep(start,end,distance)*v.parent_color.a;}
        position=mix(position,v.parent.xyz,weight);
        @if(VERTEX_NORMALS)
        normal=normalize(mix(normal,v.parent_normal,weight));
        @if(VERTEX_COLORS)
        color=vec4<f32>(mix(color.rgb,v.parent_color.rgb,weight),color.a);
    }
    @if(SURFACE_FOLIAGE || SURFACE_GROUND)
    if detail.w>0.0 {
        // Shared tree-origin phase is continuous through a world-origin rebase.
        // 64/128 m periods divide the 4096 m wrapped anchor. Apply AFTER the
        // crown morph so both representations have exactly the same endpoint.
        var root=model[3].xz+anchor.xz;
        @if(SURFACE_GROUND) {
            root=mesh_functions::mesh_position_local_to_world(model,vec4<f32>(position,1.0)).xz+anchor.xz;
        }
        let phase=dot(root,vec2<f32>(0.0981747704,0.0490873852));
        var flexibility=clamp(position.y/12.0,0.0,1.0);
        @if(VERTEX_UVS_A && SURFACE_GROUND) {flexibility=clamp(v.uv.y,0.0,1.0);}
        let bend=flexibility*flexibility*detail.w;
        let gust=sin(detail.y*1.2+phase)*0.16+sin(detail.y*2.7+phase*2.0+position.y)*0.08;
        position.x+=gust*bend;
        position.z+=sin(detail.y*1.5+phase+1.1)*0.12*bend;
    }
    out.world_position=mesh_functions::mesh_position_local_to_world(model,vec4<f32>(position,1.0));
    out.position=position_world_to_clip(out.world_position.xyz);
    @if(UNCLIPPED_DEPTH_ORTHO_EMULATION) {
        out.unclipped_depth=out.position.z;
        out.position.z=min(out.position.z,1.0);
    }
    @if(!PREPASS_PIPELINE || NORMAL_PREPASS_OR_DEFERRED_PREPASS) {
        @if(VERTEX_NORMALS)
        out.world_normal=mesh_functions::mesh_normal_local_to_world(normal,v.instance_index);
    }
    @if(VERTEX_UVS_A)
    out.uv=v.uv;
    @if(VERTEX_COLORS)
    out.color=color;
    @if(VERTEX_OUTPUT_INSTANCE_INDEX)
    out.instance_index=v.instance_index;
    @if(PREPASS_PIPELINE && MOTION_VECTOR_PREPASS) {
        let previous=mesh_functions::get_previous_world_from_local(v.instance_index);
        out.previous_world_position=mesh_functions::mesh_position_local_to_world(previous,vec4<f32>(position,1.0));
    }
    @if(VISIBILITY_RANGE_DITHER)
    out.visibility_range_dither=mesh_functions::get_visibility_range_dither_level(v.instance_index,model[3]);
    return out;
}
