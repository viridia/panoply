use bevy::asset::AssetPath;

use crate::guise::path::relative_asset_path;

#[derive(Debug, Clone, PartialEq)]
pub struct AssetRef {
    path: String,
    resolved: AssetPath<'static>,
}

impl AssetRef {
    pub fn new(path: &str) -> Self {
        Self {
            path: path.to_string(),
            resolved: AssetPath::from(path.to_string()),
        }
    }

    pub fn resolve_asset_path(&mut self, base: &AssetPath) {
        self.resolved = relative_asset_path(base, &self.path.clone().to_owned()).to_owned();
    }

    pub fn resolved(&self) -> &AssetPath {
        &self.resolved
    }
}
