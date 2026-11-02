use bevy::{asset::LoadContext, math::Vec3, reflect::TypeRegistry};
use panoply_exemplar::{AspectListDeserializer, InstanceAspects};
use serde::{
    de::{self, DeserializeSeed, Visitor},
    ser::SerializeMap,
    Deserialize, Serialize,
};
use std::fmt;

use crate::SceneryInstanceId;

/// An actor a precinct places (D24). The exemplar is a path rather than a handle loaded with the
/// precinct, as scenery's type tables are, so a missing exemplar cannot hold up the precinct's
/// load.
#[derive(Debug, Default, Clone)]
pub struct ActorInstanceData {
    /// The actor's exemplar, as a `file#Label` path.
    pub exemplar: String,

    /// Where the actor stands, in world coordinates.
    pub position: Vec3,

    /// The way the actor faces, in degrees: 0 faces +z, 90 faces +x.
    pub facing: f32,

    /// Identifies the actor within its precinct.
    pub iid: SceneryInstanceId,

    /// Aspects of this actor that override its exemplar's.
    pub aspects: InstanceAspects,
}

impl Serialize for ActorInstanceData {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut map = serializer.serialize_map(None)?;
        map.serialize_entry("exemplar", &self.exemplar)?;
        map.serialize_entry("position", &self.position)?;
        if self.facing != 0.0 {
            map.serialize_entry("facing", &self.facing)?;
        }
        if self.iid != SceneryInstanceId::None {
            map.serialize_entry("iid", &self.iid)?;
        }
        if !self.aspects.is_empty() {
            map.serialize_entry("aspects", &self.aspects)?;
        }
        map.end()
    }
}

#[derive(Deserialize)]
#[serde(field_identifier, rename_all = "snake_case")]
enum Field {
    Exemplar,
    Position,
    Facing,
    Iid,
    Aspects,
}

struct ActorInstanceVisitor<'a, 'b> {
    type_registry: &'a TypeRegistry,
    load_context: &'a mut LoadContext<'b>,
    parent_label: &'a str,
}

impl<'de> Visitor<'de> for ActorInstanceVisitor<'_, '_> {
    type Value = ActorInstanceData;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("an actor instance")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: de::MapAccess<'de>,
    {
        let mut result = ActorInstanceData::default();
        let mut exemplar = None;
        while let Some(key) = map.next_key()? {
            match key {
                Field::Exemplar => exemplar = Some(map.next_value()?),
                Field::Position => result.position = map.next_value()?,
                Field::Facing => result.facing = map.next_value()?,
                Field::Iid => result.iid = map.next_value()?,
                Field::Aspects => {
                    result.aspects =
                        InstanceAspects(map.next_value_seed(AspectListDeserializer {
                            type_registry: self.type_registry,
                            load_context: self.load_context,
                            label_prefix: self.parent_label,
                        })?);
                }
            }
        }
        result.exemplar = exemplar.ok_or_else(|| de::Error::missing_field("exemplar"))?;
        Ok(result)
    }
}

struct ActorInstanceDeserializer<'a, 'b> {
    type_registry: &'a TypeRegistry,
    load_context: &'a mut LoadContext<'b>,
    parent_label: &'a str,
}

impl<'de> DeserializeSeed<'de> for ActorInstanceDeserializer<'_, '_> {
    type Value = ActorInstanceData;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_map(ActorInstanceVisitor {
            type_registry: self.type_registry,
            load_context: self.load_context,
            parent_label: self.parent_label,
        })
    }
}

struct ActorInstanceListVisitor<'a, 'b> {
    type_registry: &'a TypeRegistry,
    load_context: &'a mut LoadContext<'b>,
    parent_label: &'a str,
}

impl<'de> Visitor<'de> for ActorInstanceListVisitor<'_, '_> {
    type Value = Vec<ActorInstanceData>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a list of actor instances")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: de::SeqAccess<'de>,
    {
        let mut result = Vec::with_capacity(seq.size_hint().unwrap_or(0));
        while let Some(instance) = seq.next_element_seed(ActorInstanceDeserializer {
            type_registry: self.type_registry,
            load_context: self.load_context,
            parent_label: self.parent_label,
        })? {
            result.push(instance);
        }
        Ok(result)
    }
}

pub(crate) struct ActorInstanceListDeserializer<'a, 'b> {
    pub(crate) type_registry: &'a TypeRegistry,
    pub(crate) load_context: &'a mut LoadContext<'b>,
    pub(crate) parent_label: &'a str,
}

impl<'de> DeserializeSeed<'de> for ActorInstanceListDeserializer<'_, '_> {
    type Value = Vec<ActorInstanceData>;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_seq(ActorInstanceListVisitor {
            type_registry: self.type_registry,
            load_context: self.load_context,
            parent_label: self.parent_label,
        })
    }
}
