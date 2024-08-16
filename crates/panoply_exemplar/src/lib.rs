//! An [`Exemplar`] is like a blueprint or template for an entity. An exemplar consists of
//! one or more [`Aspect`]s, which are like ECS components for a prototype.

#![warn(missing_docs)]

mod aspect;
mod aspect_list;
mod command;
mod exemplar;
mod instance_type;
mod loader;
/// Serialzation and deserialization functions.
pub mod ser;

use bevy::{
    app::{App, Plugin},
    asset::AssetApp,
};

pub use aspect::*;
pub use aspect_list::AspectList;
pub use aspect_list::AspectListDeserializer;
pub use command::UpdateAspects;
pub use exemplar::Exemplar;
use exemplar::ExemplarCatalog;
pub use instance_type::InstanceType;

/// Bevy plugin for exemplars.
pub struct ExemplarPlugin;

impl Plugin for ExemplarPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<ExemplarCatalog>()
            .init_asset::<Exemplar>()
            .init_asset_loader::<loader::ExemplarLoader>();
    }
}
