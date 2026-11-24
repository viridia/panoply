use std::sync::Arc;

use super::{Aspect, Exemplar, InstanceAspects};
use crate::aspect::OwnedAspects;
use bevy::{ecs::system::EntityCommand, prelude::*};
use smallvec::SmallVec;

/// Custom command that updates an entity's components guided by a exemplar.
pub struct UpdateAspects<B: Bundle> {
    /// Exemplar to attach to entity
    pub exemplar: Handle<Exemplar>,

    /// Components to insert after applying exemplar, used to trigger post-processing.
    pub finish: B,
}

impl<B: Bundle> EntityCommand for UpdateAspects<B> {
    type Out = ();

    fn apply(self, mut entity: EntityWorldMut) {
        let exemplar_assets = entity.get_resource::<Assets<Exemplar>>().unwrap();
        let mut exemplars: SmallVec<[Arc<crate::exemplar::ExemplarData>; 8]> = SmallVec::new();
        // Walk the inheritance chain, nearest first.
        let mut next = Some(&self.exemplar);
        while let Some(exemplar) = next.and_then(|handle| exemplar_assets.get(handle)) {
            exemplars.push(exemplar.0.clone());
            next = exemplar.0.extends.as_ref();
        }

        // The instance's own aspects, then the chain's, each key taken from the first to have it.
        let mut resolved: Vec<Arc<dyn Aspect>> = Vec::new();
        let instance = entity
            .get::<InstanceAspects>()
            .map(|aspects| aspects.0.clone())
            .unwrap_or_default();
        let candidates = instance.iter().chain(
            exemplars
                .iter()
                .flat_map(|exemplar| exemplar.aspects.iter()),
        );
        for aspect in candidates {
            if !resolved.iter().any(|had| had.key() == aspect.key()) {
                resolved.push(aspect.clone());
            }
        }

        // Detach what was attached and is no longer resolved, then attach the rest.
        let owned = entity
            .get_mut::<OwnedAspects>()
            .map(|mut owned| std::mem::take(&mut owned.0))
            .unwrap_or_default();
        for old in owned.iter() {
            if !resolved.iter().any(|aspect| aspect.key() == old.key()) {
                old.detach(&mut entity);
            }
        }
        for aspect in resolved.iter() {
            aspect.attach(&mut entity);
        }

        entity.insert((OwnedAspects(resolved), self.finish));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{exemplar::ExemplarData, InstanceKind, NativeAspect};

    /// A native aspect whose config is itself.
    #[derive(Component, Reflect, Clone, Default, Debug, PartialEq)]
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

    /// An aspect keyed by a string rather than its type, as a scripted one is, and inert: it
    /// attaches nothing. Two keys make two aspects of one Rust type.
    #[derive(Reflect, Clone)]
    struct KeyedAspect {
        key: String,
        stateful: bool,
    }

    impl Aspect for KeyedAspect {
        fn key(&self) -> &str {
            &self.key
        }

        fn can_attach(&self, _kind: InstanceKind) -> bool {
            true
        }

        fn attach(&self, _entity: &mut EntityWorldMut) {}

        fn detach(&self, _entity: &mut EntityWorldMut) {}

        fn clone_boxed(&self) -> Box<dyn Aspect> {
            Box::new(self.clone())
        }

        fn has_state(&self) -> bool {
            self.stateful
        }
    }

    fn keyed(key: &str, stateful: bool) -> Arc<dyn Aspect> {
        Arc::new(KeyedAspect {
            key: key.to_string(),
            stateful,
        })
    }

    fn data(aspects: Vec<Arc<dyn Aspect>>) -> ExemplarData {
        ExemplarData {
            kind: None,
            display_name: None,
            alias: Vec::new(),
            aspects,
            extends: None,
            is_abstract: false,
            formulas: Vec::new(),
        }
    }

    /// A world holding an exemplar of `aspects`, and an entity with `instance` as its own.
    fn world_with(
        aspects: Vec<Arc<dyn Aspect>>,
        instance: Vec<Arc<dyn Aspect>>,
    ) -> (World, Handle<Exemplar>, Entity) {
        let mut world = World::new();
        let mut exemplars = Assets::<Exemplar>::default();
        let handle = exemplars.add(Exemplar(Arc::new(data(aspects))));
        world.insert_resource(exemplars);
        let entity = world.spawn(InstanceAspects(instance)).id();
        (world, handle, entity)
    }

    fn update(world: &mut World, exemplar: &Handle<Exemplar>, entity: Entity) {
        UpdateAspects {
            exemplar: exemplar.clone(),
            finish: (),
        }
        .apply(world.entity_mut(entity));
    }

    fn keys(world: &World, entity: Entity) -> Vec<String> {
        world
            .get::<OwnedAspects>(entity)
            .unwrap()
            .iter()
            .map(|aspect| aspect.key().to_string())
            .collect()
    }

    #[test]
    fn aspects_are_owned_by_key() {
        let aspects = vec![
            Arc::new(Height(2.)) as Arc<dyn Aspect>,
            keyed("test::Alpha", false),
            keyed("test::Beta", false),
        ];
        let (mut world, exemplar, entity) = world_with(aspects, Vec::new());
        update(&mut world, &exemplar, entity);
        assert_eq!(
            keys(&world, entity),
            ["Height", "test::Alpha", "test::Beta"]
        );
        assert_eq!(world.get::<Height>(entity), Some(&Height(2.)));
    }

    #[test]
    fn aspects_gone_from_the_exemplar_detach() {
        let aspects = vec![
            Arc::new(Height(2.)) as Arc<dyn Aspect>,
            keyed("test::Alpha", false),
            keyed("test::Beta", false),
        ];
        let (mut world, exemplar, entity) = world_with(aspects, Vec::new());
        update(&mut world, &exemplar, entity);
        Exemplar::edit(
            &mut world.resource_mut::<Assets<Exemplar>>(),
            exemplar.id(),
            |data| {
                data.aspects.retain(|aspect| aspect.key() == "test::Alpha");
                Ok(())
            },
        )
        .unwrap();
        update(&mut world, &exemplar, entity);
        assert_eq!(keys(&world, entity), ["test::Alpha"]);
        assert_eq!(world.get::<Height>(entity), None);
    }

    #[test]
    fn an_instance_aspect_overrides_the_exemplars_of_its_key() {
        let (mut world, exemplar, entity) = world_with(
            vec![Arc::new(Height(2.)), keyed("test::Alpha", false)],
            vec![Arc::new(Height(3.)), keyed("test::Alpha", true)],
        );
        update(&mut world, &exemplar, entity);
        let owned = world.get::<OwnedAspects>(entity).unwrap();
        assert_eq!(owned.iter().count(), 2);
        assert!(owned.get("test::Alpha").unwrap().has_state());
        assert_eq!(world.get::<Height>(entity), Some(&Height(3.)));
    }

    #[test]
    fn an_entitys_aspects_are_gathered_across_kinds() {
        let (mut world, exemplar, entity) = world_with(
            vec![
                Arc::new(Height(2.)),
                keyed("test::Alpha", true),
                keyed("test::Beta", true),
            ],
            Vec::new(),
        );
        update(&mut world, &exemplar, entity);
        let stateful: Vec<&str> = world
            .get::<OwnedAspects>(entity)
            .unwrap()
            .iter()
            .filter(|aspect| aspect.has_state())
            .map(|aspect| aspect.key())
            .collect();
        assert_eq!(stateful, ["test::Alpha", "test::Beta"]);
    }
}
