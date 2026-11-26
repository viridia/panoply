//! Keeping each realm's precinct index (`PrecinctIndex`) in step with its precinct files: saving a
//! precinct lists it, and a new realm starts with an empty index.

use std::path::Path;

use bevy::prelude::*;
use panoply_precinct::{precinct_index_path, PrecinctIndex};

use crate::{
    instance_index::PrecinctPlace,
    unsaved::{asset_file, write_replacing},
};

/// Lists the precinct at `place` in its realm's index, which is rewritten only if it did not list
/// it. A realm with no index file gets one.
pub(crate) fn list_precinct(place: &PrecinctPlace) -> Result<(), BevyError> {
    let file = asset_file(Path::new(&precinct_index_path(&place.realm)));
    let mut index = match std::fs::read_to_string(&file) {
        Ok(json) => PrecinctIndex::from_json(&json)?,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => default(),
        Err(error) => return Err(error.into()),
    };
    if index.insert(place.coords) {
        write_replacing(&file, index.to_json().as_bytes())?;
        info!("Saved {}", file.display());
    }
    Ok(())
}
