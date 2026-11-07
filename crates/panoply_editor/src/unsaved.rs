//! Assets the editor has changed and not yet written. Edits change an asset in memory, where the
//! world shows them at once, and mark it with [`UnsavedAssets::mark`]; [`SaveAssets`] writes every
//! marked asset to its file. Asset events never mark an asset, so the file watcher's reload of a
//! file just saved leaves it saved.

use std::path::PathBuf;

use bevy::{
    asset::{io::file::FileAssetReader, io::AssetSourceId, UntypedAssetId},
    platform::collections::HashSet,
    prelude::*,
};
use panoply_core::NavPointsAsset;

use crate::json::to_asset_json;

/// The assets changed since they were last written.
#[derive(Resource, Default, Debug)]
pub struct UnsavedAssets(HashSet<UntypedAssetId>);

impl UnsavedAssets {
    /// Records that the asset `id` has changed.
    pub fn mark(&mut self, id: impl Into<UntypedAssetId>) {
        self.0.insert(id.into());
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// Write every unsaved asset to its file.
#[derive(Message, Clone, Copy, Debug, Default)]
pub struct SaveAssets;

/// An asset type the editor can write back to its file.
pub trait SaveableAsset: Asset {
    /// The file's contents.
    fn to_bytes(&self) -> Result<Vec<u8>, BevyError>;
}

impl SaveableAsset for NavPointsAsset {
    fn to_bytes(&self) -> Result<Vec<u8>, BevyError> {
        Ok(to_asset_json(&self.0)?.into_bytes())
    }
}

/// Adds a saveable asset type: [`SaveAssets`] writes its unsaved assets.
pub trait RegisterSaveable {
    fn register_saveable<A: SaveableAsset>(&mut self) -> &mut Self;
}

impl RegisterSaveable for App {
    fn register_saveable<A: SaveableAsset>(&mut self) -> &mut Self {
        self.add_systems(Update, save_assets::<A>)
    }
}

pub struct UnsavedAssetsPlugin;

impl Plugin for UnsavedAssetsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<UnsavedAssets>()
            .add_message::<SaveAssets>()
            .register_saveable::<NavPointsAsset>();
    }
}

/// Writes each unsaved asset of type `A` on [`SaveAssets`]. Synchronous: the files saved so far
/// are small. A failed write is logged and leaves the asset unsaved.
fn save_assets<A: SaveableAsset>(
    mut requests: MessageReader<SaveAssets>,
    mut unsaved: ResMut<UnsavedAssets>,
    assets: Res<Assets<A>>,
    server: Res<AssetServer>,
) {
    if requests.read().count() == 0 {
        return;
    }
    let ids: Vec<AssetId<A>> = unsaved
        .0
        .iter()
        .filter_map(|id| id.try_typed::<A>().ok())
        .collect();
    for id in ids {
        match save(id, &assets, &server) {
            Ok(path) => {
                info!("Saved {}", path.display());
                unsaved.0.remove(&id.untyped());
            }
            Err(error) => error!("Could not save {id:?}: {error}"),
        }
    }
}

/// Writes the asset `id` to a temporary file beside its own, then renames it over the original,
/// so a failed write leaves the original whole. Returns the file's path.
fn save<A: SaveableAsset>(
    id: AssetId<A>,
    assets: &Assets<A>,
    server: &AssetServer,
) -> Result<PathBuf, BevyError> {
    let asset = assets.get(id).ok_or("the asset is not loaded")?;
    let path = server.get_path(id).ok_or("the asset has no path")?;
    if path.source() != &AssetSourceId::Default {
        return Err(format!("{path} is not in the assets folder").into());
    }
    let file = FileAssetReader::new(AssetPlugin::default().file_path)
        .root_path()
        .join(path.path());
    let mut temporary = file.clone().into_os_string();
    temporary.push(".new");
    std::fs::write(&temporary, asset.to_bytes()?)?;
    std::fs::rename(&temporary, &file)?;
    Ok(file)
}
