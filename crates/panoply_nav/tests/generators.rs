//! Both generators' meshes of the fixtures (`fixture.rs`), Recast's at the two cell sizes PAN16e
//! weighs, with each build's time and peak memory printed
//! (`cargo test -p panoply_nav --test generators -- --nocapture`).

use std::{
    alloc::{GlobalAlloc, Layout, System},
    path::PathBuf,
    sync::atomic::{AtomicIsize, Ordering},
    time::Instant,
};

use panoply_nav::{
    clipping,
    recast::{self, RecastSettings},
    NavAreaKind, NavEdge, NavInput, NavMesh,
};

/// The system allocator, counting the bytes live and the most live at once since the last reset.
struct Counting;

static LIVE: AtomicIsize = AtomicIsize::new(0);
static PEAK: AtomicIsize = AtomicIsize::new(0);

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let pointer = unsafe { System.alloc(layout) };
        if !pointer.is_null() {
            let size = layout.size() as isize;
            let live = LIVE.fetch_add(size, Ordering::Relaxed) + size;
            PEAK.fetch_max(live, Ordering::Relaxed);
        }
        pointer
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        unsafe { System.dealloc(pointer, layout) };
        LIVE.fetch_sub(layout.size() as isize, Ordering::Relaxed);
    }
}

#[global_allocator]
static ALLOCATOR: Counting = Counting;

const FIXTURES: [&str; 2] = ["playground-0-0.msgpack", "playground-1-0.msgpack"];

fn fixture(name: &str) -> NavInput {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name);
    NavInput::read(&path).unwrap()
}

/// Builds a mesh with `build`, prints what it cost and holds, and checks it.
fn measure(label: &str, build: impl FnOnce() -> NavMesh) {
    let before = LIVE.load(Ordering::Relaxed);
    PEAK.store(before, Ordering::Relaxed);
    let start = Instant::now();
    let mesh = build();
    let elapsed = start.elapsed();
    let peak = PEAK.load(Ordering::Relaxed) - before;

    let count = |area| mesh.polygons.iter().filter(|p| p.area == area).count();
    let (doors, paths) = (count(NavAreaKind::Door), count(NavAreaKind::Path));
    let portals = count(NavAreaKind::Portal);
    let edges = |pick: fn(&NavEdge) -> bool| {
        mesh.polygons
            .iter()
            .flat_map(|p| &p.edges)
            .filter(|e| pick(e))
            .count()
    };
    let boundary = edges(|e| matches!(e, NavEdge::Boundary(_)));
    let walls = edges(|e| matches!(e, NavEdge::Wall));
    println!(
        "{label}: {:.0} ms, peak {:.1} MB, {} polygons ({doors} doors, {paths} paths, {portals} portals), \
         {} vertices, {walls} wall and {boundary} boundary edges",
        elapsed.as_secs_f64() * 1000.,
        peak as f64 / 1e6,
        mesh.polygons.len(),
        mesh.vertices.len(),
    );

    assert_eq!(mesh.check(), Ok(()), "{label}");
    assert!(!mesh.polygons.is_empty(), "{label}: no polygons");
    assert!(boundary > 0, "{label}: no boundary edges");
    assert!(doors > 0, "{label}: no doors");
    assert!(paths > 0, "{label}: no paths");
    // The lair has the stone portal; the village has none.
    if label.contains("0-0") {
        assert!(portals > 0, "{label}: no portals");
    }
}

/// One test, so the builds run one at a time and each peak is its own.
#[test]
fn generators_build_the_fixtures() {
    for name in FIXTURES {
        let input = fixture(name);
        for cell_size in [0.1, 0.15] {
            measure(&format!("{name}, Recast at {cell_size} m"), || {
                recast::build(&input, &RecastSettings { cell_size }).unwrap()
            });
        }
        measure(&format!("{name}, clipping"), || clipping::build(&input));
    }
}
