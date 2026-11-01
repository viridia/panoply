use bevy::{
    ecs::system::SystemParam, platform::collections::HashMap, prelude::*,
    world_serialization::WorldAssetRoot,
};

use crate::{
    flora::{ComputeFloraTask, FloraInstance},
    terrain_map::TerrainMap,
    water_mesh::ComputeWaterMeshTask,
    BiomesHandle, ComputeGroundMeshTask, Parcel, ParcelFloraChanged, ParcelWaterChanged,
    RebuildParcelGroundMesh, RebuildParcelTerrainFx, TerrainContoursHandle,
};

/// Terrain work still to do: assets loading, parcels whose ground, water or flora is not yet
/// built, and flora scenes waiting for their frame budget. For readiness checks, such as the
/// editor's `ViewReady`, and the readiness survey.
#[derive(SystemParam)]
pub struct TerrainPending<'w, 's> {
    server: Res<'w, AssetServer>,
    contours: Res<'w, TerrainContoursHandle>,
    biomes: Res<'w, BiomesHandle>,
    maps: Query<'w, 's, &'static TerrainMap>,
    parcels: Query<'w, 's, (Entity, &'static Parcel, &'static Visibility)>,
    fx: Query<'w, 's, Entity, (With<Parcel>, With<RebuildParcelTerrainFx>)>,
    ground: Query<
        'w,
        's,
        Entity,
        (
            With<Parcel>,
            Or<(With<RebuildParcelGroundMesh>, With<ComputeGroundMeshTask>)>,
        ),
    >,
    water: Query<
        'w,
        's,
        Entity,
        (
            With<Parcel>,
            Or<(With<ParcelWaterChanged>, With<ComputeWaterMeshTask>)>,
        ),
    >,
    flora: Query<
        'w,
        's,
        Entity,
        (
            With<Parcel>,
            Or<(With<ParcelFloraChanged>, With<ComputeFloraTask>)>,
        ),
    >,
    /// Each flora instance's parent is the parcel's flora entity, a child of the parcel.
    flora_scenes: Query<'w, 's, &'static ChildOf, (With<FloraInstance>, Without<WorldAssetRoot>)>,
    parents: Query<'w, 's, &'static ChildOf>,
}

/// One parcel's state, for the readiness survey.
pub struct ParcelStatus {
    pub realm: Entity,
    pub coords: IVec2,
    /// Not hidden by the show set (D20).
    pub shown: bool,
    /// What is still to do, in pipeline order: `fx`, `ground`, `water`, `flora`, and
    /// `flora-scenes:<count>`.
    pub pending: Vec<String>,
}

impl TerrainPending<'_, '_> {
    /// Names the first kind of work still pending, or `None` when there is none.
    pub fn waiting_on(&self) -> Option<&'static str> {
        if self.assets_loading() {
            Some("terrain assets")
        } else if !self.fx.is_empty()
            || !self.ground.is_empty()
            || !self.water.is_empty()
            || !self.flora.is_empty()
        {
            Some("parcels")
        } else if !self.flora_scenes.is_empty() {
            Some("flora scenes")
        } else {
            None
        }
    }

    /// Whether the contours, biomes or a realm's terrain map are still loading.
    pub fn assets_loading(&self) -> bool {
        self.server.load_state(&self.contours.0).is_loading()
            || self.server.load_state(&self.biomes.0).is_loading()
            || self
                .maps
                .iter()
                .any(|map| self.server.load_state(&map.handle).is_loading())
    }

    /// Every loaded parcel and what it still waits on.
    pub fn survey(&self) -> Vec<ParcelStatus> {
        let mut scenes = HashMap::<Entity, usize>::new();
        for child_of in &self.flora_scenes {
            if let Ok(parcel) = self.parents.get(child_of.parent()) {
                *scenes.entry(parcel.parent()).or_default() += 1;
            }
        }
        self.parcels
            .iter()
            .map(|(entity, parcel, visibility)| {
                let mut pending: Vec<String> = [
                    ("fx", self.fx.contains(entity)),
                    ("ground", self.ground.contains(entity)),
                    ("water", self.water.contains(entity)),
                    ("flora", self.flora.contains(entity)),
                ]
                .into_iter()
                .filter(|(_, pending)| *pending)
                .map(|(code, _)| code.to_string())
                .collect();
                if let Some(count) = scenes.get(&entity) {
                    pending.push(format!("flora-scenes:{count}"));
                }
                ParcelStatus {
                    realm: parcel.realm,
                    coords: parcel.coords,
                    shown: *visibility != Visibility::Hidden,
                    pending,
                }
            })
            .collect()
    }
}
