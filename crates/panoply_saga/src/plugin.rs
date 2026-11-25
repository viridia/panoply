//! The plugin, and the one host every script compiles against.

use std::sync::{Arc, OnceLock};

use bevy::{app::MainScheduleOrder, prelude::*};

use crate::{
    HostState, ReactionPass, ReactionReads, Reactions, ScriptModuleAsset, ScriptModuleLoader,
    natives::add_natives, reactive::run_reactions,
};

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
        let mut builder = HostState::new();
        add_natives(&mut builder);
        app.insert_resource(SagaHostBuilder(builder))
            .insert_resource(host.clone())
            .init_resource::<Reactions>()
            .init_resource::<ReactionReads>()
            .register_type::<ReactionReads>()
            .init_asset::<ScriptModuleAsset>()
            .register_asset_loader(ScriptModuleLoader { host })
            .init_schedule(ReactionPass)
            .add_systems(ReactionPass, run_reactions);
        app.world_mut()
            .resource_mut::<MainScheduleOrder>()
            .insert_after(Update, ReactionPass);
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
