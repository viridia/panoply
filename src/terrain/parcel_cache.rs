use bevy::{
    camera::{
        primitives::{Aabb, Frustum},
        visibility::RenderLayers,
    },
    math::IRect,
    prelude::*,
};
use panoply_terrain::{
    Parcel, ParcelFloraChanged, ParcelKey, ParcelTerrainFx, ParcelThumbnail, ParcelWaterChanged,
    RebuildParcelGroundMesh, RebuildParcelTerrainFx, ShapeRef, TerrainContoursHandle,
    TerrainContoursTable, TerrainContoursTableAsset, TerrainFxVertexAttr, ADJACENT_COUNT,
    CENTER_SHAPE,
};
use rapier3d::{
    na::Vector3,
    prelude::{ColliderHandle, RigidBodyBuilder},
};

use panoply_core::{Realm, RealmPhysics, Viewpoint};

use super::{
    terrain_map::{TerrainMap, TerrainMapAsset},
    PARCEL_HEIGHT_SCALE, PARCEL_SIZE_F, PARCEL_TERRAIN_FX_AREA,
};

/// Height allowance above the ground for what stands on a parcel: the tallest flora model
/// (4.45 m) at its largest scale (0.8), rounded up.
const PARCEL_CONTENT_HEIGHT: f32 = 4.0;

/// Slack around each parcel's bounds, covering ground blended toward neighboring contours at
/// the edges and a camera that moved since the frustum was last computed.
const SHOW_MARGIN: f32 = 2.0;

