//! Edits to precincts, from scenery mode's placing tools. Each changes the precinct's asset in
//! memory, where the precinct's rebuild shows it at once, marks it unsaved for the save button, and
//! can be undone. A precinct with no file has no asset until its first edit, which makes one with
//! the precinct's path, and saving writes the file and lists it in the realm's precinct index.
//!
//! The editor holds every precinct it has edited: the precinct cache drops a precinct out of view,
//! and with it the last handle to its asset, which would take unsaved edits with it.

use std::borrow::Cow;

use bevy::{ecs::system::SystemParam, platform::collections::HashSet, prelude::*};
use panoply_core::Realm;
use panoply_precinct::{precinct_path, PrecinctAsset, SceneryInstanceId, TakenInstance};
use panoply_scenery::precinct::Precinct;

use crate::{
    mode_selector::EditorMode,
    undo::{UndoAction, UndoStack},
    unsaved::UnsavedAssets,
};

/// The precincts edited this session, held so they stay loaded.
#[derive(Resource, Default)]
pub(crate) struct EditedPrecincts(HashSet<Handle<PrecinctAsset>>);

pub struct PrecinctEditPlugin;

impl Plugin for PrecinctEditPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<EditedPrecincts>();
    }
}

/// Edits precinct assets.
#[derive(SystemParam)]
pub(crate) struct PrecinctEdits<'w, 's> {
    assets: ResMut<'w, Assets<PrecinctAsset>>,
    unsaved: ResMut<'w, UnsavedAssets>,
    history: ResMut<'w, UndoStack>,
    held: ResMut<'w, EditedPrecincts>,
    /// Absent during play, which makes no edits.
    mode: Option<Res<'w, State<EditorMode>>>,
    /// Absent in tests, which edit only precincts that have assets.
    server: Option<Res<'w, AssetServer>>,
    realms: Query<'w, 's, &'static Realm>,
    commands: Commands<'w, 's>,
}

impl PrecinctEdits<'_, '_> {
    /// The precinct's asset, if it has one yet.
    pub(crate) fn get(&self, precinct: &Precinct) -> Option<&PrecinctAsset> {
        precinct.asset_in(&self.assets)
    }

    /// Changes the precinct `handle` with `edit`, which returns the edit's name for the history,
    /// as "Place 6 walls", or `None` to refuse, leaving the precinct and the history as they were.
    pub(crate) fn edit(
        &mut self,
        handle: &Handle<PrecinctAsset>,
        edit: impl FnOnce(&mut PrecinctAsset) -> Option<Cow<'static, str>>,
    ) {
        self.edit_all(std::array::from_ref(handle), |assets| {
            let [asset] = assets;
            edit(asset)
        });
    }

    /// Changes the precinct `entity`, whose component is `precinct`, with `edit`, as
    /// [`edit`](Self::edit) does. A precinct with no file is given an asset, at the path of the
    /// file saving will write, if `edit` is not refused.
    pub(crate) fn edit_precinct(
        &mut self,
        entity: Entity,
        precinct: &Precinct,
        edit: impl FnOnce(&mut PrecinctAsset) -> Option<Cow<'static, str>>,
    ) {
        if let Some(handle) = &precinct.asset {
            self.edit(handle, edit);
            return;
        }
        let mut asset = PrecinctAsset::default();
        let Some(label) = edit(&mut asset) else {
            return;
        };
        let (Some(server), Ok(realm)) = (&self.server, self.realms.get(precinct.realm)) else {
            return;
        };
        // The file is not there yet, so this load fails, and logs it, once.
        let handle: Handle<PrecinctAsset> =
            server.load(precinct_path(&realm.name, precinct.coords));
        let given = handle.clone();
        self.commands
            .entity(entity)
            .queue(move |mut entity: EntityWorldMut| {
                if let Some(mut precinct) = entity.get_mut::<Precinct>() {
                    precinct.asset = Some(given);
                }
            });
        self.edit(&handle, move |new| {
            *new = asset;
            Some(label)
        });
    }

