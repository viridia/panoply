#define FRAGMENT_WAVES 1
// #define FRAGMENT_WAVES_2 1
#define VERTEX_WAVES 1
#define FOAM 1
#define SKY 1
#define STANDARD_MATERIAL_CLEARCOAT 1

#import bevy_core_pipeline::tonemapping::tone_mapping
#import bevy_pbr::{
    lighting,
    mesh_bindings::mesh,
    mesh_functions as mfns,
    mesh_view_bindings::globals,
    mesh_view_bindings::view,
    mesh_view_bindings::lights,
    mesh_view_types,
    mesh_types::MESH_FLAGS_SHADOW_RECEIVER_BIT,
    pbr_types::{PbrInput, pbr_input_new},
    pbr_functions as fns,
    shadows,
}

@group(2) @binding(1)
var<uniform> water_color: vec4<f32>;

@group(2) @binding(2)
var<uniform> sky_color: array<vec4<f32>, 2>;

@group(2) @binding(3)
var waves: texture_2d<f32>;
@group(2) @binding(4)
var waves_sampler: sampler;

@group(2) @binding(5)
var sky: texture_2d<f32>;
@group(2) @binding(6)
var sky_sampler: sampler;

@group(2) @binding(7)
var foam: texture_2d<f32>;
@group(2) @binding(8)
var foam_sampler: sampler;

struct Vertex {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) depth_motion: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) world_position: vec4<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) depth: f32,
};

struct WaveAccum {
  amplitude: f32,
  tangent: vec2<f32>,
}

const PI: f32 = 3.14159;

fn add_wave(
    freq: f32,
    strength: f32,
    direction: vec2<f32>,
    position: vec2<f32>,
    out: ptr<function, WaveAccum>,
) {
    let phase = freq * globals.time;
    let wavelength = length(direction);
    let l = 1. / (wavelength * wavelength);
    let angle = (phase / wavelength + dot(direction, position) * l) * PI * 2.;
    (*out).amplitude += cos(angle) * strength;
    (*out).tangent += direction * l * sin(angle) * strength * PI * 2.;
}

@vertex
fn vertex(vertex: Vertex, @builtin(instance_index) instance_index: u32) -> VertexOutput {
    var out: VertexOutput;
    var position = vertex.position;
    var normal = vertex.normal;
    var wposition = mfns::mesh_position_local_to_world(
        mfns::get_world_from_local(instance_index),
        vec4<f32>(vertex.position, 1.0)
    );
    let uv = wposition.xz;

    var wave: WaveAccum;
    // freq, wavelength, strength, position, direction
#ifdef VERTEX_WAVES
    add_wave(1., .05, vec2(5.0, 5.0), uv, &wave);
    add_wave(1.5, .05, vec2(3.0, 7.0), uv, &wave);
    add_wave(2.7, .05, vec2(0.0, 10.3), uv, &wave);
    add_wave(2.3, .05, vec2(7.5, 3.0), uv, &wave);
#endif

    position.y += wave.amplitude;
    normal = normalize(vec3(wave.tangent.x * 0.5, 1.0, wave.tangent.y * 0.5));
    position.x -= normal.x * 0.7;
    position.y += 0.1;
    position.z -= normal.z * 0.7;

    out.world_position = mfns::mesh_position_local_to_world(
        mfns::get_world_from_local(instance_index),
        vec4<f32>(position, 1.0)
    );
    out.position = mfns::mesh_position_local_to_clip(
        mfns::get_world_from_local(instance_index),
        vec4<f32>(position, 1.0)
    );

    out.world_normal = mfns::mesh_normal_local_to_world(normal, instance_index);
    out.depth = vertex.depth_motion.x;
    return out;
}

struct FragmentWaveGenerator {
    freq: f32,
    strength: f32,
    direction: vec2<f32>,
}

const FRAGMENT_WAVES = array(
    FragmentWaveGenerator(0.11, 0.07, vec2(1.0, 0.0)),
    FragmentWaveGenerator(0.21, 0.03, vec2(0.9, 0.1)),
    FragmentWaveGenerator(0.51, 0.03, vec2(0.9, -.1)),

    FragmentWaveGenerator(0.12, 0.07, vec2(0.0, 1.0)),
    FragmentWaveGenerator(0.22, 0.03, vec2(0.1, 0.9)),
    FragmentWaveGenerator(0.52, 0.03, vec2(-.1, 0.9)),

    FragmentWaveGenerator(0.13, 0.05, vec2(-1.0, 0.0)),
    FragmentWaveGenerator(0.23, 0.03, vec2(-1.0, 0.0)),
    FragmentWaveGenerator(0.53, 0.03, vec2(-1.0, 0.0)),

    FragmentWaveGenerator(0.14, 0.05, vec2(0.0, -1.0)),
    FragmentWaveGenerator(0.24, 0.03, vec2(0.0, -1.0)),
    FragmentWaveGenerator(0.54, 0.03, vec2(0.0, -1.0)),
);

@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
    let uv = vec2<f32>(mesh.world_position.xz);
    let water_depth = mesh.world_position.y + mesh.depth;
    var normal = mesh.world_normal;
    var chop = vec3<f32>(0.);

