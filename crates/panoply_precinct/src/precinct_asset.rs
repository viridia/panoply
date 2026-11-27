use bevy::{
    asset::{io::Reader, AssetLoader, LoadContext},
    prelude::*,
    reflect::{TypeRegistry, TypeRegistryArc},
};
use panoply_exemplar::{AspectListDeserializer, InstanceAspects};
use panoply_saga::{FormulaQueue, SagaHost};
use serde::{
    de::{DeserializeSeed, Visitor},
    ser::{SerializeMap, SerializeTuple},
    Deserialize, Serialize,
};
use std::{
    fmt::{self, Debug},
    sync::Arc,
};
use thiserror::Error;

use crate::{
    actor_instance::ActorInstanceListDeserializer, ActorInstanceData, FloorRegionSer, InstanceKey,
};

extern crate rmp_serde as rmps;

/// Everything authored in one precinct but terrain (D24). Written with
/// [`to_bytes`](Self::to_bytes), which drops what the loader skipped: faery's `layers`, and aspect
/// types no plugin registers (D25).
#[derive(TypePath, Asset, Debug, Default, Clone)]
pub struct PrecinctAsset {
    /// Table of wall archetypes used by this precinct.
    pub scenery_types: Vec<String>,

    /// Table of floor archetypes used by this precinct.
    pub floor_surfaces: Vec<String>,

    /// The terrain effects this precinct uses, catalog entries (`file#Label`).
    pub terrain_fx_types: Vec<String>,

    /// Table of floors, spaced 1 meter apart.
    pub tiers: Vec<TierSer>,

    /// Packed terrain effect table
    pub terrain_fx: Option<Vec<i16>>,

    /// The actors the precinct places (D24).
    pub actors: Vec<ActorInstanceData>,

    /// Table of scenery instances.
    pub scenery: Vec<SceneryInstanceData>,
}

impl PrecinctAsset {
    /// The precinct's file. Internal ids are written as nil, since the loader numbers instances
    /// itself (D87).
    pub fn to_bytes(&self, registry: &TypeRegistry) -> Result<Vec<u8>, rmps::encode::Error> {
        rmps::to_vec_named(&PrecinctSer {
            precinct: self,
            registry,
        })
    }

    /// Gives a key to each instance with a reason for one that lacks it, and takes the key from
    /// each with no reason left (`docs/instances.md`, Ids). The reasons: being an actor, having a
    /// label, or, for scenery, state, from its exemplar or an aspect of its own (D88), which
    /// `needs_key`, given the exemplar's index in `scenery_types` and the instance's own aspects,
    /// says. Returns how many keys were given and how many taken.
    pub fn update_keys(
        &mut self,
        needs_key: impl Fn(usize, &InstanceAspects) -> bool,
    ) -> (usize, usize) {
        let mut next = self.next_scenery_id();
        let (mut given, mut taken) = (0, 0);
        for instance in &mut self.scenery {
            let reason = instance.label.is_some() || needs_key(instance.id, &instance.aspects);
            match (instance.iid.key(), reason) {
                (None, true) => {
                    instance.iid = SceneryInstanceId::Keyed(InstanceKey::generate());
                    given += 1;
                }
                (Some(_), false) => {
                    instance.iid = SceneryInstanceId::Internal(next);
                    next += 1;
                    taken += 1;
                }
                _ => {}
            }
        }
        for actor in &mut self.actors {
            if actor.iid.key().is_none() {
                actor.iid = SceneryInstanceId::Keyed(InstanceKey::generate());
                given += 1;
            }
        }
        (given, taken)
    }

    /// Places an actor of the exemplar at `exemplar`, a `file#Label` path, at `position` in the
    /// world's coordinates, facing `facing` degrees. It gets a key, and its id is returned.
    pub fn add_actor(
        &mut self,
        exemplar: String,
        facing: f32,
        position: Vec3,
    ) -> SceneryInstanceId {
        let iid = SceneryInstanceId::Keyed(InstanceKey::generate());
        self.actors.push(ActorInstanceData {
            exemplar,
            position,
            facing,
            iid: iid.clone(),
            label: None,
            aspects: InstanceAspects::default(),
        });
        iid
    }

