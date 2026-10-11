use bevy::{camera::visibility::RenderLayers, platform::collections::HashMap, prelude::*};
use panoply_exemplar::*;

use super::{
    floor_region::{FloorRegion, RebuildFloorAspects},
    floor_surface::FloorSurface,
    precinct_asset::{PrecinctAsset, SceneryInstanceId},
    rle::rle_decode,
    scenery_element::{SceneryElement, SceneryElementRebuildAspects},
    terrain_fx_map::{RebuildTerrainFxVertexAttrs, TerrainFxMap},
    PRECINCT_SIZE_F,
};

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
///
/// Children: one [`PrecinctTier`] per floor level, and the scenery elements (walls, fixtures).
#[derive(Component, Debug)]
pub struct Precinct {
    pub realm: Entity,
    pub render_layer: RenderLayers,
    pub coords: IVec2,
    pub visible: bool,
    pub asset: Handle<PrecinctAsset>,
}

/// One floor level of a precinct (D24): a child of the precinct, with its floor regions as its
/// children. Tiers are spaced 1 meter apart.
#[derive(Component, Debug, Default)]
pub struct PrecinctTier {
    /// Floor level.
    pub level: i32,

    /// Authored cutaway rectangles, in precinct-local coordinates (see X19).
    pub cutaways: Vec<Rect>,
}

impl Precinct {
    pub fn contains_pt(&self, v: Vec3) -> bool {
        v.x >= self.coords.x as f32 * PRECINCT_SIZE_F
            && v.x < (self.coords.x + 1) as f32 * PRECINCT_SIZE_F
            && v.z >= self.coords.y as f32 * PRECINCT_SIZE_F
            && v.z < (self.coords.y + 1) as f32 * PRECINCT_SIZE_F
    }

