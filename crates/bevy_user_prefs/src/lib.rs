use std::{fs, path::PathBuf};

use bevy::{prelude::*, utils::HashMap};
use directories::BaseDirs;

/// Resource which represents the directory where individual preferences files are stored.
/// You can access individual preferences files using the `.get_file()` method.
#[derive(Resource)]
pub struct Preferences {
    base_path: Option<PathBuf>,
    files: HashMap<String, PreferencesFile>,
}

impl Preferences {
    pub fn new(app_name: &str) -> Self {
        Self {
            base_path: if let Some(base_dirs) = BaseDirs::new() {
                let prefs_path = base_dirs.preference_dir().join(app_name);
                info!("Preferences path: {:?}", prefs_path);
                Some(prefs_path)
            } else {
                warn!("Could not find user configuration directories");
                None
            },
            files: HashMap::default(),
        }
    }

    /// Save all `PreferenceFile`s to disk
    pub fn save(&self) {
        if let Some(base_path) = &self.base_path {
            // Recursively create the preferences directory if it doesn't exist.
            let mut dir_builder = std::fs::DirBuilder::new();
            dir_builder.recursive(true);
            if let Err(e) = dir_builder.create(base_path.clone()) {
                warn!("Could not create preferences directory: {:?}", e);
                return;
            }

            for (filename, file) in self.files.iter() {
                let file_path = base_path.join(format!("{}.toml", filename));
                if let Err(e) = save_table(&file_path, &file.0) {
                    error!("Error saving preferences file: {}", e);
                }
            }
        }
    }

    /// Lazily load a `PreferencesFile`. If the file is already loaded, it will be returned
    /// immediately. If the file exists but is not loaded, it will be loaded and returned.
    /// If the file does not exist, or the base preference path cannot be determined, `None` will
    /// be returned.
    ///
    /// # Arguments
    /// * `filename` - The name of the preferences file, without the file extension.
    pub fn get<'a>(&'a mut self, filename: &str) -> Option<&'a mut PreferencesFile> {
        let Some(base_path) = &self.base_path else {
            return None;
        };

        if !self.files.contains_key(filename) {
            let file_path = base_path.join(format!("{}.toml", filename));
            let table = load_table(&file_path);
            if let Some(table) = table {
                self.files
                    .insert(filename.to_owned(), PreferencesFile(table));
            };
        }

        self.files.get_mut(filename)
    }

    /// Lazily load a preferences file, or create it if it does not exist. If the file is already
    /// loaded, it will be returned immediately. If the file exists but is not loaded, it will be
    /// loaded and returned. If the file does not exist, a new `PreferencesFile` will be created
    /// and returned (but not saved). If the base preference path cannot be determined, `None` will
    /// be returned.
    ///
    /// # Arguments
    /// * `filename` - The name of the preferences file, without the file extension.
    pub fn get_mut<'a>(&'a mut self, filename: &str) -> Option<&'a mut PreferencesFile> {
        let Some(base_path) = &self.base_path else {
            return None;
        };

        if !self.files.contains_key(filename) {
            let file_path = base_path.join(format!("{}.toml", filename));
            let table = load_table(&file_path);
            let prefs_file = if let Some(table) = table {
                PreferencesFile(table)
            } else {
                // Create new file
                PreferencesFile(toml::Table::new())
            };
            self.files.insert(filename.to_owned(), prefs_file);
        }

        self.files.get_mut(filename)
    }
}

/// Load a preferences file from disk
fn load_table(file: &PathBuf) -> Option<toml::Table> {
    if file.exists() && file.is_file() {
        let prefs_str = match fs::read_to_string(file) {
            Ok(prefs_str) => prefs_str,
            Err(e) => {
                error!("Error reading preferences file: {}", e);
                return None;
            }
        };

        let table_value = match toml::from_str::<toml::Value>(&prefs_str) {
            Ok(table_value) => table_value,
            Err(e) => {
                error!("Error parsing preferences file: {}", e);
                return None;
            }
        };

        match table_value {
            toml::Value::Table(table) => Some(table),
            _ => {
                error!("Preferences file must be a table");
                None
            }
        }
    } else {
        // Preferences file does not exist yet.
        None
    }
}

/// Save a preferences file to disk
fn save_table(file: &PathBuf, table: &toml::Table) -> Result<(), std::io::Error> {
    let toml_str = toml::to_string_pretty(&table).unwrap();
    fs::write(file, toml_str)
}

/// Represents a single preferences file containing multiple groups of settings.
pub struct PreferencesFile(pub toml::Table);

impl PreferencesFile {
    pub fn get_group(&self, group: &str) -> Option<PreferencesGroup> {
        self.0
            .get(group)
            .and_then(|v| v.as_table())
            .map(PreferencesGroup)
    }

