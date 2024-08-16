use std::fmt;

use bevy::math::Rect;
use serde::{de::Visitor, ser::SerializeSeq};

/// Serializer for Rect as Tuple
pub fn serialize<S>(value: &Vec<Rect>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::ser::Serializer,
{
    let mut state = serializer.serialize_seq(Some(value.len()))?;
    for v in value {
        let rect_tuple = (v.min.x, v.min.y, v.max.x, v.max.y);
        state.serialize_element(&rect_tuple)?;
    }
    state.end()
}

/// Deserializer for Rect as Tuple
pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<Rect>, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    deserializer.deserialize_seq(VecRectVisitor)
}

struct VecRectVisitor;

impl<'de> Visitor<'de> for VecRectVisitor {
    type Value = Vec<Rect>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a Rect")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        let mut v: Vec<Rect> = Vec::with_capacity(seq.size_hint().unwrap_or(0));
        while let Some((x0, y0, x1, y1)) = seq.next_element()? {
            v.push(Rect::new(x0, y0, x1, y1));
        }
        Ok(v)
    }
}
