use crate::schematic::Schematic;

use super::{
    floor_region::{FloorRegion, RebuildFloorAspects},
    precinct_asset::PrecinctAsset,
};
use bevy::prelude::*;

#[derive(Eq, PartialEq, Hash)]
pub struct PrecinctKey {
    pub realm: Entity,
    pub x: i32,
    pub z: i32,
}

#[derive(Component, Debug)]
pub struct Precinct {
    pub realm: Entity,
    pub coords: IVec2,
    pub visible: bool,
    pub asset: Handle<PrecinctAsset>,
    pub tiers: Vec<PrecinctTier>,
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
                    let precinct_asset = assets.get(*id).unwrap();
                    let floor_schematics: Vec<Handle<Schematic>> = precinct_asset
                        .floor_types
                        .iter()
                        .map(|s| asset_server.load(s))
                        .collect();

                    // TODO: Sync cutaway rects
                    // TODO: Sync nav mesh, physics, light sources, particles, etc.
                    // TODO: Sync actors

                    // Sync tiers
                    let mut i = 0;
                    for tier in precinct_asset.tiers.iter() {
                        // Remove old tiers that are no longer in the asset.
                        while i < precinct.tiers.len() && precinct.tiers[i].level < tier.level {
                            precinct.tiers.remove(i);
                        }

                        // Create or mutate a new tier
                        let t = if i < precinct.tiers.len() {
                            &mut precinct.tiers[i]
                        } else {
                            let new_tier = PrecinctTier {
                                level: tier.level,
                                floor_regions: Vec::new(),
                            };
                            precinct.tiers.insert(i, new_tier);
                            &mut precinct.tiers[i]
                        };
                        i += 1;

                        let mut j = 0;
                        for floor in tier.pfloors.iter() {
                            let schematic: Handle<Schematic> =
                                floor_schematics[floor.surface_index - 1].clone();
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
                                        RebuildFloorAspects,
                                    ))
                                    .set_parent(precinct_entity)
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
                    while i < precinct.tiers.len() {
                        precinct.tiers.remove(i);
                    }

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
