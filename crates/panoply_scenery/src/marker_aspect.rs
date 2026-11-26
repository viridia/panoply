//! Aspects of marker fixtures: fixtures with no model, standing for something abstract (a waymark,
//! an area, a route), which the editor draws and the game reads.
use std::sync::{Arc, LazyLock};

use bevy::{
    annotations::{DisplayName, Step},
    prelude::*,
};
use panoply_core::HexColor;
use panoply_exemplar::{annotations::AspectIcon, *};
use serde::{Deserialize, Serialize};

/// A marker's pin, drawn by the editor at the fixture.
#[derive(Component, Debug, Reflect, Clone, PartialEq, Serialize, Deserialize)]
#[reflect(Aspect, Default, Serialize, Deserialize)]
#[reflect(@AspectIcon("editor/icons/lucide/map-pin.png"))]
#[serde(default)]
pub struct MarkerPin {
    /// The color of the pin, and of the area or path it marks.
    #[reflect(@DisplayName("Color"))]
    pub color: HexColor,
}

impl Default for MarkerPin {
    fn default() -> Self {
        Self {
            color: HexColor(Srgba::rgb_u8(0xff, 0xdd, 0x88)),
        }
    }
}

impl NativeAspect for MarkerPin {
    type Config = Self;

    fn config(&self) -> Self {
        self.clone()
    }

    fn can_attach(&self, kind: InstanceKind) -> bool {
        kind == InstanceKind::Fixture
    }
}

/// An area on the ground, centered on the fixture, such as where an actor wanders or where a piece
/// of music plays. It ignores the fixture's facing, so a rect's sides lie along the world's axes
/// and a point is tested against it by its offset alone.
#[derive(Component, Debug, Reflect, Clone, Default, PartialEq, Serialize, Deserialize)]
#[reflect(Aspect, Default, Serialize, Deserialize)]
#[reflect(@AspectIcon("editor/icons/lucide/circle-dashed.png"))]
pub struct MarkerArea {
    #[serde(flatten)]
    #[reflect(@DisplayName("Shape"))]
    pub shape: MarkerAreaShape,
}

/// A [`MarkerArea`]'s outline, around the fixture.
#[derive(Debug, Reflect, Clone, PartialEq, Serialize, Deserialize)]
#[reflect(Default, Serialize, Deserialize)]
#[serde(tag = "shape", rename_all = "snake_case")]
pub enum MarkerAreaShape {
    Circle {
        #[reflect(@Step(0.05), @DisplayName("Radius"))]
        radius: f32,
    },
    /// Half the rect's width along x, and half its depth along z.
    Rect {
        #[reflect(@Step(0.05), @DisplayName("Half width"))]
        half_x: f32,
        #[reflect(@Step(0.05), @DisplayName("Half depth"))]
        half_z: f32,
    },
}

impl Default for MarkerAreaShape {
    fn default() -> Self {
        Self::Circle { radius: 2. }
    }
}

impl MarkerArea {
    /// Whether a point `offset` from the fixture is inside the area, seen from above.
    pub fn contains(&self, offset: Vec3) -> bool {
        match self.shape {
            MarkerAreaShape::Circle { radius } => offset.xz().length_squared() <= radius * radius,
            MarkerAreaShape::Rect { half_x, half_z } => {
                offset.x.abs() <= half_x && offset.z.abs() <= half_z
            }
        }
    }
}

impl NativeAspect for MarkerArea {
    type Config = Self;

    fn config(&self) -> Self {
        self.clone()
    }

    fn can_attach(&self, kind: InstanceKind) -> bool {
        kind == InstanceKind::Fixture
    }

    fn requires(&self) -> &[Arc<str>] {
        &*MARKER_PIN
    }
}

/// An ordered route, such as a guard's patrol: one fixture for the whole route, so that moving the
/// fixture moves every point. Its facing leaves them where they are.
#[derive(Component, Debug, Reflect, Clone, Default, PartialEq, Serialize, Deserialize)]
#[reflect(Aspect, Default, Serialize, Deserialize)]
#[reflect(@AspectIcon("editor/icons/lucide/route.png"))]
#[serde(default)]
pub struct MarkerPath {
    /// The route's points in order, as offsets from the fixture.
    #[reflect(@Step(0.05), @DisplayName("Points"))]
    pub points: Vec<Vec3>,
}

impl NativeAspect for MarkerPath {
    type Config = Self;

    fn config(&self) -> Self {
        self.clone()
    }

    fn can_attach(&self, kind: InstanceKind) -> bool {
        kind == InstanceKind::Fixture
    }

    fn requires(&self) -> &[Arc<str>] {
        &*MARKER_PIN
    }
}

/// What [`MarkerArea`] and [`MarkerPath`] require: a pin, whose color they are drawn in.
static MARKER_PIN: LazyLock<[Arc<str>; 1]> =
    LazyLock::new(|| [MarkerPin::short_type_path().into()]);

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::reflect::{
        serde::{TypedReflectDeserializer, TypedReflectSerializer},
        TypeRegistry,
    };
    use serde::de::DeserializeSeed;

    fn registry() -> TypeRegistry {
        let mut registry = TypeRegistry::default();
        registry.register::<MarkerPin>();
        registry.register::<MarkerArea>();
        registry.register::<MarkerAreaShape>();
        registry.register::<MarkerPath>();
        registry.register::<HexColor>();
        registry
    }

    /// `value` written and read back through the reflected serde a precinct's msgpack uses.
    fn round_trip<T: Reflect + FromReflect + bevy::reflect::Typed + TypePath>(value: &T) -> T {
        let registry = registry();
        let packed = rmp_serde::to_vec_named(&TypedReflectSerializer::new(value, &registry))
            .expect("serializes");
        let registration = registry.get(std::any::TypeId::of::<T>()).unwrap();
        let read = TypedReflectDeserializer::new(registration, &registry)
            .deserialize(&mut rmp_serde::Deserializer::from_read_ref(&packed))
            .expect("deserializes");
        T::from_reflect(&*read).unwrap()
    }

    #[test]
    fn markers_round_trip_through_msgpack() {
        let rect = MarkerArea {
            shape: MarkerAreaShape::Rect {
                half_x: 35.5,
                half_z: 36.3,
            },
        };
        assert_eq!(round_trip(&rect), rect);
        let circle = MarkerArea::default();
        assert_eq!(round_trip(&circle), circle);
        let path = MarkerPath {
            points: vec![Vec3::new(1., 0., 2.), Vec3::new(-3., 0.5, 0.)],
        };
        assert_eq!(round_trip(&path), path);
        let pin = MarkerPin {
            color: HexColor(Srgba::rgb_u8(0x84, 0xb6, 0x20)),
        };
        assert_eq!(round_trip(&pin), pin);
    }

    #[test]
    fn an_area_reads_from_its_catalog_form() {
        let area: MarkerArea =
            serde_json::from_str(r#"{"shape": "rect", "half_x": 2, "half_z": 1}"#).unwrap();
        assert!(area.contains(Vec3::new(1.9, 5., -0.9)));
        assert!(!area.contains(Vec3::new(0., 0., 1.1)));
        let pin: MarkerPin = serde_json::from_str("{}").unwrap();
        assert_eq!(pin, MarkerPin::default());
    }
}
