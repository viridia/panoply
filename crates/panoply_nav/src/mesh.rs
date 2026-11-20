//! The navigation mesh of one precinct, in the precinct's own coordinates: its corner at the
//! origin, so it spans 0 to [`PRECINCT_SIZE_F`] on x and z, and a boundary edge lies exactly on
//! one of those lines.

use bevy::{
    ecs::{component::Component, reflect::ReflectComponent},
    math::Vec3,
    reflect::{std_traits::ReflectDefault, Reflect},
};
use panoply_core::metrics::PRECINCT_SIZE_F;
use thiserror::Error;

/// How far a vertex may stray from the line it should lie on, or a turn from straight.
const TOLERANCE: f32 = 1e-4;

/// Convex polygons sharing their vertices, each edge naming what lies across it.
#[derive(Debug, Clone, Default, PartialEq, Reflect)]
pub struct NavMesh {
    pub vertices: Vec<Vec3>,
    pub polygons: Vec<NavPolygon>,
}

/// A precinct's navigation mesh, on the precinct's entity.
#[derive(Component, Debug, Clone, Default, PartialEq, Reflect)]
#[reflect(Component, Default)]
pub struct PrecinctNavMesh(pub NavMesh);

/// A convex polygon of a [`NavMesh`].
#[derive(Debug, Clone, PartialEq, Reflect)]
pub struct NavPolygon {
    /// Indices into the mesh's vertices, counter-clockwise seen from above.
    pub vertices: Vec<u32>,
    /// What lies across each edge: edge `i` runs from vertex `i` to vertex `i + 1`, the last
    /// back to the first.
    pub edges: Vec<NavEdge>,
    pub area: NavAreaKind,
    /// The polygon's surface as triangles, where it is not planar (Recast's polygons follow the
    /// ground only at their vertices); empty where the polygon is its own surface.
    #[reflect(default)]
    pub detail: Vec<[Vec3; 3]>,
}

/// What lies across a polygon's edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Reflect)]
pub enum NavEdge {
    /// Nothing walkable.
    Wall,
    /// The polygon of this index, whose edge over the same two vertices names this one.
    Neighbor(u32),
    /// The neighboring precinct on this side, joined to it when a route crosses.
    Boundary(PrecinctSide),
}

/// The kind of ground a polygon is, which routes cost differently.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Reflect)]
pub enum NavAreaKind {
    Walkable,
    /// A doorway, under a door's leaf.
    Door,
    /// A footpath, narrower than its cobbles, so routes keep off its edge.
    Path,
}

/// A side of a precinct.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Reflect)]
pub enum PrecinctSide {
    /// The line x = 0.
    NegX,
    /// The line x = [`PRECINCT_SIZE_F`].
    PosX,
    /// The line z = 0.
    NegZ,
    /// The line z = [`PRECINCT_SIZE_F`].
    PosZ,
}

impl PrecinctSide {
    /// Whether `point` lies on this side's line.
    fn holds(self, point: Vec3) -> bool {
        let (value, line) = match self {
            Self::NegX => (point.x, 0.),
            Self::PosX => (point.x, PRECINCT_SIZE_F),
            Self::NegZ => (point.z, 0.),
            Self::PosZ => (point.z, PRECINCT_SIZE_F),
        };
        (value - line).abs() <= TOLERANCE
    }
}

/// What [`NavMesh::check`] found wrong, naming the polygon and, where it applies, the edge.
#[derive(Debug, Error, PartialEq)]
pub enum NavMeshError {
    #[error("polygon {0} has fewer than three vertices")]
    TooFewVertices(usize),
    #[error("polygon {0} has a different number of edges than vertices")]
    EdgeCount(usize),
    #[error("polygon {0} names a vertex past the mesh's")]
    VertexIndex(usize),
    #[error("polygon {0} is clockwise or not convex at its vertex {1}")]
    NotConvex(usize, usize),
    #[error("polygon {0}'s edge {1} names a polygon past the mesh's, or itself")]
    NeighborIndex(usize, usize),
    #[error("polygon {0}'s edge {1} is not named back over the same vertices")]
    NeighborNotReciprocated(usize, usize),
    #[error("polygon {0}'s edge {1} is not on its side's line")]
    BoundaryOffLine(usize, usize),
}

impl NavMesh {
    /// Checks what every generator's mesh promises: each polygon convex and counter-clockwise
    /// seen from above, an edge per vertex, each neighbor naming this polygon back over the same
    /// two vertices, and each boundary edge on its side's line.
    pub fn check(&self) -> Result<(), NavMeshError> {
        for (index, polygon) in self.polygons.iter().enumerate() {
            self.check_polygon(index, polygon)?;
        }
        Ok(())
    }

