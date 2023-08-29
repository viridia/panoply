use core::fmt;
use std::marker::PhantomData;

use serde::{de::Visitor, ser::SerializeMap, Deserialize, Serialize};

use super::{selector::Selector, Style};

#[derive(Debug, Default, Clone)]
pub struct SelectorsMap<'a>(Vec<(Selector<'a>, Box<Style<'a>>)>);

impl<'a> SelectorsMap<'a> {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self(Vec::with_capacity(capacity))
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn entries(&self) -> &[(Selector<'a>, Box<Style<'a>>)] {
        &self.0
    }

    pub fn insert(&mut self, key: Selector<'a>, value: Box<Style<'a>>) {
        self.0.push((key, value))
    }
}

impl<'a> Serialize for SelectorsMap<'a> {
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

impl<'de, 'a> Deserialize<'de> for SelectorsMap<'a> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_map(SelectorsMapVisitor::<'a> {
            marker: &PhantomData,
        })
    }
}

struct SelectorsMapVisitor<'a> {
    marker: &'a PhantomData<()>,
}

impl<'de, 'a> Visitor<'de> for SelectorsMapVisitor<'a> {
    type Value = SelectorsMap<'a>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("selector map")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::MapAccess<'de>,
        A::Error: serde::de::Error,
    {
        let mut result: SelectorsMap<'a> =
            SelectorsMap::with_capacity(map.size_hint().unwrap_or(0));
        while let Some(key) = map.next_key::<String>()? {
            let sel = key
                .parse::<Selector<'a>>()
                .map_err(|err| serde::de::Error::custom(err.as_str()))?;
            let style = map.next_value::<Style>()?;
            result.0.push((sel, Box::new(style)));
        }
        Ok(result)
    }
}
