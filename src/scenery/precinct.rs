use panoply_exemplar::*;

use super::{
    floor_region::{FloorRegion, RebuildFloorAspects},
    precinct_asset::{PrecinctAsset, SceneryInstanceId},
    rle::rle_decode,
    scenery_element::{SceneryElement, SceneryElementRebuildAspects},
    terrain_fx_map::{RebuildTerrainFxVertexAttrs, TerrainFxMap},
    PRECINCT_SIZE_F,
};
use bevy::{prelude::*, render::view::RenderLayers, utils::hashbrown::HashMap};

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
}

impl Precinct {
    pub fn contains_pt(&self, v: Vec3) -> bool {
        v.x >= self.coords.x as f32 * PRECINCT_SIZE_F
            && v.x < (self.coords.x + 1) as f32 * PRECINCT_SIZE_F
            && v.z >= self.coords.y as f32 * PRECINCT_SIZE_F
            && v.z < (self.coords.y + 1) as f32 * PRECINCT_SIZE_F
    }

    pub fn rebuild_tiers(
        &mut self,
        commands: &mut Commands,
        entity: Entity,
        asset: &PrecinctAsset,
        floor_exemplars: &[Handle<Exemplar>],
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
                let exemplar: Handle<Exemplar> = floor_exemplars[floor.surface_index].clone();
                if j < t.floor_regions.len() {
                    let floor_entity = t.floor_regions[j];
                    if let Ok((floor_entity, mut floor_region)) =
                        query_floor_regions.get_mut(floor_entity)
                    {
                        // Patch floor entity.
                        let mut changed = false;
                        if floor_region.exemplar != exemplar {
                            floor_region.exemplar = exemplar.clone();
                            changed = true;
                        }
                        if floor_region.poly != floor.poly || floor_region.holes != floor.holes {
                            floor_region.poly.clone_from(&floor.poly);
                            floor_region.holes.clone_from(&floor.holes);
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
                                exemplar,
                                poly: floor.poly.clone(),
                                holes: floor.holes.clone(),
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
                                exemplar,
                                poly: floor.poly.clone(),
                                holes: floor.holes.clone(),
                            },
                            self.render_layer.clone(),
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

    pub fn rebuild_scenery_elements(
        &mut self,
        commands: &mut Commands,
        entity: Entity,
        children: Option<&Children>,
        asset: &PrecinctAsset,
        scenery_exemplars: &[Handle<Exemplar>],
        query_scenery_elements: &mut Query<&mut SceneryElement>,
    ) {
        let mut child_map = HashMap::<SceneryInstanceId, Entity>::with_capacity(128);
        if let Some(children) = children {
            for child in children.iter() {
                if let Ok(scenery_element) = query_scenery_elements.get_mut(*child) {
                    child_map.insert(scenery_element.iid.clone(), *child);
                }
            }
        }

        for elt in asset.scenery.iter() {
            let mut transform = Transform::from_translation(elt.position);
            let facing = elt.facing * std::f32::consts::PI / 180.;
            transform.rotate(Quat::from_rotation_y(facing));
            if let Some(se_ent) = child_map.remove(&elt.iid) {
                if let Ok(mut scenery_element) = query_scenery_elements.get_mut(se_ent) {
                    if scenery_element.exemplar == scenery_exemplars[elt.id] {
                        if scenery_element.position != elt.position {
                            scenery_element.position = elt.position;
                            transform.translation = elt.position;
                            commands.entity(se_ent).insert(transform);
                        }
                        if scenery_element.facing != facing {
                            scenery_element.facing = facing;
                            transform.rotation = Quat::from_rotation_y(facing);
                            commands.entity(se_ent).insert(transform);
                        }
                        continue;
                    }

                    commands.entity(se_ent).remove_parent();
                    commands.entity(se_ent).despawn_recursive();
                }
            }

            commands
                .spawn((
                    SceneryElement {
                        iid: elt.iid.clone(),
                        exemplar: scenery_exemplars[elt.id].clone(),
                        facing,
                        position: elt.position,
                    },
                    elt.aspects.clone(),
                    SpatialBundle {
                        transform,
                        ..default()
                    },
                    self.render_layer.clone(),
                    SceneryElementRebuildAspects,
                ))
                .set_parent(entity);
        }

        for se_ent in child_map.values() {
            commands.entity(*se_ent).remove_parent();
            commands.entity(*se_ent).despawn_recursive();
        }
    }

    pub fn rebuild_terrain_fx(
        &mut self,
        commands: &mut Commands,
        entity: Entity,
        asset: &PrecinctAsset,
        fx_exemplars: Vec<Handle<Exemplar>>,
    ) {
        if let Some(ref encoded) = asset.terrain_fx {
            let mut fx = TerrainFxMap::new();
            rle_decode(encoded, &mut fx.map).unwrap();
            fx.exemplars = fx_exemplars;
            commands
                .entity(entity)
                .insert((fx, RebuildTerrainFxVertexAttrs));
        }
    }

    pub fn rebuild_actors(
        &mut self,
        _commands: &mut Commands,
        _entity: Entity,
        asset: &PrecinctAsset,
    ) {
        for _ai in asset.actors.iter() {
            // let _exemplar = asset_server.load::<Exemplar>(ai.exemplar.clone());
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
    mut query_precincts: Query<(Entity, &mut Precinct, Option<&Children>)>,
    mut query_floor_regions: Query<(Entity, &mut FloorRegion)>,
    mut query_scenery_elements: Query<&mut SceneryElement>,
    mut ev_asset: EventReader<AssetEvent<PrecinctAsset>>,
    assets: ResMut<Assets<PrecinctAsset>>,
    asset_server: Res<AssetServer>,
) {
    for ev in ev_asset.read() {
        match ev {
            AssetEvent::Added { id }
            | AssetEvent::LoadedWithDependencies { id }
            | AssetEvent::Modified { id } => {
                if let Some((precinct_entity, mut precinct, precinct_children)) =
                    query_precincts.iter_mut().find(|r| r.1.asset.id() == *id)
                {
                    // TODO: Sync cutaway rects
                    // TODO: Sync nav mesh, physics, light sources, particles, etc.
                    // TODO: Sync actors

                    let precinct_asset = assets.get(*id).unwrap();
                    let floor_exemplars: Vec<Handle<Exemplar>> = precinct_asset
                        .floor_types
                        .iter()
                        .map(|s| asset_server.load(s))
                        .collect();

                    precinct.rebuild_tiers(
                        &mut commands,
                        precinct_entity,
                        precinct_asset,
                        &floor_exemplars,
                        &mut query_floor_regions,
                    );

                    let scenery_exemplars: Vec<Handle<Exemplar>> = precinct_asset
                        .scenery_types
                        .iter()
                        .map(|s| asset_server.load(s))
                        .collect();

                    precinct.rebuild_scenery_elements(
                        &mut commands,
                        precinct_entity,
                        precinct_children,
                        precinct_asset,
                        &scenery_exemplars,
                        &mut query_scenery_elements,
                    );

                    let fx_exemplars: Vec<Handle<Exemplar>> = precinct_asset
                        .terrain_fx_types
                        .iter()
                        .map(|s| asset_server.load(s))
                        .collect();

                    precinct.rebuild_terrain_fx(
                        &mut commands,
                        precinct_entity,
                        precinct_asset,
                        fx_exemplars,
                    );

                    precinct.rebuild_actors(&mut commands, precinct_entity, precinct_asset);

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
