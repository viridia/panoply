use std::any::TypeId;

use bevy::{platform::collections::HashMap, prelude::*};
use panoply_saga::Formula;
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
    /// Whether the interaction is offered now. Formulas are not evaluated yet, so an entry that
    /// has one is an error and is not offered.
    pub fn enabled(&self) -> bool {
        // TODO(PAN45c): evaluate the formula.
        if self.when.is_some() {
            error_once!(
                "Interaction '{}' has a `when`, which is not evaluated yet",
                self.id
            );
            return false;
        }
        true
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
    /// The first entry offered now that reaches `distance`.
    pub fn first_enabled(&self, distance: f32) -> Option<&InteractionDesc> {
        self.0
            .iter()
            .find(|entry| distance <= entry.reach() && entry.enabled())
    }

    /// The name of the cursor to show over the instance, however far away it is, if it offers
    /// anything now.
    pub fn hover_cursor(&self) -> Option<&str> {
        let mut enabled = self.0.iter().filter(|entry| entry.enabled());
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
mod tests {
    use super::*;

    fn entry(id: &str, distance: f32, when: Option<&str>) -> InteractionDesc {
        InteractionDesc {
            id: id.to_owned(),
            distance: Some(distance),
            when: when.map(Formula::uncompiled),
            ..default()
        }
    }

    #[test]
    fn no_when_is_enabled_and_a_when_is_not() {
        assert!(entry("a", 1., None).enabled());
        assert!(!entry("a", 1., Some("self.container.open")).enabled());
    }

    #[test]
    fn first_enabled_in_reach_wins() {
        let list = Interactions(vec![
            entry("formula", 2., Some("true")),
            entry("near", 0.5, None),
            entry("far", 2., None),
            entry("later", 2., None),
        ]);
        assert_eq!(list.first_enabled(0.4).unwrap().id, "near");
        assert_eq!(list.first_enabled(1.).unwrap().id, "far");
        assert!(list.first_enabled(3.).is_none());
    }

    #[test]
    fn hover_cursor_is_the_only_entrys_or_the_default() {
        let chest = InteractionDesc {
            cursor: Some("open-chest".to_owned()),
            ..entry("open", 1., None)
        };
        let disabled = entry("formula", 1., Some("true"));
        assert_eq!(
            Interactions(vec![chest.clone(), disabled.clone()]).hover_cursor(),
            Some("open-chest")
        );
        assert_eq!(
            Interactions(vec![entry("a", 1., None)]).hover_cursor(),
            Some(DEFAULT_INTERACTION_CURSOR)
        );
        assert_eq!(
            Interactions(vec![chest, entry("a", 1., None)]).hover_cursor(),
            Some(DEFAULT_INTERACTION_CURSOR)
        );
        assert_eq!(Interactions(vec![disabled]).hover_cursor(), None);
    }
}
