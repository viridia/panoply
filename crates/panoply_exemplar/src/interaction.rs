use std::any::TypeId;

use bevy::{platform::collections::HashMap, prelude::*};
use panoply_saga::{Formula, HostState};
use serde::Serialize;

use crate::{Aspect, DetachAspect, InstanceType, ReflectAspect, RemoveComponent};

/// How close an interaction can be performed from, when its entry does not say.
pub const DEFAULT_INTERACTION_DISTANCE: f32 = 1.5;

/// The cursor over an instance whose interaction names none, or that offers several.
pub const DEFAULT_INTERACTION_CURSOR: &str = "interact";

/// Something that can be done to an instance, as written in its exemplar. Not deserialized by
/// serde, so that loaders reach its formula through reflection.
#[derive(Debug, Reflect, Clone, Default, Serialize)]
#[reflect(Default, Serialize)]
pub struct InteractionDesc {
    /// Names the interaction. Entries may share one, each with its own `when`.
    pub id: String,

    /// Shown to the player while the interaction is available.
    pub caption: String,

    /// Name of the mouse cursor to show.
    pub cursor: Option<String>,

    /// Name of the icon to show.
    pub icon: Option<String>,

    /// How close the interaction can be performed from, in meters.
    pub distance: Option<f32>,

    /// Formula deciding whether the interaction is offered. `None` always offers it.
    pub when: Option<Formula<bool>>,

    /// Name of the action to perform, registered with
    /// [`InteractionAppExt::register_interaction_action`].
    pub action: String,
}

impl InteractionDesc {
    /// Whether the interaction is offered now on `entity`, by its `when`. A `when` that did not
    /// compile, or whose evaluation fails, offers nothing.
    pub fn enabled(&self, world: &World, host: &HostState, entity: Entity) -> bool {
        self.when
            .as_ref()
            .is_none_or(|when| when.evaluate(world, host, entity).unwrap_or(false))
    }

    /// How close the interaction can be performed from, in meters.
    pub fn reach(&self) -> f32 {
        self.distance.unwrap_or(DEFAULT_INTERACTION_DISTANCE)
    }
}

/// What can be done to an instance. An exemplar's list replaces any it inherits.
#[derive(Component, Debug, Reflect, Clone, Default, Serialize)]
#[reflect(Aspect, Default, Serialize)]
pub struct Interactions(pub Vec<InteractionDesc>);

impl Interactions {
    /// The first entry `entity` offers now that reaches `distance`.
    pub fn first_enabled(
        &self,
        distance: f32,
        world: &World,
        host: &HostState,
        entity: Entity,
    ) -> Option<&InteractionDesc> {
        self.0
            .iter()
            .find(|entry| distance <= entry.reach() && entry.enabled(world, host, entity))
    }

    /// The name of the cursor to show over `entity`, however far away it is, if it offers
    /// anything now.
    pub fn hover_cursor(&self, world: &World, host: &HostState, entity: Entity) -> Option<&str> {
        let mut enabled = self
            .0
            .iter()
            .filter(|entry| entry.enabled(world, host, entity));
        let first = enabled.next()?;
        Some(match enabled.next() {
            Some(_) => DEFAULT_INTERACTION_CURSOR,
            None => first
                .cursor
                .as_deref()
                .unwrap_or(DEFAULT_INTERACTION_CURSOR),
        })
    }
}

impl Aspect for Interactions {
    fn name(&self) -> &str {
        "Interactions"
    }

    fn can_attach(&self, _meta_type: InstanceType) -> bool {
        true
    }

    fn attach(&self, entity: &mut EntityWorldMut) -> &'static dyn DetachAspect {
        static DETACH: RemoveComponent<Interactions> = RemoveComponent::<Interactions>::new();
        entity.insert(self.clone());
        &DETACH
    }

    fn clone_boxed(&self) -> Box<dyn Aspect> {
        Box::new(self.clone())
    }
}

#[derive(Clone, Copy)]
struct InteractionAction {
    aspect: TypeId,
    trigger: fn(&mut World, Entity),
}

/// The actions an [`InteractionDesc`] can name.
#[derive(Resource, Default)]
pub struct InteractionActions(HashMap<String, InteractionAction>);

/// Registers actions for [`InteractionDesc`]s to name.
pub trait InteractionAppExt {
    /// Registers the action `name`, which `trigger` performs, normally by triggering an entity
    /// event. It can only be performed on an entity with the aspect component `A`.
    fn register_interaction_action<A: Component>(
        &mut self,
        name: &str,
        trigger: fn(&mut World, Entity),
    ) -> &mut Self;
}

