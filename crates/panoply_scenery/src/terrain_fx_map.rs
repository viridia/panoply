use bevy::prelude::*;
use panoply_core::Realm;
use panoply_terrain::{
    Parcel, ParcelCache, ParcelFloraChanged, ParcelTerrainFx, RebuildParcelGroundMesh,
    RebuildParcelTerrainFx, TerrainFxVertexAttr, PARCEL_SIZE, PARCEL_TERRAIN_FX_AREA,
    PARCEL_TERRAIN_FX_STRIDE,
};

use super::{
    precinct::{Precinct, PrecinctKey},
    precinct_cache::PrecinctCache,
    terrain_effect::TerrainEffect,
    PRECINCT_SIZE,
};

/// TerrainFx maps have an extra "skirt" of 1 meter around each edge of the precinct.
pub const TERRAIN_FX_MAP_SIZE: usize = (PRECINCT_SIZE + 2) as usize;

/// TerrainFxMap is a packed array of terrain effects. Terrain effects modify the terrain
/// parcels, but are stored with the precinct. This reduces the amount of visual repetition,
/// since parcels are often repeated across the map, and allows for more efficient storage
/// and loading.
#[derive(Component)]
pub struct TerrainFxMap {
    /// The precinct's table of effects; the map's value `n` is the `n - 1`th, 0 none.
    pub(crate) effects: Vec<Handle<TerrainEffect>>,
    pub(crate) map: [u16; TERRAIN_FX_MAP_SIZE * TERRAIN_FX_MAP_SIZE],
    pub(crate) map_vertex_attr: [TerrainFxVertexAttr; TERRAIN_FX_MAP_SIZE * TERRAIN_FX_MAP_SIZE],
}

impl TerrainFxMap {
    pub fn new() -> Self {
        Self {
            effects: Vec::new(),
            map: [0; TERRAIN_FX_MAP_SIZE * TERRAIN_FX_MAP_SIZE],
            map_vertex_attr: [TerrainFxVertexAttr::default();
                TERRAIN_FX_MAP_SIZE * TERRAIN_FX_MAP_SIZE],
        }
    }

    /// The effect on the 1 m cell whose corner is at `x`, `z` in the precinct, from -1 to
    /// [`PRECINCT_SIZE`] (the skirt included); none outside that.
    pub fn effect_at(&self, x: i32, z: i32) -> TerrainFxVertexAttr {
        let range = -1..=PRECINCT_SIZE;
        if !(range.contains(&x) && range.contains(&z)) {
            return TerrainFxVertexAttr::default();
        }
        self.map_vertex_attr[(x + 1) as usize + (z + 1) as usize * TERRAIN_FX_MAP_SIZE]
    }
}

impl Default for TerrainFxMap {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Component)]
#[component(storage = "SparseSet")]
pub struct RebuildTerrainFxVertexAttrs;

/// Marks for rebuilding each precinct whose table holds a terrain effect that changed, as an edit
/// in the property editor or a reload changes it (D92).
pub fn mark_changed_terrain_effects(
    mut events: MessageReader<AssetEvent<TerrainEffect>>,
    maps: Query<(Entity, &TerrainFxMap)>,
    mut commands: Commands,
) {
    for event in events.read() {
        let AssetEvent::Modified { id } = *event else {
            continue;
        };
        for (entity, map) in &maps {
            if map.effects.iter().any(|effect| effect.id() == id) {
                commands.entity(entity).insert(RebuildTerrainFxVertexAttrs);
            }
        }
    }
}

pub fn rebuild_terrain_fx_vertex_attrs(
    mut commands: Commands,
    mut query: Query<(Entity, &Precinct, &mut TerrainFxMap), With<RebuildTerrainFxVertexAttrs>>,
    parcel_cache: Res<ParcelCache>,
    effects: Res<Assets<TerrainEffect>>,
    server: Res<AssetServer>,
) {
    for (entity, precinct, mut terrain_fx) in query.iter_mut() {
        // Wait for the effects; one that failed to load becomes "no effect".
        let any_loading = terrain_fx.effects.iter().any(|s| {
            let state = server.load_state(s);
            !state.is_loaded() && !state.is_failed()
        });
        if any_loading {
            continue;
        }

        let fx_table: Vec<TerrainFxVertexAttr> = terrain_fx
            .effects
            .iter()
            .map(|effect| {
                effects
                    .get(effect)
                    .map(TerrainEffect::vertex_attr)
                    .unwrap_or_default()
            })
            .collect();

        let mut map_vertex_attr: [TerrainFxVertexAttr; TERRAIN_FX_MAP_SIZE * TERRAIN_FX_MAP_SIZE] =
            [TerrainFxVertexAttr::default(); TERRAIN_FX_MAP_SIZE * TERRAIN_FX_MAP_SIZE];
        for (index, id) in terrain_fx.map.iter().enumerate() {
            if *id > 0 {
                map_vertex_attr[index] = fx_table[*id as usize - 1];
            }
        }
        terrain_fx.map_vertex_attr = map_vertex_attr;

        commands
            .entity(entity)
            .remove::<RebuildTerrainFxVertexAttrs>();

        let rect = IRect::new(
            precinct.coords.x * PRECINCT_SIZE / PARCEL_SIZE,
            precinct.coords.y * PRECINCT_SIZE / PARCEL_SIZE,
            (precinct.coords.x + 1) * PRECINCT_SIZE / PARCEL_SIZE,
            (precinct.coords.y + 1) * PRECINCT_SIZE / PARCEL_SIZE,
        );
        for parcel in parcel_cache.query(precinct.realm, rect) {
            commands.entity(parcel).insert(RebuildParcelTerrainFx);
        }
    }
}

