//! Making a new, empty realm: its files are written at once (D65), then loaded directly, since
//! the folders of realms and terrain maps are read only at startup. The view cuts to the new realm
//! when it exists.

use std::{io::Write, path::Path};

use bevy::prelude::*;
use panoply_core::{realm::RealmData, GotoLocation, Realm};
use panoply_precinct::{precinct_index_path, PrecinctIndex};
use panoply_terrain::TerrainMapAsset;

use crate::{json::to_asset_json, toast::ShowToast, unsaved::asset_file};

/// Handles of the realms made this run, without which their assets would be dropped and the realms
/// despawned; and the one still waiting for its entity.
#[derive(Resource, Default)]
struct CreatedRealms {
    handles: Vec<UntypedHandle>,
    pending: Option<String>,
}

pub struct NewRealmPlugin;

impl Plugin for NewRealmPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CreatedRealms>()
            .add_message::<CreateRealm>()
            .add_systems(Update, (create_realms, finish_created_realm));
    }
}

/// Make a realm with this name.
#[derive(Message, Clone, Debug)]
pub struct CreateRealm(pub String);

/// Why `name` cannot name a realm: it becomes file names, and the loaders cut those at a dot.
pub(crate) fn realm_name_rule(name: &str) -> Option<&'static str> {
    let allowed = |c: char| c.is_alphanumeric() || c == '-' || c == '_';
    (!name.chars().all(allowed)).then_some("Letters, digits, - and _ only")
}

fn realm_path(name: &str) -> String {
    format!("realms/{name}.realm.json")
}

fn terrain_path(name: &str) -> String {
    format!("terrain/maps/{name}.terrain")
}

/// Writes a file that must not exist yet, and its folder if need be.
fn write_new(path: &str, bytes: &[u8]) -> Result<(), BevyError> {
    let path = asset_file(Path::new(path));
    if let Some(folder) = path.parent() {
        std::fs::create_dir_all(folder)?;
    }
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?;
    file.write_all(bytes)?;
    Ok(())
}

/// Writes all of a new realm's files, or none. The terrain has no parcels, and the precinct index
/// lists no precincts.
fn write_realm_files(name: &str) -> Result<(), BevyError> {
    let files = [
        (
            realm_path(name),
            to_asset_json(&RealmData::default())?.into_bytes(),
        ),
        (terrain_path(name), TerrainMapAsset::default().to_bytes()?),
        (
            precinct_index_path(name),
            PrecinctIndex::default().to_json().into_bytes(),
        ),
    ];
    for (count, (path, bytes)) in files.iter().enumerate() {
        if let Err(error) = write_new(path, bytes) {
            for (path, _) in &files[..count] {
                std::fs::remove_file(asset_file(Path::new(path)))?;
            }
            return Err(error);
        }
    }
    Ok(())
}

fn create_realms(
    mut requests: MessageReader<CreateRealm>,
    mut created: ResMut<CreatedRealms>,
    mut toasts: MessageWriter<ShowToast>,
    server: Res<AssetServer>,
) {
    for CreateRealm(name) in requests.read() {
        if let Err(error) = write_realm_files(name) {
            error!("Could not create realm {name}: {error}");
            toasts.write(ShowToast(format!("Could not create realm {name}").into()));
            continue;
        }
        let handle = server.load::<RealmData>(realm_path(name));
        created.handles.push(handle.untyped());
        created.pending = Some(name.clone());
    }
}

/// Once the new realm's entity exists, loads its terrain and moves the view there. In this order,
/// because a terrain map that loads before its realm exists is never bound to it.
fn finish_created_realm(
    mut created: ResMut<CreatedRealms>,
    realms: Query<&Realm>,
    server: Res<AssetServer>,
    mut gotos: MessageWriter<GotoLocation>,
) {
    let Some(name) = created.pending.clone() else {
        return;
    };
    if !realms.iter().any(|realm| realm.name == name) {
        return;
    }
    let terrain = server.load::<TerrainMapAsset>(terrain_path(&name));
    created.handles.push(terrain.untyped());
    created.pending = None;
    gotos.write(GotoLocation {
        realm: name,
        ..default()
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn realm_names_are_safe_file_names() {
        assert_eq!(realm_name_rule("holm-overland_2"), None);
        assert!(realm_name_rule("a.b").is_some());
        assert!(realm_name_rule("a/b").is_some());
        assert!(realm_name_rule("a b").is_some());
    }
}
