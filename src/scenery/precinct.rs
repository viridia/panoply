use crate::schematic::Schematic;

use super::{
    floor_region::{FloorRegion, FloorRegionMesh},
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

impl PrecinctTier {
    pub fn add_floor_region(&mut self, commands: &mut Commands, floor: FloorRegionMesh) {
        let _ = commands.spawn(floor).id();
        // self.floor_regions.push(floor);
    }
}

#[derive(Component)]
#[component(storage = "SparseSet")]
pub struct PrecinctAssetChanged;

#[derive(Component)]
#[component(storage = "SparseSet")]
pub struct PrecinctTiersChanged;

#[derive(Component)]
#[component(storage = "SparseSet")]
pub struct FloorRegionChanged;

/** React when precinct assets change and update the scenery. */
pub fn read_precinct_data(
    mut commands: Commands,
    mut query_precincts: Query<(Entity, &mut Precinct)>,
    mut ev_asset: EventReader<AssetEvent<PrecinctAsset>>,
    assets: ResMut<Assets<PrecinctAsset>>,
    // mut materials: ResMut<Assets<GroundMaterial>>,
    // mut images: ResMut<Assets<Image>>,
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

                    // TODO: Sync tiers
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

                        t.floor_regions.iter().for_each(|e| {
                            commands.entity(*e).remove_parent();
                            commands.entity(*e).despawn_recursive();
                        });
                        t.floor_regions.clear();
                        for floor in tier.pfloors.iter() {
                            let schematic: Handle<Schematic> =
                                floor_schematics[floor.surface_index - 1].clone();
                            let floor_entity = commands
                                .spawn((
                                    FloorRegion {
                                        schematic,
                                        poly: floor.poly.clone(),
                                    },
                                    FloorRegionChanged,
                                ))
                                .set_parent(precinct_entity)
                                .id();
                            t.floor_regions.push(floor_entity);
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
