use bevy::prelude::*;

use crate::{
    editor::{
        events::ChangeContourEvent,
        undo::{RedoEntry, UndoEntry},
        unsaved,
    },
    terrain::terrain_contours::{TerrainContour, TerrainContoursTableAsset},
};

#[derive(Debug, Event, Clone)]
pub(crate) struct UndoTerrainContourEdit {
    pub(crate) label: &'static str,
    pub(crate) handle: Handle<TerrainContoursTableAsset>,
    pub(crate) index: usize,
    pub(crate) data: TerrainContour,
}

impl UndoEntry for UndoTerrainContourEdit {
    fn label(&self) -> &'static str {
        self.label
    }

    fn undo(&self, world: &mut World) -> Box<dyn RedoEntry> {
        let mut assets = world
            .get_resource_mut::<Assets<TerrainContoursTableAsset>>()
            .unwrap();
        let asset = assets.get_mut(self.handle.id()).unwrap();
        let mut table = asset.0.write().unwrap();
        let data = table.get(self.index).clone();
        table.get_mut(self.index).clone_from(&self.data);
        drop(table);
        let mut unsaved = world.get_resource_mut::<unsaved::UnsavedAssets>().unwrap();
        unsaved
            .terrain_contours
            .insert(self.handle.clone(), unsaved::ModifiedState::Unsaved);
        world.commands().trigger(ChangeContourEvent(self.index));
        Box::new(RedoTerrainContourEdit {
            label: self.label,
            handle: self.handle.clone(),
            index: self.index,
            data,
        })
    }
}

#[derive(Debug, Event, Clone)]
pub(crate) struct RedoTerrainContourEdit {
    pub(crate) label: &'static str,
    pub(crate) handle: Handle<TerrainContoursTableAsset>,
    pub(crate) index: usize,
    pub(crate) data: TerrainContour,
}

impl RedoEntry for RedoTerrainContourEdit {
    fn label(&self) -> &str {
        self.label
    }

    fn redo(&self, world: &mut World) -> Box<dyn UndoEntry> {
        let mut assets = world
            .get_resource_mut::<Assets<TerrainContoursTableAsset>>()
            .unwrap();
        let asset = assets.get_mut(self.handle.id()).unwrap();
        let mut table = asset.0.write().unwrap();
        let data = table.get(self.index).clone();
        table.get_mut(self.index).clone_from(&self.data);
        drop(table);
        let mut unsaved = world.get_resource_mut::<unsaved::UnsavedAssets>().unwrap();
        unsaved
            .terrain_contours
            .insert(self.handle.clone(), unsaved::ModifiedState::Unsaved);
        world.commands().trigger(ChangeContourEvent(self.index));
        Box::new(UndoTerrainContourEdit {
            label: self.label,
            handle: self.handle.clone(),
            index: self.index,
            data,
        })
    }
}
