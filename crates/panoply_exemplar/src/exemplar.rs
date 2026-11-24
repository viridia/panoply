use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use panoply_saga::FormulaRef;
use std::sync::Arc;

use crate::InstanceKind;

/// Defines a prototype for instantiating a game object. A clone shares its aspects; one changed
/// through [`aspect_mut`](crate::aspect_mut) is copied first.
#[derive(TypePath, Clone)]
pub struct ExemplarData {
    /// What this exemplar's instances are, if its file says.
    pub kind: Option<InstanceKind>,

    /// Optional human-readable display name.
    pub display_name: Option<String>,

    /// List of alternate names for loading this exemplar, used when migrating a exemplar
    /// from an old name to a new one.
    pub alias: Vec<String>,

    /// List of aspects that this exemplar has.
    pub aspects: Vec<Arc<dyn crate::Aspect>>,

    /// Inherited prototype for this exemplar.
    pub extends: Option<Handle<Exemplar>>,

    /// Only extended, never placed: the editor leaves it out of the exemplars to place. Not
    /// inherited.
    pub is_abstract: bool,

    /// The formulas written in this exemplar's aspects, checked once its chain has loaded.
    pub(crate) formulas: Vec<FormulaRef>,
}

/// An exemplar is like a blueprint or template for an entity.
#[derive(TypePath, Asset)]
pub struct Exemplar(pub Arc<ExemplarData>);

impl Exemplar {
    /// Changes the exemplar `id` through `change`, which gets a copy of its data, and keeps the
    /// copy only if `change` succeeds. The data is shared, with the exemplar's aliases and with
    /// instances being realized, so it is never changed in place; every exemplar that shared it
    /// gets the copy, so aliases follow.
    pub fn edit<R>(
        exemplars: &mut Assets<Exemplar>,
        id: AssetId<Exemplar>,
        change: impl FnOnce(&mut ExemplarData) -> Result<R, BevyError>,
    ) -> Result<R, BevyError> {
        let old = exemplars
            .get(id)
            .ok_or("the exemplar is not loaded")?
            .0
            .clone();
        let mut data = (*old).clone();
        let result = change(&mut data)?;
        let new = Arc::new(data);
        let sharing: Vec<AssetId<Exemplar>> = exemplars
            .iter()
            .filter(|(_, exemplar)| Arc::ptr_eq(&exemplar.0, &old))
            .map(|(id, _)| id)
            .collect();
        for id in sharing {
            if let Some(mut exemplar) = exemplars.get_mut(id) {
                exemplar.0 = new.clone();
            }
        }
        Ok(result)
    }

    /// Whether `id` is `exemplar` or along its inheritance chain.
    pub fn chain_contains(
        exemplars: &Assets<Exemplar>,
        exemplar: AssetId<Exemplar>,
        id: AssetId<Exemplar>,
    ) -> bool {
        let mut next = Some(exemplar);
        // A chain longer than there are exemplars has a cycle.
        for _ in 0..=exemplars.len() {
            let Some(current) = next else {
                return false;
            };
            if current == id {
                return true;
            }
            next = exemplars
                .get(current)
                .and_then(|exemplar| exemplar.0.extends.as_ref())
                .map(Handle::id);
        }
        false
    }

