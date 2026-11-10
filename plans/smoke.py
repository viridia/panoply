"""Smoke test: cheap checks across the game, in one launch.

    bevy-remote-driver plans/smoke.py

- Camera: the arrows move, strafe and turn the view, which the editor's saved view tracks, and
  = and - zoom it.
- Scenery: playground precincts spawn tiers, floor regions (with outlines) and scenery models.
  Screenshots at a scenery element (walls, and the road from terrain effects) and at the origin
  (the tiered dungeon floors of precinct 0,0).
- Editor: T shows the editor panel beside the view, and its mode selector and F2 switch modes;
  Realm mode alone draws its grid.
  Physics drawing and its Show menu are in `physics_views.py`, run on demand.
- Realm panel: its stats name the view's realm; its filter box does not take the keys; Go
  travels to the point selected; + asks for a name, refusing one in use, and adds the point,
  which enables Save; Ctrl+Z and Ctrl+Shift+Z undo and redo that, returning to Realm mode from
  another, and do nothing while a text field has the keys; Save writes `world.locations.json`
  (restored afterwards); − removes the point. Create Realm refuses a name in use or one that
  cannot name a file, writes a new realm (deleted afterwards) and cuts to it. There the map size
  control grows and shrinks the map, keeps a parcel, undoes, takes clicks turned with the view,
  and saves. Change Realm filters the realms and goes back, keeping the view's coordinates.
- Go To dialog: G opens it, centered over a barrier that covers the window, and Esc closes it.
  Typing filters the list; Go travels to the point clicked, and in the filter box Up and Down
  move the selection and Enter travels, to the first point shown if none is selected. An arrow
  held there does not turn the view. An arrow
  held there leaves the view where it was.
- Parcels: those outside the view are hidden, and so are their ground meshes, but stay cached;
  once `goto` returns, the readiness survey shows nothing pending.

Settings start empty, so the panel starts hidden.
"""

from pathlib import Path

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
PANEL = "panoply_editor::shell::EditorPanelSettings"
SAVED_VIEW = "panoply_editor::saved_view::EditorViewSettings"
INSET = "panoply_core::viewport::ViewportInset"
GAME_MODE = "panoply_core::game_mode::GameMode"
EDITOR_MODE = "panoply_editor::mode_selector::EditorMode"
BUTTON_VARIANT = "bevy_feathers::controls::button::ButtonVariant"
MODE_BUTTONS = ["RealmMode", "TerrainMode", "SceneryMode", "CatalogsMode", "PlayMode"]
WAIT = {"frames": 900, "seconds": 45}
# Forest, shoreline, water, grass and rocks.
SPOT = [110.0, 0.0, 150.0]
# Far from `SPOT`, past the playground's terrain.
FAR = [400.0, 0.0, 400.0]
# The nav point "test-inn", from `world.locations.json`.
TEST_INN = [56.65786741155058, 0.0, 37.0414409317615]
WAREHOUSE = [41.59800604609949, 0.0, 41.48915667709594]
COMPUTED = "bevy_ui::ui_node::ComputedNode"
UI_GLOBAL = "bevy_ui::ui_transform::UiGlobalTransform"
MODAL = "bevy_ui_widgets::modal::ModalDialog"
TEXT = "bevy_ui::widget::text::Text"
DISABLED = "bevy_ui::interaction_states::InteractionDisabled"
ASSETS = Path(__file__).resolve().parent.parent / "assets"
LOCATIONS = ASSETS / "world.locations.json"
# The arrows fly the view at 10 m/s.
CAMERA_SPEED = 10.0


def run(driver):
    driver.present("PrimaryCamera")
    driver.caption("camera")
    camera(driver)
    driver.caption("scenery")
    scenery(driver)
    driver.caption("editor")
    editor(driver)
    driver.caption("go to dialog")
    goto_dialog(driver)
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


