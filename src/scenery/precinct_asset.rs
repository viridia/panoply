use bevy::{
    asset::{io::Reader, AssetLoader, LoadContext},
    prelude::*,
    reflect::{TypeRegistry, TypeRegistryArc},
};
use futures_lite::AsyncReadExt;
use panoply_exemplar::{AspectListDeserializer, InstanceAspects};
use serde::{
    de::{DeserializeSeed, Visitor},
    ser::SerializeTuple,
    Deserialize, Serialize,
};
use std::{
    fmt::{self, Debug},
    sync::Arc,
};
use thiserror::Error;

use crate::actors::{ActorInstance, ActorInstanceListDeserializer};

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

    #[serde(default)]
    pub(crate) actors: Vec<ActorInstance>,

    /// Table of scenery instances.
    #[serde(default)]
    pub(crate) scenery: Vec<CompressedInstance>,
    // layers?: Record<string, ILayerData>,
}

impl PrecinctAsset {
    /// Return the tier with the given level.
    pub fn find_tier(&self, level: i32) -> Option<&TierSer> {
        self.tiers.iter().find(|t| t.level == level)
    }

    /// Return a mutable reference to the tier with the given level.
    pub fn find_tier_mut(&mut self, level: i32) -> Option<&mut TierSer> {
        self.tiers.iter_mut().find(|t| t.level == level)
    }

    /// Add a new tier with the given level.
    pub fn add_tier(&mut self, level: i32) -> &mut TierSer {
        let index = self
            .tiers
            .iter()
            .position(|t| t.level >= level)
            .unwrap_or(self.tiers.len());
        self.tiers.insert(
            index,
            TierSer {
                level,
                pfloors: Vec::new(),
                cutaways: Vec::new(),
            },
        );
        &mut self.tiers[index]
    }

    /// Return the table index of the given scenery exemplay.
    pub fn scenery_type_index(&self, scenery_type: &str) -> Option<usize> {
        self.scenery_types.iter().position(|st| st == scenery_type)
    }

    /// Add a new scenery type to the precinct.
    pub fn add_scenery_type(&mut self, scenery_type: String) -> usize {
        assert!(!self.scenery_types.iter().any(|st| st == &scenery_type));
        let index = self.scenery_types.len();
        self.scenery_types.push(scenery_type);
        index
    }

    /// Add a new scenery element to the precinct. Returns the instance id.
    pub fn add_scenery_element(
        &mut self,
        arch: usize,
        facing: f32,
        position: Vec3,
    ) -> SceneryInstanceId {
        let iid = SceneryInstanceId::Internal(self.next_scenery_id());
        self.scenery.push(CompressedInstance {
            id: arch,
            facing,
            position,
            iid: iid.clone(),
            aspects: InstanceAspects::default(),
        });
        iid
    }

    pub fn remove_scenery_elements<F: Fn(&SceneryInstanceId, usize, &Vec3) -> bool>(
        &mut self,
        filter: F,
    ) -> Vec<CompressedInstance> {
        let mut removed: Vec<CompressedInstance> = Vec::new();
        self.scenery.retain(|c| {
            if filter(&c.iid, c.id, &c.position) {
                removed.push((*c).clone());
                false
            } else {
                true
            }
        });
        removed
    }

    /// Return the table index of the given floor type.
    pub fn floor_type_index(&self, floor_type: &str) -> Option<usize> {
        self.floor_types.iter().position(|ft| ft == floor_type)
    }

    /// Add a new floor type to the precinct.
    pub fn add_floor_type(&mut self, floor_type: String) -> usize {
        assert!(!self.floor_types.iter().any(|ft| ft == &floor_type));
        let index = self.floor_types.len();
        self.floor_types.push(floor_type);
        index
    }

    fn next_scenery_id(&self) -> usize {
        let mut next_id: usize = 0;
        loop {
            let found_id = next_id;
            self.scenery.iter().for_each(|c| {
                if let SceneryInstanceId::Internal(id) = c.iid {
                    next_id = next_id.max(id + 1);
                }
            });
            if next_id == found_id {
                return next_id;
            }
        }
    }
}

/** Serialized schema for a tier */
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TierSer {
    pub(crate) level: i32,

    #[serde(default)]
    pub(crate) pfloors: Vec<FloorRegionSer>,

    #[serde(default, with = "panoply_exemplar::ser::vec_rect")]
    pub(crate) cutaways: Vec<Rect>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SceneryInstanceId {
    #[default]
    None,
    Internal(usize),

    #[serde(with = "panoply_exemplar::ser::arcstring")]
    External(Arc<String>),
}

#[derive(Debug, Default, Clone)]
pub struct CompressedInstance {
    /// Archetype Id
    pub id: usize,

    /// Facing direction
    pub facing: f32,

    /// Position
    pub position: Vec3,

    /// Optional instance identifier
    pub iid: SceneryInstanceId,

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
        } else if matches!(self.iid, SceneryInstanceId::External(_)) {
            len += 1;
        }
        let mut state = serializer.serialize_tuple(len)?;
        state.serialize_element(&self.id)?;
        state.serialize_element(&self.facing)?;
        state.serialize_element(&self.position)?;
        state.serialize_element(&self.iid)?;
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
        match seq.next_element::<Vec3>() {
            Ok(Some(position)) => result.position = position,
            _ => return Err(serde::de::Error::invalid_length(2, &self)),
        }
        match seq.next_element::<SceneryInstanceId>() {
            Ok(Some(iid)) => result.iid = iid,
            _ => return Err(serde::de::Error::invalid_length(3, &self)),
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
                            precinct.actors =
                                map.next_value_seed(ActorInstanceListDeserializer {
                                    type_registry: self.type_registry,
                                    load_context: self.load_context,
                                    parent_label: "actors",
                                })?;
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

    async fn load<'a>(
        &'a self,
        reader: &'a mut Reader<'_>,
        _settings: &'a Self::Settings,
        load_context: &'a mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        let mut deserializer = rmps::Deserializer::from_read_ref(&bytes);
        let precinct_deserializer = PrecinctAssetDeserializer {
            type_registry: &self.type_registry.read(),
            load_context,
        };
        let precinct: PrecinctAsset = precinct_deserializer.deserialize(&mut deserializer)?;
        Ok(precinct)
    }

    fn extensions(&self) -> &[&str] {
        &["msgpack"]
    }
}
