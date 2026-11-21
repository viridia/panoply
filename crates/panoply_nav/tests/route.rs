//! Routes over the fixtures' meshes (`fixture.rs`) side by side, the lair at (0, 0) and the
//! village at (1, 0), with each search's time printed (`cargo test --test route -- --nocapture`).

use std::{path::PathBuf, time::Instant};

use bevy::math::{IVec2, Vec3};
use panoply_nav::{
    clipping, find_route, NavAreaKind, NavCosts, NavEdge, NavInput, NavMesh, PrecinctMesh,
    PrecinctSide, Route, RouteQuery,
};
use panoply_scenery::PRECINCT_SIZE_F;

fn mesh(name: &str) -> NavMesh {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name);
    clipping::build(&NavInput::read(&path).unwrap())
}

/// The middle of a polygon, in its precinct's coordinates.
fn middle(mesh: &NavMesh, polygon: usize) -> Vec3 {
    let vertices = &mesh.polygons[polygon].vertices;
    vertices
        .iter()
        .map(|&v| mesh.vertices[v as usize])
        .sum::<Vec3>()
        / vertices.len() as f32
}

fn timed(label: &str, route: impl FnOnce() -> Route) -> Route {
    let start = Instant::now();
    let route = route();
    println!(
        "{label}: {:.2} ms, {} waypoints, reached {}",
        start.elapsed().as_secs_f64() * 1000.,
        route.waypoints.len(),
        route.reached
    );
    route
}

#[test]
fn routes_cross_from_the_lair_into_the_village() {
    let (lair, village) = (
        mesh("playground-0-0.msgpack"),
        mesh("playground-1-0.msgpack"),
    );
    let meshes = |coords: IVec2| match (coords.x, coords.y) {
        (0, 0) => PrecinctMesh::Ready(&lair),
        (1, 0) => PrecinctMesh::Ready(&village),
        _ => PrecinctMesh::Missing,
    };

    // From a lair polygon on its seam with the village, to the village's walkable polygon
    // nearest the village's middle.
    let on_seam = lair
        .polygons
        .iter()
        .position(|p| p.edges.contains(&NavEdge::Boundary(PrecinctSide::PosX)))
        .expect("the lair has no polygon on its +x side");
    let start = middle(&lair, on_seam);
    let centre = Vec3::new(PRECINCT_SIZE_F / 2., 0., PRECINCT_SIZE_F / 2.);
    let goal = (0..village.polygons.len())
        .filter(|&p| village.polygons[p].area == NavAreaKind::Walkable)
        .map(|p| middle(&village, p))
        .min_by(|a, b| a.distance(centre).total_cmp(&b.distance(centre)))
        .unwrap()
        + Vec3::X * PRECINCT_SIZE_F;

    let query = RouteQuery {
        start,
        goal,
        reach: 0.,
        costs: NavCosts::default(),
    };
    let route = timed("lair to village", || find_route(meshes, &query).unwrap());
    assert!(route.reached, "{route:?}");
    assert!(
        route
            .waypoints
            .windows(2)
            .any(|pair| pair[0].x < PRECINCT_SIZE_F && pair[1].x >= PRECINCT_SIZE_F),
        "the route does not cross the seam: {route:?}"
    );
    assert!(route.waypoints.last().unwrap().distance(goal) < 1e-3);

    // A goal high over the village, which no route reaches: the search runs until its box or its
    // cap is spent, the slowest case.
    let query = RouteQuery {
        goal: goal + Vec3::Y * 50.,
        ..query
    };
    let route = timed("unreachable", || find_route(meshes, &query).unwrap());
    assert!(!route.reached, "{route:?}");
}