    /// Labels the instance, scenery or actor, whose id is `iid`, or with `None` clears its label.
    /// A label is a reason for a key, so an instance without one is given one. Returns the
    /// instance's id after, or `None`, changing nothing, if no instance has `iid`, the label is
    /// empty, or another instance here has it. Uniqueness across the game is the editor's to check.
    pub fn set_label(
        &mut self,
        iid: &SceneryInstanceId,
        label: Option<&str>,
    ) -> Option<SceneryInstanceId> {
        if label.is_some_and(|label| label.is_empty()) {
            return None;
        }
        let clash = label.is_some()
            && self
                .instances()
                .any(|(other, other_label)| other != iid && other_label == label);
        if clash {
            return None;
        }
        let (target, target_label) = self.instances_mut().find(|(other, _)| **other == *iid)?;
        if label.is_some() && target.key().is_none() {
            *target = SceneryInstanceId::Keyed(InstanceKey::generate());
        }
        *target_label = label.map(Arc::from);
        Some(target.clone())
    }

    /// Whether an instance, scenery or actor, has the id `iid`.
    pub fn has_instance(&self, iid: &SceneryInstanceId) -> bool {
        self.instances().any(|(other, _)| other == iid)
    }

    /// The label of the instance, scenery or actor, whose id is `iid`, if it has one.
    pub fn label(&self, iid: &SceneryInstanceId) -> Option<&str> {
        self.instances()
            .find(|(other, _)| *other == iid)
            .and_then(|(_, label)| label)
    }

    /// The labels of the precinct's scenery and actors.
    pub fn labels(&self) -> impl Iterator<Item = &str> {
        self.instances().filter_map(|(_, label)| label)
    }

    /// Each instance's id and label, scenery first, then actors.
    fn instances(&self) -> impl Iterator<Item = (&SceneryInstanceId, Option<&str>)> {
        self.scenery
            .iter()
            .map(|instance| (&instance.iid, instance.label.as_deref()))
            .chain(
                self.actors
                    .iter()
                    .map(|actor| (&actor.iid, actor.label.as_deref())),
            )
    }

    fn instances_mut(
        &mut self,
    ) -> impl Iterator<Item = (&mut SceneryInstanceId, &mut Option<Arc<str>>)> {
        self.scenery
            .iter_mut()
            .map(|instance| (&mut instance.iid, &mut instance.label))
            .chain(
                self.actors
                    .iter_mut()
                    .map(|actor| (&mut actor.iid, &mut actor.label)),
            )
    }

    /// Takes out the instance, scenery or actor, whose id is `iid`, to be put in another precinct
    /// with [`insert_instance`](Self::insert_instance). Its exemplar type stays in this precinct's
    /// table.
    pub fn take_instance(&mut self, iid: &SceneryInstanceId) -> Option<TakenInstance> {
        if let Some(index) = self
            .scenery
            .iter()
            .position(|instance| instance.iid == *iid)
        {
            let instance = self.scenery.remove(index);
            let exemplar = self.scenery_types.get(instance.id)?.clone();
            return Some(TakenInstance::Scenery { exemplar, instance });
        }
        let index = self.actors.iter().position(|actor| actor.iid == *iid)?;
        Some(TakenInstance::Actor(self.actors.remove(index)))
    }

