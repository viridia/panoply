use std::ops::Deref;

use bevy::prelude::*;
use bevy::reflect::{Reflect, ReflectDeserialize, ReflectSerialize};
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, PartialEq, Clone, Copy, Reflect)]
#[reflect(PartialEq, Serialize, Deserialize)]
pub struct HexColor(pub Color);

impl Serialize for HexColor {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let [r, g, b, a] = self.0.as_rgba_u8();
        let hex = if a == 255 {
            format!("#{:02X}{:02X}{:02X}", r, g, b)
        } else {
            format!("#{:02X}{:02X}{:02X}{:02X}", r, g, b, a)
        };
        serializer.serialize_str(&hex)
    }
}

impl<'de> Deserialize<'de> for HexColor {
    fn deserialize<D>(deserializer: D) -> Result<HexColor, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let hex = String::deserialize(deserializer)?;
        let color = Color::hex(hex).unwrap();
        Ok(HexColor(color))
    }
}

impl Deref for HexColor {
    type Target = Color;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
