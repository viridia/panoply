use bevy::{asset::LoadContext, reflect::TypeRegistry};
use serde::{
    de::{self, DeserializeSeed, Visitor},
    Deserializer,
};
use std::fmt;

use super::{aspect::Aspect, AspectDeserializer};

struct AspectListVisitor<'a, 'b> {
    type_registry: &'a TypeRegistry,
    load_context: &'a mut LoadContext<'b>,
    parent_label: &'a str,
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
            aspect.load_dependencies(self.parent_label, self.load_context);
            result.push(aspect);
        }
        Ok(result)
    }
}

pub(crate) struct AspectListDeserializer<'a, 'b> {
    pub(crate) type_registry: &'a TypeRegistry,
    pub(crate) load_context: &'a mut LoadContext<'b>,
    pub(crate) parent_label: &'a str,
}

// impl<'a, 'b> AspectListDeserializer<'a, 'b> {
//     pub(crate) fn new(
//         type_registry: &'a TypeRegistry,
//         load_context: &'a mut LoadContext,
//         schematic_name: &'a str,
//     ) -> Self {
//         AspectListDeserializer {
//             type_registry,
//             load_context,
//             parent_label: schematic_name,
//         }
//     }
// }

impl<'de, 'a, 'b> DeserializeSeed<'de> for AspectListDeserializer<'a, 'b> {
    type Value = Vec<Box<dyn Aspect>>;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_map(AspectListVisitor {
            type_registry: self.type_registry,
            load_context: self.load_context,
            parent_label: self.parent_label,
        })
    }
}