    /// Puts in an instance taken from another precinct, with its key and label, adding its exemplar
    /// to `scenery_types` if the table lacks it. An instance with no key is numbered here. Its
    /// position is taken as it is: the caller makes a scenery instance's relative to this precinct.
    /// Returns its id here.
    pub fn insert_instance(&mut self, taken: TakenInstance) -> SceneryInstanceId {
        match taken {
            TakenInstance::Scenery {
                exemplar,
                mut instance,
            } => {
                instance.id = match self.scenery_type_index(&exemplar) {
                    Some(index) => index,
                    None => self.add_scenery_type(exemplar),
                };
                if instance.iid.key().is_none() {
                    instance.iid = SceneryInstanceId::Internal(self.next_scenery_id());
                }
                let iid = instance.iid.clone();
                self.scenery.push(instance);
                iid
            }
            TakenInstance::Actor(actor) => {
                let iid = actor.iid.clone();
                self.actors.push(actor);
                iid
            }
        }
    }

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
        instance_id: Option<SceneryInstanceId>,
    ) -> SceneryInstanceId {
        let iid =
            instance_id.unwrap_or_else(|| SceneryInstanceId::Internal(self.next_scenery_id()));
        self.scenery.push(SceneryInstanceData {
            id: arch,
            facing,
            position,
            iid: iid.clone(),
            label: None,
            aspects: InstanceAspects::default(),
        });
        iid
    }

    pub fn remove_scenery_elements<F: Fn(&SceneryInstanceId, usize, &Vec3) -> bool>(
        &mut self,
        filter: F,
    ) -> Vec<SceneryInstanceData> {
        let mut removed: Vec<SceneryInstanceData> = Vec::new();
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
        self.floor_surfaces.iter().position(|ft| ft == floor_type)
    }

    /// Add a new floor type to the precinct.
    pub fn add_floor_type(&mut self, floor_type: String) -> usize {
        assert!(!self.floor_surfaces.iter().any(|ft| ft == &floor_type));
        let index = self.floor_surfaces.len();
        self.floor_surfaces.push(floor_type);
        index
    }

    /// One past the highest internal id.
    fn next_scenery_id(&self) -> usize {
        self.scenery
            .iter()
            .filter_map(|instance| match instance.iid {
                SceneryInstanceId::Internal(id) => Some(id + 1),
                _ => None,
            })
            .max()
            .unwrap_or(0)
    }

    /// Numbers each scenery instance read without an id, in file order, after the highest internal
    /// id the file holds.
    fn number_scenery(&mut self) {
        let mut next = self.next_scenery_id();
        for instance in &mut self.scenery {
            if instance.iid == SceneryInstanceId::None {
                instance.iid = SceneryInstanceId::Internal(next);
                next += 1;
            }
        }
    }
}

/// Writes a [`PrecinctAsset`] as a named map, each instance's aspects through reflection.
struct PrecinctSer<'a> {
    precinct: &'a PrecinctAsset,
    registry: &'a TypeRegistry,
}

impl Serialize for PrecinctSer<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        let precinct = self.precinct;
        let mut map = serializer.serialize_map(None)?;
        map.serialize_entry("scenery_types", &precinct.scenery_types)?;
        if !precinct.floor_surfaces.is_empty() {
            map.serialize_entry("floor_surfaces", &precinct.floor_surfaces)?;
        }
        if !precinct.terrain_fx_types.is_empty() {
            map.serialize_entry("terrain_fx_types", &precinct.terrain_fx_types)?;
        }
        if !precinct.tiers.is_empty() {
            map.serialize_entry("tiers", &precinct.tiers)?;
        }
        if let Some(terrain_fx) = &precinct.terrain_fx {
            map.serialize_entry("terrain_fx", terrain_fx)?;
        }
        if !precinct.actors.is_empty() {
            let actors: Vec<_> = precinct
                .actors
                .iter()
                .map(|actor| actor.serializable(self.registry))
                .collect();
            map.serialize_entry("actors", &actors)?;
        }
        let scenery: Vec<_> = precinct
            .scenery
            .iter()
            .map(|instance| SceneryInstanceSer {
                instance,
                registry: self.registry,
            })
            .collect();
        map.serialize_entry("scenery", &scenery)?;
        map.end()
    }
}

/// Writes a [`SceneryInstanceData`] as a tuple: exemplar index, facing, position, id, and the
/// aspects, if it has any.
struct SceneryInstanceSer<'a> {
    instance: &'a SceneryInstanceData,
    registry: &'a TypeRegistry,
}

impl Serialize for SceneryInstanceSer<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        let instance = self.instance;
        let has_aspects = !instance.aspects.is_empty();
        let mut state = serializer.serialize_tuple(if has_aspects { 5 } else { 4 })?;
        state.serialize_element(&instance.id)?;
        state.serialize_element(&instance.facing)?;
        state.serialize_element(&instance.position)?;
        state.serialize_element(&InstanceIdSer {
            iid: &instance.iid,
            label: instance.label.as_deref(),
        })?;
        if has_aspects {
            state.serialize_element(&instance.aspects.serializable(self.registry))?;
        }
        state.end()
    }
}

/** Serialized schema for a tier */
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TierSer {
    pub level: i32,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pfloors: Vec<FloorRegionSer>,

    #[serde(
        default,
        skip_serializing_if = "Vec::is_empty",
        with = "panoply_exemplar::ser::vec_rect"
    )]
    pub cutaways: Vec<Rect>,
}

