//! Edits to precincts, from scenery mode's placing tools. Each changes the precinct's asset in
//! memory, where the precinct's rebuild shows it at once, marks it unsaved for the save button, and
//! can be undone. A precinct with no file gets an empty asset at its first edit, and saving writes
//! the file.
//!
//! The editor holds every precinct it has edited: the precinct cache drops a precinct out of view,
//! and with it the last handle to its asset, which would take unsaved edits with it.

use std::borrow::Cow;

use bevy::{ecs::system::SystemParam, platform::collections::HashSet, prelude::*};
use panoply_precinct::PrecinctAsset;

use crate::{
    mode_selector::EditorMode,
    undo::{UndoAction, UndoStack},
    unsaved::UnsavedAssets,
};

/// The precincts edited this session, held so they stay loaded.
#[derive(Resource, Default)]
struct EditedPrecincts(HashSet<Handle<PrecinctAsset>>);

pub struct PrecinctEditPlugin;

impl Plugin for PrecinctEditPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<EditedPrecincts>();
    }
}

/// Edits precinct assets.
#[derive(SystemParam)]
pub(crate) struct PrecinctEdits<'w> {
    assets: ResMut<'w, Assets<PrecinctAsset>>,
    unsaved: ResMut<'w, UnsavedAssets>,
    history: ResMut<'w, UndoStack>,
    held: ResMut<'w, EditedPrecincts>,
    /// Absent during play, which makes no edits.
    mode: Option<Res<'w, State<EditorMode>>>,
}

impl PrecinctEdits<'_> {
    /// The precinct `handle`'s asset, if it has one yet.
    pub(crate) fn get(&self, handle: &Handle<PrecinctAsset>) -> Option<&PrecinctAsset> {
        self.assets.get(handle)
    }

    /// Changes the precinct `handle` with `edit`, which returns the edit's name for the history,
    /// as "Place 6 walls", or `None` to refuse, leaving the precinct and the history as they were.
    pub(crate) fn edit(
        &mut self,
        handle: &Handle<PrecinctAsset>,
        edit: impl FnOnce(&mut PrecinctAsset) -> Option<Cow<'static, str>>,
    ) {
        let Some(mode) = self.mode.as_ref().map(|mode| *mode.get()) else {
            return;
        };
        let before = self.assets.get(handle).cloned().unwrap_or_default();
        let mut after = before.clone();
        let Some(label) = edit(&mut after) else {
            return;
        };
        if let Err(error) = self.assets.insert(handle, after) {
            warn!("{label} did nothing: {error}");
            return;
        }
        info!("{label}");
        self.held.0.insert(handle.clone());
        self.unsaved.mark(handle);
        self.history.push(
            mode,
            PrecinctUndo {
                label,
                asset: handle.id(),
                precinct: before,
            },
        );
    }
}

/// Puts back a precinct as it was before an edit.
struct PrecinctUndo {
    label: Cow<'static, str>,
    asset: AssetId<PrecinctAsset>,
    precinct: PrecinctAsset,
}

impl UndoAction for PrecinctUndo {
    fn apply(self: Box<Self>, world: &mut World) -> Box<dyn UndoAction> {
        let PrecinctUndo {
            label,
            asset,
            precinct,
        } = *self;
        let replaced = {
            let mut assets = world.resource_mut::<Assets<PrecinctAsset>>();
            assets
                .get_mut(asset)
                .map(|mut current| std::mem::replace(&mut *current, precinct.clone()))
        };
        let Some(replaced) = replaced else {
            warn!("{label} did nothing: the precinct is gone");
            return Box::new(PrecinctUndo {
                label,
                asset,
                precinct,
            });
        };
        world.resource_mut::<UnsavedAssets>().mark(asset);
        Box::new(PrecinctUndo {
            label,
            asset,
            precinct: replaced,
        })
    }

    fn label(&self) -> Cow<'static, str> {
        self.label.clone()
    }
}
