use bevy::prelude::*;

use crate::{
    editor::{
        undo::{RedoEntry, UndoEntry},
        unsaved,
    },
    terrain::TerrainMapAsset,
};

#[derive(Debug, Event, Clone)]
pub(crate) struct UndoTerrainMapEdit {
    pub(crate) label: &'static str,
    pub(crate) handle: Handle<TerrainMapAsset>,
    pub(crate) before: TerrainMapAsset,
    pub(crate) after: TerrainMapAsset,
}

impl UndoEntry for UndoTerrainMapEdit {
    fn label(&self) -> &'static str {
        self.label
    }

    fn undo(&self, world: &mut World) -> Box<dyn RedoEntry> {
        let mut assets = world.get_resource_mut::<Assets<TerrainMapAsset>>().unwrap();
        let map = assets.get_mut(self.handle.id()).unwrap();
        *map = self.before.clone();
        let mut unsaved = world.get_resource_mut::<unsaved::UnsavedAssets>().unwrap();
        unsaved
            .terrain_maps
            .insert(self.handle.clone(), unsaved::ModifiedState::Unsaved);
        Box::new(self.clone())
    }
}

impl RedoEntry for UndoTerrainMapEdit {
    fn label(&self) -> &str {
        self.label
    }

    fn redo(&self, world: &mut World) -> Box<dyn UndoEntry> {
        let mut assets = world.get_resource_mut::<Assets<TerrainMapAsset>>().unwrap();
        let map = assets.get_mut(self.handle.id()).unwrap();
        *map = self.after.clone();
        let mut unsaved = world.get_resource_mut::<unsaved::UnsavedAssets>().unwrap();
        unsaved
            .terrain_maps
            .insert(self.handle.clone(), unsaved::ModifiedState::Unsaved);
        Box::new(self.clone())
    }
}
