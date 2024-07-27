use bevy::prelude::*;
use directories::BaseDirs;

#[reflect_trait]
pub trait Preferences
where
    Self: 'static + Sync + Send,
{
}

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
pub struct PreferencesPath(pub std::path::PathBuf);

#[derive(Resource, Default)]
pub struct PreferencesChannged(pub bool);

impl Plugin for PreferencesPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PreferencesChannged>()
            .add_systems(Update, save_preferences);
        if let Some(base_dirs) = BaseDirs::new() {
            let prefs_path = base_dirs.preference_dir().join(&self.app_name);
            app.insert_resource(PreferencesPath(prefs_path.clone()));
            info!("Preferences path: {:?}", prefs_path);
        } else {
            warn!("Could not find user configuration directories");
        }
    }

    fn finish(&self, app: &mut App) {
        // Only load preferences if the preferences file exists
        if let Some(prefs_dir) = app.world().get_resource::<PreferencesPath>() {
            let prefs_file = prefs_dir.0.join("prefs.ron");
            if prefs_file.exists() && prefs_file.is_file() {
                // std::fs::create_dir_all(&prefs_path.0).unwrap();
            }
        } else {
            warn!("Could not find user configuration directories");
        }
        // Load preferences
    }
}

fn save_preferences(mut changed: ResMut<PreferencesChannged>, path: Option<Res<PreferencesPath>>) {
    if changed.0 {
        if let Some(path) = path {
            let prefs_file = path.0.join("prefs.ron");
            info!("Saving preferences path: {:?}", prefs_file);
        }
        changed.0 = false;
    }
}
