#import bevy_pbr::{
    pbr_fragment::pbr_input_from_standard_material,
}

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
var<uniform> color_base: vec4<f32>;

@group(2) @binding(101)
var<uniform> color_accent: vec4<f32>;

// Allows summing of up to 4 noise octaves via a dot product.
// #define PERSIST(c) vec4<f32>(c, c*c, c*c*c, c*c*c*c) / (c + c*c + c*c*c + c*c*c*c)

fn persist(c: f32) -> vec4<f32> {
    return vec4<f32>(c, c*c, c*c*c, c*c*c*c) / (c + c*c + c*c*c + c*c*c*c);
}

@fragment
fn fragment(
    in: VertexOutput,
    @builtin(front_facing) is_front: bool,
) -> FragmentOutput {
    var persist0_9: vec4<f32> = persist(0.9);

    // generate a PbrInput struct from the StandardMaterial bindings
    var pbr_input = pbr_input_from_standard_material(in, is_front);
    pbr_input.material.base_color = color_accent;

    // pbr_input.material.base_color = vec4<f32>(fract(in.uv), 1.0, 1.0);
    // pbr_input.material.base_color.z = 0.;

#ifdef PREPASS_PIPELINE
    let out = deferred_output(in, pbr_input);
#else
    var out: FragmentOutput;
    out.color = apply_pbr_lighting(pbr_input);
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);
#endif

    return out;
}
