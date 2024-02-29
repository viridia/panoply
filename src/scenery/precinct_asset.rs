use bevy::{
    asset::{io::Reader, AssetLoader, LoadContext},
    prelude::*,
    utils::BoxedFuture,
};
use futures_lite::AsyncReadExt;
use serde::{de::Visitor, ser::SerializeTuple, Deserialize, Serialize};
use std::fmt;
use thiserror::Error;

use super::msgpack_extension::{Box2d, Vector3};

extern crate rmp_serde as rmps;

/// TODO: Remove options, use serialize_if not empty
#[derive(TypePath, Asset, Serialize, Deserialize, Debug)]
pub struct PrecinctAsset {
    /// Table of wall archetypes used by this precinct.
    #[serde(rename = "wallTypes", default)]
    wall_types: Vec<String>,

    /// Table of floor archetypes used by this precinct.
    #[serde(rename = "floorTypes", default)]
    floor_types: Vec<String>,

    /// Table of fixture archetypes used by this precinct.
    #[serde(rename = "fixtureTypes", default)]
    fixture_types: Vec<String>,

    /// Table of terrain effect archetypes used by this precinct.
    #[serde(rename = "terrainFxTypes", default)]
    terrain_fx_types: Vec<String>,

    /// Table of floors, spaced 1 meter apart.
    #[serde(default)]
    tiers: Vec<TierSer>,

    /// Packed terrain effect table
    // terrain_fx?: number[];
    // actors: Option<Vec<IActorInstanceData>>,
    /// Table of wall instances.
    #[serde(default)]
    nwalls: Vec<CompressedInstance>,

    /// Table of fixture instances.
    #[serde(default)]
    nfixtures: Vec<CompressedInstance>,
    // layers?: Record<string, ILayerData>,
}

#[derive(Serialize, Deserialize, Debug)]
struct SceneryData {
    #[serde(alias = "structure")]
    precinct: PrecinctAsset,
}

#[derive(Debug)]
pub struct CompressedInstance {
    /// Archetype Id
    pub id: usize,

    /// Facing direction
    pub facing: f32,

    /// Position
    pub position: Vec3,

    /// Archetype properties
    pub props: Option<CompressedInstanceProps>,
}

impl Serialize for CompressedInstance {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        let mut state = serializer.serialize_tuple(4)?;
        state.serialize_element(&self.id)?;
        state.serialize_element(&self.facing)?;
        state.serialize_element(&self.position)?;
        if let Some(props) = &self.props {
            state.serialize_element(props)?;
        }
        state.end()
    }
}

struct CompressedInstanceVisitor;

impl<'de> Visitor<'de> for CompressedInstanceVisitor {
    type Value = CompressedInstance;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("an compressed instance tuple")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        match seq.size_hint() {
            Some(3) => {
                let id = seq.next_element::<usize>()?.unwrap();
                let facing = seq.next_element::<f32>()?.unwrap();
                let position = seq.next_element::<Vector3>()?.unwrap();
                Ok(CompressedInstance {
                    id,
                    facing,
                    position: position.into(),
                    props: None,
                })
            }
            Some(4) => {
                let id = seq.next_element::<usize>()?.unwrap();
                let facing = seq.next_element::<f32>()?.unwrap();
                let position = seq.next_element::<Vector3>()?.unwrap();
                let props = seq.next_element::<CompressedInstanceProps>()?;
                Ok(CompressedInstance {
                    id,
                    facing,
                    position: position.into(),
                    props,
                })
            }
            _ => Err(serde::de::Error::invalid_length(0, &self)),
        }
    }
}

impl<'de> Deserialize<'de> for CompressedInstance {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_tuple(4, CompressedInstanceVisitor)
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CompressedInstanceProps {
    iid: Option<String>,
    // properties?: InstancePropertyData; // Instance vars
    // aspects?: ReadonlyArray<IAspectConfigData>;
    // inventory?: ReadonlyArray<IInventoryItemData>;
    // groupId?: string;
    // facing?: number;
}

/** Serialized schema for a tier */
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TierSer {
    level: i32,
    pfloors: Option<Vec<FloorRegionSer>>,
    cutaways: Option<Vec<Box2d>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FloorRegionSer {
    r#type: usize,
    poly: Vec<(f32, f32)>,
}

#[derive(Default)]
pub struct PrecinctAssetLoader;

#[non_exhaustive]
#[derive(Debug, Error)]
pub enum PrecinctAssetLoaderError {
    #[error("Could load precinct: {0}")]
    Io(#[from] std::io::Error),
    // #[error("Could not extract image: {0}")]
    // Image(#[from] image::ImageError),
}

impl AssetLoader for PrecinctAssetLoader {
    type Asset = PrecinctAsset;
    type Error = PrecinctAssetLoaderError;
    type Settings = ();

    fn load<'a>(
        &'a self,
        reader: &'a mut Reader,
        _settings: &'a Self::Settings,
        _load_context: &'a mut LoadContext,
    ) -> BoxedFuture<'a, Result<Self::Asset, Self::Error>> {
        Box::pin(async move {
            let mut bytes = Vec::new();
            reader.read_to_end(&mut bytes).await?;
            let mut scenery: SceneryData =
                rmps::from_slice(&bytes).expect("unable to decode precinct");
            let mut precinct = scenery.precinct;

            // println!("precinct: {:?}", serde_json::to_string(&scenery.precinct));
            Ok(precinct)
        })
    }

    fn extensions(&self) -> &[&str] {
        &["contours"]
    }
}
