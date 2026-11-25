//! What can be done to an instance: the interactions its scripted aspects declare (D114), each a
//! struct literal of the interaction type registered here. Its fields are constants but for
//! `when`, a formula, and `perform`, an effect, so that only `when` changes what is offered.

use std::sync::Arc;

use bevy::prelude::*;
use panoply_saga::{
    scripted::{AspectDecl, InteractionDecl},
    ExprType, FieldKind, HostState, ScriptModule,
};

use crate::{OwnedAspects, ScriptedAspect};

/// How close an interaction can be performed from, when it does not say.
pub const DEFAULT_INTERACTION_DISTANCE: f32 = 1.5;

/// The cursor over an instance whose interaction names none, or that offers several.
pub const DEFAULT_INTERACTION_CURSOR: &str = "interact";

/// Registers the interaction type on `host`: what an interaction may set. [`ExemplarPlugin`]
/// registers it on the game's host.
///
/// [`ExemplarPlugin`]: crate::ExemplarPlugin
pub fn add_interaction_fields(host: &mut HostState) {
    // Shown to the player while the interaction is available.
    host.add_interaction_field("caption", FieldKind::Constant(ExprType::String), true);
    // The name of the mouse cursor to show over the instance.
    host.add_interaction_field("cursor", FieldKind::Constant(ExprType::String), false);
    // How close the interaction can be performed from, in meters.
    host.add_interaction_field("distance", FieldKind::Constant(ExprType::F32), false);
    // Whether the interaction is offered now; without one, it always is.
    host.add_interaction_field("when", FieldKind::Formula(ExprType::Boolean), false);
    // What performing it does.
    host.add_interaction_field("perform", FieldKind::Effect, true);
}

/// One interaction an instance offers, declared by one of its scripted aspects.
#[derive(Clone)]
pub struct ScriptedInteraction {
    aspect: Arc<AspectDecl>,
    module: Arc<ScriptModule>,
    /// Its index in the aspect's interactions.
    index: usize,
}

impl PartialEq for ScriptedInteraction {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.aspect, &other.aspect) && self.index == other.index
    }
}

impl std::fmt::Debug for ScriptedInteraction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}::{}", self.aspect.key, self.name())
    }
}

impl ScriptedInteraction {
    fn decl(&self) -> &InteractionDecl {
        &self.aspect.interactions[self.index]
    }

    /// The interaction's name, as its aspect declares it.
    pub fn name(&self) -> &str {
        &self.decl().name
    }

    /// What the player is shown, as "Open".
    pub fn caption(&self) -> &str {
        self.decl()
            .constant::<String>("caption")
            .map_or("", String::as_str)
    }

    /// The name of the cursor to show over the instance, if the interaction names one.
    pub fn cursor(&self) -> Option<&str> {
        self.decl().constant::<String>("cursor").map(String::as_str)
    }

    /// How close the interaction can be performed from, in meters.
    pub fn reach(&self) -> f32 {
        self.decl()
            .constant::<f32>("distance")
            .copied()
            .unwrap_or(DEFAULT_INTERACTION_DISTANCE)
    }

    /// Whether `entity` offers the interaction now, by its `when`. One whose `when` fails to
    /// evaluate, as on an entity without what it reads, offers nothing.
    pub fn enabled(&self, world: &World, host: &HostState, entity: Entity) -> bool {
        let Some(index) = self.decl().field("when") else {
            return true;
        };
        self.module
            .run_formula(world, host, entity, index)
            .unwrap_or(false)
    }

    /// Performs the interaction on `entity`: runs its effect, then applies its writes.
    pub fn perform(&self, world: &mut World, host: &HostState, entity: Entity) {
        let Some(index) = self.decl().field("perform") else {
            return;
        };
        match self.module.run_effect(world, host, entity, index) {
            Ok(effects) => effects.apply(world),
            Err(err) => error!("{self:?}.perform on {entity}: {err}"),
        }
    }
}

