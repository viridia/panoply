//! Physics colliders for scenery elements.
use std::f32::consts::FRAC_PI_2;

use bevy::{
    annotations::{DisplayName, Step},
    prelude::*,
};
use panoply_core::{realm_physics::collision_groups, OwnedColliders, RealmPhysics, ToRapier};
use panoply_exemplar::Exemplar;
use rapier3d::prelude::{
    ColliderBuilder, Group, InteractionGroups, InteractionTestMode, Pose, SharedShape,
};
use serde::{Deserialize, Serialize};

use crate::{
    precinct::Precinct,
    scenery_aspect::SceneryColliders,
    scenery_element::{SceneryElement, SceneryElementRebuildColliders},
    WALL_TYPE,
};

/// A collider's shape and its dimensions, which differ by shape. Every shape but the ellipsoid
/// rests on the descriptor's `offset`, the center of its base.
#[derive(Debug, Reflect, Clone, PartialEq, Serialize, Deserialize)]
#[reflect(Default, Serialize, Deserialize)]
#[serde(tag = "shape", rename_all = "snake_case")]
pub enum ColliderShape {
    Box {
        #[reflect(@Step(0.05))]
        half_extents: Vec3,
    },
    /// A box with rounded edges, built as a plain box for now.
    Rbox {
        #[reflect(@Step(0.05))]
        half_extents: Vec3,
    },
    Sphere {
        #[reflect(@Step(0.05))]
        radius: f32,
    },
    /// Centered on the offset, not resting on it.
    Ellipsoid {
        #[reflect(@Step(0.05))]
        radii: Vec3,
    },
    /// A wedge in a box of these half extents: the floor of the box, rising to its top at +z.
    Ramp {
        #[reflect(@Step(0.05))]
        half_extents: Vec3,
        /// Quarter turns about y, relative to the element.
        #[serde(default)]
        facing: f32,
    },
    Cylinder {
        #[reflect(@Step(0.05))]
        radius: f32,
        #[reflect(@Step(0.05))]
        half_height: f32,
    },
}

impl Default for ColliderShape {
    fn default() -> Self {
        Self::Box {
            half_extents: Vec3::new(0.5, 1., 0.5),
        }
    }
}

#[derive(Debug, Reflect, Clone, Default, PartialEq, Serialize, Deserialize)]
#[reflect(Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ColliderType {
    #[default]
    Solid, // Default type
    Door,   // Openable door, affects pathfinding in game
    Ladder, // Climable ladder
    Sensor, // Generates an event when actor intersects
    Hint,   // This collider is a hint to the pathfinder, but does not actually block movement.
    Portal, // Portal volume
    Marker, // Means this collider is a proxy for picking in the editor, but has no effect in game.
}

/// Describes a physics collider on a scenery element.
#[derive(Debug, Reflect, Clone, Default, Serialize, Deserialize)]
#[reflect(Default, Serialize, Deserialize)]
pub struct ColliderDesc {
    #[serde(flatten)]
    pub shape: ColliderShape,

    /// Named `type` in a catalog. Not `r#type` here: a reflect path reads `#` as a field index.
    #[serde(rename = "type", default, skip_serializing_if = "is_solid")]
    #[reflect(@DisplayName("type"))]
    pub kind: ColliderType,

    #[reflect(@Step(0.05))]
    pub offset: Option<Vec3>,

    /** Whether this collider receives mouse events. */
    pub pickable: Option<bool>,

    /** If true, this is an animated collider that is attached to a sub-component of the tile. */
    pub animation: Option<String>,

    /** For animated tiles,. */
    pub origin: Option<Vec3>,

    /** Means we can walk on top of this collider. Affects pathfinding and physics. */
    pub walkable: Option<bool>,

    /// Extra clearance kept around the collider when navigation maps are built: its footprint
    /// grows by this much before the maps' boolean operations.
    // TODO(PAN16): read when building navigation maps.
    pub margin: Option<f32>,
}

/// Whether a collider is the default type, which a catalog leaves out.
fn is_solid(kind: &ColliderType) -> bool {
    *kind == ColliderType::Solid
}

impl ColliderDesc {
    /// The shape and its pose relative to the element, placed as faery's `createCollider.ts`
    /// placed it, since the content was authored against that: `offset` is the bottom center, so
    /// shapes other than ellipsoids are raised by their half-height; the offset turns with the
    /// element. `None` for descriptors that are not static solids (doors, sensors, hints, markers
    /// and the rest come with the features that use them) and for hulls that fail.
    pub fn shape(&self) -> Option<(SharedShape, Pose)> {
        if self.kind != ColliderType::Solid || self.animation.is_some() {
            return None;
        }
        let offset = self.offset.unwrap_or_default();
        let mut rotation = Quat::IDENTITY;
        let (shape, lift) = match self.shape {
            ColliderShape::Box { half_extents: size } | ColliderShape::Rbox { half_extents: size } => {
                (SharedShape::cuboid(size.x, size.y, size.z), size.y)
            }
            ColliderShape::Cylinder {
                radius,
                half_height,
            } => (SharedShape::cylinder(half_height, radius), half_height),
            ColliderShape::Sphere { radius } => (SharedShape::ball(radius), radius),
            ColliderShape::Ellipsoid { radii } => (convex_hull(ellipsoid_points(radii))?, 0.),
            ColliderShape::Ramp {
                half_extents,
                facing,
            } => {
                rotation = Quat::from_rotation_y(facing * FRAC_PI_2);
                (convex_hull(ramp_points(half_extents))?, half_extents.y)
            }
        };
        Some((
            shape,
            Pose::from_parts((offset + Vec3::Y * lift).to_rapier(), rotation.to_rapier()),
        ))
    }
}

