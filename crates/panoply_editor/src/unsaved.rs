//! Assets the editor has changed and not yet written. Edits change an asset in memory, where the
//! world shows them at once, and mark it with [`UnsavedAssets::mark`]; [`SaveAssets`] writes every
//! marked asset to its file. Asset events never mark an asset, so the file watcher's reload of a
//! file just saved leaves it saved.
//!
//! That reload replaces the asset with the file as saved, and arrives some time after the write, so
//! an edit made in between would be lost. Until each written asset has reloaded, [`SaveWait`] holds
//! a barrier over the window, a wait cursor, and every key.

use std::path::{Path, PathBuf};

use bevy::{
    asset::{io::file::FileAssetReader, io::AssetSourceId, AssetLoadFailedEvent, UntypedAssetId},
    picking::cursor::{EntityCursor, OverrideCursor},
    platform::collections::HashSet,
    prelude::*,
    ui::{FixedNode, OverrideClip},
    window::SystemCursorIcon,
};
use bevy_action_map::prelude::*;
use panoply_core::NavPointsAsset;
use panoply_terrain::TerrainMapAsset;

use crate::{json::to_asset_json, toast::ShowToast};

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

/// How long after a save to wait for its reloads. The watcher reports a change within a fraction of
/// a second, so a reload that has not come by now never will.
const SAVE_WAIT_TIMEOUT: f32 = 1.;

/// Above every dialog and menu.
const SAVE_BARRIER_Z: i32 = 1000;

/// The assets just written whose reloads have not yet arrived.
#[derive(Resource, Default)]
struct SaveWait {
    ids: HashSet<UntypedAssetId>,
    /// The barrier, while there is one.
    barrier: Option<Entity>,
    /// Seconds since the barrier went up.
    waited: f32,
}

/// Holds every key during a [`SaveWait`], above `widget_keys`' claim on a focused widget.
#[derive(InputContext)]
#[context(path = "editor.save_wait", tick = Render, priority = 2000)]
struct SaveWaitKeys;

/// A key [`SaveWaitKeys`] took and dropped.
struct DroppedKey;

impl ClassBinding for DroppedKey {
    const PATH: &'static str = "editor.save_wait.dropped";
}

/// The order of the systems of each saveable asset type.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) enum SaveSystems {
    /// Reads asset events. Before [`SaveSystems::Write`], so that the event of an edit made just
    /// before a save is gone by the time the save starts waiting.
    Track,
    Write,
}

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

impl SaveableAsset for TerrainMapAsset {
    fn to_bytes(&self) -> Result<Vec<u8>, BevyError> {
        Ok(TerrainMapAsset::to_bytes(self)?)
    }
}

/// Adds a saveable asset type: [`SaveAssets`] writes its unsaved assets.
pub trait RegisterSaveable {
    fn register_saveable<A: SaveableAsset>(&mut self) -> &mut Self;
}

impl RegisterSaveable for App {
    fn register_saveable<A: SaveableAsset>(&mut self) -> &mut Self {
        self.add_systems(
            Update,
            (
                track_saved_reloads::<A>.in_set(SaveSystems::Track),
                save_assets::<A>.in_set(SaveSystems::Write),
            ),
        )
    }
}

pub struct UnsavedAssetsPlugin;

impl Plugin for UnsavedAssetsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<UnsavedAssets>()
            .init_resource::<SaveWait>()
            .add_message::<SaveAssets>()
            .configure_sets(Update, SaveSystems::Track.before(SaveSystems::Write))
            .add_systems(Update, update_save_barrier.after(SaveSystems::Write))
            .add_context::<SaveWaitKeys>(|context| {
                context.active_if(save_waiting);
                context
                    .bind_class::<DroppedKey>(ControlClass::AnyButton)
                    .consume();
            })
            .add_systems(Startup, |mut commands: Commands| {
                commands.spawn((Name::new("SaveWaitKeys"), SaveWaitKeys));
            })
            .register_saveable::<NavPointsAsset>()
            .register_saveable::<TerrainMapAsset>();
    }
}

fn save_waiting(wait: Res<SaveWait>) -> bool {
    !wait.ids.is_empty()
}

/// Stops waiting for each asset of type `A` that has reloaded or failed to. Runs every frame, so
/// that no event from before a save is left to be read during its wait.
fn track_saved_reloads<A: SaveableAsset>(
    mut events: MessageReader<AssetEvent<A>>,
    mut failures: MessageReader<AssetLoadFailedEvent<A>>,
    mut wait: ResMut<SaveWait>,
) {
    for event in events.read() {
        if let AssetEvent::Modified { id } = event {
            wait.ids.remove(&id.untyped());
        }
    }
    for failure in failures.read() {
        wait.ids.remove(&failure.id.untyped());
    }
}

/// Raises the barrier and the wait cursor while a [`SaveWait`] lasts, and lowers them after.
fn update_save_barrier(
    time: Res<Time<Real>>,
    mut wait: ResMut<SaveWait>,
    mut cursor: ResMut<OverrideCursor>,
    mut commands: Commands,
) {
    if wait.ids.is_empty() && wait.barrier.is_none() {
        return;
    }
    if wait.barrier.is_some() {
        wait.waited += time.delta_secs();
        if !wait.ids.is_empty() && wait.waited >= SAVE_WAIT_TIMEOUT {
            warn!("No reload after saving: {:?}", wait.ids);
            wait.ids.clear();
        }
    }
    match (wait.ids.is_empty(), wait.barrier) {
        (false, None) => {
            let barrier = commands
                .spawn((
                    Name::new("SaveBarrier"),
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(0.),
                        right: Val::Px(0.),
                        top: Val::Px(0.),
                        bottom: Val::Px(0.),
                        ..default()
                    },
                    FixedNode,
                    OverrideClip,
                    GlobalZIndex(SAVE_BARRIER_Z),
                ))
                .id();
            wait.barrier = Some(barrier);
            wait.waited = 0.;
            cursor.0 = Some(EntityCursor::System(SystemCursorIcon::Wait));
        }
        (true, Some(barrier)) => {
            commands.entity(barrier).despawn();
            wait.barrier = None;
            cursor.0 = None;
        }
        _ => {}
    }
}

/// Writes each unsaved asset of type `A` on [`SaveAssets`]. Synchronous: the files saved so far
/// are small. A failed write is logged and leaves the asset unsaved, and is not waited for.
fn save_assets<A: SaveableAsset>(
    mut requests: MessageReader<SaveAssets>,
    mut unsaved: ResMut<UnsavedAssets>,
    mut wait: ResMut<SaveWait>,
    mut toasts: MessageWriter<ShowToast>,
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
                if server.watching_for_changes() {
                    wait.ids.insert(id.untyped());
                }
            }
            Err(error) => {
                error!("Could not save {id:?}: {error}");
                let name = server
                    .get_path(id)
                    .map_or_else(|| "an asset".to_string(), |path| path.to_string());
                toasts.write(ShowToast(format!("Could not save {name}").into()));
            }
        }
    }
}

/// The file on disk of the asset at `path` in the assets folder.
pub(crate) fn asset_file(path: &Path) -> PathBuf {
    FileAssetReader::new(AssetPlugin::default().file_path)
        .root_path()
        .join(path)
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
    let file = asset_file(path.path());
    let mut temporary = file.clone().into_os_string();
    temporary.push(".new");
    std::fs::write(&temporary, asset.to_bytes()?)?;
    std::fs::rename(&temporary, &file)?;
    Ok(file)
}