def editor(driver):
    assert driver.get_resource(PANEL, at="visible") is False, "panel shown at start"

    # The panel is 328 wide by default, its narrowest (`MIN_PANEL_WIDTH`), and the view starts
    # after it and the handle.
    driver.key("KeyT")
    driver.frames(10)
    assert driver.get_resource(PANEL, at="visible") is True, "T did not show the panel"
    left = driver.get_resource(INSET, at="left")
    assert left == Between(328.0, 348.0), f"view not beside the panel: inset {left}"

    # The mode selector starts on Realm; its buttons choose the editor mode or play, and F2
    # switches to play and back to the editor mode last chosen.
    assert editor_mode(driver) == "Realm", f"started in {editor_mode(driver)}"
    # The starting mode has its panel too, not only a mode switched to.
    driver.present("RealmPanel")
    driver.screenshot("mode_selector")
    # Realm mode's grid is drawn only in Realm mode.
    driver.present("RealmGrid")
    driver.click("TerrainMode")
    wait_mode(driver, editor_mode, "Terrain")
    expect_highlight(driver, "TerrainMode")
    driver.absent("RealmGrid")
    driver.click("PlayMode")
    wait_mode(driver, game_mode, "Play")
    driver.key("F2")
    wait_mode(driver, game_mode, "Edit")
    assert editor_mode(driver) == "Terrain", f"F2 returned to {editor_mode(driver)}"
    driver.click("RealmMode")
    wait_mode(driver, editor_mode, "Realm")
    expect_highlight(driver, "RealmMode")
    realm_panel(driver)

    driver.key("KeyT")
    driver.frames(10)
    assert driver.get_resource(INSET, at="left") == 0.0, "view still inset with the panel hidden"


def realm_panel(driver):
    driver.present("RealmPanel/warehouse")
    driver.expect("RealmPanel/Realm", TEXT, "playground")
    driver.screenshot("realm_panel")

    # The panel's filter box does not take the focus: an arrow still turns the view.
    before = driver.get("PrimaryCamera", TRANSFORM)
    hold(driver, "ArrowLeft", 0.3)
    assert driver.get("PrimaryCamera", TRANSFORM) != before, "the panel's list took the arrows"

    driver.click("RealmPanel/warehouse")
    expect_disabled(driver, "RealmPanel/Go", False)
    driver.click("RealmPanel/Go")
    arrived(driver, WAREHOUSE, "the panel's Go")

    # + refuses a name in use, and adds the point under a new one, selected.
    expect_disabled(driver, "Save", True)
    driver.click("AddNavPoint")
    driver.present("TextEntryDialog/Entry")
    driver.type("warehouse")
    driver.expect("TextEntryDialog/Hint", TEXT, "Name in use")
    expect_disabled(driver, "TextEntryDialog/Ok", True)
    driver.type("-smoke")
    expect_disabled(driver, "TextEntryDialog/Ok", False)
    driver.screenshot("new_nav_point")
    driver.click("TextEntryDialog/Ok")
    driver.absent("TextEntryDialog")
    driver.present("RealmPanel/warehouse-smoke")
    expect_disabled(driver, "Save", False)

    # With the point unsaved, Escape asks first, and Cancel stays.
    driver.key("Escape")
    driver.present("QuitDialog")
    driver.click("QuitDialog/Cancel")
    driver.absent("QuitDialog")

    # Undo removes the point, and redo brings it back. Before Save, whose reload would overwrite
    # the edits (PAN57).
    undo(driver)
    driver.absent("RealmPanel/warehouse-smoke")
    driver.expect("Toast/ToastText", TEXT, "Undo: Remove nav point warehouse-smoke")
    undo(driver, redo=True)
    driver.present("RealmPanel/warehouse-smoke")
    driver.expect("Toast/ToastText", TEXT, "Redo: Add nav point warehouse-smoke")

    # Undo from another mode returns to the mode of the edit.
    driver.click("TerrainMode")
    wait_mode(driver, editor_mode, "Terrain")
    undo(driver)
    wait_mode(driver, editor_mode, "Realm")
    driver.absent("RealmPanel/warehouse-smoke")
    undo(driver, redo=True)
    driver.present("RealmPanel/warehouse-smoke")

    # While a text field has the keys, the history does not get them.
    driver.click("AddNavPoint")
    driver.present("TextEntryDialog/Entry")
    undo(driver)
    driver.click("TextEntryDialog/Cancel")
    driver.absent("TextEntryDialog")
    driver.present("RealmPanel/warehouse-smoke")

    # Undo cleared the selection, which − needs.
    driver.click("RealmPanel/warehouse-smoke")

    # Save writes the file. The original goes back however the checks end; the file watcher then
    # reloads it.
    original = LOCATIONS.read_bytes()
    try:
        driver.click("Save")
        expect_disabled(driver, "Save", True)
        # The barrier holds edits off until the file watcher's reload, which would overwrite them.
        driver.absent("SaveBarrier")
        assert '"warehouse-smoke"' in LOCATIONS.read_text(), "Save did not write the new point"
        driver.click("RemoveNavPoint")
        driver.absent("RealmPanel/warehouse-smoke")
        expect_disabled(driver, "Save", False)
        # Saved again, so that nothing is unsaved when the plan's last Escape quits.
        driver.click("Save")
        expect_disabled(driver, "Save", True)
        driver.absent("SaveBarrier")
    finally:
        LOCATIONS.write_bytes(original)

    create_and_change_realm(driver)


