use bevy::{ecs::system::SystemParam, prelude::*, world_serialization::WorldAssetRoot};

use crate::{
    flora::{ComputeFloraTask, FloraInstance},
    terrain_map::TerrainMap,
    water_mesh::ComputeWaterMeshTask,
    BiomesHandle, ComputeGroundMeshTask, Parcel, ParcelFloraChanged, ParcelWaterChanged,
    RebuildParcelGroundMesh, RebuildParcelTerrainFx, TerrainContoursHandle,
};

/// Terrain work still to do: assets loading, parcels whose ground, water or flora is not yet
/// built, and flora scenes waiting for their frame budget. For readiness checks, such as the
/// editor's `ViewReady`.
#[derive(SystemParam)]
pub struct TerrainPending<'w, 's> {
    server: Res<'w, AssetServer>,
    contours: Res<'w, TerrainContoursHandle>,
    biomes: Res<'w, BiomesHandle>,
    maps: Query<'w, 's, &'static TerrainMap>,
    #[allow(clippy::type_complexity)]
    parcels: Query<
        'w,
        's,
        (),
        (
            With<Parcel>,
            Or<(
                With<RebuildParcelTerrainFx>,
                With<RebuildParcelGroundMesh>,
                With<ComputeGroundMeshTask>,
                With<ParcelWaterChanged>,
                With<ComputeWaterMeshTask>,
                With<ParcelFloraChanged>,
                With<ComputeFloraTask>,
            )>,
        ),
    >,
    flora: Query<'w, 's, (), (With<FloraInstance>, Without<WorldAssetRoot>)>,
}

impl TerrainPending<'_, '_> {
    /// Names the first kind of work still pending, or `None` when there is none.
    pub fn waiting_on(&self) -> Option<&'static str> {
        if self.server.load_state(&self.contours.0).is_loading()
            || self.server.load_state(&self.biomes.0).is_loading()
            || self
                .maps
                .iter()
                .any(|map| self.server.load_state(&map.handle).is_loading())
        {
            Some("terrain assets")
        } else if !self.parcels.is_empty() {
            Some("parcels")
        } else if !self.flora.is_empty() {
            Some("flora scenes")
        } else {
            None
        }
    }
}
