use bevy::{prelude::*, utils::hashbrown::HashMap};
use serde::Serialize;
use std::sync::Arc;

mod aspect;
mod command;
mod loader;

pub use aspect::*;
pub use command::UpdateAspects;

// TODO: Use type ids instead.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstanceType {
    None,
    Actor,
    Item,
    Floor,
    Wall,
    Fixture,
    TerrainFx,
}

/// Defines a prototype for instantiating a game object.
#[derive(TypePath)]
pub struct SchematicData {
    /// Type of instance that this schematic can create.
    meta_type: InstanceType,

    /// Optional human-readable display name.
    display_name: Option<String>,

    /// List of alternate names for loading this schematic.
    alias: Vec<String>,

    /// List of aspects that this schematic has.
    pub aspects: Vec<Box<dyn aspect::Aspect>>,

    /// Inherited prototype for this schematic.
    extends: Option<Handle<Schematic>>,
}

#[derive(TypePath, Asset)]
pub struct Schematic(pub Arc<SchematicData>);

impl Serialize for SchematicData {
    fn serialize<S>(&self, _serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        todo!();
    }
}

/// An asset that stores multiple metadata items.
#[derive(TypePath, Asset)]
pub struct SchematicCatalog {
    entries: HashMap<String, Handle<Schematic>>,
}

pub struct SchematicPlugin;

impl Plugin for SchematicPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<SchematicCatalog>()
            .init_asset::<Schematic>()
            .init_asset_loader::<loader::SchematicLoader>();
    }
}
