//! Edits to the view realm's terrain map, from the realm panel's map size control and terrain
//! mode's rotate buttons. Each changes the map in memory, where the grid and the ground show it at
//! once, marks it unsaved for the save button, and can be undone.

use std::borrow::Cow;

use bevy::{ecs::system::SystemParam, prelude::*};
use panoply_core::Viewpoint;
use panoply_terrain::{TerrainMap, TerrainMapAsset};

use crate::{
    mode_selector::EditorMode,
    undo::{UndoAction, UndoStack},
    unsaved::UnsavedAssets,
};

/// Edits the view realm's terrain map.
#[derive(SystemParam)]
pub(crate) struct TerrainMapEdits<'w, 's> {
    viewpoint: Res<'w, Viewpoint>,
    terrain_maps: Query<'w, 's, &'static TerrainMap>,
    assets: ResMut<'w, Assets<TerrainMapAsset>>,
    unsaved: ResMut<'w, UnsavedAssets>,
    history: ResMut<'w, UndoStack>,
    /// Absent during play, which makes no edits.
    mode: Option<Res<'w, State<EditorMode>>>,
}

impl TerrainMapEdits<'_, '_> {
    /// The view realm's map.
    pub(crate) fn map(&self) -> Option<&TerrainMapAsset> {
        let terrain = self
            .viewpoint
            .realm
            .and_then(|e| self.terrain_maps.get(e).ok())?;
        self.assets.get(&terrain.handle)
    }

    /// Changes the view realm's map with `edit`, recorded in the history as `label`. An edit that
    /// returns false has refused, and leaves the map and the history as they were.
    pub(crate) fn edit(
        &mut self,
        label: &'static str,
        edit: impl FnOnce(&mut TerrainMapAsset) -> bool,
    ) {
        let Some(mode) = self.mode.as_ref().map(|mode| *mode.get()) else {
            return;
        };
        let Some(terrain) = self
            .viewpoint
            .realm
            .and_then(|e| self.terrain_maps.get(e).ok())
        else {
            warn!("{label} did nothing: the view realm has no terrain map");
            return;
        };
        let handle = terrain.handle.clone();
        let Some(before) = self.assets.get(&handle).cloned() else {
            return;
        };
        let mut after = before.clone();
        if !edit(&mut after) {
            return;
        }
        if let Some(mut map) = self.assets.get_mut(&handle) {
            *map = after;
        }
        self.unsaved.mark(&handle);
        self.history.push(
            mode,
            TerrainMapUndo {
                label,
                asset: handle.id(),
                map: before,
            },
        );
    }
}

/// Puts back a terrain map as it was before an edit.
struct TerrainMapUndo {
    label: &'static str,
    asset: AssetId<TerrainMapAsset>,
    map: TerrainMapAsset,
}

impl UndoAction for TerrainMapUndo {
    fn apply(self: Box<Self>, world: &mut World) -> Box<dyn UndoAction> {
        let TerrainMapUndo { label, asset, map } = *self;
        let replaced = {
            let mut assets = world.resource_mut::<Assets<TerrainMapAsset>>();
            assets
                .get_mut(asset)
                .map(|mut current| std::mem::replace(&mut *current, map.clone()))
        };
        let Some(replaced) = replaced else {
            warn!("{label} did nothing: the terrain map is gone");
            return Box::new(TerrainMapUndo { label, asset, map });
        };
        world.resource_mut::<UnsavedAssets>().mark(asset);
        Box::new(TerrainMapUndo {
            label,
            asset,
            map: replaced,
        })
    }

    fn label(&self) -> Cow<'static, str> {
        self.label.into()
    }
}
