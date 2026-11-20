//! What both generators share: the agent a mesh is for, and the colliders' shapes where they
//! stand.

use bevy::math::{Quat, Vec2, Vec3};
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

/// The sides of a round shape's triangles, and of the polygon a footprint is grown by.
pub(crate) const ROUND_SEGMENTS: u32 = 16;

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

/// The convex hull of `points` grown by `grow`, counter-clockwise in x and y: each point ringed by
/// a polygon circumscribing a circle of `grow`, so the hull covers every point within `grow` of
/// the shape.
pub(crate) fn grown_hull(points: impl IntoIterator<Item = Vec2>, grow: f32) -> Vec<Vec2> {
    let mut points: Vec<Vec2> = points.into_iter().collect();
    if grow > 0. {
        let reach = grow / (std::f32::consts::PI / ROUND_SEGMENTS as f32).cos();
        points = points
            .iter()
            .flat_map(|&p| {
                (0..ROUND_SEGMENTS).map(move |i| {
                    let angle = i as f32 / ROUND_SEGMENTS as f32 * std::f32::consts::TAU;
                    p + Vec2::from_angle(angle) * reach
                })
            })
            .collect();
    }
    convex_hull(points)
}

/// The convex hull of `points` (Andrew's monotone chain), counter-clockwise in x and y.
pub(crate) fn convex_hull(mut points: Vec<Vec2>) -> Vec<Vec2> {
    points.sort_by(|a, b| a.x.total_cmp(&b.x).then(a.y.total_cmp(&b.y)));
    points.dedup();
    if points.len() < 3 {
        return points;
    }
    let turn = |o: Vec2, a: Vec2, b: Vec2| (a - o).perp_dot(b - o);
    let mut hull: Vec<Vec2> = Vec::with_capacity(points.len() * 2);
    for pass in 0..2 {
        let start = hull.len();
        let ordered: Box<dyn Iterator<Item = &Vec2>> = if pass == 0 {
            Box::new(points.iter())
        } else {
            Box::new(points.iter().rev())
        };
        for &p in ordered {
            while hull.len() >= start + 2
                && turn(hull[hull.len() - 2], hull[hull.len() - 1], p) <= 0.
            {
                hull.pop();
            }
            hull.push(p);
        }
        // The last point of each chain starts the other.
        hull.pop();
    }
    hull
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_hull_drops_inner_points() {
        let points = vec![
            Vec2::new(0., 0.),
            Vec2::new(1., 0.),
            Vec2::new(0.5, 0.5),
            Vec2::new(1., 1.),
            Vec2::new(0., 1.),
        ];
        let hull = convex_hull(points);
        assert_eq!(hull.len(), 4);
        assert!(!hull.contains(&Vec2::new(0.5, 0.5)));
    }
}
