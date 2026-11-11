//! An [`Exemplar`] is like a blueprint or template for an entity. An exemplar consists of
//! one or more [`Aspect`]s, which are like ECS components for a prototype.

#![warn(missing_docs)]

mod aspect;
mod aspect_list;
mod catalogs;
mod command;
mod exemplar;
mod instance_type;
mod interaction;
mod loader;
/// Serialzation and deserialization functions.
pub mod ser;

use bevy::{
    app::{App, Plugin},
    asset::AssetApp,
};
use panoply_saga::SagaPlugin;

pub use aspect::*;
pub use aspect_list::AspectList;
pub use aspect_list::AspectListDeserializer;
pub use catalogs::{ExemplarCatalogs, ExemplarServer};
pub use command::UpdateAspects;
pub use exemplar::Exemplar;
use exemplar::ExemplarCatalog;
pub use instance_type::InstanceType;
pub use interaction::*;

/// Bevy plugin for exemplars. Adds [`SagaPlugin`], which aspects' formulas compile with, if
/// it is not added yet.
pub struct ExemplarPlugin;

impl Plugin for ExemplarPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<SagaPlugin>() {
            app.add_plugins(SagaPlugin);
        }
        app.init_resource::<ExemplarCatalogs>()
            .init_resource::<InteractionActions>()
            .register_type::<Interactions>()
            .register_type::<InteractionDesc>()
            .register_type::<Vec<InteractionDesc>>()
            .init_asset::<ExemplarCatalog>()
            .init_asset::<Exemplar>()
            .init_asset_loader::<loader::ExemplarLoader>();
    }
}
