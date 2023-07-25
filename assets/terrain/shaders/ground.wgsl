#import bevy_pbr::mesh_view_bindings    view
#import bevy_pbr::pbr_functions         as fns
#import bevy_pbr::mesh_functions        as mfns
#import bevy_pbr::mesh_bindings         mesh
#import bevy_core_pipeline::tonemapping tone_mapping
#import "shaderlib/octaves.wgsl"
#import "shaderlib/snoise.wgsl"         snoise_2d

@group(1) @binding(1)
var grass: texture_2d<f32>;
@group(1) @binding(2)
var grass_sampler: sampler;
@group(1) @binding(3)
var dirt: texture_2d<f32>;
@group(1) @binding(4)
var dirt_sampler: sampler;

struct Vertex {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) world_position: vec4<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) slope: f32,

// varying vec3 vViewPosition;
// varying vec3 vPosition;
// varying vec4 vStyle;
// varying float vBiomeWeight[NumGroundTypes];

};

@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    var out: VertexOutput;
    out.world_position = mfns::mesh_position_local_to_world(
        mesh.model,
        vec4<f32>(vertex.position, 1.0)
    );
    out.position = mfns::mesh_position_local_to_clip(
        mesh.model,
        vec4<f32>(vertex.position, 1.0)
    );
    out.world_normal = mfns::mesh_normal_local_to_world(vertex.normal);
    out.slope = -out.world_normal.y;
    return out;
}

@fragment
fn fragment(
    @builtin(front_facing) is_front: bool,
    mesh: VertexOutput,
) -> @location(0) vec4<f32> {
    let uv = vec2<f32>(mesh.world_position.xz * 0.35);

    var pbr_input: fns::PbrInput = fns::pbr_input_new();

    let p = snoise_2d(uv);

    pbr_input.material.base_color = textureSample(grass, grass_sampler, fract(uv));
    // pbr_input.material.base_color = textureSample(dirt, dirt_sampler, fract(uv));
    // pbr_input.material.base_color = vec4<f32>(p * 0.5 + 0.5, 0., 0., 1.);

    pbr_input.material.perceptual_roughness = 1.;
    pbr_input.frag_coord = mesh.position;
    pbr_input.world_position = mesh.world_position;
    pbr_input.world_normal = fns::prepare_world_normal(
        mesh.world_normal,
        false,
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
