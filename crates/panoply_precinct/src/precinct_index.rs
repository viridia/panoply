//! The precincts of a realm that have files, so the game asks only for those: one index per realm,
//! [`precinct_index_path`], beside the precincts it lists. A list, rather than reading the folder,
//! works in packaged and web builds. The editor adds a precinct to it when it first saves one, and
//! `asset_xref.py` checks it against the folder.

use std::collections::BTreeSet;

use bevy::{
    asset::{io::Reader, AssetLoader, LoadContext},
    prelude::*,
};
use thiserror::Error;

/// The file of the precinct at `coords` in `realm`, in the assets folder:
/// `scenery/precincts/holm-overland/p010-n003.msgpack`.
pub fn precinct_path(realm: &str, coords: IVec2) -> String {
    let coord = |n: i32| match n >= 0 {
        true => format!("p{n:03}"),
        false => format!("n{:03}", -n),
    };
    format!(
        "scenery/precincts/{realm}/{}-{}.msgpack",
        coord(coords.x),
        coord(coords.y)
    )
}

/// The index of the precincts of `realm`, in the assets folder.
pub fn precinct_index_path(realm: &str) -> String {
    format!("scenery/precincts/{realm}/index.json")
}

/// The coordinates of a realm's precincts that have files.
#[derive(Asset, TypePath, Default, Clone, PartialEq, Debug)]
pub struct PrecinctIndex(BTreeSet<(i32, i32)>);

impl PrecinctIndex {
    /// The index in `json`, an array of `[x, y]` coordinates.
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        Ok(Self::of_coords(serde_json::from_str(json)?))
    }

    fn of_coords(coords: Vec<[i32; 2]>) -> Self {
        Self(coords.into_iter().map(|[x, y]| (x, y)).collect())
    }

    /// The index's file: a sorted JSON array with one `[x, y]` to a line, so a diff shows each
    /// precinct added or removed.
    pub fn to_json(&self) -> String {
        let lines: Vec<String> = self.0.iter().map(|(x, y)| format!("[{x}, {y}]")).collect();
        match lines.is_empty() {
            true => "[]\n".to_string(),
            false => format!("[\n  {}\n]\n", lines.join(",\n  ")),
        }
    }

    /// Whether the precinct at `coords` has a file.
    pub fn contains(&self, coords: IVec2) -> bool {
        self.0.contains(&(coords.x, coords.y))
    }

    /// Lists the precinct at `coords`. Returns whether it was not listed before.
    pub fn insert(&mut self, coords: IVec2) -> bool {
        self.0.insert((coords.x, coords.y))
    }
}

#[non_exhaustive]
#[derive(Debug, Error)]
pub enum PrecinctIndexLoaderError {
    #[error("Could not load precinct index: {0}")]
    Io(#[from] std::io::Error),
    #[error("Could not decode precinct index: {0}")]
    Decode(#[from] serde_json::Error),
}

/// Loads a [`PrecinctIndex`]. It claims no extension, since `.json` is the exemplar catalogs':
/// an index is always loaded by its type.
#[derive(Default, TypePath)]
pub struct PrecinctIndexLoader;

impl AssetLoader for PrecinctIndexLoader {
    type Asset = PrecinctIndex;
    type Error = PrecinctIndexLoaderError;
    type Settings = ();

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &Self::Settings,
        _load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        Ok(PrecinctIndex::of_coords(serde_json::from_slice(&bytes)?))
    }

    fn extensions(&self) -> &[&str] {
        &[]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_name_realm_and_coordinates() {
        assert_eq!(
            precinct_path("holm-overland", IVec2::new(10, -3)),
            "scenery/precincts/holm-overland/p010-n003.msgpack"
        );
        assert_eq!(
            precinct_index_path("playground"),
            "scenery/precincts/playground/index.json"
        );
    }

    /// The index is sorted, one precinct to a line, and reads back as it was written.
    #[test]
    fn the_index_round_trips() {
        let mut index = PrecinctIndex::default();
        assert_eq!(index.to_json(), "[]\n");
        assert!(index.insert(IVec2::new(2, 0)));
        assert!(index.insert(IVec2::new(-1, 5)));
        assert!(index.insert(IVec2::new(2, -4)));
        assert!(!index.insert(IVec2::new(2, 0)));
        let json = index.to_json();
        assert_eq!(json, "[\n  [-1, 5],\n  [2, -4],\n  [2, 0]\n]\n");
        assert_eq!(PrecinctIndex::from_json(&json).unwrap(), index);
        assert!(index.contains(IVec2::new(2, -4)));
        assert!(!index.contains(IVec2::new(-4, 2)));
    }
}
