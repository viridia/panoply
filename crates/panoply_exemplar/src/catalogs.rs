use bevy::{
    asset::AssetPath, ecs::system::SystemParam, platform::collections::HashMap, prelude::*,
};

use crate::{
    entry::{CatalogEntry, EntryCatalog},
    exemplar::ExemplarCatalog,
    Exemplar,
};

/// An asset kept in a catalog file, whose labels are its assets: an [`Exemplar`] in an
/// [`ExemplarCatalog`], or a catalog entry in its [`EntryCatalog`].
pub trait CatalogMember: Asset {
    /// The asset of the file that holds it.
    type Catalog: Asset;
}

impl CatalogMember for Exemplar {
    type Catalog = ExemplarCatalog;
}

impl<T: CatalogEntry> CatalogMember for T {
    type Catalog = EntryCatalog<T>;
}

/// Every catalog file requested through [`CatalogServer`], held so that each file loads once and
/// its labels stay loaded.
#[derive(Resource, Default)]
pub struct CatalogRoots(HashMap<AssetPath<'static>, UntypedHandle>);

/// Loads exemplars and catalog entries by path (`file#Label`), one load per catalog file.
///
/// Bevy starts a separate load of the whole file for each `file#Label` requested before the file
/// has loaded (Bevy issue #10549), so a precinct naming 128 exemplars from one catalog would read
/// it 128 times. This loads the catalog first and takes labeled handles once it has loaded.
#[derive(SystemParam)]
pub struct CatalogServer<'w> {
    server: Res<'w, AssetServer>,
    catalogs: ResMut<'w, CatalogRoots>,
}

impl CatalogServer<'_> {
    /// Returns the asset at `path`, or `None` while its catalog is loading (the load starts on the
    /// first call). If the catalog failed to load or has no such label, returns a handle from a
    /// plain load, which fails with Bevy's error for it.
    pub fn get<A: CatalogMember>(&mut self, path: &str) -> Option<Handle<A>> {
        let path = AssetPath::parse(path).into_owned();
        let server = &self.server;
        let catalog = self
            .catalogs
            .0
            .entry(path.without_label().into_owned())
            .or_insert_with_key(|root| server.load::<A::Catalog>(root.clone()).untyped());
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

    /// Returns the assets at `paths`, in order, once all their catalogs have loaded. Every
    /// catalog is requested on the first call, so they load in parallel.
    pub fn get_all<'a, A: CatalogMember>(
        &mut self,
        paths: impl IntoIterator<Item = &'a String>,
    ) -> Option<Vec<Handle<A>>> {
        let handles: Vec<_> = paths.into_iter().map(|path| self.get(path)).collect();
        handles.into_iter().collect()
    }
}
