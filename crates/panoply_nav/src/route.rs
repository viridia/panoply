//! Routes across the joined precinct meshes: an A* search over their polygons, one node per
//! polygon, then a funnel pass that pulls the route taut through the edges it crosses. Where the
//! goal cannot be reached, the route ends at the nearest point that can.

use std::{
    cmp::Ordering,
    collections::{BinaryHeap, HashMap},
};

use bevy::{
    ecs::system::SystemParam,
    math::{IVec2, Vec2, Vec3, Vec3Swizzles},
    prelude::*,
};
use panoply_core::metrics::PRECINCT_SIZE_F;
use panoply_scenery::precinct::Precinct;
use thiserror::Error;

use crate::{NavAreaKind, NavEdge, NavMesh, PrecinctNavMesh, PrecinctSide};

/// How far the start may be from the mesh, in meters: an actor stands on it, give or take its
/// footing on a slope or a ledge.
const START_TOLERANCE: f32 = 1.;

/// How far off the mesh a goal is still taken to be on it: a ground click lands a few
/// centimetres off the mesh, or in the band along a wall the mesh leaves for the agent's radius.
const ARRIVAL_TOLERANCE: f32 = 0.5;

/// How far beyond the start and goal the search looks, on each side (faery's planner radius).
const SEARCH_MARGIN: f32 = 16.;

/// The most polygons one search expands.
const MAX_EXPANDED: usize = 4096;

/// Lengths and areas below this are none.
const EPSILON: f32 = 1e-4;

/// What crossing each kind of area costs per meter, [`f32::INFINITY`] where a route may not go.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NavCosts {
    pub walkable: f32,
    pub door: f32,
    pub path: f32,
    pub portal: f32,
}

impl Default for NavCosts {
    /// Footpaths at half price, as faery's routes preferred them; portals closed until crossing
    /// them is built.
    fn default() -> Self {
        Self {
            walkable: 1.,
            door: 1.,
            path: 0.5,
            portal: f32::INFINITY,
        }
    }
}

impl NavCosts {
    pub fn of(&self, area: NavAreaKind) -> f32 {
        match area {
            NavAreaKind::Walkable => self.walkable,
            NavAreaKind::Door => self.door,
            NavAreaKind::Path => self.path,
            NavAreaKind::Portal => self.portal,
        }
    }

    /// The cheapest cost of an area a route may cross, which the search's estimate is scaled by
    /// so that it never overestimates.
    fn cheapest(&self) -> f32 {
        [self.walkable, self.door, self.path, self.portal]
            .into_iter()
            .filter(|c| c.is_finite() && *c > 0.)
            .fold(f32::INFINITY, f32::min)
            .min(1.)
    }
}

/// A route asked for, in world coordinates.
#[derive(Debug, Clone, PartialEq)]
pub struct RouteQuery {
    pub start: Vec3,
    pub goal: Vec3,
    /// How near the goal is near enough, as for a chest, opened from any side; 0 for a point on
    /// the ground.
    pub reach: f32,
    pub costs: NavCosts,
}

/// A route, as the points to walk through in world coordinates, the start first: where it turns,
/// and where it crosses from one polygon to the next, so that it lies on the mesh. Following it
/// needs only the points where its heading changes.
#[derive(Debug, Clone, Default, PartialEq, Reflect)]
#[reflect(Default)]
pub struct Route {
    pub waypoints: Vec<Vec3>,
    /// Whether the last waypoint is within reach of the goal, rather than only the nearest point
    /// a route could get to.
    pub reached: bool,
}

impl Route {
    /// The points where the route's heading changes, seen from above, and its ends: all that
    /// following it needs.
    pub fn turns(&self) -> Vec<Vec3> {
        let points = &self.waypoints;
        let mut kept: Vec<Vec3> = points.first().copied().into_iter().collect();
        for (i, &p) in points.iter().enumerate().skip(1) {
            let from = *kept.last().unwrap();
            let straight = points.get(i + 1).is_some_and(|&next| {
                let (a, b) = ((p - from).xz(), (next - p).xz());
                cross(a.normalize_or_zero(), b.normalize_or_zero()).abs() < TURN_EPSILON
                    && a.dot(b) > 0.
            });
            if !straight {
                kept.push(p);
            }
        }
        kept
    }
}

/// The sine of the smallest angle [`Route::turns`] counts as a turn.
const TURN_EPSILON: f32 = 1e-3;

/// The route an actor was given, for it to walk.
#[derive(Component, Debug, Clone, Default, PartialEq, Reflect)]
#[reflect(Component, Default)]
pub struct PlannedRoute(pub Route);

#[derive(Debug, Error, PartialEq)]
pub enum RouteError {
    /// A precinct the route may need has no mesh yet.
    #[error("precinct {0} has no navigation mesh yet")]
    NotReady(IVec2),
    #[error("the start is not on the navigation mesh")]
    StartOffMesh,
}

