//! A precinct's navigation mesh built by Recast ([rerecast](https://github.com/janhohenheim/rerecast)'s
//! core crate), one of the two generators PAN16e compares. The pipeline follows `bevy_rerecast`'s
//! `generate_navmesh`, with the precinct built as a Recast tile from its own geometry alone.
//!
//! Recast voxelizes: it rasterizes the input's triangles into columns of solid spans, keeps the
//! tops a body of the agent's size can stand on, erodes them by the agent's radius, and traces the
//! regions that remain back into convex polygons, with a detail mesh of triangles following the
//! ground under each.

use std::collections::HashMap;

use bevy::math::{Quat, Vec2, Vec3, Vec3Swizzles};
use panoply_core::{
    metrics::{PRECINCT_SIZE, PRECINCT_SIZE_F},
    ToBevy,
};
use panoply_scenery::{ColliderShape, ColliderType};
use rapier3d::parry::shape::TypedShape;
use rerecast::{
    AreaType, BuildContoursFlags, CompactHeightfield, ConvexVolume, DetailNavmesh, Heightfield,
    HeightfieldBuilder, PolygonNavmesh, Span, TriMesh,
};
use thiserror::Error;

use crate::{
    NavAreaKind, NavCollider, NavEdge, NavInput, NavMesh, NavPolygon, PrecinctSide, SurfaceKind,
};

/// The agent the mesh is for: a human's capsule (`characters.exem.json`).
const AGENT_RADIUS: f32 = 0.3;
const AGENT_HEIGHT: f32 = 1.4;

/// The steepest walkable ground: a normal's y of at least 0.7, as the character controller has
/// (`WALKABLE_NORMAL_Y` in `actor_physics.rs`).
const WALKABLE_NORMAL_Y: f32 = 0.7;

/// The highest ledge counted as walkable from one cell to the next. A 45° slope rises a cell's
/// width per cell, so this must exceed the largest cell size, or walkable slopes break apart.
const WALKABLE_CLIMB: f32 = 0.2;

/// Rerecast's defaults: polygons of at most six vertices; regions under 8² cells dropped and under
/// 20² merged; contours kept within 1.3 cells, their edges at most 8 agent radii long; the detail
/// mesh sampled every 6 cells, within one cell height.
const MAX_VERTICES_PER_POLYGON: u16 = 6;
const MIN_REGION_AREA: u16 = 8 * 8;
const MERGE_REGION_AREA: u16 = 20 * 20;
const MAX_SIMPLIFICATION_ERROR: f32 = 1.3;
const MAX_EDGE_RADII: u16 = 8;
const DETAIL_SAMPLE_CELLS: f32 = 6.;
const DETAIL_MAX_ERROR_CELLS: f32 = 1.;

/// The areas a door's footprint and a footpath mark; Recast reserves 0 for unwalkable and 255 for
/// walkable.
const DOOR_AREA: AreaType = AreaType(1);
const PATH_AREA: AreaType = AreaType(2);

/// How far a footpath's area reaches either side of its cells' middles. Narrower than the cells,
/// as faery's paths were, so routes keep off a path's lip; wide enough to stay several cells
/// across on the grid.
const PATH_HALF_WIDTH: f32 = 0.3;

/// The sides of a round shape's triangles, and of the polygon a footprint is grown by.
const ROUND_SEGMENTS: u32 = 16;

/// How Recast builds a mesh.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RecastSettings {
    /// The width of a cell, in meters. A precinct's side must be a whole number of cells.
    pub cell_size: f32,
}

impl Default for RecastSettings {
    /// 0.1 m: a third of the agent's radius, Recast's own rule, and fine enough that the cottage's
    /// 0.8 m doorway keeps 2 cells after erosion takes 3 from each side.
    fn default() -> Self {
        Self { cell_size: 0.1 }
    }
}

/// Why a mesh could not be built: a step of Recast's failed.
#[derive(Debug, Error, PartialEq)]
#[error("recast: {0}")]
pub struct RecastError(String);

impl RecastError {
    fn from(error: impl std::fmt::Display) -> Self {
        Self(error.to_string())
    }
}

/// The grid a build works on, derived from its settings.
struct Grid {
    cell_size: f32,
    cell_height: f32,
    /// Cells along a precinct's side.
    tile: u16,
    /// Cells of padding on every side of the precinct, and the agent's radius in cells.
    border: u16,
    radius: u16,
    height: u16,
    climb: u16,
}

