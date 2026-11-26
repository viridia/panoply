use bevy::{
    camera::visibility::RenderLayers,
    ecs::{lifecycle::HookContext, world::DeferredWorld},
    platform::collections::HashMap,
    prelude::*,
};
use panoply_core::RealmPhysics;
use panoply_exemplar::*;
use panoply_precinct::{InstanceKey, PrecinctAsset, SceneryInstanceId};
use rapier3d::prelude::RigidBodyHandle;

use super::{
    floor_region::{FloorRegion, RebuildFloorAspects},
    floor_surface::FloorSurface,
    rle::rle_decode,
    scenery_element::{
        SceneryElement, SceneryElementRebuildAspects, SceneryElementRebuildColliders,
    },
    terrain_effect::TerrainEffect,
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
#[component(on_remove = remove_precinct_body)]
pub struct Precinct {
    pub realm: Entity,
    pub render_layer: RenderLayers,
    pub coords: IVec2,
    pub visible: bool,
    /// None for a precinct with no file, which its realm's `PrecinctIndex` does not list, until the
    /// editor makes one at its first edit there.
    pub asset: Option<Handle<PrecinctAsset>>,
    /// Fixed body at the precinct's origin, holding the colliders of its elements and floors.
    pub physics: RigidBodyHandle,
}

fn remove_precinct_body(mut world: DeferredWorld, context: HookContext) {
    let precinct = world.get::<Precinct>(context.entity).unwrap();
    let (realm, body) = (precinct.realm, precinct.physics);
    if let Some(mut physics) = world.get_mut::<RealmPhysics>(realm) {
        physics.remove_body(body);
    }
}

/// One floor level of a precinct (D24): a child of the precinct, with its floor regions as its
/// children. Tiers are spaced 1 meter apart.
#[derive(Component, Debug, Default)]
pub struct PrecinctTier {
    /// Floor level.
    pub level: i32,

    /// Authored cutaway rectangles, in precinct-local coordinates (`cutaways::choose_cutaways`).
    pub cutaways: Vec<Rect>,
}

impl Precinct {
    /// The precinct's asset in `assets`, if it has one and it has loaded.
    pub fn asset_in<'a>(&self, assets: &'a Assets<PrecinctAsset>) -> Option<&'a PrecinctAsset> {
        assets.get(self.asset.as_ref()?)
    }

    /// Whether the precinct's asset is `id`.
    pub fn has_asset(&self, id: AssetId<PrecinctAsset>) -> bool {
        self.asset.as_ref().is_some_and(|asset| asset.id() == id)
    }

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

        // Update each scenery instance whose id is still there in place, if its exemplar has not
        // changed.
        let mut unmatched = Vec::new();
        for sid in asset.scenery.iter() {
            let facing = sid.facing * std::f32::consts::PI / 180.;
            let transform = Transform::from_translation(sid.position)
                .with_rotation(Quat::from_rotation_y(facing));
            if let Some(se_ent) = existing.remove(&sid.iid) {
                let mut scenery_element = q_scenery_elements.get_mut(se_ent).unwrap();
                if scenery_element.exemplar == scenery_exemplars[sid.id] {
                    if scenery_element.label != sid.label {
                        scenery_element.label.clone_from(&sid.label);
                    }
                    if scenery_element.position != sid.position || scenery_element.facing != facing
                    {
                        scenery_element.position = sid.position;
                        scenery_element.facing = facing;
                        commands
                            .entity(se_ent)
                            .insert((transform, SceneryElementRebuildColliders));
                    }
                    continue;
                }

                // The exemplar changed: replace the instance.
                commands.entity(se_ent).despawn();
            }
            unmatched.push((sid, facing, transform));
        }

        // Internal ids are numbered as the precinct loads (D87), so after a save, the reload's
        // instances carry other numbers than the elements spawned before it. An element left over
        // with the same exemplar, place and facing is the same instance: it takes the new id
        // rather than being spawned again, which would empty the place while its models load.
        let mut alike = HashMap::<(AssetId<Exemplar>, [u32; 4]), Vec<Entity>>::new();
        for se_ent in existing.values() {
            let element = q_scenery_elements.get(*se_ent).unwrap();
            alike
                .entry(element_key(
                    element.exemplar.id(),
                    element.position,
                    element.facing,
                ))
                .or_default()
                .push(*se_ent);
        }
        for (sid, facing, transform) in unmatched {
            let exemplar = &scenery_exemplars[sid.id];
            let key = element_key(exemplar.id(), sid.position, facing);
            if let Some(se_ent) = alike.get_mut(&key).and_then(Vec::pop) {
                let mut scenery_element = q_scenery_elements.get_mut(se_ent).unwrap();
                existing.remove(&scenery_element.iid);
                // A save gives keys and takes them away (D87).
                if scenery_element.iid.key() != sid.iid.key() {
                    match sid.iid.key() {
                        Some(key) => commands.entity(se_ent).insert(key),
                        None => commands.entity(se_ent).remove::<InstanceKey>(),
                    };
                }
                scenery_element.iid = sid.iid.clone();
                scenery_element.label.clone_from(&sid.label);
                continue;
            }

            // Otherwise, spawn a new instance.
            let mut spawned = commands.spawn((
                Name::new("SceneryElement"),
                SceneryElement {
                    iid: sid.iid.clone(),
                    label: sid.label.clone(),
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
            if let Some(key) = sid.iid.key() {
                spawned.insert(key);
            }
        }

        // Despawn any remaining scenery elements that were not found in the asset.
        for se_ent in existing.values() {
            commands.entity(*se_ent).despawn();
        }
    }
}

/// What makes two scenery elements alike: their exemplar, position and facing, exactly.
fn element_key(
    exemplar: AssetId<Exemplar>,
    position: Vec3,
    facing: f32,
) -> (AssetId<Exemplar>, [u32; 4]) {
    (
        exemplar,
        [
            position.x.to_bits(),
            position.y.to_bits(),
            position.z.to_bits(),
            facing.to_bits(),
        ],
    )
}

impl Precinct {
    /// Decodes the precinct's terrain effects. Every loaded precinct gets a `TerrainFxMap`, empty
    /// if the asset has none, since parcels wait for it before building ground (D11).
    pub fn rebuild_terrain_fx(
        &self,
        commands: &mut Commands,
        entity: Entity,
        asset: &PrecinctAsset,
        effects: Vec<Handle<TerrainEffect>>,
    ) {
        let mut fx = TerrainFxMap::new();
        if let Some(ref encoded) = asset.terrain_fx {
            if let Err(err) = rle_decode(encoded, &mut fx.map) {
                error!("Precinct {:?}: bad terrain effects: {err:?}", self.coords);
            }
            fx.effects = effects;
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
        // `Added` is an asset the editor made for a precinct that has no file yet.
        let (AssetEvent::LoadedWithDependencies { id }
        | AssetEvent::Modified { id }
        | AssetEvent::Added { id }) = ev
        else {
            continue;
        };
        if let Some((precinct_entity, _)) = q_precincts
            .iter()
            .find(|(_, precinct)| precinct.has_asset(*id))
        {
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
    mut catalogs: CatalogServer,
) {
    for (precinct_entity, precinct, precinct_children) in q_precincts.iter() {
        let Some(precinct_asset) = precinct.asset_in(&assets) else {
            continue;
        };

        // Every list is requested before any is waited on, so their catalogs load together.
        let scenery_exemplars = catalogs.get_all::<Exemplar>(&precinct_asset.scenery_types);
        let effects = catalogs.get_all::<TerrainEffect>(&precinct_asset.terrain_fx_types);
        let floor_surfaces = catalogs.get_all::<FloorSurface>(&precinct_asset.floor_surfaces);
        let (Some(scenery_exemplars), Some(effects), Some(floor_surfaces)) =
            (scenery_exemplars, effects, floor_surfaces)
        else {
            continue;
        };
        commands
            .entity(precinct_entity)
            .remove::<ReadPrecinctData>();

        // TODO: Sync nav mesh, light sources, particles, etc.

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

        precinct.rebuild_terrain_fx(&mut commands, precinct_entity, precinct_asset, effects);
    }
}
