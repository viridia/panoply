#import bevy_pbr::mesh_view_bindings    view
#import bevy_pbr::pbr_types             STANDARD_MATERIAL_FLAGS_ALPHA_MODE_BLEND
#import bevy_pbr::pbr_functions         as fns
#import bevy_pbr::mesh_functions        as mfns
#import bevy_pbr::mesh_bindings         mesh
#import bevy_core_pipeline::tonemapping tone_mapping
#import "shaderlib/snoise.wgsl"         snoise_2d

@group(1) @binding(1)
var<uniform> water_color: vec4<f32>;

@group(1) @binding(2)
var<uniform> sky_color: array<vec4<f32>, 2>;

// @group(1) @binding(1)
// var noise: texture_2d<f32>;
// @group(1) @binding(2)
// var noise_sampler: sampler;

struct Vertex {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    // @location(0x1000) depth: f32,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) world_position: vec4<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) depth: f32,
};

// varying vec3 vViewPosition;
// varying vec3 vNormal;
// varying vec3 vPosition;
// varying float vDepth;

// ${wave}

// void main() {
//   vec3 objectNormal = vec3(0., 1., 0.);
//   vec3 transformedNormal = normalMatrix * objectNormal;

//   vec3 transformed = vec3( position );
//   vec4 mPosition = modelMatrix * vec4(transformed, 1.0);

//   vec2 tangent = vec2(0., 0.);
//   float amplitude = 0.0;
//   // phase, wavelength, strength, position, direction
//   addWave(time * 1., .05, vec2(5.0, 5.0), mPosition.xz, amplitude, tangent);
//   addWave(time * 1.5, .05, vec2(3.0, 7.0), mPosition.xz, amplitude, tangent);
//   addWave(time * 2.7, .05, vec2(0.0, 10.3), mPosition.xz, amplitude, tangent);
//   addWave(time * 2.3, .05, vec2(7.5, 3.0), mPosition.xz, amplitude, tangent);

//   mPosition.y += amplitude;
//   objectNormal = normalize(vec3(tangent.x * 0.5, 1.0, tangent.y * 0.5));

//   mPosition.xz -= objectNormal.xz * 0.7;
//   vec4 mvPosition = viewMatrix * mPosition;

//   vPosition = mPosition.xyz;
// 	vNormal = normalize(objectNormal);
//   vViewPosition = -mvPosition.xyz;
//   vDepth = depthMotion.x;

//   gl_Position = projectionMatrix * mvPosition;

//   ${ShaderChunk.clipping_planes_vertex}
// 	${ShaderChunk.worldpos_vertex}
// 	${ShaderChunk.shadowmap_vertex}
// }`;

// const lake_frag = glsl`
// #define STANDARD

// uniform vec3 skyColor1;
// uniform vec3 skyColor2;
// uniform vec3 waterColor;

// uniform float roughness;
// uniform float metalness;
// uniform float time;

// varying vec3 vViewPosition;
// varying vec3 vNormal;
// varying vec3 vPosition;
// varying float vDepth;

// ${steppers}
// ${wave}
// ${support}
// ${noise2d}
// ${noise3dGrad}
// ${octaves}

// #undef USE_SHADOWMAP

// ${ShaderChunk.common}
// ${ShaderChunk.packing}
// ${ShaderChunk.bsdfs}
// ${ShaderChunk.lights_pars_begin}
// ${ShaderChunk.lights_physical_pars_fragment}
// ${ShaderChunk.shadowmap_pars_fragment}
// ${ShaderChunk.clipping_planes_pars_fragment}

// void main() {
//   ${ShaderChunk.clipping_planes_fragment}

// 	vec3 normal = normalize(vNormal);
//   vec3 geometryNormal = normal;
//   float waterDepth = pow(max(0., vPosition.y - vDepth), 0.5) * 2.;
//   vec3 txPos = vPosition * 0.5 + time * vec3(0.1, 0.2, 0.1);

//   float chopLevel = 0.02;
//   float chopNoise[6];
//   vec3 chopGradient[6];
//   chopNoise[0] = snoise(txPos, chopGradient[0]);
//   chopNoise[1] = snoise(txPos * 2., chopGradient[1]);
//   chopNoise[2] = snoise(txPos * 4., chopGradient[2]);
//   chopNoise[3] = snoise(txPos * 8., chopGradient[3]);
//   vec3 gradient = sumOctaves(chopGradient, 0, 3, 0.4) * chopLevel;
//   normal.x += gradient.x;
//   normal.y += gradient.z;
//   normal = normalize(normal);

