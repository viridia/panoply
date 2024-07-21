use bevy::{
    asset::LoadContext,
    prelude::*,
    reflect::{serde::TypedReflectDeserializer, TypeRegistration, TypeRegistry},
    utils::HashMap,
};
use serde::{de::DeserializeSeed, ser::SerializeMap, Deserializer, Serialize};
use std::{
    any::TypeId,
    fmt::{self, Debug},
};

use crate::InstanceType;

/// An Aspect is like an ECS component for a prototype.
#[reflect_trait]
pub trait Aspect: Reflect
where
    Self: 'static + Sync + Send,
{
    /// Type name of this aspect
    fn name(&self) -> &str;

    /// Get the [`TypeId`] for this aspect.
    fn id(&self) -> TypeId {
        std::any::TypeId::of::<Self>()
    }

    /// Whether this aspect can be applied/attached to an instance of the given type.
    fn can_attach(&self, meta_type: InstanceType) -> bool;

    /// Load any dependencies required by this aspect.
    #[allow(unused_variables)]
    fn load_dependencies(&mut self, label: &str, load_context: &mut LoadContext) {}

    /// Attach or apply this aspect to the given entity.
    fn attach(&self, entity: &mut EntityWorldMut) -> &'static dyn DetachAspect;

    /// Clone this aspect as a boxed trait object.
    fn clone_boxed(&self) -> Box<dyn Aspect>;

    //   /** Formulas to be bound to the specified properties */
    //   formulas?: IFormulaGenerators<SelfType, Props, Config>;

    //   /** Ways in which the player can interact with this instance. */
    //   interactions?: IInteraction<SelfType, Props, Config>[];

    //   /** List of quest roles associated with this actor. */
    //   questRoles?: string[];

    //   /** A task which runs when instance is instantiated and is run in the instance scope.
    //       This scope will get destroyed when the instance is unloaded.
    //    */
    //   init?: (self: SelfType, props: Props, config: Config) => void;

    //   /** Generates list of goals to be associated with this behavior (actors only) */
    //   goals?: (self: SelfType, props: Props) => GoalChildren;

    //   /** Method invoked when the actor is not doing anything. Returns the type of animation
    //       that should be run when idling.
    //    */
    //   idle?: (self: SelfType, props: Props) => string | undefined;

    //   /** Predicate function for sensor instances. */
    //   canSense?: (self: SelfType, props: Props, target: PositionableInstance) => boolean;

    //   /** If defined, this actor is conditional - only appears in specific quest stages. */
    //   present?: (self: SelfType, props: Props) => boolean;
}

/// Tracks the aspects currently attached to this entity.
#[derive(Component)]
pub struct OwnedAspects(pub(crate) HashMap<TypeId, &'static dyn DetachAspect>);

/// Object which can remove an aspect from an entity.
pub trait DetachAspect: Send + Sync {
    /// Get the [`TypeId`] for this aspect.
    fn type_id(&self) -> TypeId;

    /// Remove the aspect from the entity.
    fn detach_aspect(&self, entity: &mut EntityWorldMut);
}

/// An `DetachAspect` that removes a specific component from an entity.
pub struct RemoveComponent<T: Component> {
    _marker: std::marker::PhantomData<T>,
}

impl<T: Component> RemoveComponent<T> {
    /// Create a new `RemoveComponent` for the given component type.
    pub const fn new() -> Self {
        RemoveComponent {
            _marker: std::marker::PhantomData,
        }
    }
}

impl<T: Component> Default for RemoveComponent<T> {
    fn default() -> Self {
        Self {
            _marker: Default::default(),
        }
    }
}

impl<T: Component> DetachAspect for RemoveComponent<T> {
    fn type_id(&self) -> TypeId {
        TypeId::of::<T>()
    }

    fn detach_aspect(&self, entity: &mut EntityWorldMut) {
        entity.remove::<T>();
    }
}

pub(crate) struct AspectDeserializer<'a> {
    pub(crate) type_registration: &'a TypeRegistration,
    pub(crate) type_registry: &'a TypeRegistry,
}

impl<'a, 'de> DeserializeSeed<'de> for AspectDeserializer<'a> {
    type Value = Box<dyn Aspect>;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        let reflect_deserializer =
            TypedReflectDeserializer::new(self.type_registration, self.type_registry);
        let deserialized_value: Box<dyn Reflect> =
            match reflect_deserializer.deserialize(deserializer) {
                Ok(value) => value,
                Err(err) => {
                    error!(
                        "Error deserializing aspect: {} {:?}",
                        self.type_registration.type_info().type_path(),
                        err
                    );
                    return Err(err);
                }
            };
        let rd = self.type_registration.data::<ReflectDefault>().unwrap();
        let mut value = rd.default();
        value.apply(&*deserialized_value);
        let reflect_aspect = self
            .type_registry
            .get_type_data::<ReflectAspect>(self.type_registration.type_id())
            .unwrap();
        let aspect = reflect_aspect.get_boxed(value).unwrap();
        Ok(aspect)
    }
}

/// A list of aspects associated with a specific instance, rather than an exemplar.
#[derive(Component, Default)]
pub struct InstanceAspects(pub Vec<Box<dyn Aspect>>);

impl InstanceAspects {
    /// True if there are no aspects owned by this entity. Note that "owned" is different from
    /// "attached": The former represents aspects that have not been instantiated as ECS components
    /// but which are deserialized along with the entity. "Attached" aspects are ones that have
    /// been instantiated as ECS components, and can either be owned or copied from the exemplar.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl Debug for InstanceAspects {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for aspect in self.0.iter() {
            write!(f, "{:?}", aspect.as_reflect().type_id())?;
        }
        Ok(())
    }
}

impl Serialize for InstanceAspects {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        let map = serializer.serialize_map(Some(self.0.len()))?;
        for _aspect in self.0.iter() {
            todo!();
            // map.serialize_entry(aspect.into_reflect().type_id(), aspect)?;
        }
        map.end()
    }
}

impl Clone for InstanceAspects {
    fn clone(&self) -> Self {
        let mut aspects = Vec::with_capacity(self.0.len());
        for aspect in self.0.iter() {
            aspects.push(aspect.clone_boxed());
        }
        InstanceAspects(aspects)
    }
}
