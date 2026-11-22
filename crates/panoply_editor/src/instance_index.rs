//! The index of instances found from outside their precincts (`docs/instances.md`, Ids): one file
//! for the game, [`INDEX_PATH`], with a line for each labelled instance giving its key, label,
//! realm, precinct and kind. The editor reads it at startup and rewrites a precinct's lines as it
//! saves the precinct; `asset_xref.py` regenerates it and compares.

use std::{
    fmt,
    path::{Path, PathBuf},
};

use bevy::{platform::collections::HashSet, prelude::*};
use panoply_precinct::{InstanceKey, PrecinctAsset};
use serde::{Deserialize, Serialize};

use crate::unsaved::{asset_file, write_replacing};

/// The index's file, in the assets folder.
pub const INDEX_PATH: &str = "scenery/precincts/instances.json";

/// Where a precinct is: its realm and coordinates.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct PrecinctPlace {
    pub realm: String,
    pub coords: IVec2,
}

impl PrecinctPlace {
    /// The place of the precinct whose file is `path` in the assets folder, as
    /// `scenery/precincts/holm-overland/p010-p016.msgpack`.
    pub fn of_path(path: &Path) -> Option<Self> {
        let realm = path.parent()?.file_name()?.to_str()?.to_string();
        let (x, z) = path.file_stem()?.to_str()?.split_once('-')?;
        let coord = |text: &str| -> Option<i32> {
            let (sign, digits) = text.split_at_checked(1)?;
            let n: i32 = digits.parse().ok()?;
            match sign {
                "p" => Some(n),
                "n" => Some(-n),
                _ => None,
            }
        };
        Some(Self {
            realm,
            coords: IVec2::new(coord(x)?, coord(z)?),
        })
    }
}

/// As the editor shows it: `holm-overland (10, 16)`.
impl fmt::Display for PrecinctPlace {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({}, {})", self.realm, self.coords.x, self.coords.y)
    }
}

/// What an indexed instance is.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum InstanceKind {
    Actor,
    Scenery,
}

/// A line of the index.
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
#[serde(from = "EntryLine", into = "EntryLine")]
pub struct IndexEntry {
    pub key: InstanceKey,
    pub label: Option<String>,
    pub place: PrecinctPlace,
    pub kind: InstanceKind,
}

/// An entry as its line holds it: `[key, label, realm, [x, y], kind]`.
type EntryLine = (InstanceKey, Option<String>, String, [i32; 2], InstanceKind);

impl From<EntryLine> for IndexEntry {
    fn from((key, label, realm, [x, y], kind): EntryLine) -> Self {
        Self {
            key,
            label,
            place: PrecinctPlace {
                realm,
                coords: IVec2::new(x, y),
            },
            kind,
        }
    }
}

impl From<IndexEntry> for EntryLine {
    fn from(entry: IndexEntry) -> Self {
        let coords = entry.place.coords;
        (
            entry.key,
            entry.label,
            entry.place.realm,
            [coords.x, coords.y],
            entry.kind,
        )
    }
}

/// The index, its entries sorted by key.
#[derive(Resource, Default, Debug)]
pub struct InstanceIndex(Vec<IndexEntry>);

