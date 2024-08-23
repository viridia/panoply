use bevy::{prelude::*, utils::HashSet};

#[derive(Resource, Default)]
pub struct ModifiedAssets<T: Asset> {
    assets: HashSet<Handle<T>>,
}

impl<T: Asset> ModifiedAssets<T> {
    pub fn add(&mut self, asset_id: Handle<T>) {
        self.assets.insert(asset_id);
    }
}