    pub fn get_group_mut<'a>(&'a mut self, group: &str) -> Option<PreferencesGroupMut<'a>> {
        let entry = self
            .0
            .entry(group.to_owned())
            .or_insert_with(|| toml::Value::Table(toml::Table::new()));
        entry.as_table_mut().map(PreferencesGroupMut)
    }
}

pub struct PreferencesGroup<'a>(&'a toml::Table);
pub struct PreferencesGroupMut<'a>(&'a mut toml::Table);

impl<'a> PreferencesGroup<'a> {
    /// Read a boolean property from the group, or `None` if the property does not exist or is not
    /// a boolean.
    pub fn get_bool(&self, key: &str) -> Option<bool> {
        self.0.get(key).and_then(|v| v.as_bool())
    }

    /// Read a string property from the group, or `None` if the property does not exist or is not
    /// a string.
    pub fn get_string(&self, key: &str) -> Option<&str> {
        self.0.get(key).and_then(|v| v.as_str())
    }

    /// Read an integer property from the group, or `None` if the property does not exist or is not
    /// an integer.
    pub fn get_integer(&self, key: &str) -> Option<i64> {
        self.0.get(key).and_then(|v| v.as_integer())
    }

    /// Read a float property from the group, or `None` if the property does not exist or is not
    /// a float.
    pub fn get_float(&self, key: &str) -> Option<f64> {
        self.0.get(key).and_then(|v| v.as_float())
    }

    /// Read an `IVec2` property from the group, or `None` if the property does not exist or
    /// is not a valid `IVec2`.
    pub fn get_ivec2(&self, key: &str) -> Option<IVec2> {
        self.0.get(key).and_then(value_to_ivec2)
    }

    /// Read a `UVec2` property from the group, or `None` if the property does not exist or
    /// is not a valid `UVec2`.
    pub fn get_uvec2(&self, key: &str) -> Option<UVec2> {
        self.0.get(key).and_then(value_to_uvec2)
    }

    /// Read a `Vec2` property from the group, or `None` if the property does not exist or
    /// is not a valid `Vec2`.
    pub fn get_vec2(&self, key: &str) -> Option<Vec2> {
        self.0.get(key).and_then(value_to_vec2)
    }
}

impl<'a> PreferencesGroupMut<'a> {
    /// Delete a key from the preferences group.
    pub fn remove(&mut self, key: &str) {
        self.0.remove(key);
    }

    /// Read a boolean property from the group, or `None` if the property does not exist or is not
    /// a boolean.
    pub fn get_bool(&self, key: &str) -> Option<bool> {
        self.0.get(key).and_then(|v| v.as_bool())
    }

    /// Set a boolean property in the group.
    pub fn set_bool(&mut self, key: &str, value: bool) -> &mut Self {
        self.0.insert(key.to_owned(), toml::Value::Boolean(value));
        self
    }

    /// Read a string property from the group, or `None` if the property does not exist or is not
    /// a string.
    pub fn get_string(&self, key: &str) -> Option<&str> {
        self.0.get(key).and_then(|v| v.as_str())
    }

    /// Set a string property in the group.
    pub fn set_string(&mut self, key: &str, value: &str) -> &mut Self {
        self.0
            .insert(key.to_owned(), toml::Value::String(value.to_owned()));
        self
    }

    /// Read an integer property from the group, or `None` if the property does not exist or is not
    /// an integer.
    pub fn get_integer(&self, key: &str) -> Option<i64> {
        self.0.get(key).and_then(|v| v.as_integer())
    }

    /// Set an integer property in the group.
    pub fn set_integer(&mut self, key: &str, value: i64) -> &mut Self {
        self.0.insert(key.to_owned(), toml::Value::Integer(value));
        self
    }

    /// Read a float property from the group, or `None` if the property does not exist or is not
    /// a float.
    pub fn get_float(&self, key: &str) -> Option<f64> {
        self.0.get(key).and_then(|v| v.as_float())
    }

    /// Set a float property in the group.
    pub fn set_float(&mut self, key: &str, value: f64) -> &mut Self {
        self.0.insert(key.to_owned(), toml::Value::Float(value));
        self
    }

    /// Read an `IVec2` property from the group, or `None` if the property does not exist or
    /// is not a valid `IVec2`.
    pub fn get_ivec2(&self, key: &str) -> Option<IVec2> {
        self.0.get(key).and_then(value_to_ivec2)
    }

