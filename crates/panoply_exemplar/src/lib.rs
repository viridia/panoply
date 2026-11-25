//! An [`Exemplar`] is like a blueprint or template for an entity. An exemplar consists of
//! one or more [`Aspect`]s, which are like ECS components for a prototype.

#![warn(missing_docs)]

pub mod annotations;
mod aspect;
mod aspect_list;
mod catalogs;
mod command;
mod entry;
mod exemplar;
mod instance_kind;
mod interaction;
mod loader;
mod save;
mod script_components;
mod scripted_aspect;
/// Serialzation and deserialization functions.
pub mod ser;

use bevy::{
    app::{App, Plugin, Update},
    asset::AssetApp,
};
use panoply_saga::{SagaHostBuilder, SagaPlugin};

pub use aspect::*;
pub use aspect_list::AspectList;
pub use aspect_list::AspectListDeserializer;
pub use catalogs::{CatalogMember, CatalogRoots, CatalogServer};
pub use command::UpdateAspects;
pub use entry::{
    read_entries, CatalogEntry, CatalogEntryAppExt, EntryCatalog, EntryCatalogLoader,
    EntryCatalogLoaderError,
};
pub use exemplar::{Exemplar, ExemplarCatalog, ExemplarData};
pub use instance_kind::{InstanceKind, UnknownInstanceKind};
pub use interaction::*;
pub use script_components::{ScriptComponentAppExt, ScriptComponents};
pub use scripted_aspect::{part_from_json, part_to_json, ScriptedAspect};

/// Bevy plugin for exemplars. Adds [`SagaPlugin`], which aspects' formulas compile with, if
/// it is not added yet.
pub struct ExemplarPlugin;

impl Plugin for ExemplarPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<SagaPlugin>() {
            app.add_plugins(SagaPlugin);
        }
        add_interaction_fields(&mut app.world_mut().resource_mut::<SagaHostBuilder>().0);
        app.init_resource::<CatalogRoots>()
            .init_asset::<ExemplarCatalog>()
            .init_asset::<Exemplar>()
            .init_asset_loader::<loader::ExemplarLoader>()
            .add_systems(Update, script_components::check_script_components);
    }
}
