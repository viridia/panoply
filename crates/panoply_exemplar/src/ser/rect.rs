use std::fmt;

use bevy::math::Rect;
use serde::{de::Visitor, ser::SerializeTuple};

/// Serializer for Rect as Tuple
pub fn serialize<S>(value: &Rect, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::ser::Serializer,
{
    let mut state = serializer.serialize_tuple(4)?;
    state.serialize_element(&value.min.x)?;
    state.serialize_element(&value.min.y)?;
    state.serialize_element(&value.max.x)?;
    state.serialize_element(&value.max.y)?;
    state.end()
}

/// Deserializer for Rect as Tuple
pub fn deserialize<'de, D>(deserializer: D) -> Result<Rect, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    deserializer.deserialize_seq(RectVisitor)
}

struct RectVisitor;

impl<'de> Visitor<'de> for RectVisitor {
    type Value = Rect;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a Rect")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        let mut r = Rect::default();
        if seq.size_hint() != Some(4) {
            return Err(serde::de::Error::invalid_length(
                seq.size_hint().unwrap(),
                &"4",
            ));
        }
        r.min.x = seq.next_element()?.unwrap();
        r.min.y = seq.next_element()?.unwrap();
        r.max.x = seq.next_element()?.unwrap();
        r.max.y = seq.next_element()?.unwrap();
        Ok(r)
    }
}