/// A precinct's mesh, as the search sees it.
#[derive(Debug, Clone, Copy)]
pub enum PrecinctMesh<'m> {
    /// No precinct is loaded there: nothing to walk on.
    Missing,
    /// The precinct is loaded but its mesh is not built.
    NotReady,
    Ready(&'m NavMesh),
}

/// The routes over a realm's loaded precincts.
#[derive(SystemParam)]
pub struct NavRoutes<'w, 's> {
    precincts: Query<'w, 's, (&'static Precinct, Option<&'static PrecinctNavMesh>)>,
}

impl NavRoutes<'_, '_> {
    pub fn route(&self, realm: Entity, query: &RouteQuery) -> Result<Route, RouteError> {
        let meshes: HashMap<IVec2, Option<&NavMesh>> = self
            .precincts
            .iter()
            .filter(|(precinct, _)| precinct.realm == realm)
            .map(|(precinct, mesh)| (precinct.coords, mesh.map(|m| &m.0)))
            .collect();
        find_route(
            |coords| match meshes.get(&coords) {
                None => PrecinctMesh::Missing,
                Some(None) => PrecinctMesh::NotReady,
                Some(Some(mesh)) => PrecinctMesh::Ready(mesh),
            },
            query,
        )
    }
}

/// The corner of the precinct at `coords`, where its mesh's origin lies.
fn origin(coords: IVec2) -> Vec3 {
    Vec3::new(coords.x as f32, 0., coords.y as f32) * PRECINCT_SIZE_F
}

fn precinct_at(point: Vec2) -> IVec2 {
    (point / PRECINCT_SIZE_F).floor().as_ivec2()
}

/// Which side of the line from `o` through `a` the point `b` lies, seen from above: positive on
/// the side a polygon's inside is on, walking its edges in order.
fn side(o: Vec2, a: Vec2, b: Vec2) -> f32 {
    let (u, v) = (a - o, b - o);
    u.y * v.x - u.x * v.y
}

/// A polygon's points in world coordinates.
fn polygon_points(mesh: &NavMesh, coords: IVec2, polygon: u32) -> Vec<Vec3> {
    let origin = origin(coords);
    mesh.polygons[polygon as usize]
        .vertices
        .iter()
        .map(|&v| mesh.vertices[v as usize] + origin)
        .collect()
}

/// The point of a convex polygon nearest `point`, judged from above, at the polygon's height
/// there.
fn closest_point(points: &[Vec3], point: Vec3) -> Vec3 {
    let p = point.xz();
    let n = points.len();
    let inside = (0..n).all(|i| side(points[i].xz(), points[(i + 1) % n].xz(), p) >= -EPSILON);
    if inside {
        return Vec3::new(point.x, height_at(points, p), point.z);
    }
    (0..n)
        .map(|i| {
            let (a, b) = (points[i], points[(i + 1) % n]);
            let ab = b.xz() - a.xz();
            let t = ((p - a.xz()).dot(ab) / ab.length_squared().max(EPSILON)).clamp(0., 1.);
            a.lerp(b, t)
        })
        .min_by(|a, b| {
            a.xz()
                .distance_squared(p)
                .total_cmp(&b.xz().distance_squared(p))
        })
        .unwrap_or(point)
}

/// A convex polygon's height at `p`, inside it, from the fan of triangles about its first point.
fn height_at(points: &[Vec3], p: Vec2) -> f32 {
    let a = points[0];
    for pair in points[1..].windows(2) {
        let (b, c) = (pair[0], pair[1]);
        let area = side(a.xz(), b.xz(), c.xz());
        if area.abs() < EPSILON {
            continue;
        }
        let wb = side(c.xz(), a.xz(), p) / area;
        let wc = side(a.xz(), b.xz(), p) / area;
        let wa = 1. - wb - wc;
        if wa >= -EPSILON && wb >= -EPSILON && wc >= -EPSILON {
            return wa * a.y + wb * b.y + wc * c.y;
        }
    }
    points.iter().map(|v| v.y).sum::<f32>() / points.len() as f32
}

/// The neighbour across a side, and the side of it that faces back.
fn across(side: PrecinctSide) -> (IVec2, PrecinctSide) {
    match side {
        PrecinctSide::NegX => (IVec2::NEG_X, PrecinctSide::PosX),
        PrecinctSide::PosX => (IVec2::X, PrecinctSide::NegX),
        PrecinctSide::NegZ => (IVec2::NEG_Y, PrecinctSide::PosZ),
        PrecinctSide::PosZ => (IVec2::Y, PrecinctSide::NegZ),
    }
}

/// The coordinate that varies along a side's line.
fn along(side: PrecinctSide, point: Vec3) -> f32 {
    match side {
        PrecinctSide::NegX | PrecinctSide::PosX => point.z,
        PrecinctSide::NegZ | PrecinctSide::PosZ => point.x,
    }
}

