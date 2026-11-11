use bevy::{platform::collections::HashMap, prelude::*};
use floor_surface::{FloorSurface, FloorSurfaceLoader};
use panoply_exemplar::{InstanceType, InteractionAppExt, ScriptComponentAppExt};
use panoply_precinct::{PrecinctAsset, PrecinctAssetLoader};
use panoply_render::OutlineMaterial;
use precinct_cache::{spawn_precincts, PrecinctCache};

use self::{
    animation_aspect::{
        bind_scenery_animations, drive_scenery_animations, start_pending_sounds, SceneryAnimations,
    },
    container_aspect::{toggle_container, Container, ContainerState, ContainerToggle},
    floor_mesh::{
        gen_floor_meshes, insert_floor_meshes, rebuild_floor_materials, update_floor_aspects,
    },
    lock_aspect::{Lock, LockState},
    precinct::{mark_precinct_data, read_precinct_data},
    scenery_aspect::{LightSource, ModelComponent, SceneryColliders, SceneryMarks, SceneryModels},
    scenery_colliders::{build_se_colliders, ColliderDesc, ColliderShape, ColliderType},
    scenery_element::{spawn_se_model_instances, spawn_se_models, update_se_aspects},
    terrain_fx_aspect::{TerrainEffect, TerrainHole},
    terrain_fx_map::{rebuild_parcel_terrain_fx, rebuild_terrain_fx_vertex_attrs},
    wall_aspect::WallSize,
};

mod animation_aspect;
mod container_aspect;
mod floor_mesh;
pub mod floor_region;
mod floor_surface;
mod lock_aspect;
mod pending;
pub mod precinct;
mod precinct_cache;
mod rle;
mod scenery_aspect;
mod scenery_colliders;
pub mod scenery_element;
mod terrain_fx_aspect;
mod terrain_fx_map;
mod wall_aspect;

pub use panoply_core::metrics::{PRECINCT_SIZE, PRECINCT_SIZE_F};
pub use pending::{PrecinctStatus, SceneryPending};

pub const FLOOR_THICKNESS: f32 = 0.2; // Thickness of floors
pub const PHYSICS_FLOOR_THICKNESS: f32 = 0.1; // Thickness of floor colliders
pub const TIER_OFFSET: f32 = 0.02; // Tiers are slightly higher than the terrain.

pub const WALL_TYPE: InstanceType = InstanceType::from_str("Wall");
pub const FIXTURE_TYPE: InstanceType = InstanceType::from_str("Fixt");
pub const FLOOR_TYPE: InstanceType = InstanceType::from_str("Floor");
pub const TERRAIN_FX_TYPE: InstanceType = InstanceType::from_str("TrFx");

#[derive(Resource, Default)]
pub struct FloorOutline(pub Handle<OutlineMaterial>);

pub struct PanoplySceneryPlugin;

impl Plugin for PanoplySceneryPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(PrecinctCache::new())
            .init_asset_loader::<PrecinctAssetLoader>()
            .init_asset_loader::<FloorSurfaceLoader>()
            .init_asset::<PrecinctAsset>()
            .init_asset::<FloorSurface>()
            .init_resource::<FloorOutline>()
            .register_type::<SceneryModels>()
            .register_type::<SceneryColliders>()
            .register_type::<SceneryMarks>()
            .register_type::<LightSource>()
            .register_type::<Container>()
            .register_type::<ContainerState>()
            .register_type::<Lock>()
            .register_type::<LockState>()
            .register_script_component::<Container, ContainerState>("container")
            .register_script_component::<Lock, LockState>("lock")
            .register_interaction_action::<Container>("ContainerToggle", |world, entity| {
                world.trigger(ContainerToggle { entity });
            })
            .register_type::<SceneryAnimations>()
            .add_observer(bind_scenery_animations)
            .add_observer(toggle_container)
            .add_systems(Update, (drive_scenery_animations, start_pending_sounds))
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
            .add_systems(Startup, init_outline)
            .add_systems(
                Update,
                (
                    spawn_precincts,
                    mark_precinct_data,
                    read_precinct_data.after(mark_precinct_data),
                    // Floor processing
                    update_floor_aspects.after(read_precinct_data),
                    gen_floor_meshes.after(update_floor_aspects),
                    // Wall and fixture processing
                    update_se_aspects.after(read_precinct_data),
                    spawn_se_models.after(update_se_aspects),
                    build_se_colliders.after(update_se_aspects),
                    // TerrainFx processing
                    rebuild_terrain_fx_vertex_attrs.after(read_precinct_data),
                    rebuild_parcel_terrain_fx
                        .after(rebuild_terrain_fx_vertex_attrs)
                        .after(spawn_precincts),
                    // These poll resource handles so won't run in the same frame anyway
                    insert_floor_meshes,
                    rebuild_floor_materials,
                    spawn_se_model_instances,
                ),
            );
    }
}

fn init_outline(
    mut materials: ResMut<Assets<OutlineMaterial>>,
    mut floor_outline: ResMut<FloorOutline>,
) {
    floor_outline.0 = materials.add(OutlineMaterial {
        width: 0.015,
        ..default()
    });
}
