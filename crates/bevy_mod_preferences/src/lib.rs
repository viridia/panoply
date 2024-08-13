mod load;
mod save;
mod watch;

use bevy::{
    ecs::{component::Tick, world::Command},
    prelude::*,
};
use directories::BaseDirs;
pub use save::SavePreferences;
pub use watch::watch_prefs_changes;

/// Annotation for a type which causes the type's contents to be placed in a named table
/// in the preferences file.
#[derive(Debug, Clone, Reflect)]
pub struct PreferencesGroup(pub &'static str);

/// Annotation for a type which set the configuration key used to store the type in the
/// preferences file. If a tuple struct contains both a `PreferencesGroup` and a `PreferencesKey`,
/// the key will be placed in the group. Otherwise it will be a top-level key.
#[derive(Debug, Clone, Reflect)]
pub struct PreferencesKey(pub &'static str);

/// Resource for tracking the last tick at which preferences were saved.
#[derive(Debug, Clone, Resource)]
pub struct PreferencesSaveTick(pub Tick);

pub struct PreferencesPlugin {
    pub app_name: String,
}

impl PreferencesPlugin {
    pub fn new(app_name: &str) -> Self {
        Self {
            app_name: app_name.to_string(),
        }
    }
}

impl Default for PreferencesPlugin {
    fn default() -> Self {
        Self {
            app_name: "bevy_app".to_string(),
        }
    }
}

#[derive(Resource)]
pub struct PreferencesDir(pub std::path::PathBuf);

#[derive(Resource, Default)]
pub struct PreferencesChanged(bool);

#[derive(Resource, Default)]
pub struct PreferencesDebounceTimer(f32);

impl Plugin for PreferencesPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PreferencesChanged>()
            .init_resource::<PreferencesDebounceTimer>()
            .add_systems(Update, save_preferences);
        if let Some(base_dirs) = BaseDirs::new() {
            let prefs_path = base_dirs.preference_dir().join(&self.app_name);
            app.insert_resource(PreferencesDir(prefs_path.clone()));
            info!("Preferences path: {:?}", prefs_path);
        } else {
            warn!("Could not find user configuration directories");
        }
    }

    fn finish(&self, app: &mut App) {
        // Only load preferences if we were able to locate the user configuration directories.
        if app.world().get_resource::<PreferencesDir>().is_some() {
            load::load_preferences(app.world_mut());
        }
        let tick = app.world_mut().change_tick();
        app.world_mut().insert_resource(PreferencesSaveTick(tick));
    }
}

fn save_preferences(
    mut changed: ResMut<PreferencesChanged>,
    mut timer: ResMut<PreferencesDebounceTimer>,
    time: Res<Time>,
    mut cmd: Commands,
) {
    if changed.0 {
        timer.0 = (timer.0 - time.delta_seconds()).max(0.0);
        if timer.0 <= 0.0 {
            changed.0 = false;
            cmd.add(SavePreferences::Always);
        }
    }
}

#[derive(Default)]
pub struct SetPreferencesChanged;

impl Command for SetPreferencesChanged {
    fn apply(self, world: &mut World) {
        let mut changed = world.get_resource_mut::<PreferencesChanged>().unwrap();
        changed.0 = true;
        let mut timer = world
            .get_resource_mut::<PreferencesDebounceTimer>()
            .unwrap();
        timer.0 = 1.0;
    }
}