def create_and_change_realm(driver):
    # A realm's files are written at once, so they go however the checks end.
    files = [ASSETS / "realms" / "smoke-realm.realm.json", ASSETS / "terrain/maps/smoke-realm.terrain"]
    try:
        # Create Realm refuses a name in use, and one that cannot name a file.
        driver.click("CreateRealm")
        driver.present("TextEntryDialog/Entry")
        driver.type("playground")
        driver.expect("TextEntryDialog/Hint", TEXT, "Name in use")
        expect_disabled(driver, "TextEntryDialog/Ok", True)
        driver.click("TextEntryDialog/Cancel")
        driver.absent("TextEntryDialog")
        driver.click("CreateRealm")
        driver.present("TextEntryDialog/Entry")
        driver.type("a.b")
        driver.expect("TextEntryDialog/Hint", TEXT, "Letters, digits, - and _ only")
        expect_disabled(driver, "TextEntryDialog/Ok", True)
        driver.click("TextEntryDialog/Cancel")
        driver.absent("TextEntryDialog")

        # The view cuts to the new realm, which has no parcels and is still ready.
        driver.click("CreateRealm")
        driver.present("TextEntryDialog/Entry")
        driver.type("smoke-realm")
        driver.click("TextEntryDialog/Ok")
        driver.absent("TextEntryDialog")
        driver.expect("RealmPanel/Realm", TEXT, "smoke-realm")
        driver.expect("RealmPanel/Size", TEXT, "0 × 0 parcels")
        assert all(file.exists() for file in files), "Create Realm did not write both files"
        driver.goto([0.0, 0.0, 0.0], realm="smoke-realm")
        driver.screenshot("new_realm")

        # The map size control moves one edge a parcel at a time, and keeps at least one parcel.
        driver.click("GrowEast")
        driver.expect("RealmPanel/Size", TEXT, "1 × 1 parcels")
        driver.click("ShrinkNorth")
        driver.click("GrowSouth")
        driver.expect("RealmPanel/Size", TEXT, "1 × 2 parcels")
        undo(driver)
        driver.expect("RealmPanel/Size", TEXT, "1 × 1 parcels")
        driver.expect("Toast/ToastText", TEXT, "Undo: Resize map")
        undo(driver, redo=True)
        driver.expect("RealmPanel/Size", TEXT, "1 × 2 parcels")
        # Turned with the view, its buttons still take clicks.
        driver.goto([0.0, 0.0, 0.0], azimuth=1.0, realm="smoke-realm")
        driver.click("GrowWest")
        driver.expect("RealmPanel/Size", TEXT, "2 × 2 parcels")
        driver.screenshot("map_size")
        # Save writes the terrain map, which the reload brings back the same size.
        driver.click("Save")
        expect_disabled(driver, "Save", True)
        driver.absent("SaveBarrier")
        driver.expect("RealmPanel/Size", TEXT, "2 × 2 parcels")

        driver.goto(WAREHOUSE, realm="smoke-realm")

        # Change Realm lists the new realm, filters, and goes back.
        driver.click("ChangeRealm")
        driver.present("ChangeRealmDialog/smoke-realm")
        driver.type("playg")
        driver.absent("ChangeRealmDialog/smoke-realm")
        driver.click("ChangeRealmDialog/playground")
        driver.screenshot("change_realm")
        driver.click("ChangeRealmDialog/Go")
        driver.absent("ChangeRealmDialog")
        driver.expect("RealmPanel/Realm", TEXT, "playground")
        arrived(driver, WAREHOUSE, "Change Realm, keeping the position")
    finally:
        for file in files:
            file.unlink(missing_ok=True)


def undo(driver, redo=False):
    """Presses Ctrl+Z, or with `redo` Ctrl+Shift+Z."""
    driver.press("ControlLeft")
    if redo:
        driver.press("ShiftLeft")
    driver.key("KeyZ")
    if redo:
        driver.release("ShiftLeft")
    driver.release("ControlLeft")


