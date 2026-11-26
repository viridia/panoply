use bevy::{math::IRect, platform::collections::HashMap, prelude::*};

use panoply_core::{PortalViewRegions, Realm, RealmPhysics, ToRapier, Viewpoint};
use panoply_precinct::{precinct_index_path, precinct_path, PrecinctAsset, PrecinctIndex};
use rapier3d::prelude::RigidBodyBuilder;

use super::{
    precinct::{Precinct, PrecinctKey},
    PRECINCT_SIZE_F,
};

/// A rectangle of precincts in a realm.
struct QueryRect {
    realm: Entity,
    bounds: IRect,
}

#[derive(Resource)]
pub struct PrecinctCache {
    size: usize,
    precincts: lru::LruCache<PrecinctKey, Entity>,
}

impl Default for PrecinctCache {
    fn default() -> Self {
        Self::new()
    }
}

impl PrecinctCache {
    pub fn new() -> Self {
        Self {
            size: 128,
            precincts: lru::LruCache::unbounded(),
        }
    }

    pub fn get(&mut self, key: &PrecinctKey) -> Option<Entity> {
        self.precincts.get(key).cloned()
    }

    /// The precinct at `key`, if loaded, without marking it used.
    pub fn peek(&self, key: &PrecinctKey) -> Option<Entity> {
        self.precincts.peek(key).cloned()
    }
}

/// Each realm's [`PrecinctIndex`], by the realm's entity, loaded when the realm is first viewed.
#[derive(Resource, Default)]
pub(crate) struct PrecinctIndices(pub(crate) HashMap<Entity, Handle<PrecinctIndex>>);

/// System that manages the spawning and despawning of Precincts (scenery units) based on proximity
/// to the camera viewpoint (either the primary camera or a portal camera), and keeps the simulated
/// precincts loaded. A realm's precincts wait for its index, which names those with files; the
/// others are spawned with no asset, unless the editor holds one for them, made at its first edit.
/// A realm whose index cannot be read has none with files.
#[allow(clippy::too_many_arguments)]
pub fn spawn_precincts(
    mut commands: Commands,
    viewpoint: Res<Viewpoint>,
    portal_regions: Res<PortalViewRegions>,
    mut precinct_cache: ResMut<PrecinctCache>,
    mut query: Query<&mut Precinct>,
    mut realm_query: Query<(&Realm, &mut RealmPhysics)>,
    server: Res<AssetServer>,
    mut indices: ResMut<PrecinctIndices>,
    index_assets: Res<Assets<PrecinctIndex>>,
) {
    if viewpoint.realm.is_none() {
        return;
    }

    let distance = viewpoint.camera_distance(); // Nominally 11

    // Determine coordinates of view in precinct units.
    let view_radius = distance * 0.75 + 20.;
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
        let Ok((realm, mut realm_physics)) = realm_query.get_mut(rect.realm) else {
            return;
        };
        let index = indices
            .0
            .entry(rect.realm)
            .or_insert_with(|| server.load(precinct_index_path(&realm.name)));
        let no_index = PrecinctIndex::default();
        let index = match index_assets.get(&*index) {
            Some(index) => index,
            None if server.load_state(&*index).is_failed() => &no_index,
            None => return,
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
                        debug!("Creating precinct {} {} {}.", realm.name, x, z);
                        let coords = IVec2::new(x, z);
                        let path = precinct_path(&realm.name, coords);
                        let asset = server
                            .get_handle::<PrecinctAsset>(&path)
                            .or_else(|| index.contains(coords).then(|| server.load(&path)));
                        let origin =
                            Vec3::new(x as f32 * PRECINCT_SIZE_F, 0., z as f32 * PRECINCT_SIZE_F);
                        let physics = realm_physics.insert_body(
                            RigidBodyBuilder::fixed()
                                .translation(origin.to_rapier())
                                .build(),
                        );
                        let entity = commands.spawn((
                            Name::new(format!("Precinct:{}:{}:{}", realm.name, x, z)),
                            Precinct {
                                realm: rect.realm,
                                coords,
                                visible: true,
                                asset,
                                render_layer: realm.layer.clone(),
                                physics,
                            },
                            Transform::from_translation(origin),
                            Visibility::Inherited,
                        ));
                        precinct_cache.precincts.put(key, entity.id());
                    }
                };
            }
        }
    };

    fetch_precincts(&query_rect);
    // The simulated precincts too, though out of view (R8).
    fetch_precincts(&QueryRect {
        realm: query_rect.realm,
        bounds: viewpoint.simulated_precincts(),
    });
    // The far ends of nearby portals, in whatever realm they lie.
    for region in portal_regions.0.iter() {
        fetch_precincts(&QueryRect {
            realm: region.realm,
            bounds: region.cells(PRECINCT_SIZE_F),
        });
    }

    let size = precinct_cache.size;
    let cache = &mut precinct_cache.precincts;
    while cache.len() > size {
        let entry = cache.peek_lru();
        if let Some((_, entity)) = entry {
            if let Ok(precinct) = query.get_mut(*entity) {
                if precinct.visible {
                    break;
                } else {
                    evict_precinct(&mut commands, *entity, &precinct);
                }
            }
        }
        cache.pop_lru();
    }
}

/// Removes a precinct that has left the cache, with everything it owns.
// TODO(PAN82): collect and write the precinct's changed state here first.
fn evict_precinct(commands: &mut Commands, entity: Entity, precinct: &Precinct) {
    debug!(
        "Despawning precinct {:?} {} {}.",
        precinct.realm, precinct.coords.x, precinct.coords.y
    );
    commands.entity(entity).despawn();
}
