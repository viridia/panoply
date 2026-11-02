use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use serde::Serialize;
use std::sync::Arc;

use crate::InstanceType;

/// Defines a prototype for instantiating a game object.
#[derive(TypePath)]
pub struct ExemplarData {
    /// Type of instance that this exemplar can create.
    pub meta_type: InstanceType,

    /// Optional human-readable display name.
    pub display_name: Option<String>,

    /// List of alternate names for loading this exemplar, used when migrating a exemplar
    /// from an old name to a new one.
    pub(crate) alias: Vec<String>,

    /// List of aspects that this exemplar has.
    pub aspects: Vec<Box<dyn crate::Aspect>>,

    /// Inherited prototype for this exemplar.
    pub extends: Option<Handle<Exemplar>>,
}

/// An exemplar is like a blueprint or template for an entity.
#[derive(TypePath, Asset)]
pub struct Exemplar(pub Arc<ExemplarData>);

impl Exemplar {
    /// The aspect of type `T` that an instance of `exemplar` would get, the nearest along its
    /// inheritance chain, read without spawning an instance.
    pub fn find_aspect<'a, T: crate::Aspect>(
        exemplars: &'a Assets<Exemplar>,
        exemplar: &Handle<Exemplar>,
    ) -> Option<&'a T> {
        let mut next = exemplars.get(exemplar);
        while let Some(exemplar) = next {
            let found = exemplar
                .0
                .aspects
                .iter()
                .find_map(|aspect| aspect.as_reflect().downcast_ref::<T>());
            if found.is_some() {
                return found;
            }
            next = exemplar
                .0
                .extends
                .as_ref()
                .and_then(|handle| exemplars.get(handle));
        }
        None
    }
}

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
