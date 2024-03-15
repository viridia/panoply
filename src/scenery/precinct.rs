use crate::instancing::InstanceMap;
use panoply_exemplar::*;

use super::{
    floor_region::{FloorRegion, RebuildFloorAspects},
    precinct_asset::PrecinctAsset,
    rle::rle_decode,
    scenery_element::{SceneryElement, SceneryElementRebuildAspects},
    terrain_fx_map::{RebuildTerrainFxVertexAttrs, TerrainFxMap},
};
use bevy::{prelude::*, render::view::RenderLayers};

#[derive(Eq, PartialEq, Hash)]
pub struct PrecinctKey {
    pub realm: Entity,
    pub x: i32,
    pub z: i32,
}

/// A precinct is a 64x64 meter area of the world. Precincts store scenery elements and other
/// authored content such as terrain effects. Unlike parcels, precincts are not cloned across the
/// world, and are unique to a realm. Precincts deberately have a different grid size than parcels
/// as a way of reducing the amount of visual repetition in the map.
#[derive(Component, Debug)]
pub struct Precinct {
    pub realm: Entity,
    pub render_layer: RenderLayers,
    pub coords: IVec2,
    pub visible: bool,
    pub asset: Handle<PrecinctAsset>,
    pub tiers: Vec<PrecinctTier>,
    pub scenery_instances: InstanceMap,
}

impl Precinct {
    pub fn rebuild_tiers(
        &mut self,
        commands: &mut Commands,
        entity: Entity,
        asset: &PrecinctAsset,
        floor_schematics: &[Handle<Exemplar>],
        query_floor_regions: &mut Query<(Entity, &mut FloorRegion)>,
    ) {
        // Sync tiers
        let mut i = 0;
        for tier in asset.tiers.iter() {
            // Remove old tiers that are no longer in the asset.
            while i < self.tiers.len() && self.tiers[i].level < tier.level {
                self.tiers.remove(i);
            }

            // Create or mutate a new tier
            let t = if i < self.tiers.len() {
                &mut self.tiers[i]
            } else {
                let new_tier = PrecinctTier {
                    level: tier.level,
                    floor_regions: Vec::new(),
                };
                self.tiers.insert(i, new_tier);
                &mut self.tiers[i]
            };
            i += 1;

            let mut j = 0;
            for floor in tier.pfloors.iter() {
                let schematic: Handle<Exemplar> = floor_schematics[floor.surface_index].clone();
                if j < t.floor_regions.len() {
                    let floor_entity = t.floor_regions[j];
                    if let Ok((floor_entity, mut floor_region)) =
                        query_floor_regions.get_mut(floor_entity)
                    {
                        // Patch floor entity.
                        let mut changed = false;
                        if floor_region.schematic != schematic {
                            floor_region.schematic = schematic.clone();
                            changed = true;
                        }
                        if floor_region.poly != floor.poly {
                            floor_region.poly = floor.poly.clone();
                            changed = true;
                        }

                        if changed {
                            commands.entity(floor_entity).insert(RebuildFloorAspects);
                        }
                    } else {
                        // Overwrite floor entity components.
                        commands.entity(floor_entity).insert((
                            FloorRegion {
                                level: tier.level,
                                schematic,
                                poly: floor.poly.clone(),
                            },
                            RebuildFloorAspects,
                        ));
                    }
                } else {
                    // Insert new floor entity.
                    let floor_entity = commands
                        .spawn((
                            FloorRegion {
                                level: tier.level,
                                schematic,
                                poly: floor.poly.clone(),
                            },
                            self.render_layer,
                            RebuildFloorAspects,
                        ))
                        .set_parent(entity)
                        .id();
                    t.floor_regions.push(floor_entity);
                }
                j += 1;
            }

            // Remove any extra floor regions that no longer exist.
            while t.floor_regions.len() > j {
                println!("Removing floor region.");
                let e = t.floor_regions.pop().unwrap();
                commands.entity(e).remove_parent();
                commands.entity(e).despawn_recursive();
            }
        }

        // Remove any extra tiers that no longer exist.
        while i < self.tiers.len() {
            self.tiers.remove(i);
        }
    }