/// An instance's identity in its precinct. Its label, which names it, is kept beside it, so a
/// rename changes no identity.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub enum SceneryInstanceId {
    /// Read without a key, and not yet numbered.
    #[default]
    None,
    /// A number given as the precinct loads, neither saved nor stable across edits (D87).
    Internal(usize),
    /// A key, saved with the instance.
    Keyed(InstanceKey),
}

impl SceneryInstanceId {
    /// The key, if the instance has one.
    pub fn key(&self) -> Option<InstanceKey> {
        match self {
            SceneryInstanceId::Keyed(key) => Some(*key),
            _ => None,
        }
    }
}

/// An instance's id as a precinct file holds it: nil, its key, or its key and label as a pair.
/// Internal ids are written as nil, since the loader numbers instances itself (D87).
pub(crate) struct InstanceIdSer<'a> {
    pub(crate) iid: &'a SceneryInstanceId,
    pub(crate) label: Option<&'a str>,
}

impl Serialize for InstanceIdSer<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        match (self.iid.key(), self.label) {
            (Some(key), None) => key.serialize(serializer),
            (Some(key), Some(label)) => (key, label).serialize(serializer),
            (None, _) => serializer.serialize_unit(),
        }
    }
}

/// Reads what [`InstanceIdSer`] writes. A name where a key belongs, faery's ids as they were
/// before keys, is an error.
struct InstanceIdField(Option<(InstanceKey, Option<Arc<str>>)>);

impl<'de> Deserialize<'de> for InstanceIdField {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct IdVisitor;

        impl<'de> Visitor<'de> for IdVisitor {
            type Value = InstanceIdField;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("nil, an instance key, or a key and a label")
            }

            fn visit_unit<E>(self) -> Result<Self::Value, E> {
                Ok(InstanceIdField(None))
            }

            fn visit_none<E>(self) -> Result<Self::Value, E> {
                Ok(InstanceIdField(None))
            }

            fn visit_bytes<E: serde::de::Error>(self, bytes: &[u8]) -> Result<Self::Value, E> {
                let uuid = bevy::asset::uuid::Uuid::from_slice(bytes).map_err(E::custom)?;
                Ok(InstanceIdField(Some((uuid.into(), None))))
            }

            fn visit_str<E: serde::de::Error>(self, text: &str) -> Result<Self::Value, E> {
                match text.parse() {
                    Ok(key) => Ok(InstanceIdField(Some((key, None)))),
                    Err(_) => Err(E::custom(format!(
                        "instance id {text:?} is a name, not a key"
                    ))),
                }
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::SeqAccess<'de>,
            {
                let key = seq
                    .next_element::<InstanceKey>()?
                    .ok_or_else(|| serde::de::Error::invalid_length(0, &self))?;
                let label = seq
                    .next_element::<String>()?
                    .ok_or_else(|| serde::de::Error::invalid_length(1, &self))?;
                Ok(InstanceIdField(Some((key, Some(label.into())))))
            }
        }

        deserializer.deserialize_any(IdVisitor)
    }
}

#[derive(Debug, Default, Clone)]
pub struct SceneryInstanceData {
    /// Archetype Id
    pub id: usize,

    /// Facing direction, in degrees: 0 faces +z, 90 faces +x.
    pub facing: f32,

    /// Position, relative to the precinct.
    pub position: Vec3,

    /// The instance's identity.
    pub iid: SceneryInstanceId,

    /// The name text uses for the instance, unique across the game. Only a keyed instance has one.
    pub label: Option<Arc<str>>,

    /// List of aspects for this instance.
    pub aspects: InstanceAspects,
}

/// An instance taken out of a precinct, to be put in another
/// ([`PrecinctAsset::take_instance`]). A scenery instance carries its exemplar's path, since its
/// index in `scenery_types` is its old precinct's.
#[derive(Debug, Clone)]
pub enum TakenInstance {
    Scenery {
        exemplar: String,
        instance: SceneryInstanceData,
    },
    Actor(ActorInstanceData),
}

struct CompressedInstanceVisitor<'a, 'b> {
    type_registry: &'a TypeRegistry,
    load_context: &'a mut LoadContext<'b>,
    formulas: &'a mut FormulaQueue,
    parent_label: &'a str,
}

