use bevy::{
    asset::LoadContext,
    prelude::*,
    reflect::{
        serde::{TypedReflectDeserializer, TypedReflectSerializer},
        TypeRegistration, TypeRegistry,
    },
};
use panoply_saga::FormulaQueue;
use serde::{de::DeserializeSeed, ser::SerializeMap, Deserializer, Serialize};
use std::{
    fmt::{self, Debug},
    sync::Arc,
};

use crate::InstanceType;

/// An Aspect is like an ECS component for a prototype. It is native, a Rust type implementing
/// [`NativeAspect`], or scripted, declared in SAGA; both are keyed by [`Aspect::key`] (D107).
#[reflect_trait]
pub trait Aspect: Reflect
where
    Self: 'static + Sync + Send,
{
    /// The aspect key, which the files, the editor and an entity's [`OwnedAspects`] know it by: a
    /// native aspect's short type path, a scripted one's package path and name.
    fn key(&self) -> &str;

    /// Whether this aspect can be applied/attached to an instance of the given type.
    fn can_attach(&self, meta_type: InstanceType) -> bool;

    /// Load any dependencies required by this aspect.
    #[allow(unused_variables)]
    fn load_dependencies(&mut self, label: &str, load_context: &mut LoadContext) {}

    /// Attach or apply this aspect to the given entity.
    fn attach(&self, entity: &mut EntityWorldMut);

    /// Remove from the given entity what [`attach`](Aspect::attach) inserted, its state aside.
    fn detach(&self, entity: &mut EntityWorldMut);

    /// Clone this aspect as a boxed trait object.
    fn clone_boxed(&self) -> Box<dyn Aspect>;

    /// Whether an instance with this aspect has state: what play changes and a saved game keeps,
    /// such as a container's contents, as against the aspect's config, which is authored (D88).
    /// Such an instance needs an external id for a saved game to find it by (D87).
    fn has_state(&self) -> bool {
        false
    }

    /// The aspect as a file holds it, the value of its entry under its key. A native aspect is
    /// serialized through reflection; a scripted one writes its fields by name.
    fn to_json(&self, registry: &TypeRegistry) -> Result<serde_json::Value, BevyError> {
        crate::save::reflect_json(self.as_partial_reflect(), registry)
    }

    //   /** Formulas to be bound to the specified properties */
    //   formulas?: IFormulaGenerators<SelfType, Props, Config>;

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

/// An aspect that is a Rust type. Its config is a [`Bundle`], which attaching inserts and
/// detaching removes; the [`Aspect`] it gets from this is the typed layer behind the trait object.
pub trait NativeAspect: Reflect + TypePath + Clone
where
    Self: 'static + Sync + Send,
{
    /// The components attaching inserts: usually the aspect itself, several where systems want
    /// them apart, `()` where nothing queries it.
    type Config: Bundle;

    /// The config to insert.
    fn config(&self) -> Self::Config;

    /// Insert the aspect's state where the entity lacks it, so reapplying leaves it as it was
    /// (D74). Detaching leaves it.
    #[allow(unused_variables)]
    fn attach_state(&self, entity: &mut EntityWorldMut) {}

    /// See [`Aspect::can_attach`].
    fn can_attach(&self, meta_type: InstanceType) -> bool;

    /// See [`Aspect::load_dependencies`].
    #[allow(unused_variables)]
    fn load_dependencies(&mut self, label: &str, load_context: &mut LoadContext) {}

    /// See [`Aspect::has_state`].
    fn has_state(&self) -> bool {
        false
    }
}

impl<T: NativeAspect> Aspect for T {
    fn key(&self) -> &str {
        T::short_type_path()
    }

    fn can_attach(&self, meta_type: InstanceType) -> bool {
        NativeAspect::can_attach(self, meta_type)
    }

    fn load_dependencies(&mut self, label: &str, load_context: &mut LoadContext) {
        NativeAspect::load_dependencies(self, label, load_context);
    }

    fn attach(&self, entity: &mut EntityWorldMut) {
        entity.insert(self.config());
        self.attach_state(entity);
    }

    fn detach(&self, entity: &mut EntityWorldMut) {
        entity.remove::<T::Config>();
    }

    fn clone_boxed(&self) -> Box<dyn Aspect> {
        Box::new(self.clone())
    }

    fn has_state(&self) -> bool {
        NativeAspect::has_state(self)
    }
}

/// The aspects attached to this entity, each as its resolved definition, in the order they were
/// resolved: the instance's own, then its exemplar's, nearest first. Gathering like data from
/// aspects of different kinds iterates them (D107).
#[derive(Component)]
pub struct OwnedAspects(pub(crate) Vec<Arc<dyn Aspect>>);

impl OwnedAspects {
    /// Each attached aspect's definition.
    pub fn iter(&self) -> impl Iterator<Item = &dyn Aspect> {
        self.0.iter().map(|aspect| aspect.as_ref())
    }

    /// The definition of the attached aspect whose key is `key`.
    pub fn get(&self, key: &str) -> Option<&dyn Aspect> {
        self.iter().find(|aspect| aspect.key() == key)
    }
}

/// `aspect`, to change: shared, it is copied first, since an aspect's definition is never
/// changed under those holding it.
pub fn aspect_mut(aspect: &mut Arc<dyn Aspect>) -> &mut dyn Aspect {
    if Arc::get_mut(aspect).is_none() {
        *aspect = Arc::from(aspect.clone_boxed());
    }
    Arc::get_mut(aspect).expect("a fresh copy is not shared")
}

pub(crate) struct AspectDeserializer<'a> {
    pub(crate) type_registration: &'a TypeRegistration,
    pub(crate) type_registry: &'a TypeRegistry,
    pub(crate) formulas: &'a mut FormulaQueue,
}

impl<'de> DeserializeSeed<'de> for AspectDeserializer<'_> {
    type Value = Box<dyn Aspect>;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        let reflect_deserializer = TypedReflectDeserializer::with_processor(
            self.type_registration,
            self.type_registry,
            self.formulas,
        );
        let deserialized_value: Box<dyn PartialReflect> =
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
#[derive(Component, Default, Clone)]
pub struct InstanceAspects(pub Vec<Arc<dyn Aspect>>);

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
            write!(f, "{:?}", aspect.key())?;
        }
        Ok(())
    }
}

impl InstanceAspects {
    /// The aspects as a file holds them, a map from each aspect's key to its fields,
    /// the form [`AspectListDeserializer`](crate::AspectListDeserializer) reads.
    pub fn serializable<'a>(&'a self, registry: &'a TypeRegistry) -> SerializableAspects<'a> {
        SerializableAspects {
            aspects: &self.0,
            registry,
        }
    }
}

/// Serializes a list of aspects through reflection; see [`InstanceAspects::serializable`].
pub struct SerializableAspects<'a> {
    aspects: &'a [Arc<dyn Aspect>],
    registry: &'a TypeRegistry,
}

impl Serialize for SerializableAspects<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        let mut map = serializer.serialize_map(Some(self.aspects.len()))?;
        for aspect in self.aspects {
            // A native aspect is written straight through reflection, in the file's own format;
            // only a scripted one, which reflection cannot see into, goes through its JSON.
            if aspect.as_any().is::<crate::ScriptedAspect>() {
                let value = aspect
                    .to_json(self.registry)
                    .map_err(serde::ser::Error::custom)?;
                map.serialize_entry(aspect.key(), &value)?;
            } else {
                map.serialize_entry(
                    aspect.key(),
                    &TypedReflectSerializer::new(aspect.as_partial_reflect(), self.registry),
                )?;
            }
        }
        map.end()
    }
}