/// One way out of a polygon: into `polygon` of the precinct at `coords`, through the edge from
/// `right` to `left`, named as a walker leaving sees them.
struct Crossing {
    coords: IVec2,
    polygon: u32,
    right: Vec3,
    left: Vec3,
}

/// The crossings from the edge `a` to `b` on a precinct's `side` into the neighbour's polygons:
/// each only over the stretch where the two sides' boundary edges overlap, since each side's
/// edges are cut short by its own obstacles.
fn seam_crossings(
    neighbour: &NavMesh,
    coords: IVec2,
    facing: PrecinctSide,
    a: Vec3,
    b: Vec3,
    out: &mut Vec<Crossing>,
) {
    let (sa, sb) = (along(facing, a), along(facing, b));
    let length = sb - sa;
    if length.abs() < EPSILON {
        return;
    }
    let origin = origin(coords);
    for (index, polygon) in neighbour.polygons.iter().enumerate() {
        let n = polygon.vertices.len();
        for (i, edge) in polygon.edges.iter().enumerate() {
            if *edge != NavEdge::Boundary(facing) {
                continue;
            }
            let c = neighbour.vertices[polygon.vertices[i] as usize] + origin;
            let d = neighbour.vertices[polygon.vertices[(i + 1) % n] as usize] + origin;
            let (sc, sd) = (along(facing, c), along(facing, d));
            let lo = sa.min(sb).max(sc.min(sd));
            let hi = sa.max(sb).min(sc.max(sd));
            if hi - lo < EPSILON {
                continue;
            }
            let (t0, t1) = ((lo - sa) / length, (hi - sa) / length);
            out.push(Crossing {
                coords,
                polygon: index as u32,
                right: a.lerp(b, t0.min(t1)),
                left: a.lerp(b, t0.max(t1)),
            });
        }
    }
}

/// The polygon nearest `point` within `tolerance`, as its precinct, index and the nearest point
/// on it. An unbuilt precinct under the point is an error; others in reach are passed over.
fn nearest_polygon<'m>(
    meshes: &impl Fn(IVec2) -> PrecinctMesh<'m>,
    point: Vec3,
    tolerance: f32,
) -> Result<Option<(IVec2, u32, Vec3)>, RouteError> {
    let low = precinct_at(point.xz() - tolerance);
    let high = precinct_at(point.xz() + tolerance);
    let mut best: Option<(f32, IVec2, u32, Vec3)> = None;
    for z in low.y..=high.y {
        for x in low.x..=high.x {
            let coords = IVec2::new(x, z);
            let mesh = match meshes(coords) {
                PrecinctMesh::Ready(mesh) => mesh,
                PrecinctMesh::NotReady if coords == precinct_at(point.xz()) => {
                    return Err(RouteError::NotReady(coords))
                }
                _ => continue,
            };
            for polygon in 0..mesh.polygons.len() as u32 {
                let on = closest_point(&polygon_points(mesh, coords, polygon), point);
                let distance = on.distance(point);
                if distance <= tolerance && best.is_none_or(|b| distance < b.0) {
                    best = Some((distance, coords, polygon, on));
                }
            }
        }
    }
    Ok(best.map(|(_, coords, polygon, on)| (coords, polygon, on)))
}

/// A polygon reached by the search.
struct Node {
    coords: IVec2,
    polygon: u32,
    /// Where the route enters it, on the crossing it came through ([`entry_point`]).
    entry: Vec3,
    cost: f32,
    parent: Option<usize>,
    /// The crossing it came through, right then left.
    crossing: (Vec3, Vec3),
    closed: bool,
}

/// A node waiting in the search's queue, cheapest estimate first.
struct Open {
    estimate: f32,
    cost: f32,
    node: usize,
}

impl PartialEq for Open {
    fn eq(&self, other: &Self) -> bool {
        self.estimate == other.estimate
    }
}

impl Eq for Open {}

