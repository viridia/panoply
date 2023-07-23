use bevy::{math::IRect, prelude::*};

use crate::view::{QueryRect, Viewpoint};

use super::{
    parcel::{Parcel, ParcelKey, ParcelStatus},
    PARCEL_SIZE_F,
};

#[derive(Resource)]
pub struct ParcelCache {
    size: usize,
    parcels: lru::LruCache<ParcelKey, Entity>,
}

impl Drop for ParcelCache {
    fn drop(&mut self) {
        println!("Parcel cache dropped.");
    }
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
) {
    // Determine coordinates of view in parcel units.
    let view_radius = 16.;
    let query_rect = QueryRect {
        realm: viewpoint.realm,
        bounds: IRect::new(
            ((viewpoint.position.x - view_radius) / PARCEL_SIZE_F).floor() as i32,
            ((viewpoint.position.z - view_radius) / PARCEL_SIZE_F).floor() as i32,
            ((viewpoint.position.x + view_radius) / PARCEL_SIZE_F).ceil() as i32,
            ((viewpoint.position.z + view_radius) / PARCEL_SIZE_F).ceil() as i32,
        ),
    };

    // TODO: return here if query rects (including portals) was the same as last time.

    // Reset the visibility bits for all parcels.
    query.for_each_mut(|mut parcel| {
        parcel.visible = false;
    });

    // Function to add parcels to the cache based on a view rect.
    let mut fetch_parcels = |rect: &QueryRect| {
        // Set parcels within the query rect as visible; also load missing parcels.
        for z in rect.bounds.min.y..rect.bounds.max.y {
            for x in rect.bounds.min.x..rect.bounds.max.x {
                let key = ParcelKey {
                    realm: rect.realm,
                    x,
                    z,
                };
                let entity = parcel_cache.parcels.get(&key);
                match entity {
                    Some(entity) => {
                        if let Ok(mut parcel) = query.get_mut(*entity) {
                            parcel.visible = true;
                        }
                    }

                    None => {
                        // println!("Creating parcel {} {}.", x, z);
                        let entity = commands.spawn(Parcel {
                            realm: rect.realm,
                            coords: IVec2::new(x, z),
                            status: ParcelStatus::New,
                            visible: true,
                        });
                        parcel_cache.parcels.put(key, entity.id());
                    }
                };
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
                    commands.entity(*entity).despawn();
                }
            }
        }
        cache.pop_lru();
    }
}