    /// Set an `IVec2` property in the group.
    pub fn set_ivec2(&mut self, key: &str, value: IVec2) -> &mut Self {
        self.0.insert(
            key.to_owned(),
            toml::Value::Array(vec![
                toml::Value::Integer(value.x as i64),
                toml::Value::Integer(value.y as i64),
            ]),
        );
        self
    }

    /// Read a `UVec2` property from the group, or `None` if the property does not exist or
    /// is not a valid `UVec2`.
    pub fn get_uvec2(&self, key: &str) -> Option<UVec2> {
        self.0.get(key).and_then(value_to_uvec2)
    }

    /// Set a `UVec2` property in the group.
    pub fn set_uvec2(&mut self, key: &str, value: UVec2) -> &mut Self {
        self.0.insert(
            key.to_owned(),
            toml::Value::Array(vec![
                toml::Value::Integer(value.x as i64),
                toml::Value::Integer(value.y as i64),
            ]),
        );
        self
    }

    /// Read a `Vec2` property from the group, or `None` if the property does not exist or
    /// is not a valid `Vec2`.
    pub fn get_vec2(&self, key: &str) -> Option<Vec2> {
        self.0.get(key).and_then(value_to_vec2)
    }

    /// Set a `Vec2` property in the group.
    pub fn set_vec2(&mut self, key: &str, value: Vec2) -> &mut Self {
        self.0.insert(
            key.to_owned(),
            toml::Value::Array(vec![
                toml::Value::Float(value.x as f64),
                toml::Value::Float(value.y as f64),
            ]),
        );
        self
    }
}

fn value_to_ivec2(value: &toml::Value) -> Option<IVec2> {
    if let toml::Value::Array(a) = value {
        if a.len() == 2 {
            if let (Some(a0), Some(a1)) = (a[0].as_integer(), a[1].as_integer()) {
                return Some(IVec2::new(a0 as i32, a1 as i32));
            }
        }
    }
    None
}

fn value_to_uvec2(value: &toml::Value) -> Option<UVec2> {
    if let toml::Value::Array(a) = value {
        if a.len() == 2 {
            if let (Some(a0), Some(a1)) = (a[0].as_integer(), a[1].as_integer()) {
                if a0 >= 0 && a1 >= 0 {
                    return Some(UVec2::new(a0 as u32, a1 as u32));
                }
            }
        }
    }
    None
}

fn value_to_vec2(value: &toml::Value) -> Option<Vec2> {
    if let toml::Value::Array(a) = value {
        if a.len() == 2 {
            if let (Some(a0), Some(a1)) = (a[0].as_float(), a[1].as_float()) {
                return Some(Vec2::new(a0 as f32, a1 as f32));
            }
        }
    }
    None
}

/// Event triggered when it's time to save preferences
#[derive(Clone, Debug, Event)]
pub struct LoadPreferences;

#[derive(Resource, Default)]
pub struct PreferencesDebounceTimer(f32);

#[derive(Resource, Default)]
pub struct PreferencesChanged(bool);

/// Plugin which automatically saves preferences when they change. This uses a delay timer
/// to prevent saving preferences too frequently. Preferences will be automatically saved 1 second
/// after they have been marked as changed.
pub struct AutosavePrefsPlugin;

impl Plugin for AutosavePrefsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PreferencesDebounceTimer>();
        app.init_resource::<PreferencesChanged>();
    }

    fn finish(&self, app: &mut App) {
        // Only load preferences if we were able to locate the user configuration directories.
        let prefs = app.world().get_resource::<Preferences>().unwrap();
        if prefs.base_path.is_some() {
            info!("Loading Preferences from: {:?}", prefs.base_path);
            app.world_mut().trigger(LoadPreferences);
        }
        app.add_systems(Update, auto_save_preferences);
    }
}

fn auto_save_preferences(
    mut changed: ResMut<PreferencesChanged>,
    mut timer: ResMut<PreferencesDebounceTimer>,
    time: Res<Time>,
    mut cmd: Commands,
) {
    if changed.0 {
        timer.0 = (timer.0 - time.delta_secs()).max(0.0);
        if timer.0 <= 0.0 {
            changed.0 = false;
            cmd.queue(SavePreferences::Always);
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

#[derive(Default, PartialEq)]
pub enum SavePreferences {
    /// Save preferences only if they have changed (based on [`PreferencesChanged` resource]).
    #[default]
    IfChanged,
    /// Save preferences unconditionally.
    Always,
}

impl Command for SavePreferences {
    fn apply(self, world: &mut World) {
        let mut changed = world.get_resource_mut::<PreferencesChanged>().unwrap();
        if changed.0 || self == SavePreferences::Always {
            changed.0 = false;
            let prefs = world.get_resource::<Preferences>().unwrap();
            info!("Saving preferences");
            prefs.save();
        }
    }
}
