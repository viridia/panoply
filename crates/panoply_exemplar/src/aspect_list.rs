use bevy::{asset::LoadContext, reflect::TypeRegistry};
use serde::{
    de::{self, DeserializeSeed, Visitor},
    Deserializer,
};
use std::fmt;

/// A list of aspects.
pub type AspectList = Vec<Box<dyn Aspect>>;

use super::{aspect::Aspect, AspectDeserializer};

struct AspectListVisitor<'a, 'b> {
    type_registry: &'a TypeRegistry,
    load_context: &'a mut LoadContext<'b>,
    label_prefix: &'a str,
}

impl<'de, 'a, 'b> Visitor<'de> for AspectListVisitor<'a, 'b> {
    type Value = Vec<Box<dyn Aspect>>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("an aspect map")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::MapAccess<'de>,
    {
        let mut result: Vec<Box<dyn Aspect>> = Vec::with_capacity(map.size_hint().unwrap_or(0));
        while let Some(key) = map.next_key::<String>()? {
            let type_registration = self
                .type_registry
                .get_with_short_type_path(&key)
                .ok_or_else(|| de::Error::custom(format!("Unknown aspect type: {}", key)))?;
            let mut aspect = map.next_value_seed(AspectDeserializer {
                type_registration,
                type_registry: self.type_registry,
            })?;
            aspect.load_dependencies(self.label_prefix, self.load_context);
            result.push(aspect);
        }
        Ok(result)
    }
}

/// A deserializer for a vector of boxed aspects.
pub struct AspectListDeserializer<'a, 'b> {
    /// Reference to the type registry.
    pub type_registry: &'a TypeRegistry,

    /// Reference to the asset loader context.
    pub load_context: &'a mut LoadContext<'b>,

    /// Prefix for created materials
    pub label_prefix: &'a str,
}

impl<'de, 'a, 'b> DeserializeSeed<'de> for AspectListDeserializer<'a, 'b> {
    type Value = Vec<Box<dyn Aspect>>;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_map(AspectListVisitor {
            type_registry: self.type_registry,
            load_context: self.load_context,
            label_prefix: self.label_prefix,
        })
    }
}
