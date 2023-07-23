use bevy::{math::IRect, prelude::*};

use crate::view::Viewpoint;

use super::{
    parcel::{Parcel, ParcelBundle, ParcelKey, ParcelStatus},
    PLOT_LENGTH_F,
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
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    // Determine coordinates of view in parcel units.
    let view_radius = 16.;
    let query_rect = IRect::new(
        ((viewpoint.position.x - view_radius) / PLOT_LENGTH_F).floor() as i32,
        ((viewpoint.position.z - view_radius) / PLOT_LENGTH_F).floor() as i32,
        ((viewpoint.position.x + view_radius) / PLOT_LENGTH_F).ceil() as i32,
        ((viewpoint.position.z + view_radius) / PLOT_LENGTH_F).ceil() as i32,
    );

    // TODO: return here if query rects (including portals) was the same as last time.

    // Reset the visibility bits for all parcels.
    query.for_each_mut(|mut parcel| {
        parcel.visible = false;
    });

    let shape = shape::Cube::default();

    // Set parcels within the query rect as visible; also load missing parcels.
    let realm = viewpoint.realm;
    for z in query_rect.min.y..query_rect.max.y {
        for x in query_rect.min.x..query_rect.max.x {
            let key = ParcelKey { realm, x, z };
            let entity = parcel_cache.parcels.get(&key);
            match entity {
                Some(entity) => {
                    if let Ok(mut parcel) = query.get_mut(*entity) {
                        parcel.visible = true;
                    }
                }

                None => {
                    println!("Creating parcel {} {}.", x, z);
                    let entity = commands.spawn(ParcelBundle {
                        parcel: Parcel {
                            realm,
                            coords: IVec2::new(x, z),
                            status: ParcelStatus::New,
                            visible: true,
                        },
                        mesh: PbrBundle {
                            mesh: meshes.add(shape.into()),
                            material: materials.add(Color::GREEN.into()),
                            transform: Transform::from_xyz(
                                x as f32 * PLOT_LENGTH_F,
                                2.0,
                                z as f32 * PLOT_LENGTH_F,
                            ),
                            ..default()
                        },
                    });
                    parcel_cache.parcels.put(key, entity.id());
                }
            };
        }
    }

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
