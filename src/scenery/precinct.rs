use panoply_exemplar::*;

use crate::actors::{ActorInstance, ActorRebuildAspects};

use super::{
    floor_region::{FloorRegion, RebuildFloorAspects},
    floor_surface::FloorSurface,
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
    pub actors: Vec<Entity>,
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
        floor_surfaces: &[Handle<FloorSurface>],
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
                // let exemplar = floor_exemplars[floor.surface_index].clone();
                let surface = floor_surfaces[floor.surface_index].clone();
                if j < t.floor_regions.len() {
                    let floor_entity = t.floor_regions[j];
                    if let Ok((floor_entity, mut floor_region)) =
                        query_floor_regions.get_mut(floor_entity)
                    {
                        // Patch floor entity.
                        let mut changed = false;
                        if floor_region.surface != surface {
                            floor_region.surface = surface.clone();
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
                                surface,
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
                            Name::new("FloorRegion"),
                            FloorRegion {
                                level: tier.level,
                                surface,
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
        q_scenery_elements: &mut Query<&mut SceneryElement>,
    ) {
        // Build a map of existing scenery elements that are children of this precinct.
        let mut existing = HashMap::<SceneryInstanceId, Entity>::with_capacity(128);
        if let Some(children) = children {
            for child in children.iter() {
                if let Ok(scenery_element) = q_scenery_elements.get_mut(*child) {
                    existing.insert(scenery_element.iid.clone(), *child);
                }
            }
        }

        // Iterate through the scenery instances in the asset.
        for sid in asset.scenery.iter() {
            let mut transform = Transform::from_translation(sid.position);
            let facing = sid.facing * std::f32::consts::PI / 180.;
            transform.rotate(Quat::from_rotation_y(facing));
            // Update the scenery instance in place if the exemplar has not changed.
            if let Some(se_ent) = existing.remove(&sid.iid) {
                if let Ok(mut scenery_element) = q_scenery_elements.get_mut(se_ent) {
                    if scenery_element.exemplar == scenery_exemplars[sid.id] {
                        if scenery_element.position != sid.position {
                            scenery_element.position = sid.position;
                            transform.translation = sid.position;
                            commands.entity(se_ent).insert(transform);
                        }
                        if scenery_element.facing != facing {
                            scenery_element.facing = facing;
                            transform.rotation = Quat::from_rotation_y(facing);
                            commands.entity(se_ent).insert(transform);
                        }
                        continue;
                    }

                    // Remove the old scenery instance.
                    commands.entity(se_ent).remove_parent();
                    commands.entity(se_ent).despawn_recursive();
                }
            }

            // Otherwise, spawn a new instance.
            commands
                .spawn((
                    Name::new("SceneryElement"),
                    SceneryElement {
                        iid: sid.iid.clone(),
                        exemplar: scenery_exemplars[sid.id].clone(),
                        facing,
                        position: sid.position,
                    },
                    sid.aspects.clone(),
                    transform,
                    Visibility::Visible,
                    self.render_layer.clone(),
                    SceneryElementRebuildAspects,
                ))
                .set_parent(entity);
        }

        // Despawn any remaining scenery elements that were not found in the asset.
        for se_ent in existing.values() {
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
        commands: &mut Commands,
        // realm: Entity,
        asset: &PrecinctAsset,
        q_actors: &mut Query<&mut ActorInstance>,
    ) {
        // Build a map of existing actors. Unlike scenery elements, actors are not children
        // of the precinct entity, however they are owned by it.
        let mut existing = HashMap::<SceneryInstanceId, Entity>::with_capacity(128);
        for actor_id in self.actors.iter() {
            if let Ok(scenery_element) = q_actors.get_mut(*actor_id) {
                existing.insert(scenery_element.iid.clone(), *actor_id);
            }
        }

        // Iterate through the actor instances in the asset.
        for actor_data in asset.actors.iter() {
            let mut transform = Transform::from_translation(actor_data.position);
            let facing = actor_data.facing * std::f32::consts::PI / 180.;
            transform.rotate(Quat::from_rotation_y(facing));
            // let _exemplar = asset_server.load::<Exemplar>(ai.exemplar.clone());
            // Update the actor instance in place if the exemplar has not changed.
            if let Some(a_ent) = existing.remove(&actor_data.iid) {
                if let Ok(mut actor) = q_actors.get_mut(a_ent) {
                    if actor.exemplar == actor_data.exemplar {
                        if actor.position != actor_data.position {
                            actor.position = actor_data.position;
                            transform.translation = actor_data.position;
                            commands.entity(a_ent).insert(transform);
                        }
                        if actor.facing != facing {
                            actor.facing = facing;
                            transform.rotation = Quat::from_rotation_y(facing);
                            commands.entity(a_ent).insert(transform);
                        }
                        continue;
                    }

                    // Remove the old actor instance.
                    commands.entity(a_ent).remove_parent();
                    commands.entity(a_ent).despawn_recursive();
                }
            }

            // Otherwise, spawn a new instance.
            commands.spawn((
                Name::new("Actor"),
                ActorInstance {
                    iid: actor_data.iid.clone(),
                    exemplar: actor_data.exemplar.clone(),
                    facing,
                    position: actor_data.position,
                },
                actor_data.aspects.clone(),
                transform,
                Visibility::Visible,
                self.render_layer.clone(),
                ActorRebuildAspects,
            ));
            // .set_parent(realm);
        }

        // Despawn any remaining actors that were not found in the asset.
        for a_ent in existing.values() {
            commands.entity(*a_ent).remove_parent();
            commands.entity(*a_ent).despawn_recursive();
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
    mut q_precincts: Query<(Entity, &mut Precinct, Option<&Children>)>,
    mut q_floor_regions: Query<(Entity, &mut FloorRegion)>,
    mut q_scenery_elements: Query<&mut SceneryElement>,
    mut q_actors: Query<&mut ActorInstance>,
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
                    q_precincts.iter_mut().find(|r| r.1.asset.id() == *id)
                {
                    // TODO: Sync cutaway rects
                    // TODO: Sync nav mesh, physics, light sources, particles, etc.
                    // TODO: Sync actors

                    let precinct_asset = assets.get(*id).unwrap();
                    let floor_surfaces: Vec<Handle<FloorSurface>> = precinct_asset
                        .floor_surfaces
                        .iter()
                        .map(|s| asset_server.load(s))
                        .collect();

                    precinct.rebuild_tiers(
                        &mut commands,
                        precinct_entity,
                        precinct_asset,
                        &floor_surfaces,
                        &mut q_floor_regions,
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
                        &mut q_scenery_elements,
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

                    precinct.rebuild_actors(&mut commands, precinct_asset, &mut q_actors);

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