/// Copies each marked parcel's terrain effects out of its precinct's map, then lets the parcel
/// build its ground and flora. A parcel waits while its precinct is still loading, so ground is
/// never built without its roads and holes (D11); it goes ahead without effects if it lies
/// outside the realm's precincts or its precinct failed to load (most have no file).
pub fn rebuild_parcel_terrain_fx(
    mut commands: Commands,
    mut q_parcels: Query<(Entity, &mut Parcel), With<RebuildParcelTerrainFx>>,
    q_precincts: Query<(
        &Precinct,
        Option<&TerrainFxMap>,
        Has<RebuildTerrainFxVertexAttrs>,
    )>,
    q_realms: Query<&Realm>,
    mut precinct_cache: ResMut<PrecinctCache>,
    server: Res<AssetServer>,
) {
    for (entity, mut parcel) in q_parcels.iter_mut() {
        let mut terrain_fx: [TerrainFxVertexAttr; PARCEL_TERRAIN_FX_AREA] =
            [TerrainFxVertexAttr::default(); PARCEL_TERRAIN_FX_AREA];
        let precinct_key = PrecinctKey {
            realm: parcel.realm,
            x: (parcel.coords.x * PARCEL_SIZE).div_euclid(PRECINCT_SIZE),
            z: (parcel.coords.y * PARCEL_SIZE).div_euclid(PRECINCT_SIZE),
        };
        let precinct = precinct_cache
            .get(&precinct_key)
            .and_then(|precinct_entity| q_precincts.get(precinct_entity).ok());
        let terrain_fx_map = match precinct {
            Some((precinct, Some(terrain_fx_map), false)) => Some((precinct, terrain_fx_map)),
            Some((precinct, ..)) if !server.load_state(&precinct.asset).is_failed() => {
                // Still loading, or its effects are being rebuilt.
                continue;
            }
            Some(_) => None,
            None => {
                // As in `spawn_precincts`, the bounds' max is exclusive.
                let in_bounds = q_realms.get(parcel.realm).is_ok_and(|realm| {
                    let bounds = realm.precinct_bounds;
                    (bounds.min.x..bounds.max.x).contains(&precinct_key.x)
                        && (bounds.min.y..bounds.max.y).contains(&precinct_key.z)
                });
                if in_bounds {
                    // `spawn_precincts` has not spawned it yet.
                    continue;
                }
                None
            }
        };
        let Some((precinct, terrain_fx_map)) = terrain_fx_map else {
            parcel.terrain_fx = ParcelTerrainFx(terrain_fx);
            commands
                .entity(entity)
                .insert((RebuildParcelGroundMesh, ParcelFloraChanged))
                .remove::<RebuildParcelTerrainFx>();
            continue;
        };
        let x_offset = parcel.coords.x * PARCEL_SIZE - precinct.coords.x * PRECINCT_SIZE;
        let z_offset = parcel.coords.y * PARCEL_SIZE - precinct.coords.y * PRECINCT_SIZE;
        assert!((0..PRECINCT_SIZE).contains(&x_offset));
        assert!((0..PRECINCT_SIZE).contains(&z_offset));
        for z in 0..PARCEL_TERRAIN_FX_STRIDE {
            for x in 0..PARCEL_TERRAIN_FX_STRIDE {
                let fx_x = x + x_offset as usize;
                let fx_z = z + z_offset as usize;
                let fx_index = fx_x + fx_z * TERRAIN_FX_MAP_SIZE;
                terrain_fx[x + z * PARCEL_TERRAIN_FX_STRIDE] =
                    terrain_fx_map.map_vertex_attr[fx_index];
            }
        }
        parcel.terrain_fx = ParcelTerrainFx(terrain_fx);
        // println!("Rebuilt terrain fx for parcel {:?}", parcel.coords);
        commands
            .entity(entity)
            .insert((RebuildParcelGroundMesh, ParcelFloraChanged))
            .remove::<RebuildParcelTerrainFx>();
    }
}
