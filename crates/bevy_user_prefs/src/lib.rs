use std::path::PathBuf;

use bevy::{prelude::*, utils::HashMap};
use directories::BaseDirs;

/// Resource which represents the directory where individual preferences files are stored.
#[derive(Resource)]
pub struct Preferences {
    app_name: String,
    base_path: Option<PathBuf>,
    files: HashMap<String, PreferencesFile>,
}

impl Preferences {
    /// Save all preferences to disk
    pub fn save(&self) {
        for file in self.files.values() {
            if file.changed {
                // Save file
                todo!();
            }
        }
    }

    /// Lazily load a preferences file
    pub fn get_file(&mut self, file: Option<&str>) -> PreferencesFile {
        // Load file if not present
        todo!();
    }

    /// True if any preferences have been changed
    pub fn any_changed(&self) -> bool {
        self.files.values().any(|f| f.changed)
    }
}

/// Represents a single preferences file containing multiple groups of settings.
pub struct PreferencesFile {
    path: String,
    data: toml::Table,
    changed: bool,
}

impl PreferencesFile {
    pub fn get_group(&self, group: &str) -> Option<&toml::Table> {
        self.data.get(group).and_then(|v| v.as_table())
    }

    pub fn get_group_mut(&mut self, group: &str) -> &mut toml::Table {
        self.changed = true;
        let entry = self
            .data
            .entry(group.to_owned())
            .or_insert_with(|| toml::Value::Table(toml::Table::new()));
        entry.as_table_mut().unwrap()
    }

    pub fn set_property(&mut self, group: &str, key: &str, value: toml::Value) -> &mut Self {
        let group = self.get_group_mut(group);
        if group.get(key) != Some(&value) {
            group.insert(key.to_owned(), value);
            self.changed = true;
        }
        self
    }

    pub fn set_bool(&mut self, group: &str, key: &str, value: bool) -> &mut Self {
        self.set_property(group, key, toml::Value::Boolean(value))
    }

    pub fn set_string(&mut self, group: &str, key: &str, value: &str) -> &mut Self {
        self.set_property(group, key, toml::Value::String(value.to_owned()))
    }

    pub fn set_integer(&mut self, group: &str, key: &str, value: i64) -> &mut Self {
        self.set_property(group, key, toml::Value::Integer(value))
    }

    pub fn set_float(&mut self, group: &str, key: &str, value: f64) -> &mut Self {
        self.set_property(group, key, toml::Value::Float(value))
    }
}

/// Event broadcast when it's time to save preferences
pub struct SavingPreferences;

#[derive(Resource, Default)]
pub struct PreferencesDebounceTimer(f32);

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

impl Plugin for PreferencesPlugin {
    fn build(&self, app: &mut App) {
        let base_path = if let Some(base_dirs) = BaseDirs::new() {
            let prefs_path = base_dirs.preference_dir().join(&self.app_name);
            // app.insert_resource(PreferencesDir(prefs_path.clone()));
            info!("Preferences path: {:?}", prefs_path);
            Some(prefs_path.clone())
        } else {
            warn!("Could not find user configuration directories");
            None
        };

        app.init_resource::<PreferencesDebounceTimer>();

        // app.insert_resource(Preferences {
        //     app_name: self.app_name.clone(),
        //     base_path,
        //     files: todo!(),
        // });
        // app.init_resource::<PreferencesChanged>()
        //     .init_resource::<PreferencesDebounceTimer>()
        //     .add_systems(Update, save_preferences);
    }

    fn finish(&self, app: &mut App) {
        // Only load preferences if we were able to locate the user configuration directories.
        // if app.world().get_resource::<PreferencesDir>().is_some() {
        //     load::load_preferences(app.world_mut());
        // }
        // let tick = app.world_mut().change_tick();
        // app.world_mut().insert_resource(PreferencesSaveTick(tick));
    }
}

fn save_preferences(
    // mut changed: ResMut<PreferencesChanged>,
    mut timer: ResMut<PreferencesDebounceTimer>,
    time: Res<Time>,
    mut cmd: Commands,
) {
    // if changed.0 {
    timer.0 = (timer.0 - time.delta_secs()).max(0.0);
    if timer.0 <= 0.0 {
        // changed.0 = false;
        // cmd.queue(SavePreferences::Always);
    }
}
// }

// #[derive(Default)]
// pub struct SetPreferencesChanged;

// impl Command for SetPreferencesChanged {
//     fn apply(self, world: &mut World) {
//         let mut changed = world.get_resource_mut::<PreferencesChanged>().unwrap();
//         changed.0 = true;
//         let mut timer = world
//             .get_resource_mut::<PreferencesDebounceTimer>()
//             .unwrap();
//         timer.0 = 1.0;
//     }
// }
