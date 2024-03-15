use bevy::{
    asset::{io::Reader, AssetLoader, LoadContext},
    prelude::*,
    reflect::{TypeRegistry, TypeRegistryArc},
    utils::{thiserror::Error, BoxedFuture},
};
use futures_lite::AsyncReadExt;
use panoply_exemplar::{AspectListDeserializer, InstanceAspects};
use serde::{
    de::{DeserializeSeed, Visitor},
    ser::SerializeTuple,
    Deserialize, Serialize,
};
use std::fmt::{self, Debug};

use crate::msgpack::{Box2d, Vector3};

use super::floor_region::FloorRegionSer;

extern crate rmp_serde as rmps;

/// TODO: use serialize_if not empty
#[derive(TypePath, Asset, Serialize, Debug, Default)]
pub struct PrecinctAsset {
    /// Table of wall archetypes used by this precinct.
    pub(crate) scenery_types: Vec<String>,

    /// Table of floor archetypes used by this precinct.
    pub(crate) floor_types: Vec<String>,

    /// Table of terrain effect archetypes used by this precinct.
    #[serde(default)]
    pub(crate) terrain_fx_types: Vec<String>,

    /// Table of floors, spaced 1 meter apart.
    #[serde(default)]
    pub(crate) tiers: Vec<TierSer>,

    /// Packed terrain effect table
    pub(crate) terrain_fx: Option<Vec<i16>>,

    // actors: Option<Vec<IActorInstanceData>>,
    /// Table of scenery instances.
    #[serde(default)]
    pub(crate) scenery: Vec<CompressedInstance>,
    // layers?: Record<string, ILayerData>,
}

/** Serialized schema for a tier */
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TierSer {
    pub(crate) level: i32,

    #[serde(default)]
    pub(crate) pfloors: Vec<FloorRegionSer>,

    pub(crate) cutaways: Option<Vec<Box2d>>,
}

#[derive(Debug, Default)]
pub struct CompressedInstance {
    /// Archetype Id
    pub id: usize,

    /// Facing direction
    pub facing: f32,

    /// Position
    pub position: Vec3,

    /// Optional instance identifier
    pub iid: Option<String>,

    /// List of aspects for this instance.
    pub aspects: InstanceAspects,
}

impl Serialize for CompressedInstance {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        let mut len = 3;
        if !self.aspects.is_empty() {
            len += 2;
        } else if self.iid.is_some() {
            len += 1;
        }
        let mut state = serializer.serialize_tuple(len)?;
        state.serialize_element(&self.id)?;
        state.serialize_element(&self.facing)?;
        state.serialize_element(&self.position)?;
        if let Some(iid) = &self.iid {
            state.serialize_element(iid)?;
        } else if !self.aspects.is_empty() {
            // Serialize 'none' as placeholder
            state.serialize_element(&self.iid)?;
        }
        if !self.aspects.is_empty() {
            state.serialize_element(&self.aspects)?;
        }
        state.end()
    }
}

struct CompressedInstanceVisitor<'a, 'b> {
    type_registry: &'a TypeRegistry,
    load_context: &'a mut LoadContext<'b>,
    parent_label: &'a str,
}

impl<'de, 'a, 'b> Visitor<'de> for CompressedInstanceVisitor<'a, 'b> {
    type Value = CompressedInstance;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a compressed instance tuple")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        // Tuple of length 3..5
        let mut result: CompressedInstance = CompressedInstance::default();
        match seq.next_element::<usize>() {
            Ok(Some(id)) => result.id = id,
            _ => return Err(serde::de::Error::invalid_length(0, &self)),
        }
        match seq.next_element::<f32>() {
            Ok(Some(facing)) => result.facing = facing,
            _ => return Err(serde::de::Error::invalid_length(1, &self)),
        }
        match seq.next_element::<Vector3>() {
            Ok(Some(position)) => result.position = position.into(),
            _ => return Err(serde::de::Error::invalid_length(2, &self)),
        }
        match seq.next_element::<Option<String>>() {
            Ok(Some(iid)) => result.iid = iid,
            _ => return Ok(result),
        }
        match seq.next_element_seed(AspectListDeserializer {
            type_registry: self.type_registry,
            load_context: self.load_context,
            label_prefix: self.parent_label,
        }) {
            Ok(Some(aspects)) => result.aspects = InstanceAspects(aspects),
            _ => return Ok(result),
        }

        Ok(result)
    }
}

struct CompressedInstanceDeserializer<'a, 'b> {
    type_registry: &'a TypeRegistry,
    load_context: &'a mut LoadContext<'b>,
    parent_label: &'a str,
}

impl<'de, 'a, 'b> DeserializeSeed<'de> for CompressedInstanceDeserializer<'a, 'b> {
    type Value = CompressedInstance;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_seq(CompressedInstanceVisitor {
            type_registry: self.type_registry,
            load_context: self.load_context,
            parent_label: self.parent_label,
        })
    }
}

