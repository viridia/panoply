use core::fmt;
use std::marker::PhantomData;

use serde::{de::Visitor, ser::SerializeMap, Deserialize, Serialize};

use super::{selector::Selector, StyleAsset};

#[derive(Debug, Default, Clone, PartialEq)]
pub struct SelectorsMap(Vec<(Selector, Box<StyleAsset>)>);

impl SelectorsMap {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self(Vec::with_capacity(capacity))
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn entries(&self) -> &[(Selector, Box<StyleAsset>)] {
        &self.0
    }

    pub fn insert(&mut self, key: Selector, value: Box<StyleAsset>) {
        self.0.push((key, value))
    }
}

impl Serialize for SelectorsMap {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut map = serializer.serialize_map(Some(self.len()))?;
        for (sel, style) in self.0.iter() {
            let key = sel.to_string();
            map.serialize_entry(&key, &style)?
        }

        map.end()
    }
}

impl<'de> Deserialize<'de> for SelectorsMap {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_map(SelectorsMapVisitor {
            marker: &PhantomData,
        })
    }
}

struct SelectorsMapVisitor<'a> {
    marker: &'a PhantomData<()>,
}

impl<'de, 'a> Visitor<'de> for SelectorsMapVisitor<'a> {
    type Value = SelectorsMap;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("selector map")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::MapAccess<'de>,
        A::Error: serde::de::Error,
    {
        let mut result: SelectorsMap = SelectorsMap::with_capacity(map.size_hint().unwrap_or(0));
        while let Some(key) = map.next_key::<String>()? {
            let sel = key
                .parse::<Selector>()
                .map_err(|err| serde::de::Error::custom(err.as_str()))?;
            let style = map.next_value::<StyleAsset>()?;
            result.0.push((sel, Box::new(style)));
        }
        Ok(result)
    }
}