fn convex_hull(points: impl IntoIterator<Item = Vec3>) -> Option<SharedShape> {
    let points: Vec<_> = points.into_iter().map(ToRapier::to_rapier).collect();
    SharedShape::convex_hull(&points)
}

/// A wedge in a box of half-extents `size`: the floor of the box, rising to its top at +z.
fn ramp_points(size: Vec3) -> [Vec3; 6] {
    let Vec3 { x, y, z } = size;
    [
        Vec3::new(-x, -y, -z),
        Vec3::new(x, -y, -z),
        Vec3::new(-x, -y, z),
        Vec3::new(x, -y, z),
        Vec3::new(-x, y, z),
        Vec3::new(x, y, z),
    ]
}

/// Points on an ellipsoid of radii `size`: the poles and 7 rings of 12, as faery's
/// `makeEllipsoid` made them.
fn ellipsoid_points(size: Vec3) -> Vec<Vec3> {
    const RINGS: usize = 7;
    const SEGMENTS: usize = 12;
    let mut points = vec![Vec3::new(0., size.y, 0.), Vec3::new(0., -size.y, 0.)];
    for ring in 0..RINGS {
        let theta = (ring + 1) as f32 / (RINGS + 1) as f32 * std::f32::consts::PI;
        for segment in 0..SEGMENTS {
            let phi = segment as f32 / SEGMENTS as f32 * std::f32::consts::TAU;
            points.push(Vec3::new(
                phi.sin() * theta.sin() * size.x,
                theta.cos() * size.y,
                phi.cos() * theta.sin() * size.z,
            ));
        }
    }
    points
}

/// Builds an element's colliders on its precinct's body, from its `SceneryColliders`, after its
/// aspects are applied or it moves. The element owns them (`OwnedColliders`), so replacing or
/// despawning it removes them.
pub fn build_se_colliders(
    mut commands: Commands,
    q_elements: Query<
        (Entity, &SceneryElement, &ChildOf, Option<&SceneryColliders>),
        With<SceneryElementRebuildColliders>,
    >,
    q_precincts: Query<&Precinct>,
    mut q_realms: Query<&mut RealmPhysics>,
    exemplars: Res<Assets<Exemplar>>,
) {
    for (entity, element, parent, colliders) in q_elements.iter() {
        let mut element_commands = commands.entity(entity);
        element_commands.remove::<SceneryElementRebuildColliders>();
        let Some(colliders) = colliders else {
            element_commands.remove::<OwnedColliders>();
            continue;
        };
        let Ok(precinct) = q_precincts.get(parent.parent()) else {
            continue;
        };
        let Ok(mut physics) = q_realms.get_mut(precinct.realm) else {
            continue;
        };
        let group = match exemplars.get(&element.exemplar) {
            Some(exemplar) if exemplar.0.meta_type == WALL_TYPE => collision_groups::WALL,
            _ => collision_groups::FIXTURE,
        };
        let placement = Pose::from_parts(
            element.position.to_rapier(),
            Quat::from_rotation_y(element.facing).to_rapier(),
        );
        let handles = colliders
            .0
            .iter()
            .filter_map(ColliderDesc::shape)
            .map(|(shape, pose)| {
                let collider = ColliderBuilder::new(shape)
                    .position(placement * pose)
                    .collision_groups(InteractionGroups::new(
                        group,
                        Group::ALL,
                        InteractionTestMode::And,
                    ))
                    .user_data(entity.to_bits() as u128)
                    .build();
                physics.insert_collider(precinct.physics, collider)
            })
            .collect();
        element_commands.insert(OwnedColliders {
            realm: precinct.realm,
            handles,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_shape_is_a_key_beside_its_dimensions() {
        let text = r#"{"shape":"ramp","half_extents":[1,0.5,1],"facing":2,"offset":[0,0,1]}"#;
        let desc: ColliderDesc = serde_json::from_str(text).unwrap();
        assert_eq!(
            desc.shape,
            ColliderShape::Ramp {
                half_extents: Vec3::new(1., 0.5, 1.),
                facing: 2.,
            }
        );
        let written = serde_json::to_value(&desc).unwrap();
        assert_eq!(written["shape"], "ramp");
        assert_eq!(written["facing"], 2.0);
        // A solid collider, the default, leaves its type out.
        assert!(written.get("type").is_none());
    }

    #[test]
    fn the_type_is_read_and_written_as_type() {
        let text = r#"{"shape":"box","half_extents":[1,1,1],"type":"door"}"#;
        let desc: ColliderDesc = serde_json::from_str(text).unwrap();
        assert_eq!(desc.kind, ColliderType::Door);
        assert_eq!(serde_json::to_value(&desc).unwrap()["type"], "door");
    }
}