impl Grid {
    fn new(settings: &RecastSettings) -> Self {
        let cell_size = settings.cell_size;
        let cell_height = cell_size / 2.;
        let radius = (AGENT_RADIUS / cell_size).ceil() as u16;
        Self {
            cell_size,
            cell_height,
            tile: (PRECINCT_SIZE_F / cell_size).round() as u16,
            // Erosion treats the grid's edge as a wall; the padding keeps that away from the tile.
            border: radius + 3,
            radius,
            height: (AGENT_HEIGHT / cell_height).ceil() as u16,
            climb: (WALKABLE_CLIMB / cell_height).floor() as u16,
        }
    }

    /// A vertex's x or z in cells from the tile's corner, in meters, exact on the precinct's
    /// lines.
    fn meters(&self, cells: u16) -> f32 {
        cells as f32 * PRECINCT_SIZE_F / self.tile as f32
    }
}

/// Builds `input`'s navigation mesh.
pub fn build(input: &NavInput, settings: &RecastSettings) -> Result<NavMesh, RecastError> {
    let grid = Grid::new(settings);
    let trimesh = triangles(input);
    let Some(bounds) = trimesh.compute_aabb() else {
        return Ok(NavMesh::default());
    };
    let pad = grid.border as f32 * grid.cell_size;
    let mut heightfield = HeightfieldBuilder {
        aabb: rerecast::Aabb3d {
            min: [-pad, bounds.min.y - 1., -pad].into(),
            max: [
                PRECINCT_SIZE_F + pad,
                bounds.max.y + 1.,
                PRECINCT_SIZE_F + pad,
            ]
            .into(),
        },
        cell_size: grid.cell_size,
        cell_height: grid.cell_height,
    }
    .build()
    .map_err(RecastError::from)?;
    heightfield
        .rasterize_triangles(&trimesh, grid.climb)
        .map_err(RecastError::from)?;
    extend_edges(&mut heightfield, &grid);

    heightfield.filter_low_hanging_walkable_obstacles(grid.climb);
    heightfield.filter_ledge_spans(grid.height, grid.climb);
    heightfield.filter_walkable_low_height_spans(grid.height);
    let mut compact = heightfield
        .into_compact(grid.height, grid.climb)
        .map_err(RecastError::from)?;
    compact.erode_walkable_area(grid.radius);
    mark_areas(&mut compact, input);

    compact.build_distance_field();
    compact
        .build_regions(grid.border, MIN_REGION_AREA, MERGE_REGION_AREA)
        .map_err(RecastError::from)?;
    let contours = compact.build_contours(
        MAX_SIMPLIFICATION_ERROR,
        grid.radius * MAX_EDGE_RADII,
        BuildContoursFlags::default(),
    );
    let polygons = contours
        .into_polygon_mesh(MAX_VERTICES_PER_POLYGON)
        .map_err(RecastError::from)?;
    let detail = DetailNavmesh::new(
        &polygons,
        &compact,
        grid.cell_size * DETAIL_SAMPLE_CELLS,
        grid.cell_height * DETAIL_MAX_ERROR_CELLS,
    )
    .map_err(RecastError::from)?;
    Ok(nav_mesh(&polygons, &detail, &grid))
}

/// The input as triangles: the ground and floors walkable where their slope allows, and the solid
/// colliders' surfaces, walkable only on a collider marked walkable. Doors, hints and the other
/// types are not rasterized: [`mark_areas`] takes doors and hints.
fn triangles(input: &NavInput) -> TriMesh {
    let mut trimesh = TriMesh::default();
    let mut add = |vertices: &[Vec3], indices: &[[u32; 3]], walkable: bool| {
        let mut part = TriMesh {
            vertices: vertices.iter().map(|v| v.to_array().into()).collect(),
            indices: indices.iter().map(|&i| i.into()).collect(),
            area_types: vec![AreaType::NOT_WALKABLE; indices.len()],
        };
        if walkable {
            part.mark_walkable_triangles(WALKABLE_NORMAL_Y.acos());
        }
        trimesh.extend(part);
    };
    for surface in &input.surfaces {
        debug_assert!(matches!(
            surface.kind,
            SurfaceKind::Ground | SurfaceKind::Floor
        ));
        add(&surface.vertices, &surface.indices, true);
    }
    for collider in &input.colliders {
        if collider.collider.kind != ColliderType::Solid {
            continue;
        }
        if let Some((vertices, indices)) = placed_triangles(collider) {
            add(
                &vertices,
                &indices,
                collider.collider.walkable == Some(true),
            );
        }
    }
    trimesh
}