#ifdef FRAGMENT_WAVES
    var motion = vec2(0., 0.);
    var time_mult = 0.7;
    var weight = 0.0;
    for (var i = 0u; i < 12u; i++) {
        let direction = FRAGMENT_WAVES[i].direction;
        let frequency = FRAGMENT_WAVES[i].freq;
        var strength = FRAGMENT_WAVES[i].strength;
        // if direction.x < 0.1 {
        //     strength *= 0.1;
        // }

        let m = mat2x2(direction.x, direction.y, -direction.y, direction.x);
        let p: vec2<f32> = (uv * m + vec2(time_mult * globals.time, 0.)) * frequency;
        let d = textureSample(waves, waves_sampler, fract(p)).rgb - 0.25;
        chop += vec3f(d.x, 0.0, d.y) * strength;
        weight += strength;
    }
    normal = normalize(normal + chop / weight * 0.5);
#endif

#ifdef FRAGMENT_WAVES_2
    // var d = 1. - min(1., length(uv) / 16.);
    // var motion = vec2(0., 0.);
    var iter: f32 = 0.1;
    var frequency = 2.;
    var weight = .1;
    var speed = 1.0;
    for (var i = 0; i < 12; i++) {
        let s = sin(iter);
        let c = cos(iter);
        let direction = vec2(c, s);
        let phi = dot(uv, direction) * frequency + speed * globals.time;
        let s1 = sin(phi);
        chop.x += direction.x * s1 * 0.01;
        chop.z += direction.y * s1 * 0.01;
        frequency *= 1.123;
        speed *= 1.07;
        // weight *= 0.82;
        iter += 1232.399963;
    }
    normal = normalize(normal + chop);
#endif

    let view_vector = normalize(view.world_position - mesh.world_position.xyz);
    var reflect_vector = reflect(view_vector, normal);
    reflect_vector /= reflect_vector.y;
    let angle = dot(view_vector, mesh.world_normal);
    let opacity = 0.2 + 1.6 * pow(1.0 - angle, 2.);

#ifdef SKY
    let sky_color = mix(
        sky_color[0],
        sky_color[1],
        textureSample(sky, sky_sampler, fract(reflect_vector.xz * 0.5)).g);
#else
    let sky_color = sky_color[0];
#endif
    var color = mix(vec4(water_color.rgb, 1.0), sky_color * 0.5, opacity * 0.5 - 0.1);

#ifdef FOAM
    let n1 = textureSample(foam, foam_sampler, fract(uv * 0.15 + globals.time * vec2(0.02, 0.02))).g;
    let n2 = textureSample(foam, foam_sampler, fract(uv * 0.15 + globals.time * vec2(-0.01, 0.03))).g;
    let n3 = textureSample(foam, foam_sampler, fract(uv * 0.15 + globals.time * vec2(0.03, -0.02))).g;
    var foam_level = 0.8 - pow(water_depth * 2.0 + 0.3, 0.6) + (n1 + n2 + n3) * 0.3; // + (n1 + n2 + n3 + n4 + n5);
    foam_level = smoothstep(.3, .9, foam_level);

    color = mix(color, vec4(.8, .9, 1., 0.6), foam_level);
#endif

    let view_z = dot(vec4<f32>(
        view.view_from_world[0].z,
        view.view_from_world[1].z,
        view.view_from_world[2].z,
        view.view_from_world[3].z
    ), mesh.world_position);

    let V = fns::calculate_view(mesh.world_position, false);
    let n_directional_lights = lights.n_directional_lights;
    for (var i: u32 = 0u; i < n_directional_lights; i = i + 1u) {
        // check if this light should be skipped, which occurs if this light does not intersect with the view
        // note point and spot lights aren't skippable, as the relevant lights are filtered in `assign_lights_to_clusters`
        let light = &lights.directional_lights[i];
        if (*light).skip != 0u {
            continue;
        }

        var shadow: f32 = 1.0;
        if ((lights.directional_lights[i].flags & mesh_view_types::DIRECTIONAL_LIGHT_FLAGS_SHADOWS_ENABLED_BIT) != 0u) {
            shadow = shadows::fetch_directional_shadow(i, mesh.world_position, normal, view_z);
        }

        let L = (*light).direction_to_light.xyz;
        let H = normalize(L + V);
        let n_dot_h = dot(normal, H);
        let spec = specular_fast_pbr(n_dot_h, 0.007) * 0.0005
            + specular_fast_pbr(n_dot_h, 0.05) * 0.00002;
        color += vec4f((*light).color.rgb * spec * shadow, 0.0);
    }

    // var pbr_input: PbrInput = pbr_input_new();
    // pbr_input.material.base_color = color;
    // pbr_input.material.perceptual_roughness = 0.9;
    // pbr_input.material.metallic = 0.;
    // pbr_input.material.clearcoat = 0.2;
    // pbr_input.material.clearcoat_perceptual_roughness = 0.1;
    // pbr_input.frag_coord = mesh.position;
    // pbr_input.world_position = mesh.world_position;
    // pbr_input.world_normal = normal;
    // pbr_input.N = normalize(pbr_input.world_normal);
    // pbr_input.clearcoat_N = pbr_input.N;
    // pbr_input.V = fns::calculate_view(mesh.world_position, pbr_input.is_orthographic);
    // pbr_input.flags |= MESH_FLAGS_SHADOW_RECEIVER_BIT;
    // pbr_input.is_orthographic = false;

    // color = fns::apply_pbr_lighting(pbr_input);

    color.a = opacity * clamp(water_depth * 40. + 6.1, 0., 1.);
    return tone_mapping(color, view.color_grading);
}

fn specular_fast_pbr(NdotH: f32, roughness: f32) -> f32 {
    let a = roughness * roughness;
    let a2 = a * a;
    let d = NdotH * NdotH * (a2 - 1.0) + 1.0;
    return a2 / (d * d);
}