    /// Changes the two precincts `a` and `b` together with `edit`, as one entry in the history,
    /// as [`edit`](Self::edit) changes one. Refused if they are the same precinct.
    #[allow(dead_code)] // The instance gizmo moves instances across precinct edges (PAN92).
    pub(crate) fn edit_two(
        &mut self,
        a: &Handle<PrecinctAsset>,
        b: &Handle<PrecinctAsset>,
        edit: impl FnOnce(&mut PrecinctAsset, &mut PrecinctAsset) -> Option<Cow<'static, str>>,
    ) {
        if a == b {
            return;
        }
        self.edit_all(&[a.clone(), b.clone()], |assets| {
            let [a, b] = assets;
            edit(a, b)
        });
    }

    /// Moves the instance whose id is `iid` from the precinct `from` to `to`, keeping its key and
    /// label, to `position`: for scenery, relative to `to`, and for an actor, in the world.
    #[allow(dead_code)] // The instance gizmo moves instances across precinct edges (PAN92).
    pub(crate) fn move_instance(
        &mut self,
        from: &Handle<PrecinctAsset>,
        to: &Handle<PrecinctAsset>,
        iid: &SceneryInstanceId,
        position: Vec3,
    ) {
        self.edit_two(from, to, |from, to| {
            let mut taken = from.take_instance(iid)?;
            match &mut taken {
                TakenInstance::Scenery { instance, .. } => instance.position = position,
                TakenInstance::Actor(actor) => actor.position = position,
            }
            to.insert_instance(taken);
            Some("Move instance to another precinct".into())
        });
    }

    /// Changes the precincts `handles` with `edit`, given their assets in the same order.
    fn edit_all<const N: usize>(
        &mut self,
        handles: &[Handle<PrecinctAsset>; N],
        edit: impl FnOnce(&mut [PrecinctAsset; N]) -> Option<Cow<'static, str>>,
    ) {
        let Some(mode) = self.mode.as_ref().map(|mode| *mode.get()) else {
            return;
        };
        let before = handles
            .each_ref()
            .map(|handle| self.assets.get(handle).cloned().unwrap_or_default());
        let mut after = before.clone();
        let Some(label) = edit(&mut after) else {
            return;
        };
        for (handle, asset) in handles.iter().zip(after) {
            if let Err(error) = self.assets.insert(handle, asset) {
                warn!("{label} did nothing: {error}");
                return;
            }
        }
        info!("{label}");
        for handle in handles {
            self.held.0.insert(handle.clone());
            self.unsaved.mark(handle);
        }
        let precincts = handles
            .iter()
            .map(|handle| handle.id())
            .zip(before)
            .collect();
        self.history.push(mode, PrecinctUndo { label, precincts });
    }
}

/// Puts back precincts as they were before an edit.
struct PrecinctUndo {
    label: Cow<'static, str>,
    precincts: Vec<(AssetId<PrecinctAsset>, PrecinctAsset)>,
}

impl UndoAction for PrecinctUndo {
    fn apply(self: Box<Self>, world: &mut World) -> Box<dyn UndoAction> {
        let PrecinctUndo { label, precincts } = *self;
        let mut assets = world.resource_mut::<Assets<PrecinctAsset>>();
        if precincts.iter().any(|(id, _)| !assets.contains(*id)) {
            warn!("{label} did nothing: the precinct is gone");
            return Box::new(PrecinctUndo { label, precincts });
        }
        let precincts: Vec<_> = precincts
            .into_iter()
            .map(|(id, precinct)| {
                let mut current = assets.get_mut(id).unwrap();
                (id, std::mem::replace(&mut *current, precinct))
            })
            .collect();
        let mut unsaved = world.resource_mut::<UnsavedAssets>();
        for (id, _) in &precincts {
            unsaved.mark(*id);
        }
        Box::new(PrecinctUndo { label, precincts })
    }

    fn label(&self) -> Cow<'static, str> {
        self.label.clone()
    }
}
