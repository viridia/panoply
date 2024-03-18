use bevy::asset::Handle;
use bevy::math::Vec3;
use bevy::{asset::LoadContext, reflect::TypeRegistry};
use panoply_exemplar::{AspectListDeserializer, Exemplar, InstanceAspects};
use serde::{de, Deserialize};
use serde::{
    de::{DeserializeSeed, Visitor},
    Serialize,
};
use std::fmt::{self, Debug};

/// Serialized instance of an actor.
#[derive(Debug, Clone, Default)]
pub struct ActorInstance {
    /// Exemplar id for this actor
    pub(crate) exemplar: Handle<Exemplar>,

    /// Only for globally-defined actors.
    pub(crate) realm: Option<String>,

    /// Position within the realm (world coordinates)
    pub(crate) position: Vec3,

    /// Actor facing direction, in degrees
    pub(crate) facing: f32,

    /// Optional instance identifier
    pub(crate) iid: Option<String>,

    /// List of aspects for this instance.
    pub(crate) aspects: InstanceAspects,
    // These should probably be aspects.
    // readonly layer?: string;
    // readonly groupId?: string;
    // readonly transient?: boolean;
    // readonly ally?: string;
}

impl Serialize for ActorInstance {
    fn serialize<S>(&self, _serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        // use serde::ser::SerializeSeq;
        todo!("Implement ActorInstance serialization")

        // let mut seq = serializer.serialize_seq(Some(5))?;
        // seq.serialize_element(&self.exemplar)?;
        // seq.serialize_element(&self.realm)?;
        // seq.serialize_element(&self.position)?;
        // seq.serialize_element(&self.facing)?;
        // seq.serialize_element(&self.iid)?;
        // seq.end()
    }
}

#[derive(Deserialize)]
#[serde(field_identifier, rename_all = "snake_case")]
enum Field {
    Exemplar,
    Realm,
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

impl<'de, 'a, 'b> Visitor<'de> for ActorInstanceVisitor<'a, 'b> {
    type Value = ActorInstance;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("an actor instance")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::MapAccess<'de>,
    {
        let mut result = ActorInstance::default();
        while let Some(key) = map.next_key()? {
            match key {
                Field::Exemplar => {
                    let exemplar: String = map.next_value()?;
                    // if !result.exemplar.is_empty() {
                    //     return Err(de::Error::duplicate_field("exemplar"));
                    // }
                    result.exemplar = self.load_context.load(exemplar);
                }
                Field::Realm => {
                    if result.realm.is_some() {
                        return Err(de::Error::duplicate_field("realm"));
                    }
                    result.realm = Some(map.next_value()?);
                }
                Field::Position => {
                    if result.realm.is_some() {
                        return Err(de::Error::duplicate_field("position"));
                    }
                    result.position = map.next_value()?;
                }
                Field::Facing => {
                    if result.facing != f32::default() {
                        return Err(de::Error::duplicate_field("facing"));
                    }
                    result.facing = map.next_value()?;
                }
                Field::Iid => {
                    if result.iid.is_some() {
                        return Err(de::Error::duplicate_field("iid"));
                    }
                    result.iid = Some(map.next_value()?);
                }
                Field::Aspects => {
                    if !result.aspects.is_empty() {
                        return Err(de::Error::duplicate_field("aspects"));
                    }
                    result.aspects =
                        InstanceAspects(map.next_value_seed(AspectListDeserializer {
                            type_registry: self.type_registry,
                            load_context: self.load_context,
                            label_prefix: self.parent_label,
                        })?);
                }
            }
        }
        Ok(result)
    }
}

pub struct ActorInstanceDeserializer<'a, 'b> {
    type_registry: &'a TypeRegistry,
    load_context: &'a mut LoadContext<'b>,
    parent_label: &'a str,
}

impl<'de, 'a, 'b> DeserializeSeed<'de> for ActorInstanceDeserializer<'a, 'b> {
    type Value = ActorInstance;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_seq(ActorInstanceVisitor {
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

impl<'de, 'a, 'b> Visitor<'de> for ActorInstanceListVisitor<'a, 'b> {
    type Value = Vec<ActorInstance>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a list of actor instances")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        let mut result: Vec<ActorInstance> = Vec::with_capacity(seq.size_hint().unwrap_or(0));
        while let Some(compressed_instance) = seq.next_element_seed(ActorInstanceDeserializer {
            type_registry: self.type_registry,
            load_context: self.load_context,
            parent_label: self.parent_label,
        })? {
            result.push(compressed_instance);
        }
        Ok(result)
    }
}

pub struct ActorInstanceListDeserializer<'a, 'b> {
    pub(crate) type_registry: &'a TypeRegistry,
    pub(crate) load_context: &'a mut LoadContext<'b>,
    pub(crate) parent_label: &'a str,
}

impl<'de, 'a, 'b> DeserializeSeed<'de> for ActorInstanceListDeserializer<'a, 'b> {
    type Value = Vec<ActorInstance>;

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
