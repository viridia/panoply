use bevy::prelude::*;
use serde::Deserialize;

/// Serializer for optional hex color
pub fn serialize<S>(value: &Option<Srgba>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::ser::Serializer,
{
    match value {
        Some(value) => serializer.serialize_str(&value.to_hex()),
        None => serializer.serialize_none(),
    }
}

/// Deserializer optional hex color
pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<Srgba>, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    let hex = Option::<String>::deserialize(deserializer)?;
    match hex {
        Some(hex) => {
            let color = Srgba::hex(hex).unwrap();
            Ok(Some(color))
        }
        None => Ok(None),
    }
}