impl InstanceIndex {
    /// The index in `json`.
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        let mut entries: Vec<IndexEntry> = serde_json::from_str(json)?;
        entries.sort_by_key(|entry| entry.key);
        Ok(Self(entries))
    }

    /// The index's file: a JSON array with one entry to a line, so a diff shows each one changed.
    pub fn to_json(&self) -> String {
        let lines: Vec<String> = self
            .0
            .iter()
            .map(|entry| serde_json::to_string(entry).unwrap())
            .collect();
        match lines.is_empty() {
            true => "[]\n".to_string(),
            false => format!("[\n  {}\n]\n", lines.join(",\n  ")),
        }
    }

    /// Replaces the entries of the precinct at `place` with its labelled instances in `asset`.
    /// Returns whether that changed the index.
    pub fn replace_precinct(&mut self, place: &PrecinctPlace, asset: &PrecinctAsset) -> bool {
        let scenery = asset
            .scenery
            .iter()
            .map(|instance| (&instance.iid, &instance.label, InstanceKind::Scenery));
        let actors = asset
            .actors
            .iter()
            .map(|actor| (&actor.iid, &actor.label, InstanceKind::Actor));
        let mut entries: Vec<IndexEntry> = self
            .0
            .iter()
            .filter(|entry| entry.place != *place)
            .cloned()
            .collect();
        entries.extend(scenery.chain(actors).filter_map(|(iid, label, kind)| {
            Some(IndexEntry {
                key: iid.key()?,
                label: Some(label.as_deref()?.to_string()),
                place: place.clone(),
                kind,
            })
        }));
        entries.sort_by_key(|entry| entry.key);
        let changed = entries != self.0;
        self.0 = entries;
        changed
    }

    /// The labels the index gives the precincts not in `loaded`.
    pub fn labels_outside<'a>(
        &'a self,
        loaded: &'a HashSet<PrecinctPlace>,
    ) -> impl Iterator<Item = &'a str> {
        self.0
            .iter()
            .filter(|entry| !loaded.contains(&entry.place))
            .filter_map(|entry| entry.label.as_deref())
    }

    /// The entry for `key`.
    #[cfg(test)]
    fn get(&self, key: InstanceKey) -> Option<&IndexEntry> {
        self.0.iter().find(|entry| entry.key == key)
    }
}

pub struct InstanceIndexPlugin;

impl Plugin for InstanceIndexPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(read_index());
    }
}

/// The index's file on disk.
fn index_file() -> PathBuf {
    asset_file(Path::new(INDEX_PATH))
}

/// The index as its file holds it, or empty if it has none or cannot be read.
fn read_index() -> InstanceIndex {
    let file = index_file();
    let json = match std::fs::read_to_string(&file) {
        Ok(json) => json,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return default(),
        Err(error) => {
            error!("Could not read {}: {error}", file.display());
            return default();
        }
    };
    InstanceIndex::from_json(&json).unwrap_or_else(|error| {
        error!("Could not read {}: {error}", file.display());
        default()
    })
}

