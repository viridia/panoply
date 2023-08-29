use core::fmt;
use std::{borrow::Cow, marker::PhantomData};

use bevy::utils::HashMap;
use serde::{de::Visitor, ser::SerializeMap, Deserialize, Serialize};

use super::Expr;

#[derive(Debug, Default, Clone)]
pub struct VarsMap<'a>(HashMap<Cow<'a, str>, Expr>);

impl<'a> VarsMap<'a> {
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self(HashMap::with_capacity(capacity))
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn get(&self, key: &str) -> Option<&Expr> {
        self.0.get(key)
    }

    pub fn insert(&mut self, key: &str, value: Expr) -> Option<Expr> {
        self.0.insert(key.to_owned().into(), value)
    }
}

impl<'a> Serialize for VarsMap<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut map = serializer.serialize_map(Some(self.len()))?;
        for (key, expr) in self.0.iter() {
            map.serialize_entry(format!("--{}", key).as_str(), expr)?;
        }

        map.end()
    }
}

impl<'de, 'a> Deserialize<'de> for VarsMap<'a> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_map(VarsMapVisitor::<'a> {
            marker: &PhantomData,
        })
    }
}
struct VarsMapVisitor<'a> {
    marker: &'a PhantomData<()>,
}

impl<'de, 'a> Visitor<'de> for VarsMapVisitor<'a> {
    type Value = VarsMap<'a>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("style definition")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::MapAccess<'de>,
        A::Error: serde::de::Error,
    {
        let mut result: HashMap<Cow<'a, str>, Expr> =
            HashMap::with_capacity(map.size_hint().unwrap_or(0));
        while let Some(key) = map.next_key::<String>()? {
            let expr = map.next_value::<Expr>()?;
            if key.starts_with("--") {
                result.insert(key[2..].to_owned().into(), expr);
            } else {
                return Err(serde::de::Error::invalid_type(
                    serde::de::Unexpected::Str(&key),
                    &"Expression list",
                ));
            }
        }
        Ok(VarsMap(result))
    }
}