struct CompressedInstanceListVisitor<'a, 'b> {
    type_registry: &'a TypeRegistry,
    load_context: &'a mut LoadContext<'b>,
    parent_label: &'a str,
}

impl<'de, 'a, 'b> Visitor<'de> for CompressedInstanceListVisitor<'a, 'b> {
    type Value = Vec<CompressedInstance>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a list of compressed instances")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        let mut result: Vec<CompressedInstance> = Vec::with_capacity(seq.size_hint().unwrap_or(0));
        while let Some(compressed_instance) =
            seq.next_element_seed(CompressedInstanceDeserializer {
                type_registry: self.type_registry,
                load_context: self.load_context,
                parent_label: self.parent_label,
            })?
        {
            result.push(compressed_instance);
        }
        Ok(result)
    }
}

struct CompressedInstanceListDeserializer<'a, 'b> {
    type_registry: &'a TypeRegistry,
    load_context: &'a mut LoadContext<'b>,
    parent_label: &'a str,
}

impl<'de, 'a, 'b> DeserializeSeed<'de> for CompressedInstanceListDeserializer<'a, 'b> {
    type Value = Vec<CompressedInstance>;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_seq(CompressedInstanceListVisitor {
            type_registry: self.type_registry,
            load_context: self.load_context,
            parent_label: self.parent_label,
        })
    }
}

struct PrecinctAssetDeserializer<'a, 'b> {
    type_registry: &'a TypeRegistry,
    load_context: &'a mut LoadContext<'b>,
}

impl<'de, 'a, 'b> DeserializeSeed<'de> for PrecinctAssetDeserializer<'a, 'b> {
    type Value = PrecinctAsset;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(field_identifier, rename_all = "snake_case")]
        enum Field {
            SceneryTypes,
            Scenery,
            FloorTypes,
            TerrainFxTypes,
            TerrainFx,
            Tiers,
            Actors,
            Layers,
        }

        struct PrecinctVisitor<'a, 'b> {
            type_registry: &'a TypeRegistry,
            load_context: &'a mut LoadContext<'b>,
        }

        impl<'de, 'a, 'b> Visitor<'de> for PrecinctVisitor<'a, 'b> {
            type Value = PrecinctAsset;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("struct Precinct")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::MapAccess<'de>,
            {
                let mut precinct = PrecinctAsset::default();
                while let Some(key) = map.next_key()? {
                    match key {
                        Field::SceneryTypes => {
                            precinct.scenery_types = map.next_value()?;
                        }
                        Field::FloorTypes => {
                            precinct.floor_types = map.next_value()?;
                        }
                        Field::Scenery => {
                            precinct.scenery =
                                map.next_value_seed(CompressedInstanceListDeserializer {
                                    type_registry: self.type_registry,
                                    load_context: self.load_context,
                                    parent_label: "scenery",
                                })?;
                        }
                        Field::TerrainFxTypes => {
                            precinct.terrain_fx_types = map.next_value()?;
                        }
                        Field::TerrainFx => {
                            precinct.terrain_fx = map.next_value()?;
                        }
                        Field::Tiers => {
                            precinct.tiers = map.next_value()?;
                        }
                        Field::Actors => {
                            // TODO
                        }
                        Field::Layers => {
                            // TODO
                        }
                    }
                }
                Ok(precinct)
            }
        }

        deserializer.deserialize_map(PrecinctVisitor {
            type_registry: self.type_registry,
            load_context: self.load_context,
        })
    }
}

pub struct PrecinctAssetLoader {
    type_registry: TypeRegistryArc,
}

impl FromWorld for PrecinctAssetLoader {
    fn from_world(world: &mut World) -> Self {
        PrecinctAssetLoader {
            type_registry: world.resource::<AppTypeRegistry>().0.clone(),
        }
    }
}

#[non_exhaustive]
#[derive(Debug, Error)]
pub enum PrecinctAssetLoaderError {
    #[error("Could not load precinct: {0}")]
    Io(#[from] std::io::Error),
    #[error("Could not decode precinct: {0}")]
    Decode(#[from] rmps::decode::Error),
}

impl AssetLoader for PrecinctAssetLoader {
    type Asset = PrecinctAsset;
    type Error = PrecinctAssetLoaderError;
    type Settings = ();

    fn load<'a>(
        &'a self,
        reader: &'a mut Reader,
        _settings: &'a Self::Settings,
        load_context: &'a mut LoadContext,
    ) -> BoxedFuture<'a, Result<Self::Asset, Self::Error>> {
        Box::pin(async move {
            let mut bytes = Vec::new();
            reader.read_to_end(&mut bytes).await?;
            let mut deserializer = rmps::Deserializer::from_read_ref(&bytes);
            let precinct_deserializer = PrecinctAssetDeserializer {
                type_registry: &self.type_registry.read(),
                load_context,
            };
            let precinct: PrecinctAsset = precinct_deserializer.deserialize(&mut deserializer)?;
            Ok(precinct)
        })
    }

    fn extensions(&self) -> &[&str] {
        &["msgpack"]
    }
}
