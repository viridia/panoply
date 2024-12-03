use bevy::{math::IRect, prelude::*};
use panoply_terrain::{
    Parcel, ParcelFloraChanged, ParcelKey, ParcelTerrainFx, ParcelThumbnail, ParcelWaterChanged,
    RebuildParcelGroundMesh, RebuildParcelTerrainFx, ShapeRef, TerrainFxVertexAttr, ADJACENT_COUNT,
};
use rapier3d::{
    na::Vector3,
    prelude::{ColliderHandle, RigidBodyBuilder},
};

use crate::view::{
    picking::{PickAction, PickEvent, PickTarget},
    QueryRect,
};
use panoply_core::{Realm, RealmPhysics, Viewpoint};

use super::{
    terrain_map::{TerrainMap, TerrainMapAsset},
    PARCEL_SIZE_F, PARCEL_TERRAIN_FX_AREA,
};

#[derive(Resource)]
pub struct ParcelCache {
    size: usize,
    parcels: lru::LruCache<ParcelKey, Entity>,
}

impl ParcelCache {
    pub fn new() -> Self {
        Self {
            size: 128,
            parcels: lru::LruCache::unbounded(),
        }
    }

    pub fn size(&self) -> usize {
        self.parcels.len()
    }

    /// Query all parcels within a given rectangle.
    pub fn query(&self, realm: Entity, rect: IRect) -> ParcelRectIterator {
        ParcelRectIterator {
            cache: self,
            realm,
            rect,
            x: rect.min.x,
            z: rect.min.y,
        }
    }
}

pub struct ParcelRectIterator<'a> {
    cache: &'a ParcelCache,
    realm: Entity,
    rect: IRect,
    x: i32,
    z: i32,
}

impl Iterator for ParcelRectIterator<'_> {
    type Item = Entity;
    fn next(&mut self) -> Option<Entity> {
        while self.z < self.rect.max.y {
            while self.x < self.rect.max.x {
                let key = ParcelKey {
                    realm: self.realm,
                    x: self.x,
                    z: self.z,
                };
                self.x += 1;
                if let Some(entity) = self.cache.parcels.peek(&key) {
                    return Some(*entity);
                }
            }
            self.x = self.rect.min.x;
            self.z += 1;
        }
        None
    }
}

