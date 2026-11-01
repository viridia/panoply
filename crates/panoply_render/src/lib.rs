mod materials;
mod models;

use bevy::app::{App, Plugin};

pub use materials::{
    BlackMaterialHandle, FlameMaterial, FlameMaterialHandle, FloorNoisyMaterial,
    FloorNoisyMaterialExt, OutlineMaterial, OutlineMaterialHandle,
};

/// Panoply's custom materials, and the processing of loaded glTF models: material extras and
/// pipeline warm-up.
pub struct PanoplyRenderPlugin;

impl Plugin for PanoplyRenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((materials::MaterialsPlugin, models::ModelsPlugin));
    }
}
