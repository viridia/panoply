//! Assets the editor has changed and not yet written. Edits change an asset in memory, where the
//! world shows them at once, and mark it with [`UnsavedAssets::mark`]; [`SaveAssets`] writes every
//! marked asset to its file. Asset events never mark an asset, so the file watcher's reload of a
//! file just saved leaves it saved.
//!
//! That reload replaces the asset with the file as saved, and arrives some time after the write, so
//! an edit made in between would be lost. Until each written asset has reloaded, [`SaveWait`] holds
//! a barrier over the window, a wait cursor, and every key.

use std::path::{Path, PathBuf};

use bevy::ecs::message::MessageCursor;
use bevy::{
    asset::{
        io::file::FileAssetReader, io::AssetSourceId, AssetLoadFailedEvent, AssetMut,
        UntypedAssetId,
    },
    picking::cursor::{EntityCursor, OverrideCursor},
    platform::collections::HashSet,
    prelude::*,
    ui::{FixedNode, OverrideClip},
    window::SystemCursorIcon,
};
use bevy_action_map::prelude::*;
use panoply_core::NavPointsAsset;
use panoply_exemplar::{CatalogEntry, EntryCatalog, Exemplar, ExemplarCatalog};
use panoply_precinct::PrecinctAsset;
use panoply_terrain::TerrainMapAsset;

use crate::{
    instance_index::{update_index, InstanceIndex, PrecinctPlace},
    json::to_asset_json,
    toast::ShowToast,
};

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
pub trait SaveableAsset: Asset + Sized {
    /// The file's contents. `file` is where they go, and `world` holds what else they need.
    fn to_bytes(&self, file: &Path, world: &World) -> Result<Vec<u8>, BevyError>;

    /// Changes the asset `id` as saving it changes it, before it is written.
    fn prepare(_id: AssetId<Self>, _world: &mut World) {}

    /// Writes what else follows from the asset `id`, once its file is written.
    fn saved(_id: AssetId<Self>, _world: &mut World) -> Result<(), BevyError> {
        Ok(())
    }
}

impl SaveableAsset for NavPointsAsset {
    fn to_bytes(&self, _file: &Path, _world: &World) -> Result<Vec<u8>, BevyError> {
        Ok(to_asset_json(&self.0)?.into_bytes())
    }
}

impl SaveableAsset for TerrainMapAsset {
    fn to_bytes(&self, _file: &Path, _world: &World) -> Result<Vec<u8>, BevyError> {
        Ok(TerrainMapAsset::to_bytes(self)?)
    }
}

/// The catalog's file as it is, with its edits written in (`ExemplarCatalog::merge_into`).
impl SaveableAsset for ExemplarCatalog {
    fn to_bytes(&self, file: &Path, world: &World) -> Result<Vec<u8>, BevyError> {
        let mut json: serde_json::Value = serde_json::from_slice(&std::fs::read(file)?)?;
        self.merge_into(
            &mut json,
            world.resource::<Assets<Exemplar>>(),
            &world.resource::<AppTypeRegistry>().read(),
            world.resource::<AssetServer>(),
        )?;
        Ok(to_asset_json(&json)?.into_bytes())
    }
}

/// The file of catalog entries as it is, with its edits written in (`EntryCatalog::merge_into`).
impl<T: CatalogEntry> SaveableAsset for EntryCatalog<T> {
    fn to_bytes(&self, file: &Path, world: &World) -> Result<Vec<u8>, BevyError> {
        let mut json: serde_json::Value = serde_json::from_slice(&std::fs::read(file)?)?;
        self.merge_into(
            &mut json,
            world.resource::<Assets<T>>(),
            &world.resource::<AppTypeRegistry>().read(),
        )?;
        Ok(to_asset_json(&json)?.into_bytes())
    }
}

impl SaveableAsset for PrecinctAsset {
    fn to_bytes(&self, _file: &Path, world: &World) -> Result<Vec<u8>, BevyError> {
        Ok(PrecinctAsset::to_bytes(
            self,
            &world.resource::<AppTypeRegistry>().read(),
        )?)
    }