impl PartialOrd for Open {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Open {
    /// Reversed, since the heap pops its greatest.
    fn cmp(&self, other: &Self) -> Ordering {
        other.estimate.total_cmp(&self.estimate)
    }
}

/// The route for `query` over the meshes `meshes` gives by precinct.
pub fn find_route<'m>(
    meshes: impl Fn(IVec2) -> PrecinctMesh<'m>,
    query: &RouteQuery,
) -> Result<Route, RouteError> {
    let (start_coords, start_polygon, start) =
        nearest_polygon(&meshes, query.start, START_TOLERANCE)?.ok_or(RouteError::StartOffMesh)?;
    // A goal just off the mesh is moved onto it, and must then be reached exactly; one further
    // off, only within reach.
    let goal = nearest_polygon(&meshes, query.goal, ARRIVAL_TOLERANCE)?
        .map_or(query.goal, |(_, _, on)| on);
    let near_enough = query.reach + EPSILON;
    let cheapest = query.costs.cheapest();
    let estimate = |p: Vec3| (p.distance(goal) - near_enough).max(0.) * cheapest;
    let bounds = (
        start.xz().min(goal.xz()) - SEARCH_MARGIN,
        start.xz().max(goal.xz()) + SEARCH_MARGIN,
    );
    let in_bounds = |p: Vec3| {
        let p = p.xz();
        p.cmpge(bounds.0).all() && p.cmple(bounds.1).all()
    };
    let mesh = |coords| match meshes(coords) {
        PrecinctMesh::Ready(mesh) => mesh,
        _ => unreachable!("a node in a precinct without a mesh"),
    };

    let mut nodes = vec![Node {
        coords: start_coords,
        polygon: start_polygon,
        entry: start,
        cost: 0.,
        parent: None,
        crossing: (start, start),
        closed: false,
    }];
    let mut index = HashMap::from([((start_coords, start_polygon), 0)]);
    let mut queue = BinaryHeap::from([Open {
        estimate: estimate(start),
        cost: 0.,
        node: 0,
    }]);
    // The node nearest the goal so far, and its nearest point to it.
    let mut nearest = (f32::INFINITY, 0, start);
    let mut reached = false;
    let mut not_ready = None;
    let mut expanded = 0;
    let mut crossings = Vec::new();

    while let Some(open) = queue.pop() {
        let here = open.node;
        if nodes[here].closed || open.cost > nodes[here].cost {
            continue;
        }
        nodes[here].closed = true;
        expanded += 1;
        if expanded > MAX_EXPANDED {
            break;
        }
        let (coords, polygon) = (nodes[here].coords, nodes[here].polygon);
        let here_mesh = mesh(coords);
        let points = polygon_points(here_mesh, coords, polygon);
        let end = closest_point(&points, goal);
        let distance = end.distance(goal);
        if distance < nearest.0 {
            nearest = (distance, here, end);
        }
        if distance <= near_enough {
            reached = true;
            break;
        }

        let area_cost = query.costs.of(here_mesh.polygons[polygon as usize].area);
        // Where the start stands is crossed whatever it costs.
        let area_cost = if area_cost.is_finite() { area_cost } else { 1. };
        let edges = &here_mesh.polygons[polygon as usize].edges;
        let n = points.len();
        crossings.clear();
        for (i, edge) in edges.iter().enumerate() {
            let (a, b) = (points[i], points[(i + 1) % n]);
            match *edge {
                NavEdge::Wall => {}
                NavEdge::Neighbor(other) => crossings.push(Crossing {
                    coords,
                    polygon: other,
                    right: a,
                    left: b,
                }),
                NavEdge::Boundary(side) => {
                    let (offset, facing) = across(side);
                    let next = coords + offset;
                    match meshes(next) {
                        PrecinctMesh::Missing => {}
                        PrecinctMesh::NotReady => not_ready = not_ready.or(Some(next)),
                        PrecinctMesh::Ready(neighbour) => {
                            seam_crossings(neighbour, next, facing, a, b, &mut crossings)
                        }
                    }
                }
            }
        }

        for crossing in crossings.drain(..) {
            let area = mesh(crossing.coords).polygons[crossing.polygon as usize].area;
            if !query.costs.of(area).is_finite() {
                continue;
            }
            let entry = entry_point(crossing.right, crossing.left, nodes[here].entry, goal);
            if !in_bounds(entry) {
                continue;
            }
            let cost = nodes[here].cost + nodes[here].entry.distance(entry) * area_cost;
            let key = (crossing.coords, crossing.polygon);
            let node = Node {
                coords: crossing.coords,
                polygon: crossing.polygon,
                entry,
                cost,
                parent: Some(here),
                crossing: (crossing.right, crossing.left),
                closed: false,
            };
            let next = match index.get(&key) {
                Some(&known) if nodes[known].cost <= cost => continue,
                Some(&known) => {
                    nodes[known] = node;
                    known
                }
                None => {
                    nodes.push(node);
                    index.insert(key, nodes.len() - 1);
                    nodes.len() - 1
                }
            };
            queue.push(Open {
                estimate: cost + estimate(entry),
                cost,
                node: next,
            });
        }
    }

    if !reached {
        if let Some(coords) = not_ready {
            return Err(RouteError::NotReady(coords));
        }
    }
    let (_, last, end) = nearest;
    let mut portals = vec![(end, end)];
    let mut node = Some(last);
    while let Some(i) = node {
        portals.push(nodes[i].crossing);
        node = nodes[i].parent;
    }
    // The start node's crossing is the start itself.
    portals.reverse();
    Ok(Route {
        waypoints: route_through(&portals),
        reached,
    })
}

