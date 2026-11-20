//! The navigation input the generator's tests start from: two playground precincts, written by
//! `plans/actors.py` (`capture_nav_input`).

use std::path::PathBuf;

use panoply_nav::{NavInput, SurfaceKind};
use panoply_scenery::{ColliderType, PRECINCT_SIZE_F};

fn fixture(name: &str) -> NavInput {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name);
    NavInput::read(&path).unwrap()
}

/// Each precinct has ground (a parcel without terrain has none), every ground vertex inside the
/// precinct, and every door and hint a volume to mark.
fn check(input: &NavInput) {
    let ground = input
        .surfaces
        .iter()
        .filter(|s| s.kind == SurfaceKind::Ground);
    assert!(ground.count() > 0);
    let inside = |v: f32| (-1e-3..=PRECINCT_SIZE_F + 1e-3).contains(&v);
    for surface in input
        .surfaces
        .iter()
        .filter(|s| s.kind == SurfaceKind::Ground)
    {
        assert!(surface.vertices.iter().all(|v| inside(v.x) && inside(v.z)));
    }
    for collider in &input.colliders {
        if matches!(
            collider.collider.kind,
            ColliderType::Door | ColliderType::Hint
        ) {
            assert!(
                collider.collider.volume().is_some(),
                "{collider:?} has no volume"
            );
        }
    }
}

#[test]
fn the_lair() {
    let input = fixture("playground-0-0.msgpack");
    check(&input);
    assert!(!input.colliders.is_empty());
}

#[test]
fn the_village() {
    let input = fixture("playground-1-0.msgpack");
    check(&input);
    let has = |kind| input.colliders.iter().any(|c| c.collider.kind == kind);
    assert!(has(ColliderType::Door));
    assert!(input.surfaces.iter().any(|s| s.kind == SurfaceKind::Floor));
}