/// The interactions `aspects` declare, in aspect order, then in the order each declares them.
/// An aspect whose module is gone offers none.
pub fn interactions_of(aspects: &OwnedAspects) -> impl Iterator<Item = ScriptedInteraction> + '_ {
    aspects
        .iter()
        .filter_map(|aspect| aspect.as_any().downcast_ref::<ScriptedAspect>())
        .filter_map(|scripted| Some((scripted.decl().clone(), scripted.module()?.clone())))
        .flat_map(|(aspect, module)| {
            (0..aspect.interactions.len()).map(move |index| ScriptedInteraction {
                aspect: aspect.clone(),
                module: module.clone(),
                index,
            })
        })
}

/// The first interaction `entity` offers now that reaches `distance`.
pub fn first_enabled(
    aspects: &OwnedAspects,
    distance: f32,
    world: &World,
    host: &HostState,
    entity: Entity,
) -> Option<ScriptedInteraction> {
    interactions_of(aspects).find(|interaction| {
        distance <= interaction.reach() && interaction.enabled(world, host, entity)
    })
}

/// The name of the cursor to show over `entity`, however far away it is, if it offers anything
/// now: the one interaction's, or the default when it names none or there are several.
pub fn hover_cursor(
    aspects: &OwnedAspects,
    world: &World,
    host: &HostState,
    entity: Entity,
) -> Option<String> {
    let mut enabled =
        interactions_of(aspects).filter(|interaction| interaction.enabled(world, host, entity));
    let first = enabled.next()?;
    if enabled.next().is_some() {
        return Some(DEFAULT_INTERACTION_CURSOR.to_owned());
    }
    Some(
        first
            .cursor()
            .unwrap_or(DEFAULT_INTERACTION_CURSOR)
            .to_owned(),
    )
}

#[cfg(test)]
pub(crate) mod tests {
    use bevy::reflect::{serde::TypedReflectDeserializer, FromReflect, GetTypeRegistration};
    use futures_lite::future::block_on;
    use panoply_saga::{compile_module, Formula, FormulaQueue, FormulaRef, NoImports};
    use serde::de::DeserializeSeed;
    use serde_json::json;

    use super::*;
    use crate::Aspect;

    /// A lid, for formulas to read as `self.lid`.
    #[derive(Component, Reflect, Default)]
    pub(crate) struct Lid {
        pub(crate) open: bool,
    }

    /// A host that has `self.lid`.
    pub(crate) fn lid_host() -> HostState {
        let mut host = HostState::new();
        host.add_component::<Lid>("lid");
        host
    }

    /// A field holding a formula, as an aspect's would.
    #[derive(Reflect, Default)]
    struct WithFormula {
        when: Option<Formula<bool>>,
    }

    /// The formula `src`, read and compiled as a loader would, reading the scripted aspects of
    /// `scripted`, with the formulas read.
    pub(crate) fn load_formula(
        src: &str,
        host: &HostState,
        scripted: &[Arc<AspectDecl>],
    ) -> (Formula<bool>, Vec<FormulaRef>) {
        let json = json!({ "when": src }).to_string();
        let mut registry = bevy::reflect::TypeRegistry::new();
        registry.register::<WithFormula>();
        let registration = registry
            .get(WithFormula::get_type_registration().type_id())
            .unwrap();
        let mut formulas = FormulaQueue::default();
        let value = {
            let reader = formulas.track_json(json.as_bytes());
            let mut deserializer = serde_json::Deserializer::from_reader(reader);
            TypedReflectDeserializer::with_processor(registration, &registry, &mut formulas)
                .deserialize(&mut deserializer)
                .unwrap()
        };
        let read = formulas.read_since(0);
        block_on(formulas.compile("test.json", host, json.as_bytes(), scripted));
        let value = WithFormula::from_reflect(&*value).unwrap();
        (value.when.unwrap(), read)
    }

    /// A host with the interaction type and `self.lid`.
    fn host() -> HostState {
        let mut host = lid_host();
        add_interaction_fields(&mut host);
        host
    }

