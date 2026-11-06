"""Smoke test: cheap checks across the game, in one launch.

    bevy-remote-driver plans/smoke.py

- Camera: the arrows move, strafe and turn the view, and = and - zoom it.
- Scenery: playground precincts spawn tiers, floor regions (with outlines) and scenery models.
  Screenshots at a scenery element (walls, and the road from terrain effects) and at the origin
  (the tiered dungeon floors of precinct 0,0).
- Physics and editor: F3 draws the colliders; a ray cast down at the view center hits the
  ground, then a floor's, a fixture's and a tree's canopy colliders; T shows the editor panel
  beside the view, and its Show menu turns views off. `GroundProbe` holds where the ray hit,
  drawn as a white sphere. The Show menu is clicked twice: the second click lands only if the
  first left the menu open. Space toggles the focused checkbox, so keys reach widgets.
- Parcels: those outside the view are hidden, and so are their ground meshes, but stay cached;
  once `goto` returns, the readiness survey shows nothing pending.

Settings start empty, so physics drawing starts off and the panel hidden.
"""

from bevy_remote_driver import Between

from _panoply_driver import PanoplyDriver, readiness_grid

BIN = "panoply"
DRIVER = PanoplyDriver
NAME = "bevy_ecs::name::Name"
TRANSFORM = "bevy_transform::components::transform::Transform"
GLOBAL = "bevy_transform::components::global_transform::GlobalTransform"
MESH = "bevy_mesh::components::Mesh3d"
VISIBILITY = "bevy_camera::visibility::Visibility"
INHERITED = "bevy_camera::visibility::InheritedVisibility"
VIEW_READY = "panoply_editor::diagnostics::view_ready::ViewReady"
VIEWS = "panoply_editor::debug_views::DebugViews"
PROBE = "panoply_editor::debug_views::GroundProbe"
PANEL = "panoply_editor::shell::EditorPanelSettings"
INSET = "panoply_core::viewport::ViewportInset"
WAIT = {"frames": 900, "seconds": 45}
# Forest, shoreline, water, grass and rocks.
SPOT = [110.0, 0.0, 150.0]
# Far from `SPOT`, past the playground's terrain.
FAR = [400.0, 0.0, 400.0]
# The arrows fly the view at 10 m/s.
CAMERA_SPEED = 10.0
TERRAIN_GROUP = 1
FLOOR_GROUP = 2
FIXTURE_GROUP = 8
CANOPY_GROUP = 0x20
# (label, spot, expected top, group): a parquet floor on tier 0 (top at TIER_OFFSET), a walnut
# table (box of half-height 0.42 on the floor at 0), and a tree's cell, whose canopy is 2 m over
# ground at 0. A trunk is as tall as the canopy over its own base, so one on higher ground pokes
# through it; this cell's trunk does not reach its center.
COLLIDER_SPOTS = [
    ("floor", [58.0, 0.0, 26.62], 0.02, FLOOR_GROUP),
    ("table", [42.23, 0.0, 58.71], 0.84, FIXTURE_GROUP),
    ("canopy", [105.5, 0.0, 144.5], 2.0, CANOPY_GROUP),
]


def run(driver):
    driver.present("PrimaryCamera")
    driver.caption("camera")
    camera(driver)
    driver.caption("scenery")
    scenery(driver)
    driver.caption("physics and editor")
    physics(driver)
    driver.caption("parcels")
    parcels(driver)
    driver.exits("Escape")


def view(driver):
    return driver.get_resource(VIEW_READY)


def hold(driver, code, seconds):
    """Holds `code` for `seconds`; `key(hold=...)` returns before the release."""
    driver.press(code)
    driver.seconds(seconds)
    driver.release(code)
    driver.frames(2)


