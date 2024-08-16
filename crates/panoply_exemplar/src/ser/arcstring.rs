use std::sync::Arc;

use serde::Deserialize;

/// Serializer for Arc<String>
pub fn serialize<S>(value: &Arc<String>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::ser::Serializer,
{
    serializer.serialize_str(value)
}

/// Deserializer for Arc<String>
pub fn deserialize<'de, D>(deserializer: D) -> Result<Arc<String>, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    Ok(Arc::new(s))
}