    /// The aspects of `src`, as the module `scripts/test.saga`, attached to a new entity with
    /// their declared values, the entity's `OwnedAspects` listing them.
    fn spawn(world: &mut World, host: &HostState, src: &str) -> Entity {
        let module = block_on(compile_module("scripts/test.saga", src, host, &NoImports))
            .unwrap()
            .share();
        let registry = bevy::reflect::TypeRegistry::new();
        let aspects: Vec<Arc<dyn Aspect>> = module
            .aspects()
            .iter()
            .map(|decl| {
                let aspect = ScriptedAspect::from_json(decl, &json!({}), &registry, "t").unwrap();
                Arc::new(aspect) as Arc<dyn Aspect>
            })
            .collect();
        let mut entity = world.spawn_empty();
        for aspect in aspects.iter() {
            aspect.attach(&mut entity);
        }
        entity.insert(OwnedAspects(aspects));
        entity.id()
    }

    const CHEST: &str = r#"
        aspect Chest for Fixture {
            state open: bool = false,
            interaction open {
                caption: "Open", cursor: "open-chest", distance: 1.0,
                when: !open && !self.lid.open,
                perform: { open = true },
            }
            interaction close { caption: "Close", when: open, perform: { open = false } }
        }
    "#;

    fn names(world: &World, entity: Entity) -> Vec<String> {
        interactions_of(world.get::<OwnedAspects>(entity).unwrap())
            .map(|i| i.name().to_owned())
            .collect()
    }

    #[test]
    fn the_first_enabled_in_reach_is_offered_and_performed() {
        let host = host();
        let mut world = World::new();
        let chest = spawn(&mut world, &host, CHEST);
        world.entity_mut(chest).insert(Lid::default());
        assert_eq!(names(&world, chest), ["open", "close"]);
        let first = |world: &World, distance| {
            let aspects = world.get::<OwnedAspects>(chest).unwrap();
            first_enabled(aspects, distance, world, &host, chest)
        };

        let open = first(&world, 0.5).unwrap();
        assert_eq!(open.name(), "open");
        assert_eq!(open.caption(), "Open");
        // Beyond the open's reach, the close is not offered while the chest is shut.
        assert!(first(&world, 1.2).is_none());

        open.perform(&mut world, &host, chest);
        let close = first(&world, 1.2).unwrap();
        assert_eq!(close.name(), "close");
        assert_eq!(close.reach(), DEFAULT_INTERACTION_DISTANCE);
        close.perform(&mut world, &host, chest);
        assert_eq!(first(&world, 0.5).unwrap().name(), "open");

        // A `when` that reads what the entity lacks fails, and offers nothing.
        world.entity_mut(chest).remove::<Lid>();
        assert!(first(&world, 0.5).is_none());
    }

    #[test]
    fn hover_cursor_is_the_only_entrys_or_the_default() {
        let host = host();
        let mut world = World::new();
        let cursor = |world: &World, entity| {
            let aspects = world.get::<OwnedAspects>(entity).unwrap();
            hover_cursor(aspects, world, &host, entity)
        };
        let chest = spawn(&mut world, &host, CHEST);
        world.entity_mut(chest).insert(Lid::default());
        assert_eq!(cursor(&world, chest).as_deref(), Some("open-chest"));
        world.entity_mut(chest).insert(Lid { open: true });
        assert_eq!(cursor(&world, chest), None);

        let plain = spawn(
            &mut world,
            &host,
            "aspect A for Wall { interaction use { caption: \"Use\", perform: {} } }",
        );
        assert_eq!(
            cursor(&world, plain).as_deref(),
            Some(DEFAULT_INTERACTION_CURSOR)
        );

        let two = spawn(
            &mut world,
            &host,
            r#"aspect A for Wall {
                interaction a { caption: "A", cursor: "x", perform: {} }
                interaction b { caption: "B", cursor: "y", perform: {} }
            }"#,
        );
        assert_eq!(
            cursor(&world, two).as_deref(),
            Some(DEFAULT_INTERACTION_CURSOR)
        );
    }
}
