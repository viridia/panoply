#import bevy_pbr::{
    mesh_functions as mfns,
}

@group(2) @binding(100)
var<uniform> width: f32;

struct Vertex {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) world_position: vec4<f32>,
    @location(1) world_normal: vec3<f32>,
};

@vertex
fn vertex(vertex: Vertex, @builtin(instance_index) instance_index: u32) -> VertexOutput {
    var out: VertexOutput;
    var wposition = mfns::mesh_position_local_to_world(
        mfns::get_world_from_local(instance_index),
        vec4<f32>(vertex.position, 1.0)
    );
    wposition += vec4<f32>(vertex.normal * width, 0.0);

    out.world_position = wposition;
    out.world_normal = mfns::mesh_normal_local_to_world(vertex.normal, instance_index);
    out.position = mfns::mesh_position_local_to_clip(
        mfns::get_world_from_local(instance_index),
        vec4<f32>(vertex.position + vertex.normal * width, 1.0)
    );

    return out;
}

@fragment
fn fragment(
    @builtin(front_facing) is_front: bool,
    mesh: VertexOutput,
) -> @location(0) vec4<f32> {
  // Solid black
  return vec4<f32>(0.0, 0.0, 0.0, 1.0);
}
