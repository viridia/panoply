use bevy::{prelude::*, utils::hashbrown::HashMap};
use serde::Serialize;
use std::sync::Arc;

use crate::InstanceType;

/// Defines a prototype for instantiating a game object.
#[derive(TypePath)]
pub struct ExemplarData {
    /// Type of instance that this schematic can create.
    pub meta_type: InstanceType,

    /// Optional human-readable display name.
    pub display_name: Option<String>,

    /// List of alternate names for loading this schematic, used when migrating a schematic
    /// from an old name to a new one.
    pub(crate) alias: Vec<String>,

    /// List of aspects that this schematic has.
    pub aspects: Vec<Box<dyn crate::Aspect>>,

    /// Inherited prototype for this schematic.
    pub extends: Option<Handle<Exemplar>>,
}

/// An exemplar is like a blueprint or template for an entity.
#[derive(TypePath, Asset)]
pub struct Exemplar(pub Arc<ExemplarData>);

impl Serialize for ExemplarData {
    fn serialize<S>(&self, _serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        todo!();
    }
}

/// An asset that stores multiple exemplars.
#[derive(TypePath, Asset)]
pub struct ExemplarCatalog {
    #[allow(dead_code)]
    pub(crate) entries: HashMap<String, Handle<Exemplar>>,
}