    /// Makes the precinct's tier entities, and their floor regions, match the asset.
    pub fn rebuild_tiers(
        &self,
        commands: &mut Commands,
        entity: Entity,
        children: Option<&Children>,
        asset: &PrecinctAsset,
        floor_surfaces: &[Handle<FloorSurface>],
        q_tiers: &Query<(&PrecinctTier, Option<&Children>)>,
        q_floor_regions: &mut Query<&mut FloorRegion>,
    ) {
        let mut existing = HashMap::<i32, Entity>::new();
        for child in children.into_iter().flatten() {
            if let Ok((tier, _)) = q_tiers.get(*child) {
                existing.insert(tier.level, *child);
            }
        }

        for tier in asset.tiers.iter() {
            let (tier_entity, tier_children) = match existing.remove(&tier.level) {
                Some(tier_entity) => {
                    let (current, tier_children) = q_tiers.get(tier_entity).unwrap();
                    if current.cutaways != tier.cutaways {
                        commands.entity(tier_entity).insert(PrecinctTier {
                            level: tier.level,
                            cutaways: tier.cutaways.clone(),
                        });
                    }
                    (tier_entity, tier_children)
                }
                None => {
                    let tier_entity = commands
                        .spawn((
                            Name::new(format!("Tier:{}", tier.level)),
                            PrecinctTier {
                                level: tier.level,
                                cutaways: tier.cutaways.clone(),
                            },
                            Transform::default(),
                            Visibility::Inherited,
                            ChildOf(entity),
                        ))
                        .id();
                    (tier_entity, None)
                }
            };

            // Floor regions are matched to the asset's by position in the list.
            let mut regions = tier_children
                .into_iter()
                .flatten()
                .filter(|child| q_floor_regions.contains(**child))
                .copied()
                .collect::<Vec<_>>()
                .into_iter();
            for floor in tier.pfloors.iter() {
                let surface = floor_surfaces[floor.surface_index].clone();
                match regions.next() {
                    Some(floor_entity) => {
                        let mut floor_region = q_floor_regions.get_mut(floor_entity).unwrap();
                        let mut changed = false;
                        if floor_region.surface != surface {
                            floor_region.surface = surface;
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
                    }
                    None => {
                        commands.spawn((
                            Name::new("FloorRegion"),
                            FloorRegion {
                                level: tier.level,
                                surface,
                                poly: floor.poly.clone(),
                                holes: floor.holes.clone(),
                            },
                            Transform::default(),
                            Visibility::Inherited,
                            self.render_layer.clone(),
                            RebuildFloorAspects,
                            ChildOf(tier_entity),
                        ));
                    }
                }
            }

            // Remove any extra floor regions that no longer exist.
            for floor_entity in regions {
                commands.entity(floor_entity).despawn();
            }
        }

        // Remove any tiers that no longer exist.
        for tier_entity in existing.values() {
            commands.entity(*tier_entity).despawn();
        }
    }

    pub fn rebuild_scenery_elements(
        &self,
        commands: &mut Commands,
        entity: Entity,
        children: Option<&Children>,
        asset: &PrecinctAsset,
        scenery_exemplars: &[Handle<Exemplar>],
        q_scenery_elements: &mut Query<&mut SceneryElement>,
    ) {
        // Build a map of existing scenery elements that are children of this precinct.
        let mut existing = HashMap::<SceneryInstanceId, Entity>::with_capacity(128);
        for child in children.into_iter().flatten() {
            if let Ok(scenery_element) = q_scenery_elements.get(*child) {
                existing.insert(scenery_element.iid.clone(), *child);
            }
        }

        // Iterate through the scenery instances in the asset.
        for sid in asset.scenery.iter() {
            let facing = sid.facing * std::f32::consts::PI / 180.;
            let transform =
                Transform::from_translation(sid.position).with_rotation(Quat::from_rotation_y(facing));
            // Update the scenery instance in place if the exemplar has not changed.
            if let Some(se_ent) = existing.remove(&sid.iid) {
                let mut scenery_element = q_scenery_elements.get_mut(se_ent).unwrap();
                if scenery_element.exemplar == scenery_exemplars[sid.id] {
                    if scenery_element.position != sid.position || scenery_element.facing != facing
                    {
                        scenery_element.position = sid.position;
                        scenery_element.facing = facing;
                        commands.entity(se_ent).insert(transform);
                    }
                    continue;
                }

                // The exemplar changed: replace the instance.
                commands.entity(se_ent).despawn();
            }

            // Otherwise, spawn a new instance.
            commands.spawn((
                Name::new("SceneryElement"),
                SceneryElement {
                    iid: sid.iid.clone(),
                    exemplar: scenery_exemplars[sid.id].clone(),
                    facing,
                    position: sid.position,
                },
                sid.aspects.clone(),
                transform,
                Visibility::Inherited,
                self.render_layer.clone(),
                SceneryElementRebuildAspects,
                ChildOf(entity),
            ));
        }

        // Despawn any remaining scenery elements that were not found in the asset.
        for se_ent in existing.values() {
            commands.entity(*se_ent).despawn();
        }
    }

    /// Decodes the precinct's terrain effects. Every loaded precinct gets a `TerrainFxMap`, empty
    /// if the asset has none, since parcels wait for it before building ground (D11).
    pub fn rebuild_terrain_fx(
        &self,
        commands: &mut Commands,
        entity: Entity,
        asset: &PrecinctAsset,
        fx_exemplars: Vec<Handle<Exemplar>>,
    ) {
        let mut fx = TerrainFxMap::new();
        if let Some(ref encoded) = asset.terrain_fx {
            if let Err(err) = rle_decode(encoded, &mut fx.map) {
                error!("Precinct {:?}: bad terrain effects: {err:?}", self.coords);
            }
            fx.exemplars = fx_exemplars;
        }
        commands
            .entity(entity)
            .insert((fx, RebuildTerrainFxVertexAttrs));
    }
}

/// Marks a precinct whose asset loaded or changed, until its scenery is rebuilt.
#[derive(Component)]
#[component(storage = "SparseSet")]
pub struct ReadPrecinctData;

/** React when precinct assets change: mark the precinct for `read_precinct_data`. */
pub fn mark_precinct_data(
    mut commands: Commands,
    q_precincts: Query<(Entity, &Precinct)>,
    mut ev_asset: MessageReader<AssetEvent<PrecinctAsset>>,
) {
    for ev in ev_asset.read() {
        let (AssetEvent::LoadedWithDependencies { id } | AssetEvent::Modified { id }) = ev else {
            continue;
        };
        if let Some((precinct_entity, _)) = q_precincts.iter().find(|r| r.1.asset.id() == *id) {
            commands.entity(precinct_entity).insert(ReadPrecinctData);
        }
    }
}

/** Update a marked precinct's scenery from its asset, once its exemplar catalogs have loaded. */
pub fn read_precinct_data(
    mut commands: Commands,
    q_precincts: Query<(Entity, &Precinct, Option<&Children>), With<ReadPrecinctData>>,
    q_tiers: Query<(&PrecinctTier, Option<&Children>)>,
    mut q_floor_regions: Query<&mut FloorRegion>,
    mut q_scenery_elements: Query<&mut SceneryElement>,
    assets: Res<Assets<PrecinctAsset>>,
    asset_server: Res<AssetServer>,
    mut exemplars: ExemplarServer,
) {
    for (precinct_entity, precinct, precinct_children) in q_precincts.iter() {
        let Some(precinct_asset) = assets.get(&precinct.asset) else {
            continue;
        };

        // Both lists are requested before either is waited on, so their catalogs load together.
        let scenery_exemplars = exemplars.get_all(&precinct_asset.scenery_types);
        let fx_exemplars = exemplars.get_all(&precinct_asset.terrain_fx_types);
        let (Some(scenery_exemplars), Some(fx_exemplars)) = (scenery_exemplars, fx_exemplars)
        else {
            continue;
        };
        commands.entity(precinct_entity).remove::<ReadPrecinctData>();

        // TODO: Sync nav mesh, physics, light sources, particles, etc.
        // TODO(bevy-0.20): Sync actors, restored with actors (D24).

        let floor_surfaces: Vec<Handle<FloorSurface>> = precinct_asset
            .floor_surfaces
            .iter()
            .map(|s| asset_server.load(s))
            .collect();

        precinct.rebuild_tiers(
            &mut commands,
            precinct_entity,
            precinct_children,
            precinct_asset,
            &floor_surfaces,
            &q_tiers,
            &mut q_floor_regions,
        );

        precinct.rebuild_scenery_elements(
            &mut commands,
            precinct_entity,
            precinct_children,
            precinct_asset,
            &scenery_exemplars,
            &mut q_scenery_elements,
        );

        precinct.rebuild_terrain_fx(&mut commands, precinct_entity, precinct_asset, fx_exemplars);
    }
}