def expect_disabled(driver, selector, disabled):
    """Waits until the widget `selector` is disabled, or enabled."""

    def check():
        (entity,) = driver.select(selector)
        found = driver.call(
            "world.get_components", {"entity": entity, "components": [DISABLED], "strict": False}
        )
        has = DISABLED in found["components"]
        return has == disabled, lambda: "disabled" if has else "enabled"

    state = "disabled" if disabled else "enabled"
    driver.wait(check, f"{selector} {state}", {"seconds": 2})


def game_mode(driver):
    return driver.get_resource(f"bevy_state::state::resources::State<{GAME_MODE}>")


def editor_mode(driver):
    return driver.get_resource(f"bevy_state::state::resources::State<{EDITOR_MODE}>")


def wait_mode(driver, read, expected):
    """Waits until `read` (`game_mode` or `editor_mode`) gives `expected`."""

    def check():
        found = read(driver)
        return found == expected, lambda: f"in {found}"

    driver.wait(check, f"the mode {expected}", {"seconds": 2})


def expect_highlight(driver, chosen):
    """Of the mode selector's buttons, only `chosen` is highlighted."""
    driver.frames(2)
    for button in MODE_BUTTONS:
        variant = driver.get(button, BUTTON_VARIANT)
        expected = "Primary" if button == chosen else "Normal"
        assert variant == expected, f"{button} is {variant}, not {expected}"


def arrived(driver, spot, how):
    driver.absent("GotoDialog")
    driver.frames(2)
    here = view(driver)["position"]
    print(f"{how}: at {here}")
    assert abs(here[0] - spot[0]) < 0.01 and abs(here[2] - spot[2]) < 0.01, f"{how}: at {here}"


def goto_dialog(driver):
    # Esc closes the dialog, and only the dialog: outside it, Esc quits.
    driver.key("KeyG")
    driver.present("GotoDialog/warehouse")
    driver.frames(10)
    # The barrier covers the window, and centers the dialog in it.
    barrier = driver.get("GotoDialog", COMPUTED, at="size")
    barrier_at = driver.get("GotoDialog", UI_GLOBAL)[-2:]
    (modal,) = driver.call(
        "world.query", {"data": {"components": [COMPUTED, UI_GLOBAL]}, "filter": {"with": [MODAL]}}
    )
    dialog = modal["components"][COMPUTED]["size"]
    dialog_at = modal["components"][UI_GLOBAL][-2:]
    print(f"barrier {barrier} at {barrier_at}, dialog {dialog} at {dialog_at}")
    assert barrier_at == [barrier[0] / 2, barrier[1] / 2], "the barrier does not cover the window"
    assert dialog_at == barrier_at and dialog[1] < barrier[1], "the dialog is not centered"
    driver.screenshot("goto_dialog_open")
    driver.key("Escape")
    driver.absent("GotoDialog")

    # `tein` matches only "test-inn", by its words' starts; the filter box has focus as it opens.
    driver.key("KeyG")
    driver.present("GotoDialog/warehouse")
    driver.type("tein")
    driver.absent("GotoDialog/warehouse")
    driver.present("GotoDialog/test-inn")
    driver.click("GotoDialog/test-inn")
    driver.screenshot("goto_dialog")
    driver.click("GotoDialog/Go")
    arrived(driver, TEST_INN, "click and Go")

    # In the filter box, Down and Up move the selection, and Enter travels.
    driver.key("KeyG")
    driver.present("GotoDialog/warehouse")
    driver.type("wareh")
    driver.present("GotoDialog/warehouse2")
    driver.key("ArrowDown")
    driver.key("ArrowDown")
    driver.key("ArrowUp")
    driver.key("Enter")
    arrived(driver, WAREHOUSE, "arrows and Enter")

    # The filter box keeps a held arrow from the camera for the whole hold, not just its first
    # frame. Then, with nothing selected, Enter travels to the first point shown.
    driver.key("KeyG")
    driver.present("GotoDialog/warehouse")
    driver.type("tein")
    driver.absent("GotoDialog/warehouse")
    before = driver.get("PrimaryCamera", TRANSFORM)
    hold(driver, "ArrowLeft", 0.5)
    after = driver.get("PrimaryCamera", TRANSFORM)
    assert before == after, f"a held arrow in the filter box moved the view: {before} to {after}"
    driver.key("Enter")
    arrived(driver, TEST_INN, "Enter")


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