/// Where the search enters a polygon through the crossing from `right` to `left`: the point on it
/// that makes the walk from `from` through it to `goal` shortest, seen from above. Where the line
/// from `from` to `goal` crosses it, that crossing; otherwise the point toward the goal reflected
/// across its line, kept to the crossing. A crossing's middle would cost a long edge, as along a
/// precinct's seam, metres the route never walks. Since polygons are convex, each step it costs is
/// one a walker can take.
fn entry_point(right: Vec3, left: Vec3, from: Vec3, goal: Vec3) -> Vec3 {
    let (r, a) = (right.xz(), from.xz());
    let d = left.xz() - r;
    let length_squared = d.length_squared();
    if length_squared < EPSILON * EPSILON {
        return right;
    }
    let mut b = goal.xz();
    let (side_a, side_b) = (cross(d, a - r), cross(d, b - r));
    if side_a * side_b > 0. {
        // Both on one side: the goal's mirror image, on the other.
        b -= Vec2::new(-d.y, d.x) * (2. * side_b / length_squared);
    }
    let run = b - a;
    let denominator = cross(d, run);
    let t = if denominator.abs() < EPSILON {
        // Along the crossing's line: the point nearest `from`.
        (a - r).dot(d) / length_squared
    } else {
        cross(a - r, run) / denominator
    };
    right.lerp(left, t.clamp(0., 1.))
}

/// Whether two points are one, seen from above.
fn same(a: Vec3, b: Vec3) -> bool {
    a.xz().distance_squared(b.xz()) < EPSILON * EPSILON
}

/// The route through `portals`, each a crossing given right then left, the first and last the
/// start and end as crossings of no width: the funnel's corners, and between them a point where
/// each straight run crosses each crossing, at the mesh's height there. Each polygon is flat, so
/// the route lies on the mesh, up stairs as on level ground.
fn route_through(portals: &[(Vec3, Vec3)]) -> Vec<Vec3> {
    let corners = funnel(portals);
    let mut points = vec![corners[0].0];
    for pair in corners.windows(2) {
        let ((from, i), (to, j)) = (pair[0], pair[1]);
        let run = to.xz() - from.xz();
        for &(right, left) in &portals[i + 1..j] {
            let across = left.xz() - right.xz();
            let denominator = cross(across, run);
            if denominator.abs() < EPSILON {
                continue;
            }
            let s = (cross(from.xz() - right.xz(), run) / denominator).clamp(0., 1.);
            points.push(right.lerp(left, s));
        }
        points.push(to);
    }
    points.dedup_by(|a, b| same(*a, *b));
    points
}

fn cross(a: Vec2, b: Vec2) -> f32 {
    a.x * b.y - a.y * b.x
}