def camera(driver):
    # Facing +z, so forward is +z and a strafe is along x.
    driver.goto(SPOT, 20.0, azimuth=0.0)
    start = view(driver)["position"]
    hold(driver, "ArrowUp", 0.5)
    forward = view(driver)["position"]
    print(f"forward: {start} to {forward}")
    assert forward[2] - start[2] == Between(0.2 * CAMERA_SPEED, 0.8 * CAMERA_SPEED), forward
    assert abs(forward[0] - start[0]) < 0.1, f"forward drifted sideways: {forward}"

    # Shift+Arrow strafes rather than turns.
    driver.press("ShiftLeft")
    hold(driver, "ArrowLeft", 0.5)
    driver.release("ShiftLeft")
    strafed = view(driver)["position"]
    print(f"strafed: {forward} to {strafed}")
    assert abs(strafed[0] - forward[0]) == Between(0.2 * CAMERA_SPEED, 0.8 * CAMERA_SPEED)
    assert abs(strafed[2] - forward[2]) < 0.1, f"strafe moved forward: {strafed}"

    before = driver.get("PrimaryCamera", TRANSFORM)["rotation"]
    hold(driver, "ArrowRight", 0.3)
    after = driver.get("PrimaryCamera", TRANSFORM)["rotation"]
    assert before != after, f"the view did not turn: {before}"

    # Zooming in brings the camera closer. (Plans run with their own settings, so this never
    # touches the developer's saved zoom.)
    driver.key("Equal")
    driver.seconds(0.4)
    near = view(driver)["camera_distance"]
    driver.key("Minus")
    driver.seconds(0.4)
    far = view(driver)["camera_distance"]
    print(f"zoomed: {near} in, {far} out")
    assert near < far, f"= did not zoom in: {near}, then {far}"


def named(driver, components=(), with_=()):
    """Rows of every named entity with `with_`, with `components` where present."""
    return driver.call(
        "world.query",
        {
            "data": {"components": [NAME], "option": list(components)},
            "filter": {"with": [NAME, *with_]},
        },
    )


def by_name(driver, prefix, components=(), with_=()):
    return [
        row
        for row in named(driver, components, with_)
        if row["components"][NAME].startswith(prefix)
    ]


def position(row):
    """World position from a reflected `GlobalTransform` (an affine: 3x3 matrix, translation)."""
    value = row["components"][GLOBAL]
    flat = value if isinstance(value, list) else sum(value.values(), [])
    return flat[-3:]


def scenery(driver):
    driver.present("SceneryElement", timeout=WAIT)
    driver.present("FloorRegion", timeout=WAIT)
    driver.present("Ground", timeout=WAIT)

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
    # The element with the lowest x (then z), so each run visits the same spot, and loads the same
    # precincts, whatever order the query returns.
    element = min(elements, key=lambda row: (position(row)[0], position(row)[2]))
    spots = [("element", position(element)), ("floors", [0.0, 0.0, 0.0])]
    for label, (x, _, z) in spots:
        print(f"{label} at ({x:.1f}, {z:.1f})")
        driver.goto([x, 0.0, z], 15.0)
        driver.screenshot(label)