    pub fn rebuild_walls(
        &mut self,
        commands: &mut Commands,
        entity: Entity,
        asset: &PrecinctAsset,
        scenery_schematics: &[Handle<Exemplar>],
    ) {
        // What do we want to do here?
        // We cannot place the model yet, because transforms are not loaded.
        // We need to spawn an entity per secenery element I think.
        for elt in asset.scenery.iter() {
            let mut transform = Transform::from_xyz(elt.position.x, elt.position.y, elt.position.z);
            let facing = elt.facing * std::f32::consts::PI / 180.;
            transform.rotate(Quat::from_rotation_y(facing));
            commands
                .spawn((
                    SceneryElement {
                        schematic: scenery_schematics[elt.id].clone(),
                        facing,
                        position: elt.position,
                    },
                    elt.aspects.clone(),
                    SpatialBundle {
                        transform,
                        ..default()
                    },
                    self.render_layer,
                    SceneryElementRebuildAspects,
                ))
                .set_parent(entity);
        }
    }

    pub fn rebuild_terrain_fx(
        &mut self,
        commands: &mut Commands,
        entity: Entity,
        asset: &PrecinctAsset,
        fx_schematics: Vec<Handle<Exemplar>>,
    ) {
        if let Some(ref encoded) = asset.terrain_fx {
            let mut fx = TerrainFxMap::new();
            rle_decode(encoded, &mut fx.map).unwrap();
            fx.schematics = fx_schematics;
            commands
                .entity(entity)
                .insert((fx, RebuildTerrainFxVertexAttrs));
        }
    }
}

#[derive(Debug, Default)]
pub struct PrecinctTier {
    /// Floor level. Floors are spaced 1 meter apart.
    pub level: i32,

    /// List of polygonal floor regions.
    pub floor_regions: Vec<Entity>,
    // public floorObstacles: ComputedFloorRegionObstacles;

    // private floorPhysics: ComputedFloorPhysics;
    // private floorMesh: ComputedFloorMesh;
    // private wallPhysics: ComputedWallPhysics;
    // private cutawayRects: ICutawayRect[] = [];
    // private cutawayRectsAtom = createAtom();
}

#[derive(Component)]
#[component(storage = "SparseSet")]
pub struct PrecinctAssetChanged;

#[derive(Component)]
#[component(storage = "SparseSet")]
pub struct PrecinctTiersChanged;

#[derive(Component)]
#[component(storage = "SparseSet")]
pub struct PrecinctRebuildScenery;

/** React when precinct assets change and update the scenery. */
pub fn read_precinct_data(
    mut commands: Commands,
    mut query_precincts: Query<(Entity, &mut Precinct)>,
    mut query_floor_regions: Query<(Entity, &mut FloorRegion)>,
    mut ev_asset: EventReader<AssetEvent<PrecinctAsset>>,
    assets: ResMut<Assets<PrecinctAsset>>,
    asset_server: Res<AssetServer>,
) {
    for ev in ev_asset.read() {
        match ev {
            AssetEvent::Added { id }
            | AssetEvent::LoadedWithDependencies { id }
            | AssetEvent::Modified { id } => {
                if let Some((precinct_entity, mut precinct)) =
                    query_precincts.iter_mut().find(|r| r.1.asset.id() == *id)
                {
                    // TODO: Sync cutaway rects
                    // TODO: Sync nav mesh, physics, light sources, particles, etc.
                    // TODO: Sync actors

                    let precinct_asset = assets.get(*id).unwrap();
                    let floor_schematics: Vec<Handle<Exemplar>> = precinct_asset
                        .floor_types
                        .iter()
                        .map(|s| asset_server.load(s))
                        .collect();

                    precinct.rebuild_tiers(
                        &mut commands,
                        precinct_entity,
                        precinct_asset,
                        &floor_schematics,
                        &mut query_floor_regions,
                    );

                    let scenery_schematics: Vec<Handle<Exemplar>> = precinct_asset
                        .scenery_types
                        .iter()
                        .map(|s| asset_server.load(s))
                        .collect();

                    precinct.rebuild_walls(
                        &mut commands,
                        precinct_entity,
                        precinct_asset,
                        &scenery_schematics,
                    );

                    let fx_schematics: Vec<Handle<Exemplar>> = precinct_asset
                        .terrain_fx_types
                        .iter()
                        .map(|s| asset_server.load(s))
                        .collect();

                    precinct.rebuild_terrain_fx(
                        &mut commands,
                        precinct_entity,
                        precinct_asset,
                        fx_schematics,
                    );

                    commands
                        .entity(precinct_entity)
                        .remove::<PrecinctAssetChanged>();
                }
            }

            AssetEvent::Removed { id } => {
                let _ = id;
            }

            AssetEvent::Unused { id: _ } => {}
        }
    }
}
