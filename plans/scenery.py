"""Scenery: playground precincts spawn tiers, floor regions (with outlines) and scenery models.

    python3 crates/bevy_remote_driver/client/run.py plans/scenery.py

Waits for scenery to load, counts each kind of scenery entity, then takes a screenshot at a
scenery element (walls, and the road from terrain effects) and at the origin (the tiered dungeon
floors of precinct 0,0). Also checks that ground still builds now that it waits for precinct
terrain effects (D11).
"""

BIN = "panoply"
GOTO = "panoply_core::viewpoint::GotoLocation"
NAME = "bevy_ecs::name::Name"
GLOBAL = "bevy_transform::components::global_transform::GlobalTransform"
MESH = "bevy_mesh::components::Mesh3d"
INHERITED = "bevy_camera::visibility::InheritedVisibility"
DISTANCE = 15.0
WAIT = {"frames": 900, "seconds": 45}


def rows(driver, components, with_=()):
    return driver._call(
        "world.query",
        {
            "data": {"components": [NAME], "option": components},
            "filter": {"with": [NAME, *with_]},
        },
    )


def by_name(driver, prefix, components=(), with_=()):
    return [
        row
        for row in rows(driver, list(components), with_)
        if row["components"][NAME].startswith(prefix)
    ]


def position(row):
    """World position from a reflected `GlobalTransform` (an affine: 3x3 matrix, translation)."""
    value = row["components"][GLOBAL]
    flat = value if isinstance(value, list) else sum(value.values(), [])
    return flat[-3:]


def run(driver):
    driver.present("PrimaryCamera")
    driver.present("SceneryElement", timeout=WAIT)
    driver.present("FloorRegion", timeout=WAIT)
    driver.present("Ground", timeout=WAIT)
    driver.frames(240)

    precincts = by_name(driver, "Precinct:")
    tiers = by_name(driver, "Tier:")
    floors = by_name(driver, "FloorRegion", [GLOBAL, INHERITED], [MESH])
    outlines = by_name(driver, "FloorOutline")
    elements = by_name(driver, "SceneryElement", [GLOBAL])
    print(
        f"{len(precincts)} precincts, {len(tiers)} tiers, {len(floors)} floor meshes, "
        f"{len(outlines)} floor outlines, {len(elements)} scenery elements"
    )
    assert tiers, "no tiers"
    assert floors, "no floor region got a mesh"
    assert all(row["components"][INHERITED] for row in floors), "a floor region is hidden"

    # Floor regions have identity transforms (their outlines are in precinct coordinates), so the
    # floors are visited at a fixed spot.
    spots = [("element", position(elements[0])), ("floors", [0.0, 0.0, 0.0])]
    for label, (x, _, z) in spots:
        print(f"{label} at ({x:.1f}, {z:.1f})")
        driver.message(
            GOTO,
            {"realm": "playground", "position": [x, 0.0, z], "camera_distance": DISTANCE},
        )
        driver.frames(240)
        driver.screenshot(label)
    driver.exits("Escape")