impl InteractionAppExt for App {
    fn register_interaction_action<A: Component>(
        &mut self,
        name: &str,
        trigger: fn(&mut World, Entity),
    ) -> &mut Self {
        self.world_mut()
            .get_resource_or_init::<InteractionActions>()
            .0
            .insert(
                name.to_owned(),
                InteractionAction {
                    aspect: TypeId::of::<A>(),
                    trigger,
                },
            );
        self
    }
}

/// Performs the action `name` on `target`. An unknown action, or a target without the aspect the
/// action needs, is an error.
pub fn perform_interaction(world: &mut World, target: Entity, name: &str) {
    let Some(action) = world.resource::<InteractionActions>().0.get(name).copied() else {
        error!("Unknown interaction action '{name}'");
        return;
    };
    let Ok(entity) = world.get_entity(target) else {
        return;
    };
    if !entity.contains_type_id(action.aspect) {
        error!("{target} lacks the aspect that interaction action '{name}' needs");
        return;
    }
    (action.trigger)(world, target);
}

#[cfg(test)]
pub(crate) mod tests {
    use bevy::reflect::{serde::TypedReflectDeserializer, FromReflect, GetTypeRegistration};
    use panoply_saga::{FormulaQueue, FormulaRef};
    use serde::de::DeserializeSeed;

    use super::*;

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

    /// `Interactions` read from `json` as a loader reads them, compiled against `host`, with the
    /// formulas read.
    pub(crate) fn load_interactions(
        json: &str,
        host: &HostState,
    ) -> (Interactions, Vec<FormulaRef>) {
        let mut registry = bevy::reflect::TypeRegistry::new();
        registry.register::<Interactions>();
        let registration = registry
            .get(Interactions::get_type_registration().type_id())
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
        futures_lite::future::block_on(formulas.compile("test.json", host, json.as_bytes()));
        (Interactions::from_reflect(&*value).unwrap(), read)
    }

    fn entry(id: &str, distance: f32, when: Option<&str>) -> InteractionDesc {
        InteractionDesc {
            id: id.to_owned(),
            distance: Some(distance),
            when: when.map(Formula::uncompiled),
            ..default()
        }
    }

    #[test]
    fn no_when_is_enabled_and_an_uncompiled_when_is_not() {
        let (world, host) = (World::new(), HostState::new());
        let e = Entity::PLACEHOLDER;
        assert!(entry("a", 1., None).enabled(&world, &host, e));
        assert!(!entry("a", 1., Some("true")).enabled(&world, &host, e));
    }

    #[test]
    fn when_is_evaluated_on_the_entity() {
        let host = lid_host();
        let (list, _) = load_interactions(
            r#"[
  {"id": "open", "caption": "Open", "when": "!self.lid.open", "action": "a"},
  {"id": "close", "caption": "Close", "when": "self.lid.open", "action": "a"}
]"#,
            &host,
        );
        let mut world = World::new();
        let shut = world.spawn(Lid { open: false }).id();
        let open = world.spawn(Lid { open: true }).id();
        let lidless = world.spawn(()).id();
        let first = |e| {
            list.first_enabled(1., &world, &host, e)
                .map(|d| d.id.as_str())
        };
        assert_eq!(first(shut), Some("open"));
        assert_eq!(first(open), Some("close"));
        // Without the component, the evaluation fails and nothing is offered.
        assert_eq!(first(lidless), None);
    }

    #[test]
    fn first_enabled_in_reach_wins() {
        let (world, host) = (World::new(), HostState::new());
        let e = Entity::PLACEHOLDER;
        let list = Interactions(vec![
            entry("formula", 2., Some("true")),
            entry("near", 0.5, None),
            entry("far", 2., None),
            entry("later", 2., None),
        ]);
        let first = |distance| list.first_enabled(distance, &world, &host, e);
        assert_eq!(first(0.4).unwrap().id, "near");
        assert_eq!(first(1.).unwrap().id, "far");
        assert!(first(3.).is_none());
    }

    #[test]
    fn hover_cursor_is_the_only_entrys_or_the_default() {
        let chest = InteractionDesc {
            cursor: Some("open-chest".to_owned()),
            ..entry("open", 1., None)
        };
        let disabled = entry("formula", 1., Some("true"));
        let (world, host) = (World::new(), HostState::new());
        let cursor = |list: Vec<InteractionDesc>| {
            Interactions(list)
                .hover_cursor(&world, &host, Entity::PLACEHOLDER)
                .map(str::to_owned)
        };
        assert_eq!(
            cursor(vec![chest.clone(), disabled.clone()]).as_deref(),
            Some("open-chest")
        );
        assert_eq!(
            cursor(vec![entry("a", 1., None)]).as_deref(),
            Some(DEFAULT_INTERACTION_CURSOR)
        );
        assert_eq!(
            cursor(vec![chest, entry("a", 1., None)]).as_deref(),
            Some(DEFAULT_INTERACTION_CURSOR)
        );
        assert_eq!(cursor(vec![disabled]), None);
    }
}