/// System that manages the spawning and despawning of Parcels (terrain units) based on proximity
/// to the camera viewpoint (either the primary camera or a portal camera).
pub fn spawn_parcels(
    mut commands: Commands,
    viewpoint: Res<Viewpoint>,
    mut parcel_cache: ResMut<ParcelCache>,
    mut q_parcels: Query<(&mut Parcel, Option<&ParcelThumbnail>)>,
    mut q_realms: Query<(&Realm, &mut RealmPhysics, &TerrainMap)>,
    terrain_map_assets: Res<Assets<TerrainMapAsset>>,
    server: Res<AssetServer>,
) {
    if viewpoint.realm.is_none() {
        return;
    }

    let distance = viewpoint.camera_distance(); // Nominally 20

    // Determine coordinates of view in parcel units.
    let view_radius = distance * 0.75 + 20.;
    let query_rect = QueryRect {
        realm: viewpoint.realm.expect("Realm id expected"),
        bounds: IRect::new(
            ((viewpoint.position.x - view_radius) / PARCEL_SIZE_F).floor() as i32,
            ((viewpoint.position.z - view_radius) / PARCEL_SIZE_F).floor() as i32,
            ((viewpoint.position.x + view_radius) / PARCEL_SIZE_F).ceil() as i32,
            ((viewpoint.position.z + view_radius) / PARCEL_SIZE_F).ceil() as i32,
        ),
    };

    // TODO: return here if query rects (including portals) was the same as last time.
    // ONLY if terrain maps haven't changed?

    // Reset the visibility bits for all parcels.
    q_parcels.iter_mut().for_each(|(mut parcel, thumbnail)| {
        if thumbnail.is_none() {
            parcel.visible = false;
        }
    });

    // Function to add parcels to the cache based on a view rect.
    let mut fetch_parcels = |rect: &QueryRect| {
        // let realm_id = rect.realm;
        if let Ok((realm, mut realm_physics, terrain)) = q_realms.get_mut(rect.realm) {
            if !server.load_state(&terrain.handle).is_loaded() {
                return;
            }
            let terrain_map = terrain_map_assets
                .get(&terrain.handle)
                .expect("expecting terrain map");

            // Set parcels within the query rect as visible; also load missing parcels.
            for z in rect.bounds.min.y..rect.bounds.max.y {
                for x in rect.bounds.min.x..rect.bounds.max.x {
                    let key = ParcelKey {
                        realm: rect.realm,
                        x,
                        z,
                    };
                    let mut contours: [ShapeRef; 9] = [ShapeRef::new(); ADJACENT_COUNT];
                    terrain_map.adjacent_shapes(&mut contours, IVec2::new(x, z));
                    let biomes = terrain_map.adjacent_biomes(IVec2::new(x, z));
                    let entity = parcel_cache.parcels.get(&key);
                    match entity {
                        Some(entity) => {
                            // Update existing parcel
                            if let Ok((mut parcel, _)) = q_parcels.get_mut(*entity) {
                                if parcel.contours != contours || parcel.biomes != biomes {
                                    parcel.contours = contours;
                                    parcel.biomes = biomes;
                                    // println!("Parcel {} {} changed: {:?}.", x, z, biomes);
                                    commands.entity(*entity).insert((
                                        RebuildParcelGroundMesh,
                                        ParcelWaterChanged,
                                        ParcelFloraChanged,
                                        RebuildParcelTerrainFx,
                                    ));
                                }
                                parcel.visible = true;
                            }
                        }

                        None => {
                            // println!("Creating parcel {} {}; biomes: {:?}.", x, z, biomes);
                            // Insert new parcel
                            let rb_builder = RigidBodyBuilder::fixed().translation(Vector3::new(
                                x as f32 * PARCEL_SIZE_F,
                                0.,
                                z as f32 * PARCEL_SIZE_F,
                            ));
                            let rigid_body = rb_builder.build();
                            let rb_handle = realm_physics.insert_body(rigid_body);
                            let entity = commands.spawn((
                                Parcel {
                                    realm: rect.realm,
                                    coords: IVec2::new(x, z),
                                    visible: true,
                                    contours,
                                    biomes,
                                    ground_entity: None,
                                    water_entity: None,
                                    flora_entity: None,
                                    terrain_fx: ParcelTerrainFx(
                                        [TerrainFxVertexAttr::default(); PARCEL_TERRAIN_FX_AREA],
                                    ),
                                    physics: rb_handle,
                                    terrain_collider: ColliderHandle::default(),
                                },
                                Name::new(format!("Parcel:{}:{}:{}", realm.name, x, z)),
                                Transform::from_xyz(
                                    x as f32 * PARCEL_SIZE_F,
                                    0.,
                                    z as f32 * PARCEL_SIZE_F,
                                ),
                                Visibility::Visible,
                                RebuildParcelGroundMesh,
                                ParcelWaterChanged,
                                ParcelFloraChanged,
                                RebuildParcelTerrainFx,
                            ));
                            // TODO: Restore picking
                            // entity.insert((On::<Pointer<Down>>::run(
                            //     move |mut ev: ListenerMut<Pointer<Down>>,
                            //           mut commands: Commands| {
                            //         ev.stop_propagation();
                            //         commands.trigger(PickEvent {
                            //             action: PickAction::Down(ev.hit.position.unwrap()),
                            //             target: PickTarget::Parcel(ev.listener()),
                            //         });
                            //     },
                            // ), On::<Pointer<DragStart>>::run(
                            //     move |mut ev: ListenerMut<Pointer<DragStart>>,
                            //           mut commands: Commands| {
                            //         ev.stop_propagation();
                            //         commands.trigger(PickEvent {
                            //             action: PickAction::DragStart{
                            //                 realm: realm_id,
                            //                 pos: ev.hit.position.unwrap()
                            //             },
                            //             target: PickTarget::Parcel(ev.listener()),
                            //         });
                            //     },
                            // ), On::<Pointer<Drag>>::run(
                            //     move |mut ev: ListenerMut<Pointer<Drag>>,
                            //           mut commands: Commands| {
                            //         ev.stop_propagation();
                            //         commands.trigger(PickEvent {
                            //             action: PickAction::Drag,
                            //             target: PickTarget::Parcel(ev.listener()),
                            //         });
                            //     },
                            // ), On::<Pointer<DragEnd>>::run(
                            //     move |mut ev: ListenerMut<Pointer<DragEnd>>,
                            //           mut commands: Commands| {
                            //         ev.stop_propagation();
                            //         commands.trigger(PickEvent {
                            //             action: PickAction::DragEnd,
                            //             target: PickTarget::Parcel(ev.listener()),
                            //         });
                            //     },
                            // )));
                            parcel_cache.parcels.put(key, entity.id());
                        }
                    };
                }
            }
        }
    };

    fetch_parcels(&query_rect);

    let size = parcel_cache.size;
    let cache = &mut parcel_cache.parcels;
    while cache.len() > size {
        let entry = cache.peek_lru();
        if let Some((_, entity)) = entry {
            if let Ok((parcel, thumbnail)) = q_parcels.get_mut(*entity) {
                if parcel.visible || thumbnail.is_some() {
                    break;
                } else {
                    commands.entity(*entity).despawn_recursive();
                }
            }
        }
        cache.pop_lru();
    }
}

pub(crate) fn cleanup_parcel_physics(world: &mut World) {
    world
        .register_component_hooks::<Parcel>()
        .on_remove(|mut world, entity, _component| {
            let parcel = world.get::<Parcel>(entity).unwrap();
            let rb_handle = parcel.physics;
            let Some(mut realm_physics) = world.get_mut::<RealmPhysics>(parcel.realm) else {
                return;
            };
            realm_physics.remove_body(rb_handle);
        });
}