/// Shadows from a sun lower than this (sine of elevation) are treated as cast from this height,
/// so a grazing sun does not stretch a parcel's shadow reach across the whole map.
const MIN_SUN_SINE: f32 = 0.25;

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
    pub fn query(&self, realm: Entity, rect: IRect) -> ParcelRectIterator<'_> {
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
    mut q_parcels: Query<(&mut Parcel, Option<&ParcelThumbnail>, &mut Visibility)>,
    mut q_realms: Query<(&Realm, &mut RealmPhysics, &TerrainMap)>,
    q_cameras: Query<(&Camera, &Frustum, &RenderLayers)>,
    q_lights: Query<(&DirectionalLight, &GlobalTransform, &RenderLayers)>,
    terrain_map_assets: Res<Assets<TerrainMapAsset>>,
    r_contours_handle: Res<TerrainContoursHandle>,
    r_contours_assets: Res<Assets<TerrainContoursTableAsset>>,
    server: Res<AssetServer>,
) {
    if viewpoint.realm.is_none() {
        return;
    }

    let Some(contours_asset) = r_contours_assets.get(&r_contours_handle.0) else {
        return;
    };

    let contours_table = contours_asset.0.read().unwrap();
    let distance = viewpoint.camera_distance(); // Nominally 20

    // Determine coordinates of view in parcel units.
    let view_radius = distance * 0.75 + 20.;
    let realm_id = viewpoint.realm.expect("Realm id expected");
    let query_rect = IRect::new(
        ((viewpoint.position.x - view_radius) / PARCEL_SIZE_F).floor() as i32,
        ((viewpoint.position.z - view_radius) / PARCEL_SIZE_F).floor() as i32,
        ((viewpoint.position.x + view_radius) / PARCEL_SIZE_F).ceil() as i32,
        ((viewpoint.position.z + view_radius) / PARCEL_SIZE_F).ceil() as i32,
    );

    // TODO: return here if query rects (including portals) was the same as last time.
    // ONLY if terrain maps haven't changed?

    // Reset the visibility bits for all parcels.
    q_parcels.iter_mut().for_each(|(mut parcel, thumbnail, _)| {
        if thumbnail.is_none() {
            parcel.visible = false;
        }
    });

    // Function to add parcels to the cache based on a view rect.
    let mut fetch_parcels = |realm_id: Entity, bounds: IRect| {
        if let Ok((realm, mut realm_physics, terrain)) = q_realms.get_mut(realm_id) {
            if !server.load_state(&terrain.handle).is_loaded() {
                return;
            }
            let terrain_map = terrain_map_assets
                .get(&terrain.handle)
                .expect("expecting terrain map");

            // Set parcels within the query rect as visible; also load missing parcels.
            for z in bounds.min.y..bounds.max.y {
                for x in bounds.min.x..bounds.max.x {
                    let key = ParcelKey {
                        realm: realm_id,
                        x,
                        z,
                    };
                    let mut contours: [ShapeRef; 9] = [ShapeRef::new(); ADJACENT_COUNT];
                    terrain_map.adjacent_shapes(&mut contours, IVec2::new(x, z));
                    let center = contours_table.get(contours[CENTER_SHAPE].shape as usize);
                    let biomes = terrain_map.adjacent_biomes(IVec2::new(x, z));
                    let entity = parcel_cache.parcels.get(&key);
                    let flora = center.into_flora_square(contours[CENTER_SHAPE].rotation);

                    match entity {
                        Some(entity) => {
                            // Update existing parcel
                            if let Ok((mut parcel, _, _)) = q_parcels.get_mut(*entity) {
                                if parcel.contours != contours
                                    || parcel.biomes != biomes
                                    || parcel.has_terrain != center.has_terrain
                                {
                                    parcel.contours = contours;
                                    parcel.biomes = biomes;
                                    parcel.has_terrain = center.has_terrain;
                                    // println!("Parcel {} {} changed: {:?}.", x, z, biomes);
                                    commands.entity(*entity).insert((
                                        RebuildParcelGroundMesh,
                                        ParcelWaterChanged,
                                        ParcelFloraChanged,
                                        RebuildParcelTerrainFx,
                                    ));
                                }

                                if parcel.has_water != center.has_water {
                                    parcel.has_water = center.has_water;
                                    commands.entity(*entity).insert(ParcelWaterChanged);
                                }

                                if parcel.flora != flora {
                                    parcel.flora = flora;
                                    commands.entity(*entity).insert(ParcelFloraChanged);
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
                            // println!("Creating parcel {} {}; center shape {}", x, z, center.id);
                            let entity = commands.spawn((
                                Parcel {
                                    realm: realm_id,
                                    coords: IVec2::new(x, z),
                                    visible: true,
                                    contours,
                                    biomes,
                                    ground_entity: None,
                                    water_entity: None,
                                    flora_entity: None,
                                    flora,
                                    terrain_fx: ParcelTerrainFx(
                                        [TerrainFxVertexAttr::default(); PARCEL_TERRAIN_FX_AREA],
                                    ),
                                    physics: rb_handle,
                                    terrain_collider: ColliderHandle::default(),
                                    has_terrain: center.has_terrain,
                                    has_water: center.has_water,
                                },
                                Name::new(format!("Parcel:{}:{}:{}", realm.name, x, z)),
                                Transform::from_xyz(
                                    x as f32 * PARCEL_SIZE_F,
                                    0.,
                                    z as f32 * PARCEL_SIZE_F,
                                ),
                                Visibility::Inherited,
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

    fetch_parcels(realm_id, query_rect);

    // Loaded parcels are shown only if they, or the shadows they cast, can be in view of a
    // camera that renders their realm: the primary camera, or a portal camera looking into
    // another realm. The rest stay loaded, ready for scrolling, but skip all rendering work.
    let cameras: Vec<_> = q_cameras
        .iter()
        .filter(|(camera, ..)| camera.is_active)
        .map(|(_, frustum, layers)| (frustum, layers))
        .collect();
    // Every shadow-casting light (sun, moon) widens the bounds by its own shadow direction.
    let lights: Vec<_> = q_lights
        .iter()
        .filter(|(light, ..)| light.shadow_maps_enabled)
        .map(|(_, transform, layers)| (shadow_reach(transform.forward().as_vec3()), layers))
        .collect();
    q_parcels
        .iter_mut()
        .for_each(|(parcel, thumbnail, mut visibility)| {
            let shown = parcel.visible
                && (thumbnail.is_some()
                    || q_realms.get(parcel.realm).is_ok_and(|(realm, ..)| {
                        let reaches = lights
                            .iter()
                            .filter(|(_, layers)| layers.intersects(&realm.layer))
                            .map(|(reach, _)| *reach);
                        let bounds = parcel_show_bounds(&parcel, &contours_table, reaches);
                        cameras.iter().any(|(frustum, layers)| {
                            layers.intersects(&realm.layer)
                                && frustum.intersects_obb_identity(&bounds)
                        })
                    }));
            visibility.set_if_neq(if shown {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            });
        });

    let size = parcel_cache.size;
    let cache = &mut parcel_cache.parcels;
    while cache.len() > size {
        let entry = cache.peek_lru();
        if let Some((_, entity)) = entry {
            if let Ok((parcel, thumbnail, _)) = q_parcels.get_mut(*entity) {
                if parcel.visible || thumbnail.is_some() {
                    break;
                } else {
                    commands.entity(*entity).despawn();
                }
            }
        }
        cache.pop_lru();
    }
}

/// Horizontal distance a shadow travels per meter of caster height, along the light's direction.
fn shadow_reach(light_direction: Vec3) -> Vec2 {
    let sine = (-light_direction.y).max(MIN_SUN_SINE);
    light_direction.xz() / sine
}

/// World-space bounds of everything a parcel can put on screen: its ground and what stands on
/// it, plus the ground its shadows can fall on, for each light's shadow reach.
fn parcel_show_bounds(
    parcel: &Parcel,
    contours_table: &TerrainContoursTable,
    shadow_reaches: impl Iterator<Item = Vec2>,
) -> Aabb {
    let heights = contours_table
        .get(parcel.contours[CENTER_SHAPE].shape as usize)
        .height
        .elts();
    let lowest = heights.iter().copied().min().unwrap_or(0) as f32 * PARCEL_HEIGHT_SCALE;
    let highest = heights.iter().copied().max().unwrap_or(0) as f32 * PARCEL_HEIGHT_SCALE;
    let origin = parcel.coords.as_vec2() * PARCEL_SIZE_F;
    let mut min = Vec3::new(origin.x, lowest, origin.y) - SHOW_MARGIN;
    let mut max = Vec3::new(
        origin.x + PARCEL_SIZE_F,
        highest + PARCEL_CONTENT_HEIGHT,
        origin.y + PARCEL_SIZE_F,
    ) + SHOW_MARGIN;
    // The tallest caster's shadow lands this far away, on ground as low as the parcel's.
    let span = max.y - min.y;
    let (ground_min, ground_max) = (min, max);
    for reach in shadow_reaches {
        let shadow = reach * span;
        min.x = min.x.min(ground_min.x + shadow.x);
        min.z = min.z.min(ground_min.z + shadow.y);
        max.x = max.x.max(ground_max.x + shadow.x);
        max.z = max.z.max(ground_max.z + shadow.y);
    }
    Aabb::from_min_max(min, max)
}

pub(crate) fn cleanup_parcel_physics(world: &mut World) {
    world
        .register_component_hooks::<Parcel>()
        .on_remove(|mut world, hook| {
            let parcel = world.get::<Parcel>(hook.entity).unwrap();
            let rb_handle = parcel.physics;
            let Some(mut realm_physics) = world.get_mut::<RealmPhysics>(parcel.realm) else {
                return;
            };
            realm_physics.remove_body(rb_handle);
        });
}