/// The shortest line through `portals`, as in [`route_through`]: its corners, each with the
/// index of the crossing it lies on. The simple stupid funnel algorithm (Mikko Mononen's),
/// judged from above.
fn funnel(portals: &[(Vec3, Vec3)]) -> Vec<(Vec3, usize)> {
    let start = portals[0].0;
    let mut corners = vec![(start, 0)];
    let (mut apex, mut right, mut left) = (start, start, start);
    let (mut right_index, mut left_index) = (0, 0);
    let mut i = 1;
    while i < portals.len() {
        let (next_right, next_left) = portals[i];
        // The right side narrows the funnel, unless it crosses the left, which is then a corner,
        // the apex the funnel starts again from.
        if side(apex.xz(), right.xz(), next_right.xz()) >= 0. {
            if same(apex, right) || side(apex.xz(), left.xz(), next_right.xz()) < 0. {
                right = next_right;
                right_index = i;
            } else {
                (apex, right) = (left, left);
                corners.push((apex, left_index));
                right_index = left_index;
                i = left_index + 1;
                continue;
            }
        }
        if side(apex.xz(), left.xz(), next_left.xz()) <= 0. {
            if same(apex, left) || side(apex.xz(), right.xz(), next_left.xz()) > 0. {
                left = next_left;
                left_index = i;
            } else {
                (apex, left) = (right, right);
                corners.push((apex, right_index));
                left_index = right_index;
                i = right_index + 1;
                continue;
            }
        }
        i += 1;
    }
    let last = portals.len() - 1;
    if !same(corners.last().unwrap().0, portals[last].0) {
        corners.push((portals[last].0, last));
    }
    corners
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::NavPolygon;

    /// A mesh of unit squares at the given (x, z) cells of a precinct, each square's edges a
    /// neighbour where the next cell is in the mesh, a boundary on the precinct's sides, and
    /// otherwise a wall. `area` gives each cell's area.
    fn grid(cells: &[(u32, u32)], area: impl Fn(u32, u32) -> NavAreaKind) -> NavMesh {
        let mut vertices = Vec::new();
        let mut vertex = |x: u32, z: u32| {
            let p = Vec3::new(x as f32, 0., z as f32);
            match vertices.iter().position(|&v| v == p) {
                Some(i) => i as u32,
                None => {
                    vertices.push(p);
                    vertices.len() as u32 - 1
                }
            }
        };
        let cell_index = |x: u32, z: u32| cells.iter().position(|&c| c == (x, z));
        let size = PRECINCT_SIZE_F as u32;
        let polygons = cells
            .iter()
            .map(|&(x, z)| {
                // Counter-clockwise from above, as the generator's: +z, then +x.
                let corners = [(x, z), (x, z + 1), (x + 1, z + 1), (x + 1, z)];
                let vertices = corners.iter().map(|&(cx, cz)| vertex(cx, cz)).collect();
                // Each edge's cell across it, or the side it lies on.
                let across = [
                    (x.checked_sub(1).map(|x| (x, z)), x == 0, PrecinctSide::NegX),
                    (Some((x, z + 1)), z + 1 == size, PrecinctSide::PosZ),
                    (Some((x + 1, z)), x + 1 == size, PrecinctSide::PosX),
                    (z.checked_sub(1).map(|z| (x, z)), z == 0, PrecinctSide::NegZ),
                ];
                let edges = across
                    .iter()
                    .map(|&(cell, on_side, side)| {
                        if on_side {
                            NavEdge::Boundary(side)
                        } else {
                            cell.and_then(|(cx, cz)| cell_index(cx, cz))
                                .map_or(NavEdge::Wall, |i| NavEdge::Neighbor(i as u32))
                        }
                    })
                    .collect();
                let area = area(x, z);
                NavPolygon {
                    vertices,
                    edges,
                    area,
                    door: (area == NavAreaKind::Door).then_some(0),
                }
            })
            .collect();
        NavMesh {
            vertices,
            polygons,
            doors: None,
        }
    }

    fn walkable(_: u32, _: u32) -> NavAreaKind {
        NavAreaKind::Walkable
    }

    fn query(start: (f32, f32), goal: (f32, f32)) -> RouteQuery {
        RouteQuery {
            start: Vec3::new(start.0, 0., start.1),
            goal: Vec3::new(goal.0, 0., goal.1),
            reach: 0.,
            costs: NavCosts::default(),
        }
    }

    fn one<'m>(mesh: &'m NavMesh) -> impl Fn(IVec2) -> PrecinctMesh<'m> + 'm {
        move |coords| {
            if coords == IVec2::ZERO {
                PrecinctMesh::Ready(mesh)
            } else {
                PrecinctMesh::Missing
            }
        }
    }

    fn flat(points: &[Vec3]) -> Vec<(f32, f32)> {
        points.iter().map(|p| (p.x, p.z)).collect()
    }

    /// A route's turns, seen from above.
    fn turns(route: &Route) -> Vec<(f32, f32)> {
        flat(&route.turns())
    }

    fn near(a: (f32, f32), b: (f32, f32)) -> bool {
        (a.0 - b.0).abs() < 1e-3 && (a.1 - b.1).abs() < 1e-3
    }

    #[test]
    fn the_grid_is_a_good_mesh() {
        let mesh = grid(&[(0, 0), (1, 0), (1, 1), (63, 5)], walkable);
        assert_eq!(mesh.check(), Ok(()));
    }

    #[test]
    fn within_one_polygon_the_route_is_straight() {
        let mesh = grid(&[(3, 3)], walkable);
        let route = find_route(one(&mesh), &query((3.2, 3.2), (3.8, 3.7))).unwrap();
        assert!(route.reached);
        assert_eq!(flat(&route.waypoints), [(3.2, 3.2), (3.8, 3.7)]);
    }

    #[test]
    fn a_straight_corridor_needs_no_corners() {
        let cells: Vec<_> = (0..6).map(|x| (x + 2, 2)).collect();
        let mesh = grid(&cells, walkable);
        let route = find_route(one(&mesh), &query((2.5, 2.5), (7.5, 2.8))).unwrap();
        assert_eq!(turns(&route), [(2.5, 2.5), (7.5, 2.8)]);
        // A point on each of the five edges crossed.
        assert_eq!(route.waypoints.len(), 7, "{route:?}");
    }

    #[test]
    fn a_route_up_a_ramp_lies_on_it() {
        // Level from x 0 to 2, rising 2 m from x 2 to 4, level again to x 6.
        let cells: Vec<_> = (0..6).map(|x| (x + 1, 2)).collect();
        let mut mesh = grid(&cells, walkable);
        let height = |x: f32| (x - 3.).clamp(0., 2.);
        for v in &mut mesh.vertices {
            v.y = height(v.x);
        }
        let mut asked = query((1.5, 2.5), (6.5, 2.5));
        asked.start.y = height(1.5);
        asked.goal.y = height(6.5);
        let route = find_route(one(&mesh), &asked).unwrap();
        assert!(route.reached);
        for p in &route.waypoints {
            assert!(
                (p.y - height(p.x)).abs() < 1e-3,
                "{p} off the ramp: {route:?}"
            );
        }
        assert!(route.waypoints.iter().any(|p| (p.x - 3.).abs() < 1e-3));
        assert!(route.waypoints.iter().any(|p| (p.x - 5.).abs() < 1e-3));
    }

    /// An L: along x from (2, 2) to (6, 2), then up z to (6, 6). The route turns at the inside
    /// corner, (6, 3).
    fn ell() -> NavMesh {
        let mut cells: Vec<_> = (2..7).map(|x| (x, 2)).collect();
        cells.extend((3..7).map(|z| (6, z)));
        grid(&cells, walkable)
    }

    #[test]
    fn the_route_turns_at_the_inside_corner() {
        let mesh = ell();
        for (start, goal) in [((2.5, 2.5), (6.5, 6.5)), ((6.5, 6.5), (2.5, 2.5))] {
            let route = find_route(one(&mesh), &query(start, goal)).unwrap();
            let points = turns(&route);
            assert_eq!(points.len(), 3, "{points:?}");
            assert!(near(points[1], (6., 3.)), "{points:?}");
        }
    }

    #[test]
    fn a_footpath_is_preferred_over_a_shorter_walk() {
        // Two ways from (2, 2) to (2, 12): straight up column 2, about 10 m, or out along a
        // footpath in column 4 and back, about 14 m at half the cost, joined by rows 2 and 12.
        let mut cells: Vec<_> = (2..5).map(|x| (x, 2)).collect();
        cells.extend((2..5).map(|x| (x, 12)));
        cells.extend((3..12).map(|z| (2, z)));
        cells.extend((3..12).map(|z| (4, z)));
        let path = |x: u32, z: u32| {
            if x > 2 || z == 2 || z == 12 {
                NavAreaKind::Path
            } else {
                NavAreaKind::Walkable
            }
        };
        let mesh = grid(&cells, path);
        let route = find_route(one(&mesh), &query((2.5, 2.5), (2.5, 12.5))).unwrap();
        assert!(route.waypoints.iter().any(|p| p.x > 3.), "{route:?}");

        // Without the footpath, the straight way.
        let mesh = grid(&cells, walkable);
        let route = find_route(one(&mesh), &query((2.5, 2.5), (2.5, 12.5))).unwrap();
        assert!(route.waypoints.iter().all(|p| p.x < 3.), "{route:?}");
    }

    #[test]
    fn a_blocked_area_is_gone_around() {
        // A row of three with a portal in the middle, and a detour around it through row 3.
        let cells = [(2, 2), (3, 2), (4, 2), (2, 3), (3, 3), (4, 3)];
        let portal = |x: u32, z: u32| {
            if (x, z) == (3, 2) {
                NavAreaKind::Portal
            } else {
                NavAreaKind::Walkable
            }
        };
        let mesh = grid(&cells, portal);
        let route = find_route(one(&mesh), &query((2.5, 2.2), (4.5, 2.2))).unwrap();
        assert!(route.reached);
        assert!(route.waypoints.iter().any(|p| p.z >= 3.), "{route:?}");
    }

    #[test]
    fn an_unreachable_goal_ends_at_the_nearest_point() {
        let mesh = grid(&[(2, 2), (3, 2)], walkable);
        let route = find_route(one(&mesh), &query((2.5, 2.5), (6., 2.5))).unwrap();
        assert!(!route.reached);
        assert!(near(*flat(&route.waypoints).last().unwrap(), (4., 2.5)));
    }

    #[test]
    fn within_reach_is_reached() {
        let mesh = grid(&[(2, 2), (3, 2)], walkable);
        let mut asked = query((2.5, 2.5), (5., 2.5));
        asked.reach = 1.2;
        let route = find_route(one(&mesh), &asked).unwrap();
        assert!(route.reached);
        assert!(near(*flat(&route.waypoints).last().unwrap(), (4., 2.5)));
    }

    #[test]
    fn a_start_off_the_mesh_fails() {
        let mesh = grid(&[(2, 2)], walkable);
        let result = find_route(one(&mesh), &query((10., 10.), (2.5, 2.5)));
        assert_eq!(result, Err(RouteError::StartOffMesh));
    }

    /// Precinct (0, 0) open along its +x side over z 2 to 5, and (1, 0) open along its -x side
    /// over z 4 to 7: they overlap over z 4 to 5.
    fn seam() -> (NavMesh, NavMesh) {
        let west: Vec<_> = (2..5).map(|z| (63, z)).collect();
        let east: Vec<_> = (4..7).map(|z| (0, z)).collect();
        (grid(&west, walkable), grid(&east, walkable))
    }

    #[test]
    fn a_seam_is_crossed_only_where_both_sides_are_open() {
        let (west, east) = seam();
        let meshes = |coords: IVec2| match (coords.x, coords.y) {
            (0, 0) => PrecinctMesh::Ready(&west),
            (1, 0) => PrecinctMesh::Ready(&east),
            _ => PrecinctMesh::Missing,
        };
        let route = find_route(meshes, &query((63.5, 2.5), (64.5, 6.5))).unwrap();
        assert!(route.reached);
        let crossing = route
            .waypoints
            .windows(2)
            .find_map(|pair| {
                let (a, b) = (pair[0], pair[1]);
                (a.x < 64. && b.x >= 64.).then(|| a.lerp(b, (64. - a.x) / (b.x - a.x)))
            })
            .expect("the route does not cross the seam");
        assert!(
            (4. - 1e-3..=5. + 1e-3).contains(&crossing.z),
            "crosses at {crossing}, outside the overlap: {route:?}"
        );
    }

    #[test]
    fn a_neighbour_without_a_mesh_is_not_ready() {
        let (west, _) = seam();
        let meshes = |coords: IVec2| match (coords.x, coords.y) {
            (0, 0) => PrecinctMesh::Ready(&west),
            (1, 0) => PrecinctMesh::NotReady,
            _ => PrecinctMesh::Missing,
        };
        let result = find_route(meshes, &query((63.5, 2.5), (64.5, 6.5)));
        assert_eq!(result, Err(RouteError::NotReady(IVec2::new(1, 0))));
    }

    /// A mesh of the given rectangles, each `(x0, z0, x1, z1)` with the edges as `grid`'s, in
    /// order -x, +z, +x, -z.
    fn rects(rects: &[((u32, u32, u32, u32), [NavEdge; 4])]) -> NavMesh {
        let mut vertices = Vec::new();
        let mut vertex = |x: u32, z: u32| {
            let p = Vec3::new(x as f32, 0., z as f32);
            match vertices.iter().position(|&v| v == p) {
                Some(i) => i as u32,
                None => {
                    vertices.push(p);
                    vertices.len() as u32 - 1
                }
            }
        };
        let polygons = rects
            .iter()
            .map(|&((x0, z0, x1, z1), edges)| NavPolygon {
                vertices: [(x0, z0), (x0, z1), (x1, z1), (x1, z0)]
                    .iter()
                    .map(|&(x, z)| vertex(x, z))
                    .collect(),
                edges: edges.to_vec(),
                area: NavAreaKind::Walkable,
                door: None,
            })
            .collect();
        NavMesh {
            vertices,
            polygons,
            doors: None,
        }
    }

    #[test]
    fn long_edges_do_not_turn_the_route_aside() {
        use NavEdge::{Neighbor as N, Wall as W};
        // Three tall polygons side by side from x 1 to 7, z 5 to 25, their shared edges 20 m long,
        // and under them a row of three small squares, z 3 to 5. Straight along z 6 from the
        // first tall one to the third is 4 m; down through the squares and back up, about 4.8.
        let mesh = rects(&[
            ((1, 5, 3, 25), [W, W, N(1), N(3)]),
            ((3, 5, 5, 25), [N(0), W, N(2), N(4)]),
            ((5, 5, 7, 25), [N(1), W, W, N(5)]),
            ((1, 3, 3, 5), [W, N(0), N(4), W]),
            ((3, 3, 5, 5), [N(3), N(1), N(5), W]),
            ((5, 3, 7, 5), [N(4), N(2), W, W]),
        ]);
        assert_eq!(mesh.check(), Ok(()));
        let route = find_route(one(&mesh), &query((2., 6.), (6., 6.))).unwrap();
        assert!(route.reached);
        assert_eq!(turns(&route), [(2., 6.), (6., 6.)], "{route:?}");
    }

    #[test]
    fn the_funnel_keeps_a_straight_line_through_wide_portals() {
        let p = |x: f32, z: f32| Vec3::new(x, 0., z);
        let start = p(0., 0.);
        let end = p(4., 0.);
        // Crossings at x = 1, 2, 3, each from z -1 to 1. Leaving a square of `grid` through its
        // +x edge, from (x + 1, z + 1) to (x + 1, z), +z is on the walker's right.
        let portals = [
            (start, start),
            (p(1., 1.), p(1., -1.)),
            (p(2., 1.), p(2., -1.)),
            (p(3., 1.), p(3., -1.)),
            (end, end),
        ];
        let corners: Vec<_> = funnel(&portals).into_iter().map(|(p, _)| p).collect();
        assert_eq!(flat(&corners), [(0., 0.), (4., 0.)]);
        assert_eq!(route_through(&portals).len(), 5);
    }
}
