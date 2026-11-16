// Shared observer for forward, depth and shadow passes, including floating-origin rebases.
import bevy_pbr::render::{mesh_functions,view_transformations::position_world_to_clip};
@if(PREPASS_PIPELINE)
import bevy_pbr::prepass::io::VertexOutput;
@else
import bevy_pbr::render::forward_io::VertexOutput;
@group(constants::MATERIAL_BIND_GROUP) @binding(102) var<uniform> observer:vec4<f32>;
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
        let distance=max(delta.x,delta.y);
        var weight=0.0;
        if v.parent.w>0.0 {weight=smoothstep(v.parent.w-192.0,v.parent.w,distance)*v.parent_color.a;}
        position=mix(position,v.parent.xyz,weight);
        @if(VERTEX_NORMALS)
        normal=normalize(mix(normal,v.parent_normal,weight));
        @if(VERTEX_COLORS)
        color=vec4<f32>(mix(color.rgb,v.parent_color.rgb,weight),color.a);
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
    @if(MOTION_VECTOR_PREPASS) {
        let previous=mesh_functions::get_previous_world_from_local(v.instance_index);
        out.previous_world_position=mesh_functions::mesh_position_local_to_world(previous,vec4<f32>(position,1.0));
    }
    @if(VISIBILITY_RANGE_DITHER)
    out.visibility_range_dither=mesh_functions::get_visibility_range_dither_level(v.instance_index,model[3]);
    return out;
}
