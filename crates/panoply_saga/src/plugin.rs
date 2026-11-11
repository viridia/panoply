//! The plugin, and the one host every script compiles against.

use std::sync::{Arc, OnceLock};

use bevy::prelude::*;

use crate::{HostState, ScriptModuleAsset, ScriptModuleLoader};

/// Builds the host as plugins build, then freezes it, and loads `.saga` modules.
pub struct SagaPlugin;

/// The host while plugins build, for them to register their types and functions with. Gone
/// once the app finishes building.
#[derive(Resource)]
pub struct SagaHostBuilder(pub HostState);

/// The host every script compiles and runs against, shared immutably once the app has finished
/// building. Loaders take a clone when they are made, before it is frozen.
#[derive(Resource, Clone, Default)]
pub struct SagaHost(Arc<OnceLock<HostState>>);

impl SagaHost {
    /// The frozen host.
    ///
    /// # Panics
    ///
    /// Before [`SagaPlugin`] finishes, which is before any asset loads.
    pub fn get(&self) -> &HostState {
        self.0
            .get()
            .expect("the SAGA host is read before SagaPlugin finished")
    }
}

impl Plugin for SagaPlugin {
    fn build(&self, app: &mut App) {
        let host = SagaHost::default();
        app.insert_resource(SagaHostBuilder(HostState::new()))
            .insert_resource(host.clone())
            .init_asset::<ScriptModuleAsset>()
            .register_asset_loader(ScriptModuleLoader { host });
    }

    fn finish(&self, app: &mut App) {
        let world = app.world_mut();
        let builder = world
            .remove_resource::<SagaHostBuilder>()
            .expect("SagaPlugin inserts the builder");
        if world.resource::<SagaHost>().0.set(builder.0).is_err() {
            error!("The SAGA host was frozen twice");
        }
    }
}
