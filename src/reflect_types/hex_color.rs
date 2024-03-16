use std::ops::Deref;

use bevy::prelude::*;
use bevy::reflect::{Reflect, ReflectDeserialize, ReflectSerialize};
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, PartialEq, Clone, Copy, Reflect)]
#[reflect(PartialEq, Serialize, Deserialize)]
pub struct HexColor(pub Srgba);

impl Serialize for HexColor {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.0.to_hex())
    }
}

impl<'de> Deserialize<'de> for HexColor {
    fn deserialize<D>(deserializer: D) -> Result<HexColor, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let hex = String::deserialize(deserializer)?;
        let color = Srgba::hex(hex).unwrap();
        Ok(HexColor(color))
    }
}

impl Deref for HexColor {
    type Target = Srgba;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<HexColor> for LinearRgba {
    fn from(color: HexColor) -> Self {
        color.0.into()
    }
}