def physics(driver):
    driver.goto(SPOT, 20.0)
    assert driver.get_resource(VIEWS, at="physics") is False, "physics drawing on at start"
    assert driver.get_resource(PROBE, at="hit") is None, "probe set while drawing is off"
    assert driver.get_resource(PANEL, at="visible") is False, "panel shown at start"

    driver.key("F3")
    driver.frames(10)
    assert driver.get_resource(VIEWS, at="physics") is True, "F3 did not turn physics on"
    hit = driver.get_resource(PROBE, at="hit")
    print(f"ground under the view center: {hit}")
    assert hit is not None, "the ray hit nothing"
    x, y, z = hit
    assert abs(x - SPOT[0]) < 0.01 and abs(z - SPOT[2]) < 0.01, f"hit off the view center: {hit}"
    assert -20.0 < y < 40.0, f"hit at an unlikely height: {y}"
    groups = driver.get_resource(PROBE, at="groups")
    assert groups == TERRAIN_GROUP, f"hit groups {groups:#x}, not terrain"
    driver.screenshot("colliders")

    # Scenery and canopy colliders: the probe stops on top of each.
    for label, spot, top, group in COLLIDER_SPOTS:
        driver.goto(spot, 20.0)
        driver.frames(5)
        hit = driver.get_resource(PROBE, at="hit")
        groups = driver.get_resource(PROBE, at="groups")
        print(f"{label}: hit {hit}, groups {groups:#x}")
        assert hit is not None, f"{label}: the ray hit nothing"
        assert hit[1] == Between(top - 0.01, top + 0.01), f"{label}: hit at {hit[1]}, not {top}"
        assert groups == group, f"{label}: hit groups {groups:#x}, not {group:#x}"
        if label == "table":
            driver.screenshot("scenery_colliders")
    driver.screenshot("tree_colliders")

    # The panel is 300 wide by default, and the view starts after it and the handle.
    driver.key("KeyT")
    driver.frames(10)
    assert driver.get_resource(PANEL, at="visible") is True, "T did not show the panel"
    left = driver.get_resource(INSET, at="left")
    assert left == Between(300.0, 320.0), f"view not beside the panel: inset {left}"

    driver.click("Show")
    driver.click("Terrain")
    driver.frames(5)
    groups = driver.get_resource(VIEWS, at="physics_groups")
    assert groups & TERRAIN_GROUP == 0, f"Terrain still drawn: groups {groups:#x}"
    driver.screenshot("show_menu")
    # Widgets get keys from the mapper (`widget_keys`): Space toggles the checkbox just clicked.
    driver.key("Space")
    driver.frames(5)
    groups = driver.get_resource(VIEWS, at="physics_groups")
    assert groups & TERRAIN_GROUP, f"Space did not toggle the focused checkbox: {groups:#x}"
    driver.click("Physics")
    driver.frames(5)
    assert driver.get_resource(VIEWS, at="physics") is False, "Physics did not turn off"

    driver.key("KeyT")
    driver.frames(10)
    assert driver.get_resource(INSET, at="left") == 0.0, "view still inset with the panel hidden"


def parcel_visibility(driver):
    """Map of parcel name to its `Visibility`."""
    return {
        row["components"][NAME]: row["components"][VISIBILITY]
        for row in named(driver, [VISIBILITY], [VISIBILITY])
        if row["components"][NAME].startswith("Parcel:")
    }


def visible_ground(driver):
    return sum(
        1
        for row in named(driver, [INHERITED], [INHERITED])
        if row["components"][NAME] == "Ground" and row["components"][INHERITED]
    )


def report(driver, label):
    cached = parcel_visibility(driver)
    hidden = sum(1 for v in cached.values() if v == "Hidden")
    print(
        f"{label}: {len(cached)} parcels cached, {hidden} hidden, "
        f"{visible_ground(driver)} ground meshes visible"
    )
    return cached


def parcels(driver):
    driver.goto(SPOT, 20.0)
    first = report(driver, "first spot")
    in_view = {name for name, v in first.items() if v != "Hidden"}
    assert in_view, "no parcels in view"
    # Once `goto` has waited for the view, the survey shows nothing left to do, and the same
    # parcels shown as the query above.
    survey = driver.readiness()
    print(readiness_grid(survey))
    stuck = [p for p in survey["parcels"] + survey["precincts"] if p["pending"]]
    assert survey["ready"] and not stuck, f"still pending after goto: {stuck[:5]}"
    shown = sum(1 for p in survey["parcels"] if p["shown"])
    assert shown == len(in_view), f"survey shows {shown} parcels, the query {len(in_view)}"

    driver.goto(FAR, 20.0)
    second = report(driver, "far spot")
    stale = [name for name in in_view if name in second]
    still_shown = [name for name in stale if second[name] != "Hidden"]
    print(f"first spot's parcels still cached: {len(stale)}, of which shown: {len(still_shown)}")
    assert not still_shown, f"out-of-view parcels still shown: {still_shown[:5]}"
