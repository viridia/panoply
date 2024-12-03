use bevy::prelude::*;
use serde::Deserialize;

/// Serializer for hex color
pub fn serialize<S>(value: &Srgba, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::ser::Serializer,
{
    serializer.serialize_str(&value.to_hex())
}

/// Deserializer hex color
pub fn deserialize<'de, D>(deserializer: D) -> Result<Srgba, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    let hex = String::deserialize(deserializer)?;
    let color = Srgba::hex(hex).unwrap();
    Ok(color)
}
