use bevy::{asset::LoadState, math::IRect, prelude::*};

use crate::{
    view::{QueryRect, Viewpoint},
    world::Realm,
};

use super::{
    parcel::{
        Parcel, ParcelContourChanged, ParcelFloraChanged, ParcelKey, ParcelWaterChanged, ShapeRef,
        ADJACENT_COUNT,
    },
    terrain_map::{TerrainMap, TerrainMapAsset},
    PARCEL_SIZE_F,
};

#[derive(Resource)]
pub struct ParcelCache {
    size: usize,
    parcels: lru::LruCache<ParcelKey, Entity>,
}

impl ParcelCache {
    pub fn new() -> Self {
        Self {
            size: 64,
            parcels: lru::LruCache::unbounded(),
        }
    }

    pub fn _size(&self) -> usize {
        self.parcels.len()
    }
}

/// System that manages the spawning and despawning of Parcels (terrain units) based on proximity
/// to the camera viewpoint (either the primary camera or a portal camera).
pub fn spawn_parcels(
    mut commands: Commands,
    viewpoint: Res<Viewpoint>,
    mut parcel_cache: ResMut<ParcelCache>,
    mut query: Query<&mut Parcel>,
    realm_query: Query<(&Realm, &TerrainMap)>,
    terrain_map_assets: Res<Assets<TerrainMapAsset>>,
    server: Res<AssetServer>,
) {
    if viewpoint.realm.is_none() {
        return;
    }

    // Determine coordinates of view in parcel units.
    let view_radius = 32.;
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
    query.iter_mut().for_each(|mut parcel| {
        parcel.visible = false;
    });

    // Function to add parcels to the cache based on a view rect.
    let mut fetch_parcels = |rect: &QueryRect| {
        let realm = realm_query.get(rect.realm);
        if realm.is_ok() {
            let (_, terrain) = realm.unwrap();
            if server.load_state(&terrain.handle) != LoadState::Loaded {
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
                            if let Ok(mut parcel) = query.get_mut(*entity) {
                                if parcel.contours != contours || parcel.biomes != biomes {
                                    parcel.contours = contours;
                                    parcel.biomes = biomes;
                                    commands.entity(*entity).insert((
                                        ParcelContourChanged,
                                        ParcelWaterChanged,
                                        ParcelFloraChanged,
                                    ));
                                }
                                parcel.visible = true;
                            }
                        }

                        None => {
                            // println!("Creating parcel {} {}.", x, z);
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
                                },
                                SpatialBundle {
                                    transform: Transform::from_xyz(
                                        x as f32 * PARCEL_SIZE_F,
                                        0.,
                                        z as f32 * PARCEL_SIZE_F,
                                    ),
                                    ..default()
                                },
                                ParcelContourChanged,
                                ParcelWaterChanged,
                                ParcelFloraChanged,
                            ));
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
            if let Ok(parcel) = query.get_mut(*entity) {
                if parcel.visible {
                    break;
                } else {
                    commands.entity(*entity).despawn_recursive();
                }
            }
        }
        cache.pop_lru();
    }
}
