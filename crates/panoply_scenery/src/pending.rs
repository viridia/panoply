use bevy::{
    ecs::system::SystemParam, platform::collections::HashMap, prelude::*,
    world_serialization::WorldAssetRoot,
};

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
/// editor's `ViewReady`, and the readiness survey. A precinct with no file fails to load, and
/// counts as done (X34).
#[derive(SystemParam)]
pub struct SceneryPending<'w, 's> {
    server: Res<'w, AssetServer>,
    precincts: Query<'w, 's, (Entity, &'static Precinct)>,
    read: Query<'w, 's, Entity, With<ReadPrecinctData>>,
    fx: Query<'w, 's, Entity, With<RebuildTerrainFxVertexAttrs>>,
    aspects: Query<'w, 's, Entity, With<SceneryElementRebuildAspects>>,
    models: Query<'w, 's, Entity, With<SceneryElementRebuildModels>>,
    #[allow(clippy::type_complexity)]
    floors: Query<
        'w,
        's,
        Entity,
        Or<(
            With<RebuildFloorAspects>,
            With<RebuildFloorMesh>,
            With<ComputeFloorMeshTask>,
            With<RebuildFloorMaterials>,
        )>,
    >,
    scenes: Query<'w, 's, Entity, (With<SceneryElementMesh>, Without<WorldAssetRoot>)>,
    parents: Query<'w, 's, &'static ChildOf>,
}

/// One precinct's state, for the readiness survey.
pub struct PrecinctStatus {
    pub realm: Entity,
    pub coords: IVec2,
    /// The asset's load state, as `Debug` prints it (`Loaded`, `Failed(..)`, ...).
    pub asset: String,
    /// What is still to do: `read` and `fx` on the precinct itself, then counts of what under it
    /// waits: `aspects:<n>` and `models:<n>` (elements), `floors:<n>`, `scenes:<n>`.
    pub pending: Vec<String>,
}

impl SceneryPending<'_, '_> {
    /// Names the first kind of work still pending, or `None` when there is none.
    pub fn waiting_on(&self) -> Option<&'static str> {
        if self
            .precincts
            .iter()
            .any(|(_, precinct)| self.server.load_state(&precinct.asset).is_loading())
        {
            Some("precinct assets")
        } else if !self.read.is_empty() || !self.fx.is_empty() {
            Some("precincts")
        } else if !self.aspects.is_empty() || !self.models.is_empty() {
            Some("scenery elements")
        } else if !self.floors.is_empty() {
            Some("floors")
        } else if !self.scenes.is_empty() {
            Some("scenery scenes")
        } else {
            None
        }
    }

    /// Every loaded precinct and what it, and what it owns, still waits on.
    pub fn survey(&self) -> Vec<PrecinctStatus> {
        // Elements are a precinct's children, floors its grandchildren (under a tier), and element
        // scenes children of elements.
        let mut counts = HashMap::<(Entity, &'static str), usize>::new();
        let stages: [(&'static str, Vec<Entity>); 4] = [
            ("aspects", self.aspects.iter().collect()),
            ("models", self.models.iter().collect()),
            ("floors", self.floors.iter().collect()),
            ("scenes", self.scenes.iter().collect()),
        ];
        for (code, entities) in &stages {
            for entity in entities {
                if let Some(precinct) = self.precinct_of(*entity) {
                    *counts.entry((precinct, code)).or_default() += 1;
                }
            }
        }
        self.precincts
            .iter()
            .map(|(entity, precinct)| {
                let mut pending: Vec<String> = [
                    ("read", self.read.contains(entity)),
                    ("fx", self.fx.contains(entity)),
                ]
                .into_iter()
                .filter(|(_, pending)| *pending)
                .map(|(code, _)| code.to_string())
                .collect();
                for (code, _) in &stages {
                    if let Some(count) = counts.get(&(entity, *code)) {
                        pending.push(format!("{code}:{count}"));
                    }
                }
                PrecinctStatus {
                    realm: precinct.realm,
                    coords: precinct.coords,
                    asset: format!("{:?}", self.server.load_state(&precinct.asset)),
                    pending,
                }
            })
            .collect()
    }

    /// The precinct an entity belongs to, up to three levels up.
    fn precinct_of(&self, entity: Entity) -> Option<Entity> {
        let mut current = entity;
        for _ in 0..3 {
            current = self.parents.get(current).ok()?.parent();
            if self.precincts.contains(current) {
                return Some(current);
            }
        }
        None
    }
}
