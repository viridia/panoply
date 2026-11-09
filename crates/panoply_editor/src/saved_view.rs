//! The editor's view, saved between runs, so an iteration need not navigate back. Starting in play,
//! the camera follows the player instead, and the saved view waits for a run started in the editor.

use bevy::{
    asset::LoadedFolder,
    prelude::*,
    settings::{ReflectSettingsGroup, SaveSettingsDeferred, SettingsGroup},
};
use panoply_core::{realm::RealmsHandleResource, GameMode, Realm, Viewpoint};

use crate::{mode_selector::EditorModeSettings, shell::SAVE_DELAY};

/// Where the editor's view was. `realm` is `None` until a first save.
#[derive(Resource, SettingsGroup, Reflect, Clone, PartialEq, Debug, Default)]
#[reflect(Resource, SettingsGroup, Default)]
#[settings_group(group = "editor_view")]
pub struct EditorViewSettings {
    pub realm: Option<String>,
    pub position: Vec3,
    pub azimuth: f32,
}

pub struct SavedViewPlugin;

impl Plugin for SavedViewPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, sync_editor_view);
    }
}

/// Restores the view once every realm exists, then saves it as it changes in the editor. Nothing is
/// saved before the restore, which would overwrite the view about to be restored.
fn sync_editor_view(
    folders: Res<Assets<LoadedFolder>>,
    folder: Res<RealmsHandleResource>,
    realms: Query<(Entity, &Realm)>,
    mode: Res<EditorModeSettings>,
    mut settings: ResMut<EditorViewSettings>,
    mut viewpoint: ResMut<Viewpoint>,
    mut commands: Commands,
    mut restored: Local<bool>,
) {
    if !*restored {
        let Some(loaded) = folders.get(&folder.0) else {
            return;
        };
        if realms.iter().count() < loaded.handles.len() {
            return;
        }
        *restored = true;
        let saved = realms
            .iter()
            .find(|(_, realm)| settings.realm.as_ref() == Some(&realm.name));
        // A saved realm that is gone leaves the app's default in place.
        if let (GameMode::Edit, Some((entity, _))) = (mode.game_mode, saved) {
            viewpoint.realm = Some(entity);
            viewpoint.position = settings.position;
            viewpoint.azimuth = settings.azimuth;
            viewpoint.follow = None;
        }
        return;
    }

    if mode.game_mode != GameMode::Edit || !viewpoint.is_changed() {
        return;
    }
    let Some((_, realm)) = viewpoint.realm.and_then(|e| realms.get(e).ok()) else {
        return;
    };
    let view = EditorViewSettings {
        realm: Some(realm.name.clone()),
        position: viewpoint.position,
        azimuth: viewpoint.azimuth,
    };
    if settings.set_if_neq(view) {
        commands.queue(SaveSettingsDeferred(SAVE_DELAY));
    }
}
