use bevy::{math::IRect, prelude::*};

use crate::{
    view::{QueryRect, Viewpoint},
    world::Realm,
};

use super::{
    precinct::{Precinct, PrecinctKey},
    PRECINCT_SIZE_F,
};

#[derive(Resource)]
pub struct PrecinctCache {
    size: usize,
    precincts: lru::LruCache<PrecinctKey, Entity>,
}

impl PrecinctCache {
    pub fn new() -> Self {
        Self {
            size: 64,
            precincts: lru::LruCache::unbounded(),
        }
    }

    pub fn size(&self) -> usize {
        self.precincts.len()
    }

    pub fn get(&mut self, key: &PrecinctKey) -> Option<Entity> {
        self.precincts.get(key).cloned()
    }
}

/// System that manages the spawning and despawning of Precincts (scenery units) based on proximity
/// to the camera viewpoint (either the primary camera or a portal camera).
pub fn spawn_precincts(
    mut commands: Commands,
    viewpoint: Res<Viewpoint>,
    mut precinct_cache: ResMut<PrecinctCache>,
    mut query: Query<&mut Precinct>,
    realm_query: Query<&Realm>,
    server: Res<AssetServer>,
) {
    if viewpoint.realm.is_none() {
        return;
    }

    // Determine coordinates of view in precinct units.
    let view_radius = 32.;
    let query_rect = QueryRect {
        realm: viewpoint.realm.expect("Realm id expected"),
        bounds: IRect::new(
            ((viewpoint.position.x - view_radius) / PRECINCT_SIZE_F).floor() as i32,
            ((viewpoint.position.z - view_radius) / PRECINCT_SIZE_F).floor() as i32,
            ((viewpoint.position.x + view_radius) / PRECINCT_SIZE_F).ceil() as i32,
            ((viewpoint.position.z + view_radius) / PRECINCT_SIZE_F).ceil() as i32,
        ),
    };

    // TODO: return here if query rects (including portals) was the same as last time.
    // ONLY if terrain maps haven't changed?

    // Reset the visibility bits for all precincts.
    query.iter_mut().for_each(|mut precinct| {
        precinct.visible = false;
    });

    // Function to add precincts to the cache based on a view rect.
    let mut fetch_precincts = |rect: &QueryRect| {
        let Ok(realm) = realm_query.get(rect.realm) else {
            return;
        };
        // Only query scenery within the precinct bounds of the realm.
        let intersect = realm.precinct_bounds.intersect(rect.bounds);
        // Set precincts within the query rect as visible; also load missing precincts.
        for z in intersect.min.y..intersect.max.y {
            for x in intersect.min.x..intersect.max.x {
                let key = PrecinctKey {
                    realm: rect.realm,
                    x,
                    z,
                };
                let entity = precinct_cache.precincts.get(&key);
                match entity {
                    Some(entity) => {
                        if let Ok(mut precinct) = query.get_mut(*entity) {
                            precinct.visible = true;
                        }
                    }

                    None => {
                        println!("Creating precinct {} {} {}.", realm.name, x, z);
                        let asset_path = format!(
                            "scenery/precincts/{}/{}-{}.msgpack",
                            realm.name,
                            precinct_coord(x),
                            precinct_coord(z)
                        );
                        let asset = server.load(asset_path);
                        let entity = commands.spawn((
                            Precinct {
                                realm: rect.realm,
                                coords: IVec2::new(x, z),
                                visible: true,
                                asset,
                                tiers: Vec::new(),
                                render_layer: realm.layer.clone(),
                            },
                            SpatialBundle {
                                transform: Transform::from_xyz(
                                    x as f32 * PRECINCT_SIZE_F,
                                    0.,
                                    z as f32 * PRECINCT_SIZE_F,
                                ),
                                ..default()
                            },
                        ));
                        precinct_cache.precincts.put(key, entity.id());
                    }
                };
            }
        }
    };

    fetch_precincts(&query_rect);

    let size = precinct_cache.size;
    let cache = &mut precinct_cache.precincts;
    while cache.len() > size {
        let entry = cache.peek_lru();
        if let Some((_, entity)) = entry {
            if let Ok(precinct) = query.get_mut(*entity) {
                if precinct.visible {
                    break;
                } else {
                    println!(
                        "Despawning precinct {:#?} {} {}.",
                        precinct.realm, precinct.coords.x, precinct.coords.y
                    );
                    commands.entity(*entity).despawn_recursive();
                }
            }
        }
        cache.pop_lru();
    }
}

fn precinct_coord(n: i32) -> String {
    if n >= 0 {
        format!("p{:03}", n)
    } else {
        format!("n{:03}", -n)
    }
}
