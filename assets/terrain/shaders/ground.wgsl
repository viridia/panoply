#import bevy_pbr::mesh_vertex_output    MeshVertexOutput
#import bevy_pbr::mesh_view_bindings    view
#import bevy_pbr::pbr_types             STANDARD_MATERIAL_FLAGS_DOUBLE_SIDED_BIT
#import bevy_core_pipeline::tonemapping tone_mapping
#import bevy_pbr::pbr_functions as fns

@group(1) @binding(1)
var grass: texture_2d<f32>;
@group(1) @binding(2)
var grass_sampler: sampler;

// Vertex shader

// struct VertexOutput {
//     @builtin(position) clip_position: vec4<f32>,
// };

// @vertex
// fn vs_main(
//     @builtin(vertex_index) in_vertex_index: u32,
// ) -> VertexOutput {
//     var out: VertexOutput;
//     let x = f32(1 - i32(in_vertex_index)) * 0.5;
//     let y = f32(i32(in_vertex_index & 1u) * 2 - 1) * 0.5;
//     out.clip_position = vec4<f32>(x, y, 0.0, 1.0);
//     return out;
// }

@fragment
fn fragment(
    @builtin(front_facing) is_front: bool,
    mesh: MeshVertexOutput,
) -> @location(0) vec4<f32> {
    let uv = fract(vec2<f32>(mesh.world_position.xz * 0.35));

    var pbr_input: fns::PbrInput = fns::pbr_input_new();

    pbr_input.material.base_color = textureSample(grass, grass_sampler, uv);
    pbr_input.material.perceptual_roughness = 1.;
    pbr_input.frag_coord = mesh.position;
    pbr_input.world_position = mesh.world_position;
    pbr_input.world_normal = fns::prepare_world_normal(
        mesh.world_normal,
        (pbr_input.material.flags & STANDARD_MATERIAL_FLAGS_DOUBLE_SIDED_BIT) != 0u,
        is_front,
    );

    pbr_input.is_orthographic = false;

    pbr_input.N = fns::apply_normal_mapping(
        pbr_input.material.flags,
        mesh.world_normal,
        view.mip_bias,
    );
    pbr_input.V = fns::calculate_view(mesh.world_position, pbr_input.is_orthographic);

    return tone_mapping(fns::pbr(pbr_input), view.color_grading);
}
