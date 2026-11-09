import bevy_pbr::render::{
    mesh_view_bindings as view_bindings,
    mesh_view_types,
    shadows,
    forward_io::{FragmentOutput, VertexOutput},
    mesh_types::MESH_FLAGS_SHADOW_RECEIVER_BIT,
    pbr_fragment::pbr_input_from_standard_material,
    pbr_functions::{alpha_discard, apply_pbr_lighting, main_pass_post_lighting_processing}
};

// Scheuermann's two lobes: a bright one, and a broader one tinted by the hair. Each is shifted
// along the strand so the bands separate.
const PRIMARY_SHININESS: f32 = 70.0;
const PRIMARY_SHIFT: f32 = 0.1;
const PRIMARY_STRENGTH: f32 = 0.07;
const PRIMARY_WHITENESS: f32 = 0.5;
const SECONDARY_SHININESS: f32 = 20.0;
const SECONDARY_SHIFT: f32 = -0.1;
const SECONDARY_STRENGTH: f32 = 0.07;
// How much the shine fades on light hair, by the hair's luminance. Both lobes' colors grow with
// the base color, so without this light hair gets over twice the shine of dark hair.
const LIGHT_HAIR_FADE: f32 = 0.8;
// Light hair passes light from strand to strand, which softens its terminator and lifts its shaded
// side. How far past the terminator that reaches (in NdotL), and how much of it shows on hair of
// luminance 1; dark hair absorbs it, so it scales with luminance.
const SCATTER_WRAP: f32 = 0.5;
const SCATTER_STRENGTH: f32 = 1.0;
const FRAC_1_PI: f32 = 0.31830988;

/// Kajiya-Kay: bright where the half vector is perpendicular to the strand, a band across it.
fn strand_specular(
    strand: vec3<f32>,
    normal: vec3<f32>,
    H: vec3<f32>,
    shift: f32,
    shininess: f32,
) -> f32 {
    let T = normalize(strand + normal * shift);
    let TdotH = dot(T, H);
    return pow(sqrt(max(1.0 - TdotH * TdotH, 0.0)), shininess);
}

@fragment
fn fragment(in: VertexOutput, @builtin(front_facing) is_front: bool) -> FragmentOutput {
    var pbr_input = pbr_input_from_standard_material(in, is_front);
    pbr_input.material.base_color = alpha_discard(
            pbr_input.material.flags,
            pbr_input.material.alpha_cutoff,
            pbr_input.material.base_color,
        );

    var out: FragmentOutput;
    out.color = apply_pbr_lighting(pbr_input);

    let strand = normalize(in.world_tangent.xyz);
    let N = pbr_input.N;
    let V = pbr_input.V;
    let base = pbr_input.material.base_color.rgb;
    let luminance = saturate(dot(base, vec3<f32>(0.2126, 0.7152, 0.0722)));
    let fade = 1.0 - LIGHT_HAIR_FADE * luminance;
    let primary_color = mix(base, vec3<f32>(1.0), PRIMARY_WHITENESS) * PRIMARY_STRENGTH * fade;
    let secondary_color = base * SECONDARY_STRENGTH * fade;

    let view_from_world = view_bindings::view.view_from_world;
    let view_z = dot(
            vec4<f32>(
                view_from_world[0].z,
                view_from_world[1].z,
                view_from_world[2].z,
                view_from_world[3].z,
            ),
            pbr_input.world_position,
        );

    var shine = vec3<f32>(0.0);
    for(var i: u32 = 0u; i < view_bindings::lights.n_directional_lights; i = i + 1u) {
        let light = &view_bindings::lights.directional_lights[i];
        let L = (*light).direction_to_light;
        let NdotL = saturate(dot(N, L));

        var shadow: f32 = 1.0;
        if (pbr_input.flags & MESH_FLAGS_SHADOW_RECEIVER_BIT) != 0u
            && ((*light).flags & mesh_view_types::DIRECTIONAL_LIGHT_FLAGS_SHADOWS_ENABLED_BIT)
            != 0u {
            shadow = shadows::fetch_directional_shadow(
                    i,
                    pbr_input.world_position,
                    pbr_input.world_normal,
                    view_z,
                    pbr_input.frag_coord.xy,
                );
        }

        // Only what plain diffuse (NdotL * shadow) left out, so the lit side is no brighter.
        let wrapped = saturate((dot(N, L) + SCATTER_WRAP) / (1.0 + SCATTER_WRAP));
        let scatter = max(wrapped - NdotL * shadow, 0.0) * SCATTER_STRENGTH * luminance;
        shine += base * FRAC_1_PI * (*light).color.rgb * scatter;

        if NdotL <= 0.0 {
            continue;
        }

        let H = normalize(L + V);
        let primary = strand_specular(strand, N, H, PRIMARY_SHIFT, PRIMARY_SHININESS);
        let secondary = strand_specular(strand, N, H, SECONDARY_SHIFT, SECONDARY_SHININESS);
        shine += (primary_color * primary + secondary_color * secondary) * (*light).color.rgb
            * NdotL * shadow;
    }
    out.color = vec4<f32>(out.color.rgb + shine * view_bindings::view.exposure, out.color.a);

    out.color = main_pass_post_lighting_processing(pbr_input, out.color);
    return out;
}