/// Rewrites the index's entries for the precinct `place` from `asset`, and its file if they
/// changed.
pub(crate) fn update_index(
    index: &mut InstanceIndex,
    place: &PrecinctPlace,
    asset: &PrecinctAsset,
) -> Result<(), BevyError> {
    if index.replace_precinct(place, asset) {
        let file = index_file();
        write_replacing(&file, index.to_json().as_bytes())?;
        info!("Saved {}", file.display());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use bevy::{ecs::system::RunSystemOnce, math::Vec3};
    use panoply_precinct::SceneryInstanceId;

    use super::*;

    const CHEST: &str = "exemplars/furnishings.exem.json#WoodenChest";

    fn place(x: i32, y: i32) -> PrecinctPlace {
        PrecinctPlace {
            realm: "holm-overland".into(),
            coords: IVec2::new(x, y),
        }
    }

    #[test]
    fn a_place_is_read_from_its_path() {
        let path = Path::new("scenery/precincts/holm-overland/p010-n003.msgpack");
        assert_eq!(PrecinctPlace::of_path(path), Some(place(10, -3)));
        assert_eq!(place(10, 16).to_string(), "holm-overland (10, 16)");
        assert_eq!(PrecinctPlace::of_path(Path::new("x/q010-p000.msgpack")), None);
    }

    /// The index lists only labelled instances, one to a line, and reads back as it was written.
    #[test]
    fn the_index_holds_labelled_instances() {
        let mut asset = PrecinctAsset {
            scenery_types: vec![CHEST.into()],
            ..default()
        };
        let chest = asset.add_scenery_element(0, 0., Vec3::ZERO, None);
        asset.set_label(&chest, Some("strongbox")).unwrap();
        asset.add_scenery_element(0, 0., Vec3::X, None);
        let guard = asset.add_actor("exemplars/characters.exem.json#Guard".into(), 0., Vec3::Z);
        let stone = asset.add_actor("exemplars/characters.exem.json#Guard".into(), 0., Vec3::Y);
        asset.set_label(&stone, Some("sgt.stone")).unwrap();

        let mut index = InstanceIndex::default();
        assert!(index.replace_precinct(&place(10, 16), &asset));
        assert!(!index.replace_precinct(&place(10, 16), &asset));
        assert_eq!(index.0.len(), 2);
        assert!(index.get(guard.key().unwrap()).is_none());
        let json = index.to_json();
        assert_eq!(json.lines().count(), 4);
        assert!(json.contains(&format!(
            r#"["{}","sgt.stone","holm-overland",[10,16],"actor"]"#,
            stone.key().unwrap()
        )));
        assert_eq!(InstanceIndex::from_json(&json).unwrap().0, index.0);
    }

    /// Labels in use across the game: the index's for precincts not loaded.
    #[test]
    fn labels_outside_the_loaded_precincts() {
        let mut asset = PrecinctAsset::default();
        let stone = asset.add_actor("exemplars/characters.exem.json#Guard".into(), 0., Vec3::Y);
        asset.set_label(&stone, Some("sgt.stone")).unwrap();
        let mut index = InstanceIndex::default();
        index.replace_precinct(&place(10, 16), &asset);
        let loaded = HashSet::from_iter([place(10, 16)]);
        assert_eq!(index.labels_outside(&loaded).count(), 0);
        let loaded = HashSet::from_iter([place(1, 1)]);
        assert_eq!(index.labels_outside(&loaded).collect::<Vec<_>>(), ["sgt.stone"]);
    }

    /// A labelled instance moved into another precinct keeps its key, and its line in the index
    /// names its new precinct once both are saved.
    #[test]
    fn the_index_follows_a_move() {
        let mut a = PrecinctAsset {
            scenery_types: vec![CHEST.into()],
            ..default()
        };
        let mut b = PrecinctAsset::default();
        let chest = a.add_scenery_element(0, 0., Vec3::ZERO, None);
        let chest = a.set_label(&chest, Some("strongbox")).unwrap();
        let key = chest.key().unwrap();
        let mut index = InstanceIndex::default();
        index.replace_precinct(&place(1, 1), &a);
        index.replace_precinct(&place(1, 2), &b);
        assert_eq!(index.get(key).unwrap().place, place(1, 1));

        let mut world = edit_world([&a, &b]);
        let handles = world.resource::<Handles>().0.clone();
        world
            .run_system_once(move |mut edits: crate::precinct_edit::PrecinctEdits| {
                edits.move_instance(&handles[0], &handles[1], &chest, Vec3::X);
            })
            .unwrap();
        let handles = &world.resource::<Handles>().0;
        let assets = world.resource::<Assets<PrecinctAsset>>();
        (a, b) = (
            assets.get(&handles[0]).unwrap().clone(),
            assets.get(&handles[1]).unwrap().clone(),
        );
        assert!(!a.has_instance(&SceneryInstanceId::Keyed(key)));
        assert_eq!(b.scenery[0].position, Vec3::X);
        index.replace_precinct(&place(1, 1), &a);
        index.replace_precinct(&place(1, 2), &b);
        let entry = index.get(key).unwrap();
        assert_eq!(entry.place, place(1, 2));
        assert_eq!(entry.label.as_deref(), Some("strongbox"));
    }

    /// Handles to the precincts of [`edit_world`].
    #[derive(Resource)]
    struct Handles(Vec<Handle<PrecinctAsset>>);

    /// A world in which [`PrecinctEdits`](crate::precinct_edit::PrecinctEdits) can edit `assets`.
    fn edit_world<const N: usize>(assets: [&PrecinctAsset; N]) -> World {
        use crate::{
            mode_selector::EditorMode, precinct_edit::EditedPrecincts, undo::UndoStack,
            unsaved::UnsavedAssets,
        };
        let mut world = World::new();
        world.init_resource::<Assets<PrecinctAsset>>();
        world.init_resource::<UnsavedAssets>();
        world.init_resource::<UndoStack>();
        world.init_resource::<EditedPrecincts>();
        world.insert_resource(State::new(EditorMode::Scenery));
        let mut store = world.resource_mut::<Assets<PrecinctAsset>>();
        let handles = assets.map(|asset| store.add(asset.clone())).to_vec();
        world.insert_resource(Handles(handles));
        world
    }
}
