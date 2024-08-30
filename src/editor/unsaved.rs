use bevy::{
    asset::{
        saver::{AssetSaver, SavedAsset},
        ErasedLoadedAsset, LoadedAsset,
    },
    ecs::{system::SystemState, world::Command},
    prelude::*,
    tasks::{block_on, futures_lite::future, AsyncComputeTaskPool, Task},
    utils::{HashMap, HashSet},
};
use futures_lite::AsyncWriteExt;

use crate::{
    scenery::precinct_asset::{PrecinctAsset, PrecinctAssetSaver, PrecinctAssetSaverError},
    terrain::{
        terrain_contours::TerrainContoursTableAsset, terrain_groups::TerrainGroupsAsset,
        TerrainMapAsset,
    },
    world::{RealmData, WorldLocationsAsset},
};

#[derive(Debug, Default)]
pub enum ModifiedState<A: Asset, E> {
    #[default]
    Unsaved,
    Saving(Task<Result<Handle<A>, E>>),
}

#[derive(Debug, Default, Resource)]
pub struct UnsavedAssets {
    pub realms: HashSet<Handle<RealmData>>,
    pub terrain_maps: HashSet<Handle<TerrainMapAsset>>,
    pub terrain_groups: HashSet<Handle<TerrainGroupsAsset>>,
    pub terrain_contours: HashSet<Handle<TerrainContoursTableAsset>>,
    pub precincts:
        HashMap<Handle<PrecinctAsset>, ModifiedState<PrecinctAsset, PrecinctAssetSaverError>>,
    pub locations: HashSet<Handle<WorldLocationsAsset>>,
}

impl UnsavedAssets {
    pub fn is_empty(&self) -> bool {
        self.realms.is_empty()
            && self.terrain_maps.is_empty()
            && self.terrain_groups.is_empty()
            && self.terrain_contours.is_empty()
            && self.precincts.is_empty()
            && self.locations.is_empty()
    }
}

pub struct SaveCommand;

impl Command for SaveCommand {
    fn apply(self, world: &mut World) {
        let mut system_state: SystemState<(
            Res<AssetServer>,
            Res<Assets<PrecinctAsset>>,
            Res<AppTypeRegistry>,
            ResMut<UnsavedAssets>,
        )> = SystemState::new(world);
        let task_pool = AsyncComputeTaskPool::get();

        let (server, precincts, type_registry, mut unsaved_assets) = system_state.get_mut(world);
        for (precinct_handle, state) in unsaved_assets.precincts.iter_mut() {
            // Don't save if we're already saving
            if matches!(state, ModifiedState::Saving(_)) {
                continue;
            }
            let precinct_asset = precincts.get(precinct_handle).unwrap().clone();
            let precinct_handle = precinct_handle.clone();
            let server = server.clone();
            let registry = type_registry.0.clone();
            let task = task_pool.spawn(async move {
                let path = server.get_path(&precinct_handle).unwrap();
                let source = server.get_source(path.source()).unwrap();
                let writer = source.writer().unwrap();
                let mut file_path = path.path().to_path_buf();
                file_path.set_extension(format!("{}.new", path.get_full_extension().unwrap()));
                let mut write = writer.write(file_path.as_path()).await.unwrap();
                let saver = PrecinctAssetSaver::new(registry);
                let loaded_precinct = LoadedAsset::new_with_dependencies(precinct_asset, None);
                let erased = ErasedLoadedAsset::from(loaded_precinct);
                let saved = SavedAsset::from_loaded(&erased).unwrap();
                saver.save(&mut *write, saved, &()).await?;
                write.close().await?;
                writer.rename(file_path.as_path(), path.path()).await?;
                Ok(precinct_handle)
            });
            *state = ModifiedState::Saving(task);
        }
    }
}

pub fn receive_asset_saving(mut unsaved: ResMut<UnsavedAssets>) {
    let finished_saving = unsaved
        .precincts
        .iter_mut()
        .filter_map(|(_handle, state)| {
            if let ModifiedState::Saving(task) = state {
                let status = block_on(future::poll_once(task));
                match status {
                    Some(Ok(handle)) => {
                        return Some(handle);
                    }
                    Some(Err(e)) => {
                        println!("Error saving precinct: {:?}", e);
                    }
                    _ => {}
                }
            }
            None
        })
        .collect::<Vec<_>>();
    if !finished_saving.is_empty() {
        for handle in finished_saving {
            // This can happen if the asset was modified while saving
            if matches!(unsaved.precincts.get(&handle), Some(ModifiedState::Unsaved)) {
                continue;
            }
            unsaved.precincts.remove(&handle);
        }
    }
}
