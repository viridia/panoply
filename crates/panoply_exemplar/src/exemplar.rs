use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use panoply_saga::FormulaRef;
use serde::Serialize;
use std::any::TypeId;
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

    /// The formulas written in this exemplar's aspects, checked once its chain has loaded.
    pub(crate) formulas: Vec<FormulaRef>,
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

    /// Whether an instance of `exemplar` would get the aspect whose type is `aspect`, from it or
    /// along its inheritance chain.
    pub fn has_aspect(
        exemplars: &Assets<Exemplar>,
        exemplar: &ExemplarData,
        aspect: TypeId,
    ) -> bool {
        let mut next = Some(exemplar);
        while let Some(exemplar) = next {
            if exemplar.aspects.iter().any(|a| a.id() == aspect) {
                return true;
            }
            next = exemplar
                .extends
                .as_ref()
                .and_then(|handle| exemplars.get(handle))
                .map(|exemplar| &*exemplar.0);
        }
        false
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
