//! The agent a mesh is for, and the colliders' shapes where they stand.

use bevy::math::{Quat, Vec3};
use panoply_core::ToBevy;
use rapier3d::parry::shape::TypedShape;

use crate::NavCollider;

/// The agent the mesh is for: a human's capsule (`characters.exem.json`).
pub(crate) const AGENT_RADIUS: f32 = 0.3;
pub(crate) const AGENT_HEIGHT: f32 = 1.4;

/// The steepest walkable ground: a normal's y of at least 0.7, as the character controller has
/// (`WALKABLE_NORMAL_Y` in `actor_physics.rs`).
pub(crate) const WALKABLE_NORMAL_Y: f32 = 0.7;

/// The highest ledge counted as walkable from one surface to the next.
pub(crate) const WALKABLE_CLIMB: f32 = 0.2;

/// The sides of a round shape's triangles.
const ROUND_SEGMENTS: u32 = 16;

/// A collider's volume as triangles, where it stands in the precinct.
pub(crate) fn placed_triangles(collider: &NavCollider) -> Option<(Vec<Vec3>, Vec<[u32; 3]>)> {
    let (shape, pose) = collider.collider.volume()?;
    let (vertices, indices) = match shape.as_typed_shape() {
        TypedShape::Cuboid(cuboid) => cuboid.to_trimesh(),
        TypedShape::Cylinder(cylinder) => cylinder.to_trimesh(ROUND_SEGMENTS),
        TypedShape::Ball(ball) => ball.to_trimesh(ROUND_SEGMENTS, ROUND_SEGMENTS / 2),
        TypedShape::ConvexPolyhedron(hull) => hull.to_trimesh(),
        _ => return None,
    };
    let (offset, turn) = (pose.translation.to_bevy(), pose.rotation.to_bevy());
    let facing = Quat::from_rotation_y(collider.facing);
    let vertices = vertices
        .into_iter()
        .map(|v| collider.position + facing * (offset + turn * v.to_bevy()))
        .collect();
    Some((vertices, indices))
}
