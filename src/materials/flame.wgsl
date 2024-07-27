#import bevy_pbr::{
    mesh_functions as mfns,
    mesh_view_bindings::globals,
}

@group(2) @binding(100)
var noise: texture_2d<f32>;
@group(2) @binding(101)
var noise_sampler: sampler;

struct Vertex {
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>,
    @location(1) normal: vec4<f32>,
    @location(2) color: vec4<f32>,
    @location(3) uv: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) uv: vec2<f32>,
};

@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    var out: VertexOutput;
    let world_from_local = mfns::get_world_from_local(vertex.instance_index);
    out.position = mfns::mesh_position_local_to_clip(world_from_local, vec4<f32>(vertex.position, 1.0));
    out.color = vertex.color;
    out.uv = vertex.uv;
    return out;
}

@fragment
fn fragment(
    @builtin(front_facing) is_front: bool,
    mesh: VertexOutput,
) -> @location(0) vec4<f32> {
  	let uv: vec2<f32> = mesh.uv;

    let time: f32 = globals.time;
    let noise_1 = textureSample(noise, noise_sampler, fract(uv + vec2f(0., time * 0.3)));
    let noise_2 = textureSample(noise, noise_sampler, fract(uv + vec2f(0., time * 0.5)));
    let alpha = smoothstep(0.5, 0.9, (noise_1.g + noise_2.g) * 0.5 + mesh.color.g);
    return vec4f(0.5, alpha * 0.3, 0.1, 0.1) * alpha;
}
