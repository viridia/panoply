use bevy::{pbr::ExtendedMaterial, prelude::*, render::render_resource::Face, utils::HashMap};
use panoply_exemplar::InstanceType;
use precinct_cache::{spawn_precincts, PrecinctCache};

use crate::materials::{OutlineMaterial, OutlineMaterialExtension};

use self::{
    floor_aspect::{FloorGeometry, FloorNav, NoiseFloorSurface, StdFloorSurface},
    floor_mesh::{
        gen_floor_meshes, insert_floor_meshes, rebuild_floor_materials, update_floor_aspects,
    },
    // floor_noise::FloorNoiseMaterial,
    precinct::read_precinct_data,
    precinct_asset::{PrecinctAsset, PrecinctAssetLoader},
    scenery_aspect::{LightSource, ModelComponent, SceneryColliders, SceneryMarks, SceneryModels},
    scenery_colliders::{ColliderDesc, ColliderShape, ColliderType},
    scenery_element::{spawn_se_model_instances, spawn_se_models, update_se_aspects},
    terrain_fx_aspect::{TerrainEffect, TerrainHole},
    terrain_fx_map::{rebuild_parcel_terrain_fx, rebuild_terrain_fx_vertex_attrs},
    wall_aspect::WallSize,
};

pub mod floor_aspect;
mod floor_mesh;
// mod floor_noise;
mod floor_region;
pub mod precinct;
mod precinct_asset;
mod precinct_cache;
mod rle;
mod scenery_aspect;
mod scenery_colliders;
mod scenery_element;
mod terrain_fx_aspect;
mod terrain_fx_map;
mod wall_aspect;

pub const PRECINCT_SIZE: i32 = 64;
pub const PRECINCT_SIZE_F: f32 = PRECINCT_SIZE as f32;

pub const FLOOR_THICKNESS: f32 = 0.2; // Thickness of floors
pub const PHYSICS_FLOOR_THICKNESS: f32 = 0.1; // Thickness of floor colliders
pub const TIER_OFFSET: f32 = 0.02 - 2.; // Tiers are slightly higher than the terrain.

pub const WALL_TYPE: InstanceType = InstanceType::from_str("Wall");
pub const FIXTURE_TYPE: InstanceType = InstanceType::from_str("Fixt");
pub const FLOOR_TYPE: InstanceType = InstanceType::from_str("Floor");
pub const TERRAIN_FX_TYPE: InstanceType = InstanceType::from_str("TrFx");

#[derive(Resource, Default)]
pub struct FloorOutline(pub Handle<OutlineMaterial>);

pub struct SceneryPlugin;

impl Plugin for SceneryPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(PrecinctCache::new())
            .init_asset_loader::<PrecinctAssetLoader>()
            .init_asset::<PrecinctAsset>()
            .init_resource::<FloorOutline>()
            .register_type::<StdFloorSurface>()
            .register_type::<NoiseFloorSurface>()
            .register_type::<FloorGeometry>()
            .register_type::<FloorNav>()
            .register_type::<SceneryModels>()
            .register_type::<SceneryColliders>()
            .register_type::<SceneryMarks>()
            .register_type::<LightSource>()
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
            .register_type::<Option<Vec3>>()
            .register_type::<HashMap<String, Vec<Vec3>>>()
            .register_type::<HashMap<String, String>>()
            // .add_plugins(MaterialPlugin::<
            //     ExtendedMaterial<StandardMaterial, FloorNoiseMaterial>,
            // >::default())
            .add_systems(Startup, init_outline)
            .add_systems(
                Update,
                (
                    spawn_precincts,
                    read_precinct_data,
                    // Floor processing
                    update_floor_aspects.after(read_precinct_data),
                    gen_floor_meshes.after(update_floor_aspects),
                    // Wall and fixture processing
                    update_se_aspects.after(read_precinct_data),
                    spawn_se_models.after(update_se_aspects),
                    // TerrainFx processing
                    rebuild_terrain_fx_vertex_attrs.after(read_precinct_data),
                    rebuild_parcel_terrain_fx.after(rebuild_terrain_fx_vertex_attrs),
                    // These poll resource handles so won't run in the same frame anyway
                    insert_floor_meshes,
                    rebuild_floor_materials,
                    spawn_se_model_instances,
                ),
            );
    }
}

fn init_outline(
    mut materials: ResMut<Assets<ExtendedMaterial<StandardMaterial, OutlineMaterialExtension>>>,
    mut floor_outline: ResMut<FloorOutline>,
) {
    floor_outline.0 = materials.add(ExtendedMaterial {
        base: StandardMaterial {
            base_color: Color::BLACK,
            unlit: true,
            cull_mode: Some(Face::Front),
            ..Default::default()
        },
        extension: OutlineMaterialExtension { width: 0.015 },
    });
}
