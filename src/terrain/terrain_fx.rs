//! TerrainFx are used to modify the appearance of the terrain. Terrain parcels are normally
//! stamped copies of a single mesh, which can lead to a repetitive appearance. TerrainFx allows
//! for a more varied appearance by overriding the terrain mesh. TerrainFx are stored with the
//! precinct, and are applied to the terrain parcels when they are stamped.
use bevy::prelude::*;
use bitflags::bitflags;
use serde::{
    de::{SeqAccess, Visitor},
    ser::SerializeSeq,
    Deserialize, Deserializer, Serialize,
};

/// Types of terrain effects.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Default, Reflect)]
#[reflect(Default, Deserialize, Serialize)]
pub struct TerrainTypes(u8);

bitflags! {
    impl TerrainTypes: u8 {
        /// Cobblestone road
        const Road = 1 << 0;
        /// Dark, earthy soil
        const Soil = 1 << 1;
        /// Trodden path or trail
        const Path = 1 << 2;
        /// Paved stone
        const Stone = 1 << 3;
    }
}

impl Serialize for TerrainTypes {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(self.bits().count_ones() as usize))?;
        if self.contains(TerrainTypes::Road) {
            seq.serialize_element("road")?;
        }
        if self.contains(TerrainTypes::Soil) {
            seq.serialize_element("soil")?;
        }
        if self.contains(TerrainTypes::Path) {
            seq.serialize_element("path")?;
        }
        if self.contains(TerrainTypes::Stone) {
            seq.serialize_element("stone")?;
        }
        seq.end()
    }
}

impl<'de> Deserialize<'de> for TerrainTypes {
    fn deserialize<D>(deserializer: D) -> Result<TerrainTypes, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct TerrainTypeFlagsVisitor;
        impl<'de> Visitor<'de> for TerrainTypeFlagsVisitor {
            type Value = TerrainTypes;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a sequence of points")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                let mut flags = TerrainTypes::default();
                while let Some(ty) = seq.next_element::<&str>()? {
                    match ty {
                        "road" => flags |= TerrainTypes::Road,
                        "soil" => flags |= TerrainTypes::Soil,
                        "path" => flags |= TerrainTypes::Path,
                        "stone" => flags |= TerrainTypes::Stone,
                        _ => {
                            warn!("Unknown terrain effect type: {}", ty);
                        }
                    }
                }
                Ok(flags)
            }
        }

        deserializer.deserialize_seq(TerrainTypeFlagsVisitor)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Default)]
pub struct TerrainOptions(u8);

bitflags! {
    impl TerrainOptions: u8 {
        /// The terrain effect is continuous with respect to the x-axis. If false, then
        /// the effect is separate by tiles.
        const ContinuousX = 1 << 0;
        /// The terrain effect is continuous with respect to the y-axis. If false, then
        /// the effect is separate by tiles.
        const ContinuousY = 1 << 1;
        /// A hole in the terrain, used for caves or mine entrances.
        const Hole = 1 << 2;
        /// Override the normal smoothing and make the terrain mesh flat.
        const Flatten = 1 << 3;
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct TerrainFxVertexAttr {
    pub(crate) effect: TerrainTypes,
    pub(crate) effect_strength: u8,
    pub(crate) elevation: i8,
    pub(crate) options: TerrainOptions,
}