    fn check_polygon(&self, index: usize, polygon: &NavPolygon) -> Result<(), NavMeshError> {
        let count = polygon.vertices.len();
        if count < 3 {
            return Err(NavMeshError::TooFewVertices(index));
        }
        if polygon.edges.len() != count {
            return Err(NavMeshError::EdgeCount(index));
        }
        let points = polygon
            .vertices
            .iter()
            .map(|&v| self.vertices.get(v as usize).copied())
            .collect::<Option<Vec<_>>>()
            .ok_or(NavMeshError::VertexIndex(index))?;

        // Counter-clockwise seen from above is a turn whose cross product points up (+y).
        // Straight runs are allowed; a polygon of only straight runs has no area.
        let mut area = 0.;
        for i in 0..count {
            let (a, b, c) = (points[i], points[(i + 1) % count], points[(i + 2) % count]);
            if (b - a).cross(c - b).y < -TOLERANCE {
                return Err(NavMeshError::NotConvex(index, (i + 1) % count));
            }
            area += a.cross(b).y;
        }
        if area <= TOLERANCE {
            return Err(NavMeshError::NotConvex(index, 0));
        }

        for (edge, kind) in polygon.edges.iter().enumerate() {
            let (a, b) = (polygon.vertices[edge], polygon.vertices[(edge + 1) % count]);
            match *kind {
                NavEdge::Wall => {}
                NavEdge::Neighbor(other) => {
                    let neighbor = self
                        .polygons
                        .get(other as usize)
                        .filter(|_| other as usize != index)
                        .ok_or(NavMeshError::NeighborIndex(index, edge))?;
                    let n = neighbor.vertices.len();
                    let named_back = (0..n).any(|j| {
                        neighbor.vertices[j] == b
                            && neighbor.vertices[(j + 1) % n] == a
                            && neighbor.edges.get(j) == Some(&NavEdge::Neighbor(index as u32))
                    });
                    if !named_back {
                        return Err(NavMeshError::NeighborNotReciprocated(index, edge));
                    }
                }
                NavEdge::Boundary(side) => {
                    let on_line = |v: u32| side.holds(self.vertices[v as usize]);
                    if !(on_line(a) && on_line(b)) {
                        return Err(NavMeshError::BoundaryOffLine(index, edge));
                    }
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Two unit squares side by side along x, the first on the precinct's corner, sharing the
    /// edge x = 1.
    fn two_squares() -> NavMesh {
        let point = |x: f32, z: f32| Vec3::new(x, 0., z);
        NavMesh {
            vertices: vec![
                point(0., 0.),
                point(0., 1.),
                point(1., 1.),
                point(1., 0.),
                point(2., 1.),
                point(2., 0.),
            ],
            polygons: vec![
                NavPolygon {
                    vertices: vec![0, 1, 2, 3],
                    edges: vec![
                        NavEdge::Boundary(PrecinctSide::NegX),
                        NavEdge::Wall,
                        NavEdge::Neighbor(1),
                        NavEdge::Boundary(PrecinctSide::NegZ),
                    ],
                    area: NavAreaKind::Walkable,
                    detail: Vec::new(),
                },
                NavPolygon {
                    vertices: vec![3, 2, 4, 5],
                    edges: vec![
                        NavEdge::Neighbor(0),
                        NavEdge::Wall,
                        NavEdge::Wall,
                        NavEdge::Wall,
                    ],
                    area: NavAreaKind::Door,
                    detail: Vec::new(),
                },
            ],
        }
    }

    #[test]
    fn two_squares_sharing_an_edge_pass() {
        assert_eq!(two_squares().check(), Ok(()));
    }

    #[test]
    fn a_clockwise_polygon_fails() {
        let mut mesh = two_squares();
        mesh.polygons[1].vertices.reverse();
        mesh.polygons[0].edges[2] = NavEdge::Wall;
        mesh.polygons[1].edges = vec![NavEdge::Wall; 4];
        assert!(matches!(mesh.check(), Err(NavMeshError::NotConvex(1, _))));
    }

    #[test]
    fn a_concave_polygon_fails() {
        let mut mesh = two_squares();
        // A notch pushed into the second square's far side, at (1.5, 0.5).
        mesh.vertices.push(Vec3::new(1.5, 0., 0.5));
        mesh.polygons[1].vertices = vec![3, 2, 4, 6, 5];
        mesh.polygons[1].edges.push(NavEdge::Wall);
        assert!(matches!(mesh.check(), Err(NavMeshError::NotConvex(1, _))));
    }

    #[test]
    fn a_neighbor_not_named_back_fails() {
        let mut mesh = two_squares();
        mesh.polygons[1].edges[0] = NavEdge::Wall;
        assert_eq!(
            mesh.check(),
            Err(NavMeshError::NeighborNotReciprocated(0, 2))
        );
    }

    #[test]
    fn a_boundary_edge_off_its_line_fails() {
        let mut mesh = two_squares();
        mesh.polygons[0].edges[0] = NavEdge::Boundary(PrecinctSide::PosX);
        assert_eq!(mesh.check(), Err(NavMeshError::BoundaryOffLine(0, 0)));
    }

    #[test]
    fn an_edge_per_vertex() {
        let mut mesh = two_squares();
        mesh.polygons[0].edges.pop();
        assert_eq!(mesh.check(), Err(NavMeshError::EdgeCount(0)));
    }
}
