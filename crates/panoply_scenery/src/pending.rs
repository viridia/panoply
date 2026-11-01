use bevy::{ecs::system::SystemParam, prelude::*, world_serialization::WorldAssetRoot};

use crate::{
    floor_mesh::ComputeFloorMeshTask,
    floor_region::{RebuildFloorAspects, RebuildFloorMaterials, RebuildFloorMesh},
    precinct::{Precinct, ReadPrecinctData},
    scenery_element::{
        SceneryElementMesh, SceneryElementRebuildAspects, SceneryElementRebuildModels,
    },
    terrain_fx_map::RebuildTerrainFxVertexAttrs,
};

/// Scenery work still to do: precinct assets loading, precincts, elements and floors not yet
/// built, and element scenes waiting for their frame budget. For readiness checks, such as the
/// editor's `ViewReady`. A precinct with no file fails to load, and counts as done (X34).
#[derive(SystemParam)]
pub struct SceneryPending<'w, 's> {
    server: Res<'w, AssetServer>,
    precincts: Query<'w, 's, &'static Precinct>,
    building: Query<'w, 's, (), Or<(With<ReadPrecinctData>, With<RebuildTerrainFxVertexAttrs>)>>,
    elements: Query<
        'w,
        's,
        (),
        Or<(
            With<SceneryElementRebuildAspects>,
            With<SceneryElementRebuildModels>,
        )>,
    >,
    #[allow(clippy::type_complexity)]
    floors: Query<
        'w,
        's,
        (),
        Or<(
            With<RebuildFloorAspects>,
            With<RebuildFloorMesh>,
            With<ComputeFloorMeshTask>,
            With<RebuildFloorMaterials>,
        )>,
    >,
    models: Query<'w, 's, (), (With<SceneryElementMesh>, Without<WorldAssetRoot>)>,
}

impl SceneryPending<'_, '_> {
    /// Names the first kind of work still pending, or `None` when there is none.
    pub fn waiting_on(&self) -> Option<&'static str> {
        if self
            .precincts
            .iter()
            .any(|precinct| self.server.load_state(&precinct.asset).is_loading())
        {
            Some("precinct assets")
        } else if !self.building.is_empty() {
            Some("precincts")
        } else if !self.elements.is_empty() {
            Some("scenery elements")
        } else if !self.floors.is_empty() {
            Some("floors")
        } else if !self.models.is_empty() {
            Some("scenery scenes")
        } else {
            None
        }
    }
}