impl<'de> Visitor<'de> for CompressedInstanceVisitor<'_, '_> {
    type Value = SceneryInstanceData;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a compressed instance tuple")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        // Tuple of length 3..5
        let mut result: SceneryInstanceData = SceneryInstanceData::default();
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
        match seq.next_element::<InstanceIdField>()? {
            Some(InstanceIdField(Some((key, label)))) => {
                result.iid = SceneryInstanceId::Keyed(key);
                result.label = label;
            }
            Some(InstanceIdField(None)) => {}
            None => return Err(serde::de::Error::invalid_length(3, &self)),
        }
        // The aspects are optional, but an error in them must not be swallowed: the rest of the
        // tuple would be left unread, and the next instance would parse from the wrong place.
        if let Some(aspects) = seq.next_element_seed(AspectListDeserializer {
            type_registry: self.type_registry,
            load_context: self.load_context,
            formulas: self.formulas,
            label_prefix: self.parent_label,
            // TODO(PAN135): scripted aspects on instances.
            scripted: &[],
        })? {
            result.aspects = InstanceAspects(aspects);
        }

        Ok(result)
    }
}

struct CompressedInstanceDeserializer<'a, 'b> {
    type_registry: &'a TypeRegistry,
    load_context: &'a mut LoadContext<'b>,
    formulas: &'a mut FormulaQueue,
    parent_label: &'a str,
}

impl<'de> DeserializeSeed<'de> for CompressedInstanceDeserializer<'_, '_> {
    type Value = SceneryInstanceData;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_seq(CompressedInstanceVisitor {
            type_registry: self.type_registry,
            load_context: self.load_context,
            formulas: self.formulas,
            parent_label: self.parent_label,
        })
    }
}

struct CompressedInstanceListVisitor<'a, 'b> {
    type_registry: &'a TypeRegistry,
    load_context: &'a mut LoadContext<'b>,
    formulas: &'a mut FormulaQueue,
    parent_label: &'a str,
}

impl<'de> Visitor<'de> for CompressedInstanceListVisitor<'_, '_> {
    type Value = Vec<SceneryInstanceData>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a list of compressed instances")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        let mut result: Vec<SceneryInstanceData> = Vec::with_capacity(seq.size_hint().unwrap_or(0));
        while let Some(compressed_instance) =
            seq.next_element_seed(CompressedInstanceDeserializer {
                type_registry: self.type_registry,
                load_context: self.load_context,
                formulas: self.formulas,
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
    formulas: &'a mut FormulaQueue,
    parent_label: &'a str,
}

impl<'de> DeserializeSeed<'de> for CompressedInstanceListDeserializer<'_, '_> {
    type Value = Vec<SceneryInstanceData>;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_seq(CompressedInstanceListVisitor {
            type_registry: self.type_registry,
            load_context: self.load_context,
            formulas: self.formulas,
            parent_label: self.parent_label,
        })
    }
}

struct PrecinctAssetDeserializer<'a, 'b> {
    type_registry: &'a TypeRegistry,
    load_context: &'a mut LoadContext<'b>,
    formulas: &'a mut FormulaQueue,
}

impl<'de> DeserializeSeed<'de> for PrecinctAssetDeserializer<'_, '_> {
    type Value = PrecinctAsset;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize, Debug)]
        #[serde(field_identifier, rename_all = "snake_case")]
        enum Field {
            SceneryTypes,
            Scenery,
            FloorSurfaces,
            TerrainFxTypes,
            TerrainFx,
            Tiers,
            Actors,
            Layers,
        }

        struct PrecinctVisitor<'a, 'b> {
            type_registry: &'a TypeRegistry,
            load_context: &'a mut LoadContext<'b>,
            formulas: &'a mut FormulaQueue,
        }

        impl<'de> Visitor<'de> for PrecinctVisitor<'_, '_> {
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
                        Field::FloorSurfaces => {
                            precinct.floor_surfaces = map.next_value()?;
                        }
                        Field::Scenery => {
                            precinct.scenery =
                                map.next_value_seed(CompressedInstanceListDeserializer {
                                    type_registry: self.type_registry,
                                    load_context: self.load_context,
                                    formulas: self.formulas,
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
                                    formulas: self.formulas,
                                    parent_label: "actors",
                                })?;
                        }
                        Field::Layers => {
                            // faery's editing layers, which Panoply does not have.
                            map.next_value::<serde::de::IgnoredAny>()?;
                        }
                    }
                }
                precinct.number_scenery();
                Ok(precinct)
            }
        }

        deserializer.deserialize_map(PrecinctVisitor {
            type_registry: self.type_registry,
            load_context: self.load_context,
            formulas: self.formulas,
        })
    }
}

