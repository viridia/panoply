use bevy::{
    asset::{io::Reader, AssetLoader, LoadContext},
    platform::collections::HashSet,
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

use crate::{actor_instance::ActorInstanceListDeserializer, ActorInstanceData, FloorRegionSer};

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

    /// Table of terrain effect archetypes used by this precinct.
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

    /// Gives an external id to each scenery instance that lacks one and for which `needs_id`,
    /// given its index in `scenery_types`, is true. Each is named for its exemplar's label in
    /// kebab case, numbered with the lowest number the precinct does not yet use, as `chest-2`.
    /// Returns how many were given one.
    pub fn assign_external_ids(&mut self, needs_id: impl Fn(usize) -> bool) -> usize {
        let mut taken: HashSet<String> = self
            .scenery
            .iter()
            .map(|instance| &instance.iid)
            .chain(self.actors.iter().map(|actor| &actor.iid))
            .filter_map(|iid| match iid {
                SceneryInstanceId::External(name) => Some((**name).clone()),
                _ => None,
            })
            .collect();
        let bases: Vec<String> = self
            .scenery_types
            .iter()
            .map(|path| kebab_case(path.rsplit('#').next().unwrap_or(path)))
            .collect();
        let mut next = vec![1usize; bases.len()];
        let mut count = 0;
        for instance in &mut self.scenery {
            if matches!(instance.iid, SceneryInstanceId::External(_)) || !needs_id(instance.id) {
                continue;
            }
            let name = loop {
                let name = format!("{}-{}", bases[instance.id], next[instance.id]);
                next[instance.id] += 1;
                if !taken.contains(&name) {
                    break name;
                }
            };
            taken.insert(name.clone());
            instance.iid = SceneryInstanceId::External(Arc::new(name));
            count += 1;
        }
        count
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

/// `label` in kebab case: `WoodenChest` as `wooden-chest`.
fn kebab_case(label: &str) -> String {
    let mut result = String::with_capacity(label.len() + 4);
    let mut previous: Option<char> = None;
    for c in label.chars() {
        if c.is_uppercase() {
            if previous.is_some_and(|p| p.is_lowercase() || p.is_ascii_digit()) {
                result.push('-');
            }
            result.extend(c.to_lowercase());
        } else if matches!(c, '.' | '_' | ' ' | '-') {
            if !result.ends_with('-') {
                result.push('-');
            }
        } else {
            result.push(c);
        }
        previous = Some(c);
    }
    result
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
        state.serialize_element(match &instance.iid {
            SceneryInstanceId::Internal(_) => &SceneryInstanceId::None,
            iid => iid,
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
pub struct SceneryInstanceData {
    /// Archetype Id
    pub id: usize,

    /// Facing direction, in degrees: 0 faces +z, 90 faces +x.
    pub facing: f32,

    /// Position
    pub position: Vec3,

    /// Optional instance identifier
    pub iid: SceneryInstanceId,

    /// List of aspects for this instance.
    pub aspects: InstanceAspects,
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
        match seq.next_element::<SceneryInstanceId>() {
            Ok(Some(iid)) => result.iid = iid,
            _ => return Err(serde::de::Error::invalid_length(3, &self)),
        }
        // The aspects are optional, but an error in them must not be swallowed: the rest of the
        // tuple would be left unread, and the next instance would parse from the wrong place.
        if let Some(aspects) = seq.next_element_seed(AspectListDeserializer {
            type_registry: self.type_registry,
            load_context: self.load_context,
            formulas: self.formulas,
            label_prefix: self.parent_label,
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
        formulas.compile(&path, self.host.get(), &bytes).await;
        Ok(precinct)
    }

    fn extensions(&self) -> &[&str] {
        &["msgpack"]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_in_kebab_case() {
        assert_eq!(kebab_case("WoodenChest"), "wooden-chest");
        assert_eq!(kebab_case("Chest2Large"), "chest2-large");
        assert_eq!(kebab_case("Proto.Guard"), "proto-guard");
        assert_eq!(kebab_case("chest"), "chest");
    }

    /// Ids go to the instances that need them, numbered from the lowest free number, around the
    /// names the precinct already uses; an instance with an id keeps it.
    #[test]
    fn external_ids_take_the_lowest_free_number() {
        let mut precinct = PrecinctAsset {
            scenery_types: vec![
                "exemplars/furnishings.exem.json#WoodenChest".into(),
                "exemplars/walls.exem.json#Wall".into(),
            ],
            ..default()
        };
        let named = SceneryInstanceId::External(Arc::new("wooden-chest-1".into()));
        precinct.add_scenery_element(0, 0., Vec3::ZERO, Some(named.clone()));
        precinct.add_scenery_element(0, 0., Vec3::X, None);
        precinct.add_scenery_element(1, 0., Vec3::Y, None);
        precinct.add_scenery_element(0, 0., Vec3::Z, None);
        assert_eq!(precinct.assign_external_ids(|index| index == 0), 2);
        let ids: Vec<_> = precinct.scenery.iter().map(|s| s.iid.clone()).collect();
        assert_eq!(ids[0], named);
        assert_eq!(
            ids[1],
            SceneryInstanceId::External(Arc::new("wooden-chest-2".into()))
        );
        assert_eq!(ids[2], SceneryInstanceId::Internal(1));
        assert_eq!(
            ids[3],
            SceneryInstanceId::External(Arc::new("wooden-chest-3".into()))
        );
    }
}
