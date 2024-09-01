use bevy::{
    asset::{
        saver::{AssetSaver, SavedAsset},
        AssetPath, ErasedLoadedAsset, LoadedAsset,
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
        terrain_contours::{TerrainContoursTableAsset, TerrainContoursTableSaver},
        terrain_groups::TerrainGroupsAsset,
        TerrainMapAsset, TerrainMapSaver,
    },
    world::{RealmData, WorldLocationsAsset},
};

#[derive(Debug, Default)]
pub enum ModifiedState<A: Asset, E> {
    #[default]
    Unsaved,
    Saving(Task<Result<Handle<A>, E>>),
}

#[derive(Default, Resource)]
pub struct UnsavedAssets {
    pub realms: HashSet<Handle<RealmData>>,
    pub terrain_maps: HashMap<
        Handle<TerrainMapAsset>,
        ModifiedState<TerrainMapAsset, <TerrainMapSaver as AssetSaver>::Error>,
    >,
    pub terrain_groups: HashSet<Handle<TerrainGroupsAsset>>,
    pub terrain_contours: HashMap<
        Handle<TerrainContoursTableAsset>,
        ModifiedState<TerrainContoursTableAsset, <TerrainContoursTableSaver as AssetSaver>::Error>,
    >,
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

// #[derive(Default)]
// pub struct UnsavedAssetSet<A: AssetSaver> {
//     assets: HashMap<Handle<A::Asset>, ModifiedState<A::Asset, A::Error>>,
// }

// impl<A: AssetSaver> UnsavedAssetSet<A> {
//     pub fn add(&mut self, handle: Handle<A::Asset>) {
//         self.assets.insert(handle, ModifiedState::Unsaved);
//     }
// }

pub struct SaveCommand;

impl SaveCommand {
    fn get_temp_file_path(path: &AssetPath) -> std::path::PathBuf {
        let mut file_path = path.path().to_path_buf();
        file_path.set_extension(format!("{}.new", path.get_full_extension().unwrap()));
        file_path
    }
}

impl Command for SaveCommand {
    fn apply(self, world: &mut World) {
        let mut system_state: SystemState<(
            Res<AssetServer>,
            Res<Assets<PrecinctAsset>>,
            Res<Assets<TerrainMapAsset>>,
            Res<Assets<TerrainContoursTableAsset>>,
            Res<AppTypeRegistry>,
            ResMut<UnsavedAssets>,
        )> = SystemState::new(world);
        let task_pool = AsyncComputeTaskPool::get();

        let (server, precincts, terrain_maps, terrain_contours, type_registry, mut unsaved_assets) =
            system_state.get_mut(world);
        for (asset_handle, state) in unsaved_assets.precincts.iter_mut() {
            // Don't save if we're already saving
            if matches!(state, ModifiedState::Saving(_)) {
                continue;
            }
            let asset = precincts.get(asset_handle).unwrap().clone();
            let asset_handle = asset_handle.clone();
            let server = server.clone();
            let registry = type_registry.0.clone();
            let task = task_pool.spawn(async move {
                let path = server.get_path(&asset_handle).unwrap();
                let source = server.get_source(path.source()).unwrap();
                let file_path = Self::get_temp_file_path(&path);
                let writer = source.writer().unwrap();
                let mut write = writer.write(file_path.as_path()).await.unwrap();
                let saver = PrecinctAssetSaver::new(registry);
                let loaded_precinct = LoadedAsset::new_with_dependencies(asset, None);
                let erased = ErasedLoadedAsset::from(loaded_precinct);
                let saved = SavedAsset::from_loaded(&erased).unwrap();
                saver.save(&mut *write, saved, &()).await?;
                write.close().await?;
                writer.rename(file_path.as_path(), path.path()).await?;
                Ok(asset_handle)
            });
            *state = ModifiedState::Saving(task);
        }

        for (asset_handle, state) in unsaved_assets.terrain_maps.iter_mut() {
            // Don't save if we're already saving
            if matches!(state, ModifiedState::Saving(_)) {
                continue;
            }
            let asset = terrain_maps.get(asset_handle).unwrap().clone();
            let asset_handle = asset_handle.clone();
            let server = server.clone();
            // let registry = type_registry.0.clone();
            let task = task_pool.spawn(async move {
                let path = server.get_path(&asset_handle).unwrap();
                let source = server.get_source(path.source()).unwrap();
                let file_path = Self::get_temp_file_path(&path);
                let writer = source.writer().unwrap();
                let mut write = writer.write(file_path.as_path()).await.unwrap();
                let saver = TerrainMapSaver;
                let loaded_precinct = LoadedAsset::new_with_dependencies(asset, None);
                let erased = ErasedLoadedAsset::from(loaded_precinct);
                let saved = SavedAsset::from_loaded(&erased).unwrap();
                saver.save(&mut *write, saved, &()).await?;
                write.close().await?;
                writer.rename(file_path.as_path(), path.path()).await?;
                Ok(asset_handle)
            });
            *state = ModifiedState::Saving(task);
        }

        for (asset_handle, state) in unsaved_assets.terrain_contours.iter_mut() {
            // Don't save if we're already saving
            if matches!(state, ModifiedState::Saving(_)) {
                continue;
            }
            let asset = terrain_contours.get(asset_handle).unwrap().clone();
            let asset_handle = asset_handle.clone();
            let server = server.clone();
            // let registry = type_registry.0.clone();
            let task = task_pool.spawn(async move {
                let path = server.get_path(&asset_handle).unwrap();
                let source = server.get_source(path.source()).unwrap();
                let file_path = Self::get_temp_file_path(&path);
                let writer = source.writer().unwrap();
                let mut write = writer.write(file_path.as_path()).await.unwrap();
                let saver = TerrainContoursTableSaver;
                let loaded_precinct = LoadedAsset::new_with_dependencies(asset, None);
                let erased = ErasedLoadedAsset::from(loaded_precinct);
                let saved = SavedAsset::from_loaded(&erased).unwrap();
                saver.save(&mut *write, saved, &()).await?;
                write.close().await?;
                writer.rename(file_path.as_path(), path.path()).await?;
                Ok(asset_handle)
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

    let finished_saving = unsaved
        .terrain_maps
        .iter_mut()
        .filter_map(|(_handle, state)| {
            if let ModifiedState::Saving(task) = state {
                let status = block_on(future::poll_once(task));
                match status {
                    Some(Ok(handle)) => {
                        return Some(handle);
                    }
                    Some(Err(e)) => {
                        println!("Error saving terrain map: {:?}", e);
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
            if matches!(
                unsaved.terrain_maps.get(&handle),
                Some(ModifiedState::Unsaved)
            ) {
                continue;
            }
            unsaved.terrain_maps.remove(&handle);
        }
    }

    let finished_saving = unsaved
        .terrain_contours
        .iter_mut()
        .filter_map(|(_handle, state)| {
            if let ModifiedState::Saving(task) = state {
                let status = block_on(future::poll_once(task));
                match status {
                    Some(Ok(handle)) => {
                        return Some(handle);
                    }
                    Some(Err(e)) => {
                        println!("Error saving contours: {:?}", e);
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
            if matches!(
                unsaved.terrain_contours.get(&handle),
                Some(ModifiedState::Unsaved)
            ) {
                continue;
            }
            unsaved.terrain_contours.remove(&handle);
        }
    }
}
