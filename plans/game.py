"""The game: cheap checks across it, in one launch, and a glance at each editor mode.

    bevy-remote-driver plans/game.py

- Time: a frame catches up at most four fixed steps.
- Camera: the arrows move, strafe and turn the view, which the editor's saved view tracks, and
  = and - zoom it.
- Scenery: playground precincts spawn tiers, floor regions (with outlines) and scenery models.
  Screenshots at a scenery element (walls, and the road from terrain effects) and at the origin
  (the tiered dungeon floors of precinct 0,0).
- Actors: after that tour, which evicts the ground under the actors at the start, none is falling.
- Editor: T shows the editor panel beside the view, starting on Realm. Each mode's button
  chooses it, highlighted, its panel shown, in a screenshot each; F2 switches to play and back to
  the mode last chosen, and T hides the panel again. Each area's own checks are in its plan,
  tagged `editor`: `scenery.py`, `properties.py`, `terrain.py`, `realm.py`.
- Parcels: those outside the view are hidden, and so are their ground meshes, but stay cached;
  once `goto` returns, the readiness survey shows nothing pending.

Settings start empty, so the panel starts hidden.
"""

from bevy_remote_driver import Between

from _editor import (
    GLOBAL,
    INHERITED,
    INSET,
    MESH,
    MOTION,
    NAME,
    PANEL,
    SPOT,
    VISIBILITY,
    WAIT,
    by_name,
    editor_mode,
    expect_highlight,
    game_mode,
    hold,
    named,
    view,
    wait_mode,
)
from _panoply_driver import PanoplyDriver, readiness_grid

BIN = "panoply"
DRIVER = PanoplyDriver
SAVED_VIEW = "panoply_editor::saved_view::EditorViewSettings"
VIRTUAL = "bevy_time::time::Time<bevy_time::virt::Virtual>"
# Far from `SPOT`, past the playground's terrain.
FAR = [400.0, 0.0, 400.0]
# The arrows fly the view at 10 m/s.
CAMERA_SPEED = 10.0
# Each editor mode, and its panel where it has one.
MODES = [
    ("Realm", "RealmPanel"),
    ("Terrain", "TerrainPanel"),
    ("Scenery", "SceneryPanel"),
    ("Catalogs", None),
]


def run(driver):
    driver.present("PrimaryCamera")
    # A frame catches up at most four fixed steps, 62.5 ms at 64 Hz, so overload slows the game
    # rather than freezing it (D68).
    cap = driver.get_resource(VIRTUAL, at="context.max_delta")
    assert cap == {"secs": 0, "nanos": 62_500_000}, f"catch-up cap {cap}, not 62.5 ms"
    driver.caption("camera")
    camera(driver)
    driver.caption("scenery")
    scenery(driver)
    driver.caption("actors")
    actors_hold(driver)
    driver.caption("editor")
    editor(driver)
    driver.caption("parcels")
    parcels(driver)
    driver.exits("Escape")


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
    # An arrow turning the view is checked with the realm panel's filter box, in `realm.py`.

    saved = driver.get_resource(SAVED_VIEW)
    assert saved["realm"] == "playground", saved
    assert all(abs(a - b) < 0.5 for a, b in zip(saved["position"], strafed)), saved

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


def actors_hold(driver):
    """After the tour, which evicts the parcels and precincts around the actors at the start, no
    actor is falling: one whose ground is gone holds still until it is built again."""
    driver.frames(10)
    rows = named(driver, [MOTION], [MOTION])
    falling = [row["components"][NAME] for row in rows if row["components"][MOTION]["falling"]]
    print(f"{len(rows)} actors with physics, {len(falling)} falling")
    assert rows, "no actor has physics"
    assert not falling, f"actors falling after the tour: {falling}"


def position(row):
    """World position from a reflected `GlobalTransform` (an affine: 3x3 matrix, translation)."""
    value = row["components"][GLOBAL]
    flat = value if isinstance(value, list) else sum(value.values(), [])
    return flat[-3:]


def scenery(driver):
    driver.present("SceneryElement", timeout=WAIT)
    driver.present("FloorRegion", timeout=WAIT)

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


def editor(driver):
    """Each editor mode opens, at `SPOT`; the areas' own checks are in their plans."""
    driver.goto(SPOT, 20.0)
    assert driver.get_resource(PANEL, at="visible") is False, "panel shown at start"
    # The panel is 328 wide by default, its narrowest (`MIN_PANEL_WIDTH`), and the view starts
    # after it and the handle.
    driver.key("KeyT")
    driver.frames(10)
    assert driver.get_resource(PANEL, at="visible") is True, "T did not show the panel"
    left = driver.get_resource(INSET, at="left")
    assert left == Between(328.0, 348.0), f"view not beside the panel: inset {left}"
    assert editor_mode(driver) == "Realm", f"started in {editor_mode(driver)}"

    for mode, panel in MODES:
        driver.click(f"{mode}Mode")
        wait_mode(driver, editor_mode, mode)
        expect_highlight(driver, f"{mode}Mode")
        if panel:
            driver.present(panel)
        driver.frames(5)
        driver.screenshot(f"{mode.lower()}_mode")

    driver.click("PlayMode")
    wait_mode(driver, game_mode, "Play")
    driver.key("F2")
    wait_mode(driver, game_mode, "Edit")
    assert editor_mode(driver) == "Catalogs", f"F2 returned to {editor_mode(driver)}"

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
