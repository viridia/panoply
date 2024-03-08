use bevy::{pbr::ExtendedMaterial, prelude::*, render::render_resource::Face, utils::HashMap};
use precinct_cache::{spawn_precincts, PrecinctCache};

use crate::materials::OutlineMaterial;

use self::{
    floor_aspect::{FloorGeometry, FloorNav, NoiseFloorSurface, StdFloorSurface},
    floor_mesh::{
        gen_floor_meshes, insert_floor_meshes, rebuild_floor_materials, update_floor_aspects,
    },
    floor_noise::FloorNoiseMaterial,
    msgpack_extension::Vector3,
    precinct::read_precinct_data,
    precinct_asset::{PrecinctAsset, PrecinctAssetLoader},
    scenery_aspect::{ModelComponent, SceneryColliders, SceneryMarks, SceneryModels},
    scenery_colliders::{ColliderDesc, ColliderShape, ColliderType},
    terrain_fx_aspect::{TerrainEffect, TerrainHole},
    wall_aspect::WallSize,
};

pub mod floor_aspect;
mod floor_mesh;
mod floor_noise;
mod floor_region;
mod msgpack_extension;
mod precinct;
mod precinct_asset;
mod precinct_cache;
mod scenery_aspect;
mod scenery_colliders;
mod terrain_fx_aspect;
mod wall_aspect;

pub const PRECINCT_SIZE: i32 = 64;
pub const PRECINCT_SIZE_F: f32 = PRECINCT_SIZE as f32;

pub const FLOOR_THICKNESS: f32 = 0.2; // Thickness of floors
pub const PHYSICS_FLOOR_THICKNESS: f32 = 0.1; // Thickness of floor colliders
pub const TIER_OFFSET: f32 = 0.02 - 2.; // Tiers are slightly higher than the terrain.

#[derive(Resource, Default)]
pub struct FloorOutline(pub Handle<ExtendedMaterial<StandardMaterial, OutlineMaterial>>);

pub struct SceneryPlugin;

impl Plugin for SceneryPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(PrecinctCache::new())
            .register_asset_loader(PrecinctAssetLoader)
            .init_asset::<PrecinctAsset>()
            .init_resource::<FloorOutline>()
            .register_type::<StdFloorSurface>()
            .register_type::<NoiseFloorSurface>()
            .register_type::<FloorGeometry>()
            .register_type::<FloorNav>()
            .register_type::<SceneryModels>()
            .register_type::<SceneryColliders>()
            .register_type::<SceneryMarks>()
            .register_type::<WallSize>()
            .register_type::<TerrainEffect>()
            .register_type::<TerrainHole>()
            .register_type::<ModelComponent>()
            .register_type::<Vec<ModelComponent>>()
            .register_type::<ColliderDesc>()
            .register_type::<ColliderShape>()
            .register_type::<ColliderType>()
            .register_type::<Vec<ColliderDesc>>()
            .register_type::<Vec<String>>()
            .register_type::<HashMap<String, Vec<Vec3>>>()
            .register_type::<Vector3>()
            .register_type::<HashMap<String, String>>()
            .add_plugins(MaterialPlugin::<
                ExtendedMaterial<StandardMaterial, FloorNoiseMaterial>,
            >::default())
            .add_systems(Startup, init_outline)
            .add_systems(
                Update,
                (
                    spawn_precincts,
                    (
                        read_precinct_data,
                        update_floor_aspects,
                        apply_deferred,
                        gen_floor_meshes,
                    )
                        .chain(),
                    insert_floor_meshes,
                    rebuild_floor_materials,
                ),
            );
    }
}

fn init_outline(
    mut materials: ResMut<Assets<ExtendedMaterial<StandardMaterial, OutlineMaterial>>>,
    mut floor_outline: ResMut<FloorOutline>,
) {
    floor_outline.0 = materials.add(ExtendedMaterial {
        base: StandardMaterial {
            base_color: Color::BLACK,
            unlit: true,
            cull_mode: Some(Face::Front),
            ..Default::default()
        },
        extension: OutlineMaterial { width: 0.015 },
    });
}