    /// The aspect of type `T` that an instance of `exemplar` would get, the nearest along its
    /// inheritance chain, read without spawning an instance.
    pub fn find_aspect<T: crate::Aspect>(
        exemplars: &Assets<Exemplar>,
        exemplar: impl Into<AssetId<Exemplar>>,
    ) -> Option<&T> {
        let mut next = exemplars.get(exemplar.into());
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

    /// Whether an instance of `exemplar` would get an aspect that has state
    /// ([`Aspect::has_state`](crate::Aspect::has_state)), from it or along its chain.
    pub fn has_state(exemplars: &Assets<Exemplar>, exemplar: AssetId<Exemplar>) -> bool {
        let mut next = exemplars.get(exemplar);
        // A chain longer than there are exemplars has a cycle.
        for _ in 0..=exemplars.len() {
            let Some(exemplar) = next else {
                return false;
            };
            if exemplar.0.aspects.iter().any(|aspect| aspect.has_state()) {
                return true;
            }
            next = exemplar
                .0
                .extends
                .as_ref()
                .and_then(|handle| exemplars.get(handle));
        }
        false
    }

    /// Whether an instance of `exemplar` would get the aspect whose key is `aspect`, from it or
    /// along its inheritance chain.
    pub fn has_aspect(exemplars: &Assets<Exemplar>, exemplar: &ExemplarData, aspect: &str) -> bool {
        let mut next = Some(exemplar);
        while let Some(exemplar) = next {
            if exemplar.aspects.iter().any(|a| a.key() == aspect) {
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

/// An asset that stores multiple exemplars.
#[derive(TypePath, Asset)]
pub struct ExemplarCatalog {
    /// Every exemplar in the catalog by label, aliases included.
    pub(crate) entries: HashMap<String, Handle<Exemplar>>,

    /// Each exemplar's data as loaded, by its own name, aliases left out. An edit replaces an
    /// exemplar's data rather than changing it, so this stays as the file says; saving compares
    /// it with the exemplar's data now to find what was edited.
    pub(crate) loaded: Vec<(String, Arc<ExemplarData>)>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{aspect_mut, NativeAspect};

    #[derive(Component, Reflect, Clone, Default)]
    struct Height(f32);

    impl NativeAspect for Height {
        type Config = Self;

        fn config(&self) -> Self {
            self.clone()
        }

        fn can_attach(&self, _kind: InstanceKind) -> bool {
            true
        }
    }

    fn data(height: f32, extends: Option<Handle<Exemplar>>) -> ExemplarData {
        ExemplarData {
            kind: None,
            display_name: None,
            alias: Vec::new(),
            aspects: vec![Arc::new(Height(height))],
            extends,
            is_abstract: false,
            formulas: Vec::new(),
        }
    }

    fn height(exemplars: &Assets<Exemplar>, id: AssetId<Exemplar>) -> f32 {
        let aspect = exemplars.get(id).unwrap().0.aspects[0].as_reflect();
        aspect.downcast_ref::<Height>().unwrap().0
    }

    fn set_height(data: &mut ExemplarData, value: f32) {
        let aspect = aspect_mut(&mut data.aspects[0]).as_reflect_mut();
        aspect.downcast_mut::<Height>().unwrap().0 = value;
    }

    #[test]
    fn an_edit_reaches_the_exemplars_aliases() {
        let mut exemplars = Assets::<Exemplar>::default();
        let shared = Arc::new(data(1., None));
        let exemplar = exemplars.add(Exemplar(shared.clone())).id();
        let alias = exemplars.add(Exemplar(shared)).id();
        let other = exemplars.add(Exemplar(Arc::new(data(1., None)))).id();

        Exemplar::edit(&mut exemplars, exemplar, |data| {
            set_height(data, 2.);
            Ok(())
        })
        .unwrap();
        assert_eq!(height(&exemplars, exemplar), 2.);
        assert_eq!(height(&exemplars, alias), 2.);
        assert_eq!(height(&exemplars, other), 1.);
    }

    #[test]
    fn a_failed_edit_changes_nothing() {
        let mut exemplars = Assets::<Exemplar>::default();
        let exemplar = exemplars.add(Exemplar(Arc::new(data(1., None)))).id();
        let result = Exemplar::edit(&mut exemplars, exemplar, |data| {
            set_height(data, 2.);
            Err::<(), _>("refused".into())
        });
        assert!(result.is_err());
        assert_eq!(height(&exemplars, exemplar), 1.);
    }

    #[test]
    fn a_chain_contains_its_ancestors_only() {
        let mut exemplars = Assets::<Exemplar>::default();
        let base = exemplars.add(Exemplar(Arc::new(data(1., None))));
        let child = exemplars.add(Exemplar(Arc::new(data(1., Some(base.clone())))));
        let other = exemplars.add(Exemplar(Arc::new(data(1., None))));
        assert!(Exemplar::chain_contains(&exemplars, child.id(), base.id()));
        assert!(Exemplar::chain_contains(&exemplars, child.id(), child.id()));
        assert!(!Exemplar::chain_contains(&exemplars, base.id(), child.id()));
        assert!(!Exemplar::chain_contains(
            &exemplars,
            child.id(),
            other.id()
        ));
    }
}
