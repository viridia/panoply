use bevy::{asset::Handle, math::Vec2, prelude::*};
use panoply_exemplar::Exemplar;
use serde::{
    de::{SeqAccess, Visitor},
    ser::SerializeSeq,
    Deserialize, Serialize,
};

#[derive(Component, Debug, Clone, Default)]
pub struct FloorRegion {
    /// Floor level
    pub level: i32,

    /// Schematic reference
    pub exemplar: Handle<Exemplar>,

    /// Polygonal outline of floor
    pub poly: Vec<Vec2>,

    /// Holes in the polygon.
    pub holes: Vec<Vec<Vec2>>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct FloorRegionSer {
    /// Index into archetypes table
    #[serde(alias = "type")]
    pub surface_index: usize,

    /// List of 2d polygonal vertices.
    #[serde(
        serialize_with = "serialize_poly",
        deserialize_with = "deserialize_poly"
    )]
    pub poly: Vec<Vec2>,

    #[serde(default)]
    pub holes: Vec<Vec<Vec2>>,
}

impl PartialEq for FloorRegionSer {
    fn eq(&self, other: &Self) -> bool {
        self.surface_index == other.surface_index
            && self.poly == other.poly
            && self.holes == other.holes
    }
}

#[derive(Component)]
#[component(storage = "SparseSet")]
pub struct RebuildFloorAspects;

#[derive(Component)]
#[component(storage = "SparseSet")]
pub struct RebuildFloorMesh;

#[derive(Component)]
#[component(storage = "SparseSet")]
pub struct RebuildFloorMaterials;

fn serialize_poly<S>(poly: &Vec<Vec2>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    let mut seq = serializer.serialize_seq(Some(poly.len()))?;
    for point in poly {
        let tuple: (f32, f32) = (point.x, point.y);
        seq.serialize_element(&tuple)?;
    }
    seq.end()
}

fn deserialize_poly<'de, D>(deserializer: D) -> Result<Vec<Vec2>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    struct PolyVisitor;

    impl<'de> Visitor<'de> for PolyVisitor {
        type Value = Vec<Vec2>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a sequence of points")
        }

        fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where
            A: SeqAccess<'de>,
        {
            let mut points: Vec<Vec2> = Vec::new();
            while let Some(point) = seq.next_element::<(f32, f32)>()? {
                points.push(Vec2::new(point.0, point.1));
            }
            Ok(points)
        }
    }

    deserializer.deserialize_seq(PolyVisitor)
}
