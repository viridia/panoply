#define FRAGMENT_WAVES_2 1
#define VERTEX_WAVES 1
// #define FRAGMENT_WAVES 1

#import bevy_pbr::mesh_view_bindings    globals
#import bevy_pbr::mesh_view_bindings    view
#import bevy_pbr::pbr_functions         as fns
#import bevy_pbr::mesh_functions        as mfns
#import bevy_pbr::mesh_bindings         mesh
#import bevy_core_pipeline::tonemapping tone_mapping
// #import "shaderlib/noised.wgsl"         noised

@group(1) @binding(1)
var<uniform> water_color: vec4<f32>;

@group(1) @binding(2)
var noise: texture_2d<f32>;
@group(1) @binding(3)
var noise_sampler: sampler;

@group(1) @binding(4)
var sky: texture_2d<f32>;
@group(1) @binding(5)
var sky_sampler: sampler;

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
fn vertex(vertex: Vertex) -> VertexOutput {

    var out: VertexOutput;
    var position = vertex.position;
    var normal = vertex.normal;
    var wposition = mfns::mesh_position_local_to_world(
        mesh.model,
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
    position.z -= normal.z * 0.7;

    out.world_position = mfns::mesh_position_local_to_world(
        mesh.model,
        vec4<f32>(position, 1.0)
    );
    out.position = mfns::mesh_position_local_to_clip(
        mesh.model,
        vec4<f32>(position, 1.0)
    );

    out.world_normal = mfns::mesh_normal_local_to_world(normal);
    out.depth = vertex.depth_motion.x;
    return out;
}

struct CurrentWave {
    dir: vec2<f32>,
    rot: vec2<f32>,
    period: f32,
    speed: f32,
}

const currents: array<CurrentWave, 1> = array<CurrentWave, 1>(
    CurrentWave(vec2<f32>(0.5, 0.7), vec2<f32>(0.8, 0.1), 1., 0.5),
);

// Calculates wave value and its derivative,
// for the wave direction, position in space, wave frequency and time
fn wavedx(position: vec2<f32>, direction: vec2<f32>, frequency: f32, timeshift: f32) -> vec2<f32> {
  let x = dot(direction, position) * frequency + timeshift;
  let wave = exp(sin(x) - 1.0);
  let dx = wave * cos(x);
  return vec2(wave, -dx);
}

@fragment
fn fragment(
    @builtin(front_facing) is_front: bool,
    mesh: VertexOutput,
) -> @location(0) vec4<f32> {
    let uv = vec2<f32>(mesh.world_position.xz);

    //   let water_depth = pow(max(0., mesh.world_position.y - mesh.depth), 0.5) * 2.;
    let water_depth = mesh.world_position.y + mesh.depth;

    var normal = mesh.world_normal;
    var chop = vec3<f32>(0.);

#ifdef FRAGMENT_WAVES
    var iter: f32 = 0.;
    var frequency = 2.0;
    var time_mult = 2.0;
    var weight = 0.2;
    for (var i = 0; i < 8; i++) {
        let p: vec2<f32> = vec2(sin(iter), cos(iter));
        let w = wavedx(uv, p, frequency, globals.time * time_mult);
        chop.x += w.x * weight;
        chop.z += w.y * weight;

        frequency *= 1.18;
        time_mult *= 1.07;
        weight *= 0.82;
        iter += 1232.399963;
    }
    normal = normalize(normal + chop);
#endif
#ifdef FRAGMENT_WAVES_2
    var d = 1. - min(1., length(uv) / 16.);
    var motion = vec2(d, 0.);
    var iter: f32 = 1.;
    var frequency = 0.05;
    var time_mult = 0.9;
    var weight = .1;
    for (var i = 0; i < 12; i++) {
        let s = sin(iter);
        let c = cos(iter);
        let w = dot(motion, vec2(c, s)) + 1.;
        let m = mat2x2(c, s, -s, c);
        let p: vec2<f32> = (uv * m + vec2(time_mult * globals.time, 0.)) * frequency;
        let n = textureSample(noise, noise_sampler, fract(p)).x;
        let d = vec3<f32>(
            n - textureSample(noise, noise_sampler, fract(p + vec2<f32>(1.0 / 16.0, 0.))).x,
            0.,
            n - textureSample(noise, noise_sampler, fract(p + vec2<f32>(0., 1.0 / 16.0))).x
        );
        chop += d * weight * w;
        frequency *= 1.20;
        // time_mult *= 1.07;
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

    let sky_color = textureSample(sky, sky_sampler, fract(reflect_vector.xz * 0.2));
    let color = mix(vec4(water_color.rgb, 0.9), sky_color, opacity - 0.1);

    var pbr_input: fns::PbrInput = fns::pbr_input_new();
    pbr_input.material.base_color = color;
    pbr_input.material.perceptual_roughness = 0.1;
    pbr_input.material.metallic = 0.5;
    pbr_input.frag_coord = mesh.position;
    pbr_input.world_position = mesh.world_position;
    pbr_input.world_normal = fns::prepare_world_normal(
        normal,
        false,
        is_front,
    );

    pbr_input.is_orthographic = false;
    pbr_input.N = fns::apply_normal_mapping(
        pbr_input.material.flags,
        normal,
        view.mip_bias,
    );
    pbr_input.V = fns::calculate_view(mesh.world_position, pbr_input.is_orthographic);

    var out_color = fns::pbr(pbr_input);
    out_color.a = opacity * clamp(water_depth * 4., 0., 1.);
    return tone_mapping(out_color, view.color_grading);
}