    /// Gives a key to each instance with a reason for one, and takes it from each with none left
    /// (D87). While the watcher runs, the asset changes without an event, so that the reload, not
    /// this, ends the save's wait, and brings the keys to the precinct's entities.
    fn prepare(id: AssetId<Self>, world: &mut World) {
        let Some(precinct) = world.resource::<Assets<PrecinctAsset>>().get(id) else {
            return;
        };
        let server = world.resource::<AssetServer>();
        let exemplars = world.resource::<Assets<Exemplar>>();
        let has_state: Vec<bool> = precinct
            .scenery_types
            .iter()
            .map(|path| {
                server
                    .get_handle::<Exemplar>(path.clone())
                    .is_some_and(|handle| Exemplar::has_state(exemplars, handle.id()))
            })
            .collect();
        let watching = server.watching_for_changes();
        let mut assets = world.resource_mut::<Assets<PrecinctAsset>>();
        let precinct = match watching {
            true => assets.get_mut_untracked(id),
            false => assets.get_mut(id).map(AssetMut::into_inner),
        };
        let Some(precinct) = precinct else {
            return;
        };
        let (given, taken) = precinct.update_keys(|index| has_state[index]);
        if given + taken > 0 {
            info!("Gave {given} instances keys, and took {taken} keys");
        }
    }

    /// Rewrites the precinct's lines in the instance index.
    fn saved(id: AssetId<Self>, world: &mut World) -> Result<(), BevyError> {
        let path = world
            .resource::<AssetServer>()
            .get_path(id)
            .ok_or("the precinct has no path")?;
        let place = PrecinctPlace::of_path(path.path())
            .ok_or_else(|| format!("{path} is not named for a precinct"))?;
        world.resource_scope(|world, mut index: Mut<InstanceIndex>| {
            let precinct = world
                .resource::<Assets<PrecinctAsset>>()
                .get(id)
                .ok_or("the precinct is not loaded")?;
            update_index(&mut index, &place, precinct)
        })
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
            .register_saveable::<TerrainMapAsset>()
            .register_saveable::<ExemplarCatalog>()
            .register_saveable::<PrecinctAsset>();
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
/// Exclusive, since an asset's contents may need anything in the world.
fn save_assets<A: SaveableAsset>(
    world: &mut World,
    mut requests: Local<MessageCursor<SaveAssets>>,
) {
    if requests
        .read(world.resource::<Messages<SaveAssets>>())
        .count()
        == 0
    {
        return;
    }
    let ids: Vec<AssetId<A>> = world
        .resource::<UnsavedAssets>()
        .0
        .iter()
        .filter_map(|id| id.try_typed::<A>().ok())
        .collect();
    for id in ids {
        A::prepare(id, world);
        match save(id, world) {
            Ok(path) => {
                info!("Saved {}", path.display());
                if let Err(error) = A::saved(id, world) {
                    error!("After saving {}: {error}", path.display());
                    world.write_message(ShowToast(
                        format!("Could not finish saving {}", path.display()).into(),
                    ));
                }
                world
                    .resource_mut::<UnsavedAssets>()
                    .0
                    .remove(&id.untyped());
                if world.resource::<AssetServer>().watching_for_changes() {
                    world.resource_mut::<SaveWait>().ids.insert(id.untyped());
                }
            }
            Err(error) => {
                error!("Could not save {id:?}: {error}");
                let name = world
                    .resource::<AssetServer>()
                    .get_path(id)
                    .map_or_else(|| "an asset".to_string(), |path| path.to_string());
                world.write_message(ShowToast(format!("Could not save {name}").into()));
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
fn save<A: SaveableAsset>(id: AssetId<A>, world: &World) -> Result<PathBuf, BevyError> {
    let asset = world
        .resource::<Assets<A>>()
        .get(id)
        .ok_or("the asset is not loaded")?;
    let path = world
        .resource::<AssetServer>()
        .get_path(id)
        .ok_or("the asset has no path")?;
    if path.source() != &AssetSourceId::Default {
        return Err(format!("{path} is not in the assets folder").into());
    }
    let file = asset_file(path.path());
    write_replacing(&file, &asset.to_bytes(&file, world)?)?;
    Ok(file)
}

/// Writes `bytes` to a temporary file beside `file`, then renames it over `file`, so a failed write
/// leaves `file` whole.
pub(crate) fn write_replacing(file: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let mut temporary = file.to_path_buf().into_os_string();
    temporary.push(".new");
    std::fs::write(&temporary, bytes)?;
    std::fs::rename(&temporary, file)
}
