use bevy::{asset::LoadState, prelude::*};

use crate::terrain::{
    Parcel, ParcelCache, ParcelFloraChanged, ParcelTerrainFx, RebuildParcelGroundMesh,
    RebuildParcelTerrainFx, TerrainFxVertexAttr, TerrainOptions, PARCEL_SIZE,
    PARCEL_TERRAIN_FX_AREA, PARCEL_TERRAIN_FX_STRIDE,
};
use panoply_exemplar::*;

use super::{
    precinct::{Precinct, PrecinctKey},
    precinct_cache::PrecinctCache,
    terrain_fx_aspect::{TerrainEffect, TerrainHole},
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
    pub(crate) exemplars: Vec<Handle<Exemplar>>,
    pub(crate) map: [u16; TERRAIN_FX_MAP_SIZE * TERRAIN_FX_MAP_SIZE],
    pub(crate) map_vertex_attr: [TerrainFxVertexAttr; TERRAIN_FX_MAP_SIZE * TERRAIN_FX_MAP_SIZE],
}

impl TerrainFxMap {
    pub fn new() -> Self {
        Self {
            exemplars: Vec::new(),
            map: [0; TERRAIN_FX_MAP_SIZE * TERRAIN_FX_MAP_SIZE],
            map_vertex_attr: [TerrainFxVertexAttr::default();
                TERRAIN_FX_MAP_SIZE * TERRAIN_FX_MAP_SIZE],
        }
    }
}

#[derive(Component)]
#[component(storage = "SparseSet")]
pub struct RebuildTerrainFxVertexAttrs;

pub fn rebuild_terrain_fx_vertex_attrs(
    mut commands: Commands,
    mut query: Query<(Entity, &Precinct, &mut TerrainFxMap), With<RebuildTerrainFxVertexAttrs>>,
    parcel_cache: Res<ParcelCache>,
    exemplar_assets: Res<Assets<Exemplar>>,
    server: Res<AssetServer>,
) {
    for (entity, precinct, mut terrain_fx) in query.iter_mut() {
        let all_loaded = terrain_fx
            .exemplars
            .iter()
            .all(|s| server.load_state(s) == LoadState::Loaded);
        if !all_loaded {
            continue;
        }

        let fx_table: Vec<TerrainFxVertexAttr> = terrain_fx
            .exemplars
            .iter()
            .map(|s| {
                // Note that we're accessing the exemplar directly in this case instead
                // of applying the aspects to an entity, since the individual effects are
                // not associated with a single entity but with vertex attributes.
                let s = exemplar_assets.get(s).unwrap();
                let mut vxt_attr = TerrainFxVertexAttr::default();
                for aspect in s.0.aspects.iter() {
                    // TODO: Extends - require async resolution of the exemplar.

                    // Terrain effects
                    if let Some(eff) = aspect.as_any().downcast_ref::<TerrainEffect>() {
                        vxt_attr.effect = eff.effect;
                        vxt_attr.effect_strength = eff.effect_strength.unwrap_or(0.);
                        vxt_attr.elevation = eff.elevation.unwrap_or(0.);
                        if eff.continuous_x.unwrap_or(false) {
                            vxt_attr.options |= TerrainOptions::ContinuousX;
                        }
                        if eff.continuous_y.unwrap_or(false) {
                            vxt_attr.options |= TerrainOptions::ContinuousY;
                        }
                    }

                    // Terrain holes
                    if aspect.as_any().downcast_ref::<TerrainHole>().is_some() {
                        vxt_attr.options |= TerrainOptions::Hole;
                    }
                }
                vxt_attr
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

pub fn rebuild_parcel_terrain_fx(
    mut commands: Commands,
    mut q_parcels: Query<(Entity, &mut Parcel), With<RebuildParcelTerrainFx>>,
    q_precincts: Query<(&Precinct, &TerrainFxMap)>,
    mut precinct_cache: ResMut<PrecinctCache>,
) {
    for (entity, mut parcel) in q_parcels.iter_mut() {
        let mut terrain_fx: [TerrainFxVertexAttr; PARCEL_TERRAIN_FX_AREA] =
            [TerrainFxVertexAttr::default(); PARCEL_TERRAIN_FX_AREA];
        let precinct_key = PrecinctKey {
            realm: parcel.realm,
            x: (parcel.coords.x * PARCEL_SIZE).div_euclid(PRECINCT_SIZE),
            z: (parcel.coords.y * PARCEL_SIZE).div_euclid(PRECINCT_SIZE),
        };
        let Some(precinct_entity) = precinct_cache.get(&precinct_key) else {
            // println!("No precinct entity for parcel {:?}", parcel.coords);
            commands.entity(entity).remove::<RebuildParcelTerrainFx>();
            continue;
        };
        let Ok((precinct, terrain_fx_map)) = q_precincts.get(precinct_entity) else {
            // println!("No precinct for parcel {:?}", parcel.coords);
            commands.entity(entity).remove::<RebuildParcelTerrainFx>();
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
