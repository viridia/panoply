#import bevy_core_pipeline::tonemapping::tone_mapping
#import bevy_pbr::{
    mesh_view_bindings::view,
    mesh_functions as mfns,
    mesh_bindings::mesh,
    mesh_types::MESH_FLAGS_SHADOW_RECEIVER_BIT,
    pbr_types::{PbrInput, pbr_input_new},
    pbr_functions as fns,
}

@group(2) @binding(1)
var noise: texture_2d<f32>;
@group(2) @binding(2)
var noise_sampler: sampler;

@group(2) @binding(3)
var grass: texture_2d<f32>;
@group(2) @binding(4)
var grass_sampler: sampler;

@group(2) @binding(5)
var dirt: texture_2d<f32>;
@group(2) @binding(6)
var dirt_sampler: sampler;

@group(2) @binding(7)
var moss: texture_2d<f32>;
@group(2) @binding(8)
var moss_sampler: sampler;

@group(2) @binding(9)
var cobbles: texture_2d<f32>;
@group(2) @binding(10)
var cobbles_sampler: sampler;

@group(2) @binding(20)
var biomes: texture_2d<u32>;
// @group(1) @binding(10)
// var biomes_sampler: sampler;

@group(2) @binding(22)
var<uniform> water_color: vec4<f32>;

@group(2) @binding(23)
var<uniform> realm_offset: vec2<f32>;

const NUM_GROUND_TYPES = 9u;
const GT_ROCK = 0u;
const GT_DIRT = 1u;
const GT_GRASS = 2u;
const GT_MOSS = 3u;
const GT_TAIGA = 4u;
const GT_SAND = 5u;
const GT_TUNDRA = 6u;
const GT_SNOW = 7u;
const GT_CHAPARRAL = 8u;

const UV_ROT = mat2x2<f32>(
    vec2<f32>(0.8775825618903728, 0.479425538604203),
    vec2<f32>(-0.479425538604203, 0.8775825618903728));

// struct BiomeWeights {
//     weights: array<f32, 10>,
// }

struct Vertex {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) terrain_style: vec2<u32>,
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
    // @location(6) biome_weights: BiomeWeights,

    @location(7) terrain_style: vec4<f32>,
};

struct BiomeSurfaceAttrs {
    // Roughness of this biome surface
    roughness: f32,
    blend_var: f32,
    blend_t0: f32,
    blend_t1: f32,
    edge_var: f32,
    edge_t0: f32,
    edge_t1: f32,

    // Texture scale
    tx_scale: f32,

    // Darkened color which shows up at edges of top surface (near roads etc.).
    edge_tint: vec3<f32>,
}

const dirt_biome = BiomeSurfaceAttrs(
    0.99,
    0.1, 0.4, 0.5,
    0.5, 0.45, 0.9,
    0.1, // tx_scale
    vec3<f32>(0.65, 0.65, 0.65));
const grass_biome = BiomeSurfaceAttrs(
    0.99,
    0.5, 0.45, 0.55,
    0.5, 0.48, 0.60,
    0.35, // tx_scale
    vec3<f32>(0.20, 0.5, 0.30));
const moss_biome = BiomeSurfaceAttrs(
    0.99,
    0.8, 0.65, 0.67,
    0.8, 0.65, 0.85,
    0.45, // tx_scale
    vec3<f32>(0.6, 0.6, 0.6));

struct TerrainFxSurfaceAttrs {
    // Roughness of this biome surface
    // roughness: f32,
    // blend_var: f32,
    // blend_t0: f32,
    // blend_t1: f32,
    // edge_var: f32,
    // edge_t0: f32,
    // edge_t1: f32,

    // Texture scale
    tx_scale: f32,

    // Darkened color which shows up at edges of top surface (near roads etc.).
    // edge_tint: vec3<f32>,
}

const cobbles_fx = TerrainFxSurfaceAttrs(
    0.45, // tx_scale
);

struct SurfaceAccum {
    color: vec4<f32>,
    terrain_noise: f32,
    under_noise: f32,
    under_mix: f32,
    under_darken: f32,
}

fn blend_biome(
        out: ptr<function, SurfaceAccum>,
        biome: BiomeSurfaceAttrs,
        weight: f32,
        tx_color: vec3<f32>) {
    let blend = smoothstep(
        biome.blend_t0, biome.blend_t1, weight + (*out).terrain_noise * biome.blend_var);
    let edge_blend = smoothstep(
        biome.edge_t0, biome.edge_t1, weight + (*out).terrain_noise * biome.edge_var);
    (*out).color = mix(
        (*out).color,
        vec4(
            mix(
                tx_color * biome.edge_tint,
                tx_color,
                min(edge_blend, 1. - (*out).under_darken)
            ),
            biome.roughness
        ),
        blend);
}

