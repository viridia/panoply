//! Physics colliders for scenery elements.
use std::f32::consts::FRAC_PI_2;

use bevy::prelude::*;
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

#[derive(Debug, Default, Reflect, Clone, PartialEq, Serialize, Deserialize)]
#[reflect(Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ColliderShape {
    #[default]
    Box,
    Rbox,
    Sphere,
    Ellipsoid,
    Ramp,
    Cylinder,
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
    pub shape: ColliderShape,

    #[serde(default)]
    pub r#type: ColliderType,

    pub size: Option<Vec3>, // Note that for spheres, only y is used.
    pub offset: Option<Vec3>,

    // Direction facing (0-3) relative to wall tile.
    pub facing: Option<f32>,

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

/// Half-extents when a descriptor has no size.
const DEFAULT_SIZE: Vec3 = Vec3::new(0.5, 1., 0.5);

impl ColliderDesc {
    /// The shape and its pose relative to the element, placed as faery's `createCollider.ts`
    /// placed it, since the content was authored against that: `size` holds half-extents and
    /// `offset` the bottom center, so shapes other than ellipsoids are raised by their
    /// half-height; the offset turns with the element; only ramps turn by `facing`. `None` for
    /// descriptors that are not static solids (doors, sensors, hints, markers and the rest come
    /// with the features that use them) and for hulls that fail.
    pub fn shape(&self) -> Option<(SharedShape, Pose)> {
        if self.r#type != ColliderType::Solid || self.animation.is_some() {
            return None;
        }
        let size = self.size.unwrap_or(DEFAULT_SIZE);
        let offset = self.offset.unwrap_or_default();
        let mut rotation = Quat::IDENTITY;
        let (shape, lift) = match self.shape {
            ColliderShape::Box | ColliderShape::Rbox => {
                (SharedShape::cuboid(size.x, size.y, size.z), size.y)
            }
            ColliderShape::Cylinder => (SharedShape::cylinder(size.y, size.x), size.y),
            ColliderShape::Sphere => {
                let radius = size.max_element();
                (SharedShape::ball(radius), radius)
            }
            ColliderShape::Ellipsoid => (convex_hull(ellipsoid_points(size))?, 0.),
            ColliderShape::Ramp => {
                rotation = Quat::from_rotation_y(self.facing.unwrap_or(0.) * FRAC_PI_2);
                (convex_hull(ramp_points(size))?, size.y)
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
