//! Msgpack extension for three.js types
use bevy::prelude::*;
use serde::{de::Visitor, ser::SerializeTuple, Deserialize, Serialize};
use std::fmt;

extern crate rmp_serde as rmps;

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename = "_ExtStruct")]
struct ExtStruct((i8, serde_bytes::ByteBuf));

#[derive(Debug, Clone, Copy)]
pub struct Vector3 {
    x: f32,
    y: f32,
    z: f32,
}

impl Serialize for Vector3 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        let mut state = serializer.serialize_tuple(4)?;
        state.serialize_element(&self.x)?;
        state.serialize_element(&self.y)?;
        state.serialize_element(&self.z)?;
        state.end()
    }
}

impl From<Vector3> for Vec3 {
    fn from(v: Vector3) -> Self {
        Vec3::new(v.x, v.y, v.z)
    }
}

struct Vector3Visitor;

impl<'de> Visitor<'de> for Vector3Visitor {
    type Value = Vector3;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a box")
    }

    fn visit_newtype_struct<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let ext = ExtStruct::deserialize(deserializer)?;
        let (x, y, z) =
            rmps::from_slice::<(f32, f32, f32)>(ext.0 .1.into_vec().as_slice()).unwrap();
        Ok(Vector3 { x, y, z })
    }
}

impl<'de> Deserialize<'de> for Vector3 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_newtype_struct("Vector3", Vector3Visitor)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Box2d {
    min: Vec2,
    max: Vec2,
}

impl Serialize for Box2d {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        let mut state = serializer.serialize_tuple(4)?;
        state.serialize_element(&self.min.x)?;
        state.serialize_element(&self.min.y)?;
        state.serialize_element(&self.max.x)?;
        state.serialize_element(&self.max.y)?;
        state.end()
    }
}

struct Box2dVisitor;

impl<'de> Visitor<'de> for Box2dVisitor {
    type Value = Box2d;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a box")
    }

    fn visit_newtype_struct<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let ext = ExtStruct::deserialize(deserializer)?;
        let (x0, y0, x1, y1) =
            rmps::from_slice::<(f32, f32, f32, f32)>(ext.0 .1.into_vec().as_slice()).unwrap();
        Ok(Box2d {
            min: Vec2::new(x0, y0),
            max: Vec2::new(x1, y1),
        })
    }
}

impl<'de> Deserialize<'de> for Box2d {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_newtype_struct("Box2", Box2dVisitor)
    }
}
