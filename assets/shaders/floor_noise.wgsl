#import bevy_pbr::{
    pbr_fragment::pbr_input_from_standard_material,
}
#import "shaders/lib/snoise.wgsl"::snoise_2d

#ifdef PREPASS_PIPELINE
#import bevy_pbr::{
    prepass_io::{VertexOutput, FragmentOutput},
    pbr_deferred_functions::deferred_output,
}
#else
#import bevy_pbr::{
    forward_io::{VertexOutput, FragmentOutput},
    pbr_functions::{apply_pbr_lighting, main_pass_post_lighting_processing},
}
#endif

@group(2) @binding(100)
var<uniform> color: vec4<f32>;

@group(2) @binding(101)
var<uniform> color_alt: vec4<f32>;

@group(2) @binding(102)
var<uniform> roughness: f32;

@group(2) @binding(103)
var<uniform> roughness_alt: f32;

@group(2) @binding(104)
var noise: texture_2d<f32>;
@group(2) @binding(105)
var noise_sampler: sampler;

const UV_ROT = mat2x2<f32>(
    vec2<f32>(0.8775825618903728, 0.479425538604203),
    vec2<f32>(-0.479425538604203, 0.8775825618903728));

@fragment
fn fragment(
    in: VertexOutput,
    @builtin(front_facing) is_front: bool,
) -> FragmentOutput {
	let uv: vec2<f32> = in.uv * UV_ROT;
	let n = textureSample(noise, noise_sampler, fract(uv * 0.07)).x;
	let n2 = textureSample(noise, noise_sampler, fract(uv * 0.07 * 2.)).x;

	// generate a PbrInput struct from the StandardMaterial bindings
	var pbr_input = pbr_input_from_standard_material(in, is_front);
	let material_select = smoothstep(0.35, 0.65, (n + n2) * 0.5 + 0.2);
	pbr_input.material.base_color = mix(color, color_alt, material_select);
	pbr_input.material.perceptual_roughness = mix(roughness, roughness_alt, material_select);

#ifdef PREPASS_PIPELINE
    let out = deferred_output(in, pbr_input);
#else
    var out: FragmentOutput;
    out.color = apply_pbr_lighting(pbr_input);
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);
#endif

    return out;
}