//   vec3 viewVector = normalize(cameraPosition - vPosition);
//   vec3 reflectVector = reflect(viewVector, normal);
//   float angle = dot(viewVector, normal);

//   reflectVector /= reflectVector.y;
//   float skyNoise[8];
//   float coeff = 1.;
//   for (int i = 0; i < 6; i++) {
//     skyNoise[i] = snoise(reflectVector.xz * coeff);
//     coeff *= 2.;
//   }
//   float skyMix = sumOctaves(skyNoise, 0, 5, 0.5);
//   vec3 skyColor = mix(skyColor1, skyColor2, smootherstep(-0.7, 0.3, skyMix));

//   ReflectedLight reflectedLight = ReflectedLight(vec3(0.0), vec3(0.0), vec3(0.0), vec3(0.0));

//   float opacity = 0.1 + 1.6 * pow(1.0 - angle, 2.);
//   vec4 diffuseColor = mix(vec4(waterColor, .4), vec4(skyColor, 1.), opacity);

//   if (waterDepth < 1.5) {
//     float n1 = snoise(vPosition.xz * 2. + vec2(time * 0.2, time * 0.3)) * 0.2;
//     float n2 = snoise(vPosition.xz * 3. + vec2(-time * 0.3, time * 0.2)) * 0.2;
//     float n3 = snoise(vPosition.xz * 7. + vec2(time * 0.4, -time * 0.4)) * 0.2;
//     float n4 = snoise(vPosition.xz * 11. + vec2(-time * 0.25, -time * 0.2)) * 0.1;
//     float n5 = snoise(vPosition.xz * 17. + vec2(-time * 0.25, -time * 0.2)) * 0.1;

//     float foamLevel = 1.6 - pow(waterDepth, 0.8) + (n1 + n2 + n3 + n4 + n5);
//     foamLevel = smoothstep(.3, .9, foamLevel);
//     diffuseColor = mix(diffuseColor, vec4(.8, .9, 1., 0.6), foamLevel);
//   }
//   diffuseColor.a *= clamp(waterDepth * 4., 0., 1.);

//   float roughnessFactor = roughness;
//   float metalnessFactor = metalness;

// 	// accumulation
// 	${ShaderChunk.lights_physical_fragment}
// 	${ShaderChunk.lights_fragment_begin}
//   ${ShaderChunk.lights_fragment_end}

// 	vec3 outgoingLight =
// 		reflectedLight.directDiffuse +
// 		reflectedLight.indirectDiffuse +
// 		reflectedLight.directSpecular +
// 		reflectedLight.indirectSpecular;
//   // gl_FragColor = vec4(outgoingLight, sqrt(1. - angle * angle));

//   gl_FragColor = diffuseColor;
// }`;

// export class LakeMaterial extends ShaderMaterial {
//   constructor() {
//     super({
//       uniforms: uniforms,
//       fragmentShader: lake_frag,
//       vertexShader: lake_vert,
//       lights: false,
//       depthWrite: false,
//       transparent: true,
//       clipping: true,
//     });
//   }
// }

// export const lakeMaterial = new LakeMaterial();

@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    var out: VertexOutput;
    var wposition = mfns::mesh_position_local_to_world(
        mesh.model,
        vec4<f32>(vertex.position, 1.0)
    );

    out.world_position = wposition;
    out.position = mfns::mesh_position_local_to_clip(
        mesh.model,
        vec4<f32>(vertex.position, 1.0)
    );

    out.world_normal = mfns::mesh_normal_local_to_world(vertex.normal);
    // out.depth = vertex.depth;
    return out;
}

@fragment
fn fragment(
    @builtin(front_facing) is_front: bool,
    mesh: VertexOutput,
) -> @location(0) vec4<f32> {
  let uv = vec2<f32>(mesh.world_position.xz);

  let color = vec4(water_color.rgb, 0.6);
  let view_vector = normalize(view.world_position - mesh.world_position.xyz);
  let reflect_vector = reflect(view_vector, mesh.world_normal);
  let angle = dot(view_vector, mesh.world_normal);
  let opacity = 0.1 + 1.6 * pow(1.0 - angle, 2.);

  var pbr_input: fns::PbrInput = fns::pbr_input_new();
  pbr_input.material.base_color = color;
  pbr_input.material.perceptual_roughness = 0.1;
  pbr_input.material.metallic = 0.5;
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

  var out_color = fns::pbr(pbr_input);
  out_color.a = opacity;
  return tone_mapping(out_color, view.color_grading);
}