/// A collider's volume as triangles, where it stands in the precinct.
fn placed_triangles(collider: &NavCollider) -> Option<(Vec<Vec3>, Vec<[u32; 3]>)> {
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

/// Fills the padding around the precinct with copies of its edge columns, each padding column
/// taking the spans of the nearest column inside: the precinct is built from its own geometry
/// alone, and padding left empty would erode the mesh back from the precinct's lines. So open
/// ground stays open to the line, and a wall meeting the line continues past it.
fn extend_edges(heightfield: &mut Heightfield, grid: &Grid) {
    let (first, last) = (grid.border, grid.border + grid.tile - 1);
    let width = heightfield.width;
    for z in 0..heightfield.height {
        for x in 0..width {
            let (source_x, source_z) = (x.clamp(first, last), z.clamp(first, last));
            if (source_x, source_z) == (x, z) {
                continue;
            }
            let column = |x: u16, z: u16| x as usize + z as usize * width as usize;

            // The source column's spans, bottom to top.
            let mut copied = Vec::new();
            let mut key = heightfield.spans[column(source_x, source_z)];
            while let Some(k) = key {
                let span = heightfield.span(k);
                copied.push((span.min, span.max, span.area));
                key = span.next;
            }

            // Whatever was rasterized into the padding goes.
            let mut key = heightfield.spans[column(x, z)].take();
            while let Some(k) = key {
                key = heightfield.allocated_spans.remove(k).and_then(|s| s.next);
            }

            // Linked from the top down, so each span's next is already in place.
            let mut next = None;
            for (min, max, area) in copied.into_iter().rev() {
                next = Some(heightfield.allocated_spans.insert(Span {
                    min,
                    max,
                    area,
                    next,
                }));
            }
            heightfield.spans[column(x, z)] = next;
        }
    }
}

/// Marks, after erosion, the areas the input gives: footpaths, then doors, then what blocks, each
/// over the last where they overlap, so a hint beside a door wins. Marking only changes spans
/// already walkable.
fn mark_areas(compact: &mut CompactHeightfield, input: &NavInput) {
    let volumes = path_volumes(input)
        .into_iter()
        .chain(
            input
                .colliders
                .iter()
                .filter(|c| c.collider.kind == ColliderType::Door)
                .filter_map(door_volume),
        )
        .chain(input.colliders.iter().filter_map(blocking_volume));
    for volume in volumes {
        compact.mark_convex_poly_area(&volume);
    }
}

/// Each footpath cell's area, as faery shaped it (`addTerrainPaths`): a square about the cell's
/// middle, an arm to each side whose neighbour is a path, and a corner filled where both sides and
/// the diagonal are, all [`PATH_HALF_WIDTH`] from the middle line. Each spans the heights of the
/// ground at the cell's corners, give or take a climb, so a floor over a road is not marked.
fn path_volumes(input: &NavInput) -> Vec<ConvexVolume> {
    let mut ground: HashMap<(i32, i32), f32> = HashMap::new();
    for surface in input
        .surfaces
        .iter()
        .filter(|s| s.kind == SurfaceKind::Ground)
    {
        for v in &surface.vertices {
            ground.insert((v.x.round() as i32, v.z.round() as i32), v.y);
        }
    }
    let (lo, hi) = (0.5 - PATH_HALF_WIDTH, 0.5 + PATH_HALF_WIDTH);
    let mut volumes = Vec::new();
    for z in 0..PRECINCT_SIZE {
        for x in 0..PRECINCT_SIZE {
            if !input.path_at(x, z) {
                continue;
            }
            let heights: Vec<f32> = [(x, z), (x + 1, z), (x, z + 1), (x + 1, z + 1)]
                .iter()
                .filter_map(|corner| ground.get(corner).copied())
                .collect();
            if heights.is_empty() {
                continue;
            }
            let min_y = heights.iter().copied().fold(f32::MAX, f32::min) - WALKABLE_CLIMB;
            let max_y = heights.iter().copied().fold(f32::MIN, f32::max) + WALKABLE_CLIMB;
            let path = |dx: i32, dz: i32| input.path_at(x + dx, z + dz);
            // Rectangles in the cell, from 0 to 1 along each axis.
            let mut pieces = vec![(lo, hi, lo, hi)];
            let arms = [
                (-1, 0, (0., lo, lo, hi)),
                (1, 0, (hi, 1., lo, hi)),
                (0, -1, (lo, hi, 0., lo)),
                (0, 1, (lo, hi, hi, 1.)),
            ];
            pieces.extend(
                arms.iter()
                    .filter(|(dx, dz, _)| path(*dx, *dz))
                    .map(|a| a.2),
            );
            let corners = [
                (-1, -1, (0., lo, 0., lo)),
                (1, -1, (hi, 1., 0., lo)),
                (-1, 1, (0., lo, hi, 1.)),
                (1, 1, (hi, 1., hi, 1.)),
            ];
            pieces.extend(
                corners
                    .iter()
                    .filter(|(dx, dz, _)| path(*dx, 0) && path(0, *dz) && path(*dx, *dz))
                    .map(|c| c.2),
            );
            for (x0, x1, z0, z1) in pieces {
                let (x0, x1) = (x as f32 + x0, x as f32 + x1);
                let (z0, z1) = (z as f32 + z0, z as f32 + z1);
                volumes.push(ConvexVolume {
                    vertices: vec![
                        [x0, z0].into(),
                        [x1, z0].into(),
                        [x1, z1].into(),
                        [x0, z1].into(),
                    ],
                    min_y,
                    max_y,
                    area: PATH_AREA,
                });
            }
        }
    }
    volumes
}

/// A door's doorway, as the `DOOR` area: the closed leaf's footprint grown by the agent's radius
/// on each side across its thickness. The leaf alone is a cell or two thick on the grid, a sliver
/// that contour simplification would lose and close the doorway with; grown, it spans walls up to
/// 0.6 m thick (the cottage's are 0.4 m).
fn door_volume(door: &NavCollider) -> Option<ConvexVolume> {
    let mut desc = door.collider.clone();
    if let ColliderShape::Box { half_extents } = &mut desc.shape {
        if half_extents.x < half_extents.z {
            half_extents.x += AGENT_RADIUS;
        } else {
            half_extents.z += AGENT_RADIUS;
        }
    }
    let grown = NavCollider {
        collider: desc,
        ..door.clone()
    };
    volume(&grown, 0., DOOR_AREA)
}

/// Where routes do not go: a hint's footprint, and a solid's grown by its margin, each grown by
/// the agent's radius too, since marking comes after erosion and would otherwise let a body
/// overlap it.
fn blocking_volume(collider: &NavCollider) -> Option<ConvexVolume> {
    let desc = &collider.collider;
    let margin = match desc.kind {
        ColliderType::Hint => 0.,
        ColliderType::Solid => desc.margin.filter(|&m| m > 0.)?,
        _ => return None,
    };
    volume(collider, AGENT_RADIUS + margin, AreaType::NOT_WALKABLE)
}

/// A collider's footprint grown by `grow`, from a climb below its bottom to its top, marking
/// `area`.
fn volume(collider: &NavCollider, grow: f32, area: AreaType) -> Option<ConvexVolume> {
    let (vertices, _) = placed_triangles(collider)?;
    let (min_y, max_y) = vertices.iter().fold((f32::MAX, f32::MIN), |(lo, hi), v| {
        (lo.min(v.y), hi.max(v.y))
    });
    let mut points: Vec<Vec2> = vertices.iter().map(|v| v.xz()).collect();
    if grow > 0. {
        // Each point ringed by a polygon circumscribing a circle of `grow`, so the hull of them
        // all covers every point within `grow` of the shape.
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
    let hull = convex_hull(points);
    (hull.len() >= 3).then(|| ConvexVolume {
        vertices: hull.iter().map(|p| p.to_array().into()).collect(),
        min_y: min_y - WALKABLE_CLIMB,
        max_y,
        area,
    })
}

/// The convex hull of `points` (Andrew's monotone chain), counter-clockwise in x and y.
fn convex_hull(mut points: Vec<Vec2>) -> Vec<Vec2> {
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

/// Rerecast's polygons and detail as a [`NavMesh`]: vertices in meters, their heights from the
/// detail mesh, which samples the ground where the polygons' own heights can be off by the climb.
fn nav_mesh(polygons: &PolygonNavmesh, detail: &DetailNavmesh, grid: &Grid) -> NavMesh {
    let origin_y = polygons.aabb.min.y;
    let mut vertices: Vec<Vec3> = polygons
        .vertices
        .iter()
        .map(|v| {
            Vec3::new(
                grid.meters(v.x),
                origin_y + v.y as f32 * polygons.cell_height,
                grid.meters(v.z),
            )
        })
        .collect();
    let mut placed = vec![false; vertices.len()];
    let nvp = polygons.max_vertices_per_polygon as usize;
    let mut mesh_polygons = Vec::with_capacity(polygons.polygon_count());

    for (index, indices) in polygons.polygons().enumerate() {
        let indices: Vec<u32> = indices.map(u32::from).collect();
        let neighbors = &polygons.polygon_neighbors[index * nvp..][..indices.len()];
        let edges = neighbors.iter().map(|&n| edge(n)).collect();

        let sub = &detail.meshes[index];
        let sub_vertices =
            &detail.vertices[sub.base_vertex_index as usize..][..sub.vertex_count as usize];
        // A sub-mesh starts with its polygon's vertices, in order.
        for (&v, sampled) in indices.iter().zip(sub_vertices) {
            if !placed[v as usize] {
                vertices[v as usize].y = sampled.y;
                placed[v as usize] = true;
            }
        }
        let point = |i: u8| {
            let v = sub_vertices[i as usize];
            Vec3::new(v.x, v.y, v.z)
        };
        // Clockwise in rerecast; turned to run as the polygons do.
        let triangles = detail.triangles[sub.base_triangle_index as usize..]
            [..sub.triangle_count as usize]
            .iter()
            .map(|&[a, b, c]| [point(a), point(c), point(b)])
            .collect();

        mesh_polygons.push(NavPolygon {
            vertices: indices,
            edges,
            area: match polygons.areas[index] {
                DOOR_AREA => NavAreaKind::Door,
                PATH_AREA => NavAreaKind::Path,
                _ => NavAreaKind::Walkable,
            },
            detail: triangles,
        });
    }
    NavMesh {
        vertices,
        polygons: mesh_polygons,
    }
}

/// What lies across an edge, from rerecast's neighbor entry: no connection, a polygon, or the
/// tile's side, flagged by the border region's bit.
fn edge(neighbor: u16) -> NavEdge {
    const BORDER: u16 = 0x8000;
    if neighbor == PolygonNavmesh::NO_CONNECTION {
        NavEdge::Wall
    } else if neighbor & BORDER != 0 {
        NavEdge::Boundary(match neighbor & 0x3 {
            0 => PrecinctSide::NegX,
            1 => PrecinctSide::PosZ,
            2 => PrecinctSide::PosX,
            _ => PrecinctSide::NegZ,
        })
    } else {
        NavEdge::Neighbor(neighbor as u32)
    }
}

#[cfg(test)]
mod tests {
    use panoply_scenery::ColliderDesc;

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

    #[test]
    fn a_door_grows_across_its_thickness() {
        let door = NavCollider {
            position: Vec3::ZERO,
            facing: 0.,
            collider: ColliderDesc {
                shape: ColliderShape::Box {
                    half_extents: Vec3::new(0.05, 0.8, 0.4),
                },
                kind: ColliderType::Door,
                ..Default::default()
            },
        };
        let volume = door_volume(&door).unwrap();
        let xs = volume.vertices.iter().map(|v| v.x);
        let width = xs.clone().fold(f32::MIN, f32::max) - xs.fold(f32::MAX, f32::min);
        assert!((width - 0.7).abs() < 1e-4, "{width}");
    }

    #[test]
    fn edges_from_neighbor_entries() {
        assert_eq!(edge(PolygonNavmesh::NO_CONNECTION), NavEdge::Wall);
        assert_eq!(edge(7), NavEdge::Neighbor(7));
        assert_eq!(edge(0x8002), NavEdge::Boundary(PrecinctSide::PosX));
    }
}