#[derive(TypePath)]
pub struct PrecinctAssetLoader {
    type_registry: TypeRegistryArc,
    host: SagaHost,
}

impl FromWorld for PrecinctAssetLoader {
    fn from_world(world: &mut World) -> Self {
        PrecinctAssetLoader {
            type_registry: world.resource::<AppTypeRegistry>().0.clone(),
            host: world.resource::<SagaHost>().clone(),
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

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &Self::Settings,
        load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        let mut formulas = FormulaQueue::default();
        let precinct = PrecinctAssetDeserializer {
            type_registry: &self.type_registry.read(),
            load_context: &mut *load_context,
            formulas: &mut formulas,
        }
        .deserialize(&mut rmps::Deserializer::from_read_ref(&bytes))?;
        let path = load_context.path().to_string();
        formulas.compile(&path, self.host.get(), &bytes, &[]).await;
        Ok(precinct)
    }

    fn extensions(&self) -> &[&str] {
        &["msgpack"]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CHEST: &str = "exemplars/furnishings.exem.json#WoodenChest";
    const WALL: &str = "exemplars/walls.exem.json#Wall";
    const GUARD: &str = "exemplars/characters.exem.json#Guard";

    /// An aspect with state, which an instance may have as its own.
    #[derive(Reflect, Clone, Default)]
    struct Stash;

    impl panoply_exemplar::NativeAspect for Stash {
        type Config = ();

        fn config(&self) {}

        fn can_attach(&self, _kind: panoply_exemplar::InstanceKind) -> bool {
            true
        }

        fn has_state(&self) -> bool {
            true
        }
    }

    /// Whether an instance needs a key, as the editor says: the chest's exemplar has state, and so
    /// does a `Stash` of an instance's own.
    fn needs_key(index: usize, aspects: &InstanceAspects) -> bool {
        index == 0 || aspects.0.iter().any(|aspect| aspect.has_state())
    }

    /// Keys go to the instances with a reason for one, and are taken from those with none left:
    /// a chest, a labelled wall, a wall with a `Stash` of its own and an actor keep or gain keys,
    /// while a wall whose label was cleared loses its own.
    #[test]
    fn keys_follow_their_reasons() {
        let mut precinct = PrecinctAsset {
            scenery_types: vec![CHEST.into(), WALL.into()],
            ..default()
        };
        let chest = precinct.add_scenery_element(0, 0., Vec3::ZERO, None);
        let plain = precinct.add_scenery_element(1, 0., Vec3::X, None);
        let labelled = precinct.add_scenery_element(1, 0., Vec3::Y, None);
        let cleared = precinct.add_scenery_element(1, 0., Vec3::Z, None);
        let labelled = precinct.set_label(&labelled, Some("gate")).unwrap();
        let cleared = precinct.set_label(&cleared, Some("door")).unwrap();
        precinct.set_label(&cleared, None).unwrap();
        precinct.add_scenery_element(1, 0., Vec3::NEG_X, None);
        precinct.scenery[4].aspects = InstanceAspects(vec![Arc::new(Stash)]);
        precinct.actors.push(ActorInstanceData {
            exemplar: GUARD.into(),
            ..default()
        });

        assert_eq!(precinct.update_keys(needs_key), (3, 1));
        let ids: Vec<_> = precinct.scenery.iter().map(|s| s.iid.clone()).collect();
        assert!(ids[0].key().is_some() && ids[0] != chest);
        assert_eq!(ids[1], plain);
        assert_eq!(ids[2], labelled);
        assert!(matches!(ids[3], SceneryInstanceId::Internal(_)));
        assert!(ids[4].key().is_some());
        assert!(precinct.actors[0].iid.key().is_some());
        assert_eq!(precinct.update_keys(needs_key), (0, 0));
    }

    /// A label keeps the key it was given with, through a change and a clear; a label another
    /// instance here has, or an empty one, is refused.
    #[test]
    fn a_label_keeps_the_key() {
        let mut precinct = PrecinctAsset {
            scenery_types: vec![WALL.into()],
            ..default()
        };
        let wall = precinct.add_scenery_element(0, 0., Vec3::ZERO, None);
        let guard = precinct.add_actor(GUARD.into(), 0., Vec3::X);
        let keyed = precinct.set_label(&wall, Some("gate")).unwrap();
        assert!(keyed.key().is_some());
        assert_eq!(
            precinct.set_label(&keyed, Some("postern")),
            Some(keyed.clone())
        );
        assert_eq!(precinct.set_label(&guard, Some("postern")), None);
        assert_eq!(precinct.set_label(&guard, Some("")), None);
        assert_eq!(
            precinct.set_label(&guard, Some("stone")),
            Some(guard.clone())
        );
        assert_eq!(precinct.set_label(&keyed, None), Some(keyed.clone()));
        assert_eq!(precinct.set_label(&wall, Some("gate")), None);
        let labels: Vec<_> = precinct.labels().collect();
        assert_eq!(labels, ["stone"]);
    }

    /// An instance moved from one precinct to another keeps its key and label, and the receiving
    /// precinct gains its exemplar.
    #[test]
    fn a_move_keeps_the_key() {
        let mut a = PrecinctAsset {
            scenery_types: vec![WALL.into(), CHEST.into()],
            ..default()
        };
        let mut b = PrecinctAsset {
            scenery_types: vec![WALL.into()],
            ..default()
        };
        b.add_scenery_element(0, 0., Vec3::ZERO, None);
        let chest = a.add_scenery_element(1, 0., Vec3::ZERO, None);
        let chest = a.set_label(&chest, Some("strongbox")).unwrap();

        let taken = a.take_instance(&chest).unwrap();
        assert_eq!(b.insert_instance(taken), chest);
        assert!(!a.has_instance(&chest));
        assert_eq!(b.scenery_types, [WALL, CHEST]);
        let moved = &b.scenery[1];
        assert_eq!((moved.id, &moved.iid), (1, &chest));
        assert_eq!(moved.label.as_deref(), Some("strongbox"));
    }

    /// A moved instance with no key is numbered in its new precinct.
    #[test]
    fn a_moved_wall_is_numbered_anew() {
        let mut a = PrecinctAsset {
            scenery_types: vec![WALL.into()],
            ..default()
        };
        let mut b = a.clone();
        b.add_scenery_element(0, 0., Vec3::ZERO, None);
        a.add_scenery_element(0, 0., Vec3::ZERO, None);
        let wall = a.add_scenery_element(0, 0., Vec3::X, None);
        let taken = a.take_instance(&wall).unwrap();
        assert_eq!(b.insert_instance(taken), SceneryInstanceId::Internal(1));
    }

    /// An id round-trips through msgpack and JSON: none, a key, and a key with a label.
    #[test]
    fn ids_round_trip() {
        let key = SceneryInstanceId::Keyed(InstanceKey::generate());
        let cases = [
            (SceneryInstanceId::Internal(3), None),
            (key.clone(), None),
            (key.clone(), Some("sgt.stone")),
        ];
        for (iid, label) in cases {
            let written = InstanceIdSer { iid: &iid, label };
            let expected = iid.key().map(|key| (key, label.map(Arc::from)));
            let packed = rmps::to_vec_named(&written).unwrap();
            let read: InstanceIdField = rmps::from_slice(&packed).unwrap();
            assert_eq!(read.0, expected);
            let json = serde_json::to_string(&written).unwrap();
            let read: InstanceIdField = serde_json::from_str(&json).unwrap();
            assert_eq!(read.0, expected);
        }
        // msgpack's `bin 8` of 16 bytes: the key, not its text.
        let packed = rmps::to_vec_named(&InstanceIdSer {
            iid: &key,
            label: None,
        })
        .unwrap();
        assert_eq!(packed[..2], [0xC4, 16]);
    }

    /// A name where a key belongs is an error.
    #[test]
    fn a_name_is_not_a_key() {
        let packed = rmps::to_vec_named("swd0").unwrap();
        assert!(rmps::from_slice::<InstanceIdField>(&packed).is_err());
    }
}
