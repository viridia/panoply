use bevy::{
    asset::AssetPath, ecs::system::SystemParam, platform::collections::HashMap, prelude::*,
};

use crate::{exemplar::ExemplarCatalog, Exemplar};

/// Every exemplar catalog requested through [`ExemplarServer`], held so that each file loads
/// once and its exemplars stay loaded.
#[derive(Resource, Default)]
pub struct ExemplarCatalogs(HashMap<AssetPath<'static>, Handle<ExemplarCatalog>>);

/// Loads exemplars by path (`file#Label`), one load per catalog file.
///
/// Bevy starts a separate load of the whole file for each `file#Label` requested before the file
/// has loaded (Bevy issue #10549), so a precinct naming 128 exemplars from one catalog would read
/// it 128 times. This loads the catalog first and takes labeled handles once it has loaded.
#[derive(SystemParam)]
pub struct ExemplarServer<'w> {
    server: Res<'w, AssetServer>,
    catalogs: ResMut<'w, ExemplarCatalogs>,
}

impl ExemplarServer<'_> {
    /// Returns the exemplar at `path`, or `None` while its catalog is loading (the load starts on
    /// the first call). If the catalog failed to load or has no such label, returns a handle
    /// from a plain load, which fails with Bevy's error for it.
    pub fn get(&mut self, path: &str) -> Option<Handle<Exemplar>> {
        let path = AssetPath::parse(path).into_owned();
        let server = &self.server;
        let catalog = self
            .catalogs
            .0
            .entry(path.without_label().into_owned())
            .or_insert_with_key(|root| server.load(root.clone()));
        let state = self.server.load_state(&*catalog);
        if state.is_loading() {
            return None;
        }
        if state.is_loaded() {
            if let Some(handle) = self.server.get_handle(&path) {
                return Some(handle);
            }
        }
        Some(self.server.load(path))
    }

    /// Returns the exemplars at `paths`, in order, once all their catalogs have loaded. Every
    /// catalog is requested on the first call, so they load in parallel.
    pub fn get_all<'a>(
        &mut self,
        paths: impl IntoIterator<Item = &'a String>,
    ) -> Option<Vec<Handle<Exemplar>>> {
        let handles: Vec<_> = paths.into_iter().map(|path| self.get(path)).collect();
        handles.into_iter().collect()
    }
}
