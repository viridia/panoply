use std::sync::Arc;

use super::{Aspect, Exemplar, InstanceAspects};
use crate::{aspect::OwnedAspects, new_aspect, InstanceKind};
use bevy::{ecs::system::EntityCommand, prelude::*, reflect::TypeRegistry};
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
        if resolved.iter().any(|aspect| !aspect.requires().is_empty()) {
            let registry = entity
                .world()
                .get_resource::<AppTypeRegistry>()
                .cloned()
                .unwrap_or_default();
            let kind = exemplars.iter().find_map(|exemplar| exemplar.kind);
            let (with, failures) = with_requirements(resolved, kind, &registry.read());
            resolved = with;
            for failure in failures {
                match entity.get::<Name>() {
                    Some(name) => error!("{name} ({}): {failure}", entity.id()),
                    None => error!("{}: {failure}", entity.id()),
                }
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

/// `resolved`, an instance's aspects, with each aspect they require and lack at its defaults,
/// and those it requires in turn (D121). An aspect requiring one that cannot attach at its
/// defaults is left out, as is anything it alone brought in, and each is reported.
fn with_requirements(
    resolved: Vec<Arc<dyn Aspect>>,
    kind: Option<InstanceKind>,
    registry: &TypeRegistry,
) -> (Vec<Arc<dyn Aspect>>, Vec<String>) {
    let mut excluded: Vec<Arc<str>> = Vec::new();
    let mut failures = Vec::new();
    // Each pass leaves out one more aspect or ends, so it ends within as many passes as
    // there are aspects.
    loop {
        let mut aspects: Vec<Arc<dyn Aspect>> = resolved
            .iter()
            .filter(|aspect| !excluded.iter().any(|key| **key == *aspect.key()))
            .cloned()
            .collect();
        let mut failed = None;
        let mut next = 0;
        'aspects: while next < aspects.len() {
            let requirer = aspects[next].clone();
            for key in requirer.requires() {
                if aspects.iter().any(|aspect| aspect.key() == &**key) {
                    continue;
                }
                let required = if excluded.contains(key) {
                    Err("it is not attached".to_string())
                } else {
                    required_aspect(requirer.as_ref(), key, kind, registry)
                };
                match required {
                    Ok(aspect) => aspects.push(aspect),
                    Err(reason) => {
                        failed = Some((Arc::<str>::from(requirer.key()), key.clone(), reason));
                        break 'aspects;
                    }
                }
            }
            next += 1;
        }
        let Some((requirer, key, reason)) = failed else {
            return (aspects, failures);
        };
        failures.push(format!(
            "{requirer} is not attached: it requires {key}, which cannot attach at its \
             defaults, as {reason}"
        ));
        excluded.push(requirer);
    }
}

/// The aspect of key `key` that `requirer` requires, at its defaults, for an instance of
/// `kind`; or why there is none.
pub fn required_aspect(
    requirer: &dyn Aspect,
    key: &str,
    kind: Option<InstanceKind>,
    registry: &TypeRegistry,
) -> Result<Arc<dyn Aspect>, String> {
    let decls = requirer.required_decls();
    // A scripted field without a default must be given one (D121), rather than taking its
    // type's.
    if let Some(decl) = decls.iter().find(|decl| &*decl.key == key) {
        if let Some(field) = decl.fields.iter().find(|field| field.default.is_none()) {
            return Err(format!("its field {} has no default", field.name));
        }
    }
    let aspect = new_aspect(registry, decls.iter(), key)?;
    match kind {
        Some(kind) if !aspect.can_attach(kind) => Err(format!("it does not attach to {kind}")),
        _ => Ok(aspect),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{exemplar::ExemplarData, NativeAspect, ReflectAspect};

    /// A native aspect whose config is itself.
    #[derive(Component, Reflect, Clone, Default, Debug, PartialEq)]
    #[reflect(Aspect, Default)]
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

    /// The aspects `src` declares, as the module `scripts/guard.saga`, each at its defaults.
    fn scripted(src: &str) -> Vec<Arc<dyn Aspect>> {
        let module = futures_lite::future::block_on(panoply_saga::compile_module(
            "scripts/guard.saga",
            src,
            &panoply_saga::HostState::new(),
            &panoply_saga::NoImports,
        ))
        .unwrap()
        .share();
        let registry = TypeRegistry::new();
        module
            .aspects()
            .iter()
            .map(|decl| Arc::new(crate::ScriptedAspect::new(decl, &registry).unwrap()) as _)
            .collect()
    }

    /// `world_with`, its registry knowing `Height`.
    fn registered(
        aspects: Vec<Arc<dyn Aspect>>,
        instance: Vec<Arc<dyn Aspect>>,
    ) -> (World, Handle<Exemplar>, Entity) {
        let (mut world, exemplar, entity) = world_with(aspects, instance);
        let registry = AppTypeRegistry::default();
        registry.write().register::<Height>();
        world.insert_resource(registry);
        (world, exemplar, entity)
    }

    const GUARD: &str = "scripts::guard::Guard";
    const MARK: &str = "scripts::guard::Mark";

    #[test]
    fn a_required_aspect_attaches_at_its_defaults() {
        // Guard requires Mark, which requires Height in turn.
        let module = scripted(
            "#[require(Mark)] aspect Guard for Fixture { }
            #[require(Height)] aspect Mark for Fixture { count: i32 = 3 }",
        );
        let (mut world, exemplar, entity) = registered(vec![module[0].clone()], Vec::new());
        update(&mut world, &exemplar, entity);
        assert_eq!(keys(&world, entity), [GUARD, MARK, "Height"]);
        assert_eq!(world.get::<Height>(entity), Some(&Height(0.)));

        // One the instance has is kept, not replaced by its defaults.
        let (mut world, exemplar, entity) =
            registered(vec![module[0].clone()], vec![Arc::new(Height(3.))]);
        update(&mut world, &exemplar, entity);
        assert_eq!(keys(&world, entity), ["Height", GUARD, MARK]);
        assert_eq!(world.get::<Height>(entity), Some(&Height(3.)));
    }

    #[test]
    fn a_required_field_without_a_default_must_be_given() {
        let module = scripted(
            "#[require(Mark)] aspect Guard for Fixture { }
            aspect Mark for Fixture { count: i32 }",
        );
        let exemplar_aspects = vec![Arc::new(Height(2.)) as Arc<dyn Aspect>, module[0].clone()];
        let (mut world, exemplar, entity) = registered(exemplar_aspects.clone(), Vec::new());
        update(&mut world, &exemplar, entity);
        assert_eq!(keys(&world, entity), ["Height"]);

        // Given by the instance, it is attached, and so is the aspect requiring it.
        let (mut world, exemplar, entity) = registered(exemplar_aspects, vec![module[1].clone()]);
        update(&mut world, &exemplar, entity);
        assert_eq!(keys(&world, entity), [MARK, "Height", GUARD]);
    }

    /// A native aspect requiring `Height`.
    #[derive(Component, Reflect, Clone, Default)]
    #[reflect(Aspect, Default)]
    struct Ledge;

    impl NativeAspect for Ledge {
        type Config = Self;

        fn config(&self) -> Self {
            self.clone()
        }

        fn can_attach(&self, _kind: InstanceKind) -> bool {
            true
        }

        fn requires(&self) -> &[Arc<str>] {
            static HEIGHT: std::sync::LazyLock<[Arc<str>; 1]> =
                std::sync::LazyLock::new(|| ["Height".into()]);
            &*HEIGHT
        }
    }

    #[test]
    fn a_native_aspect_requires_another() {
        let (mut world, exemplar, entity) = registered(vec![Arc::new(Ledge)], Vec::new());
        update(&mut world, &exemplar, entity);
        assert_eq!(keys(&world, entity), ["Ledge", "Height"]);
        assert_eq!(world.get::<Height>(entity), Some(&Height(0.)));
    }

    #[test]
    fn a_required_aspect_must_be_registered() {
        let module = scripted("#[require(Weight)] aspect Guard for Fixture { }");
        let (mut world, exemplar, entity) = registered(module, Vec::new());
        update(&mut world, &exemplar, entity);
        assert!(keys(&world, entity).is_empty());
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
