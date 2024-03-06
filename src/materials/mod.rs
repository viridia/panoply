use bevy::{pbr::ExtendedMaterial, prelude::*};
mod outline;

pub use self::outline::OutlineMaterial;

pub struct MaterialsPlugin;

impl Plugin for MaterialsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<
            ExtendedMaterial<StandardMaterial, OutlineMaterial>,
        >::default());
    }
}