@vertex
fn vertex(vertex: Vertex, @builtin(instance_index) instance_index: u32) -> VertexOutput {
    var out: VertexOutput;
    var wposition = mfns::mesh_position_local_to_world(
        mfns::get_world_from_local(instance_index),
        vec4<f32>(vertex.position, 1.0)
    );

    let parcel_coords = floor(wposition.xz / 16.0) - realm_offset;
    let parcel_coords_i = vec2<i32>(i32(parcel_coords.x), i32(parcel_coords.y));
    let biome_selection = vec4<u32>(
        textureLoad(biomes, parcel_coords_i, 0).r,
        textureLoad(biomes, (parcel_coords_i + vec2(0, 1)), 0).r,
        textureLoad(biomes, (parcel_coords_i + vec2(1, 0)), 0).r,
        textureLoad(biomes, (parcel_coords_i + vec2(1, 1)), 0).r
    );

    let parcel_uv = fract(wposition.xz / 16.);
    let biome_interpolation = vec4<f32>(
        (1. - parcel_uv.x) * (1. - parcel_uv.y),
        (1. - parcel_uv.x) * parcel_uv.y,
        parcel_uv.x * (1. - parcel_uv.y),
        parcel_uv.x * parcel_uv.y
    );

    // Compute weights for each ground cover type.
    var first_layer = true;
    var biome_weight = array<f32, NUM_GROUND_TYPES>();
    for (var i = 0u; i < NUM_GROUND_TYPES; i++) {
        var eq = vec4<f32>();
        eq.x = select(0., 1., i == biome_selection.x);
        eq.y = select(0., 1., i == biome_selection.y);
        eq.z = select(0., 1., i == biome_selection.z);
        eq.w = select(0., 1., i == biome_selection.w);
        // let eq = vec4(equal(vec4<u32>(i), biome_selection));
        var weight = dot(eq, biome_interpolation);
        // The first non-zero layer covers the entire parcel.
        if (dot(eq, eq) > 0. && first_layer) {
            weight = 1.;
            first_layer = false;
        }
        biome_weight[i] = weight;
    }

    out.world_position = wposition;
    out.position = mfns::mesh_position_local_to_clip(
        mfns::get_world_from_local(instance_index),
        vec4<f32>(vertex.position, 1.0)
    );

    out.terrain_style = vec4(
		f32(vertex.terrain_style.x & 0xffu) * 1.0 / 255.0,
		f32((vertex.terrain_style.x >> 8u) & 0xffu) * 1.0 / 255.0,
		f32((vertex.terrain_style.x >> 16u) & 0xffu) * 1.0 / 255.0,
		f32((vertex.terrain_style.x >> 24u) & 0xffu) * 1.0 / 255.0
	);

    out.world_normal = mfns::mesh_normal_local_to_world(vertex.normal, instance_index);
    out.slope = -out.world_normal.y;
    out.biome_weight_0 = vec4<f32>(
        biome_weight[0],
        biome_weight[1],
        biome_weight[2],
        biome_weight[3]
    );
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

	let fx_cobbles = mesh.terrain_style.x;
	let fx_soil = mesh.terrain_style.y;
	let fx_earth = mesh.terrain_style.z;
	// let fx_path = mesh.terrain_style.w;

    let slope = 1.0 - pow(mesh.slope, 2.);

    var sfc: SurfaceAccum;
    sfc.color = vec4<f32>(0., 0., 0., 1.);
    sfc.terrain_noise = textureSample(noise, noise_sampler, fract(uv * 0.04)).x;
    sfc.under_darken = 0.;
    sfc.under_mix = 0.;
    sfc.under_noise = textureSample(noise, noise_sampler, fract(uv * 0.2)).x;

    let dirt_color = textureSample(dirt, dirt_sampler, fract(uv * dirt_biome.tx_scale));

    // vec3 underColor = dirtColor.xyz;
    let under_roughness = 0.9 - (dirt_color.r - dirt_color.g - dirt_color.b) * 0.8; // Roughness for underlayers
    // let under_noise = dot(persist0_6, terrainNoise_2_5) * 0.5;

    let slope_mix = slope + sfc.terrain_noise * 0.5;
    sfc.under_mix = max(sfc.under_mix, smoothstep(0.35, 0.55, slope_mix));
    sfc.under_darken = max(sfc.under_darken, smoothstep(0.1, 0.6, slope_mix));

    var under_color = dirt_color.xyz;

	// Packed and dried earth effects.
    if fx_earth > 0. {
        let fx_blend: f32 = fx_earth;
        // under_color = dirt_color.rgb;
		// float pathBlend = smoothstep(0., .5, fxPath);
		// underColor = mix(
		// 	dirtColor.xyz * 0.5,
		// 	dirtColor.xyz * (1. - vBiomeWeight[Moss] * 0.5),
		// 	pathBlend);
		// underMix = max(underMix, smoothstep(0., 0.9, fxPath + underNoise * 0.6));
		sfc.under_darken = max(
            sfc.under_darken,
            smoothstep(0.2, 0.7, fx_earth * 1.4 + sfc.under_noise * 1.4));
		sfc.under_mix = max(sfc.under_mix, fx_blend);
    }

	// Tilled soil effects - farm fields, graves, etc.
    if fx_soil > 0. {
        let fx_blend: f32 = fx_soil;
        under_color = dirt_color.rgb * vec3f(.7, .6, .6);

		// float soilBlend = smoothstep(0., .3, fxSoil);
		// vec4 soilColor = texture2D(dirtTexture, vPosition.xz * 0.5) * vec4(.7, .6, .6, 1.);
		// underColor = mix(underColor, soilColor.rgb, soilBlend);
		// underRoughness = mix(underRoughness, 0.9, soilBlend);
		// underMix = max(underMix, smoothstep(0.3, 1.1, fxSoil * 2. + underNoise * 0.6));
		sfc.under_darken = max(
            sfc.under_darken,
            smoothstep(0.3, 1.1, fx_soil * 2. + sfc.under_noise * 0.6));
		sfc.under_mix = max(sfc.under_mix, fx_blend);
    }

	// Cobblestone effects.
    if fx_cobbles > 0. {
        let cobbles_color = textureSample(cobbles, cobbles_sampler, fract(uv * UV_ROT * cobbles_fx.tx_scale));
		// vec4 cobblesColor = texture2D(cobblesTexture, vPosition.xz * 0.5);
		let cracks = cobbles_color.r + cobbles_color.g + cobbles_color.b;
		// float noise = dot(persist0_9, terrainNoise_2_5) * 0.5;
		// float roadBlend = smoothstep(0.6, 0.9, fxRoad + underNoise + cracks * 0.9);
        // let fx_blend: f32 = fx_cobbles;
        let fx_blend: f32 = smoothstep(0.6, 0.9, fx_cobbles + sfc.under_noise + cracks * 0.9);
		let dark_blend = smoothstep(0.0, 0.8, fx_cobbles * 1.7 + sfc.under_noise * 1.7 - 0.05);
		// underRoughness = mix(underRoughness, 1.0 - cracks * 0.7, roadBlend);
		sfc.under_mix = max(sfc.under_mix, fx_blend);
        under_color = mix(
            mix(under_color, under_color * 0.7, dark_blend),
            cobbles_color.rgb, fx_blend);
		sfc.under_darken = max(sfc.under_darken, dark_blend);
    }

    // No terrain fx underwater
    sfc.under_mix = max(0., min(sfc.under_mix, 2.0 + mesh.world_position.y * 3.));
    sfc.under_darken = max(0., min(sfc.under_darken, 2.0 + mesh.world_position.y * 3.));

    // Dirt surface
    let bw_dirt = mesh.biome_weight_0.y;
    if bw_dirt > 0. {
        blend_biome(&sfc, dirt_biome, bw_dirt, dirt_color.rgb);
    }

    // Grass surface
    let bw_grass = mesh.biome_weight_0.z;
    if bw_grass > 0. {
        let grass_color = textureSample(grass, grass_sampler, fract(uv * UV_ROT * grass_biome.tx_scale));
        blend_biome(&sfc, grass_biome, bw_grass, grass_color.rgb);
    }

    // Moss surface
    let bw_moss = mesh.biome_weight_0.w;
    if bw_moss > 0. {
        let moss_color = textureSample(moss, moss_sampler, fract(uv * UV_ROT * moss_biome.tx_scale));
        blend_biome(&sfc, moss_biome, bw_moss, moss_color.rgb);
    }

    // Mix top layer and under layer.
    let combined = mix(sfc.color, vec4<f32>(under_color, under_roughness), sfc.under_mix);
    var diffuse_color = vec4<f32>(combined.xyz, 1.0);
    let roughness = combined.w;

    // If underwater, then mix in dark blue
    diffuse_color = mix(diffuse_color, water_color, clamp(-0.2 - mesh.world_position.y, 0., 0.7));

    var pbr_input: PbrInput = pbr_input_new();
    pbr_input.material.base_color = diffuse_color;
    pbr_input.material.perceptual_roughness = roughness;
    pbr_input.frag_coord = mesh.position;
    pbr_input.world_position = mesh.world_position;
    pbr_input.world_normal = fns::prepare_world_normal(
        mesh.world_normal,
        false,
        is_front,
    );
    pbr_input.flags |= MESH_FLAGS_SHADOW_RECEIVER_BIT;

    pbr_input.is_orthographic = false;

    pbr_input.N = normalize(pbr_input.world_normal);
    // let TBN = fns::calculate_tbn_mikktspace(mesh.world_normal, mesh.world_tangent);
    // pbr_input.N = fns::apply_normal_mapping(
    //     pbr_input.material.flags,
    //     TBN,
    //     false, // double_sided,
    //     is_front,
    //     view.mip_bias,
    // );
    pbr_input.V = fns::calculate_view(mesh.world_position, pbr_input.is_orthographic);

    return tone_mapping(fns::apply_pbr_lighting(pbr_input), view.color_grading);
}
