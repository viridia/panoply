#import bevy_pbr::mesh_view_bindings    view
#import bevy_pbr::pbr_functions         as fns
#import bevy_pbr::mesh_functions        as mfns
#import bevy_pbr::mesh_bindings         mesh
#import bevy_core_pipeline::tonemapping tone_mapping
#import "shaderlib/snoise.wgsl"         snoise_2d

@group(1) @binding(1)
var noise: texture_2d<f32>;
@group(1) @binding(2)
var noise_sampler: sampler;
@group(1) @binding(3)
var grass: texture_2d<f32>;
@group(1) @binding(4)
var grass_sampler: sampler;
@group(1) @binding(5)
var dirt: texture_2d<f32>;
@group(1) @binding(6)
var dirt_sampler: sampler;

const NUM_GROUND_TYPES = 8u;

struct Vertex {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) world_position: vec4<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) slope: f32,

// varying vec4 vStyle;

    // WGSL doesn't like arrays in vertex outputs, so we'll have to use vectors. Ugh.
    // x0 = Rock
    // y0 = Dirt
    // z0 = Grass
    // w0 = Moss
    // x1 = Taiga
    // y1 = Sand
    // z1 = Tundra
    // w1 = Snow
    // x2 = Chaparral
    @location(3) biome_weight_0: vec4<f32>,
    @location(4) biome_weight_1: vec4<f32>,
    @location(5) biome_weight_2: vec4<f32>,
};

struct BiomeSpec {
    roughness: f32,
    blend_min: f32,
    blend_max: f32,
    edge_blend_min: f32,
    edge_blend_max: f32,
    tx_scale: f32,
    edge_tint: vec3<f32>,

		// float blend = smoothstep(0.4, 0.6, vBiomeWeight[Grass] + terraNoise * 0.5);
		// float edgeBlend = smoothstep(0.5, 0.7, vBiomeWeight[Grass] + terraNoise * 0.5);
		// vec4 grassColor = texture2D(grassTexture, vPosition.xz * 0.35);
		// topLayerColor = mix(
		// 	topLayerColor,
		// 	vec4(
		// 		mix(
		// 			grassColor.rgb * vec3(0.25, 0.6, 0.35),
		// 			grassColor.rgb,
		// 			min(edgeBlend, 1. - underDarken)
		// 		),
		// 		1.0
		// 	),
		// 	blend);

}

const grass_biome = BiomeSpec(0.99, 0., 0., 0., 0., 0., vec3<f32>(0.20, 0.6, 0.30));

// Allows summing of up to 4 noise octaves via a dot product.
fn persist(c: f32) -> vec4<f32> {
    return vec4<f32>(c, c*c, c*c*c, c*c*c*c) / (c + c*c + c*c*c + c*c*c*c);
}

struct SurfaceAccum {
    color: vec4<f32>,
    terrain_noise: f32,
    under_darken: f32,
}

fn blend_biome(out: ptr<function, SurfaceAccum>, weight: f32, tx_color: vec3<f32>) {
    let blend = smoothstep(0.4, 0.6, weight + (*out).terrain_noise * 0.5);
    let edge_blend = smoothstep(0.5, 0.7, weight + (*out).terrain_noise * 0.5);
    (*out).color = mix(
        (*out).color,
        vec4(
            mix(
                tx_color * vec3(0.20, 0.6, 0.30),
                tx_color,
                min(edge_blend, 1. - (*out).under_darken)
            ),
            1.0
        ),
        blend);
}

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
    out.biome_weight_0 = vec4<f32>(0., 0., 1., 0.);
    out.biome_weight_1 = vec4<f32>(0., 0., 0., 0.);
    out.biome_weight_2 = vec4<f32>(0., 0., 0., 0.);
    return out;
}

@fragment
fn fragment(
    @builtin(front_facing) is_front: bool,
    mesh: VertexOutput,
) -> @location(0) vec4<f32> {
    let uv = vec2<f32>(mesh.world_position.xz);

	let slope = 1.0 - pow(mesh.slope, 2.);

    let bw_dirt = mesh.biome_weight_0.y;
    let bw_grass = mesh.biome_weight_0.z;

    var sfc: SurfaceAccum;
    sfc.color = vec4<f32>(0., 0., 0., 1.);
    sfc.under_darken = 0.;
    sfc.terrain_noise = textureSample(noise, noise_sampler, fract(uv * 0.05)).x;

    var pbr_input: fns::PbrInput = fns::pbr_input_new();

    let dirt_color = textureSample(dirt, dirt_sampler, fract(uv * 0.1));

	// vec3 underColor = dirtColor.xyz;
	let under_roughness = 0.9 - (dirt_color.r - dirt_color.g - dirt_color.b) * 0.8; // Roughness for underlayers
	var under_mix = 0.0; // Mix factor for top layer and underlayer
	// var under_darken = 0.0; // Top layers get slightly darker near edges.
	// let under_noise = dot(persist0_6, terrainNoise_2_5) * 0.5;

	// vec4 dirtColor = texture2D(dirtTexture, uv * 0.1);

	let slope_mix = slope + sfc.terrain_noise * 0.5;
	under_mix = max(under_mix, smoothstep(0.35, 0.55, slope_mix));
	sfc.under_darken = max(sfc.under_darken, smoothstep(0.1, 0.6, slope_mix));

	// No top coats underwater
	// under_mix = min(max(under_mix, -mesh.world_position.y * 3.), 1.);
    var under_color = dirt_color.xyz;

    let grass_color = textureSample(grass, grass_sampler, fract(uv * 0.35));
	if bw_grass > 0. {
        blend_biome(&sfc, bw_grass, grass_color.rgb);
	}

	// Mix top layer and under layer.
	let combined = mix(sfc.color, vec4<f32>(under_color, under_roughness), under_mix);
	let diffuse_color = vec4<f32>(combined.xyz, 1.0);
	let roughness = combined.w;

	// If underwater, then mix in dark blue
	// diffuseColor = mix(diffuseColor, vec4(waterColor, 1.), clamp(-0.2-vPosition.y, 0., 0.7));

    pbr_input.material.base_color = diffuse_color;
    // pbr_input.material.base_color = vec4<f32>(under_mix, under_mix, under_mix, 1.);

    // pbr_input.material.base_color = vec4<f32>(p * 0.5 + 0.5, 0., 0., 1.);

    pbr_input.material.perceptual_roughness = roughness;
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
