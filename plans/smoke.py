"""Smoke test: cheap checks across the game, in one launch.

    bevy-remote-driver plans/smoke.py

- Time: a frame catches up at most four fixed steps.
- Camera: the arrows move, strafe and turn the view, which the editor's saved view tracks, and
  = and - zoom it.
- Scenery: playground precincts spawn tiers, floor regions (with outlines) and scenery models.
  Screenshots at a scenery element (walls, and the road from terrain effects) and at the origin
  (the tiered dungeon floors of precinct 0,0).
- Actors: after that tour, which evicts the ground under the actors at the start, none is falling.
- Editor: T shows the editor panel beside the view, and its mode selector and F2 switch modes;
  Realm mode alone draws its grid. Terrain mode's palette chooses its tool, kept through play;
  its rotate buttons wait for a selected parcel, whose contour is drawn. A brush tool's cursor
  follows the pointer over that parcel and stays on its edge while dragged off it. Scenery mode
  lists abstract exemplars too, and shows the chosen exemplar's properties: a number typed there
  changes it, undoably, and Save writes that one line into its catalog (restored afterwards). A
  text field edits on Enter and Escape reverts it; a model field's button (an actor's skin, a
  fixture's model) opens a picker of its kind's models by file, its model selected, and Save or a
  double-click sets it; an optional
  field's × removes it and the heading's menu adds one; each saves as its own line. A collider list adds and removes items,
  a collider's shape select switches its variant, and an added collider saves into the array.
  Physics drawing and its Show menu are in `physics_views.py`, run on demand.
- Realm panel: its stats name the view's realm; its filter box does not take the keys; Go
  travels to the point selected; + asks for a name, refusing one in use, and adds the point,
  which enables Save; Ctrl+Z and Ctrl+Shift+Z undo and redo that, returning to Realm mode from
  another, and do nothing while a text field has the keys; Save writes `world.locations.json`
  (restored afterwards); − removes the point. Create Realm refuses a name in use or one that
  cannot name a file, writes a new realm (deleted afterwards) and cuts to it. There the map size
  control grows and shrinks the map, keeps a parcel, undoes, takes clicks turned with the view,
  and saves; the playground's selected parcel does not count there, and a parcel selected there
  turns, undoably. Change Realm lists the new realm and goes back, keeping the view's
  coordinates.
- Go To dialog: G opens it, centered over a barrier that covers the window, and Esc closes it.
  Its filter box has the keys as it opens: typing filters the list, a held arrow leaves the view
  where it was, Up and Down move the selection, and Enter travels to the point selected, or to
  the first shown if none is. A double-click travels to its point.
- Parcels: those outside the view are hidden, and so are their ground meshes, but stay cached;
  once `goto` returns, the readiness survey shows nothing pending.

Settings start empty, so the panel starts hidden.
"""

import difflib
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
TERRAIN_TOOL = "panoply_editor::mode_selector::TerrainTool"
SCENERY_TOOL = "panoply_editor::mode_selector::SceneryTool"
SELECTION = "panoply_editor::parcel_selection::ParcelSelection"
MOTION = "panoply_actors::actor_physics::ActorMotion"
VIRTUAL = "bevy_time::time::Time<bevy_time::virt::Virtual>"
BUTTON_VARIANT = "bevy_feathers::controls::button::ButtonVariant"
MODE_BUTTONS = ["RealmMode", "TerrainMode", "SceneryMode", "CatalogsMode", "PlayMode"]
TOOL_BUTTONS = [
    "RaiseDraw", "LowerDraw", "FlattenDraw", "RaiseRect", "LowerRect", "FlattenRect",
    "DrawTrees", "DrawShrubs", "DrawHerbs", "EraseFlora", "RotateCcw", "RotateCw",
]
SCENERY_BUTTONS = [
    "PlaceFloor", "PlaceWall", "PlaceFixture", "PlaceActor", "DrawTerrainFx", "Interact",
    "Layers", "RectSelect", "RotateCcw", "RotateCw",
]
WAIT = {"frames": 900, "seconds": 45}
# Forest, shoreline, water, grass and rocks.
SPOT = [110.0, 0.0, 150.0]
# The parcel under `SPOT`, 16 m on a side.
SPOT_PARCEL = [6, 9]
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
NUMERIC = "bevy_ui_widgets::numeric::NumericValue"
SELECTED_TREE_ITEM = "bevy_ui_widgets::tree::SelectedTreeItem"
EXPANDED = "bevy_ui::interaction_states::Expanded"
SCENERY_CHOICES = "panoply_editor::exemplar_list::SceneryChoices"
TERRAIN_CURSOR = "panoply_editor::terrain_cursor::TerrainCursor"
REALM_GRID = "panoply_editor::mode_overlays::RealmGrid"
WINDOW = "bevy_window::window::Window"
WINDOW_EVENT = "bevy_window::event::WindowEvent"
ASSETS = Path(__file__).resolve().parent.parent / "assets"
LOCATIONS = ASSETS / "world.locations.json"
CHARACTERS = ASSETS / "exemplars" / "characters.exem.json"
FURNISHINGS = ASSETS / "exemplars" / "furnishings.exem.json"
# The arrows fly the view at 10 m/s.
CAMERA_SPEED = 10.0


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
    # An arrow turning the view is checked with the realm panel's filter box.

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
    # Realm mode's grid has precincts; terrain mode's, parcels only.
    assert driver.get("RealmGrid", REALM_GRID)["parcels_only"] is False
    driver.click("TerrainMode")
    wait_mode(driver, editor_mode, "Terrain")
    expect_highlight(driver, "TerrainMode")
    wait_mode(driver, lambda d: d.get("RealmGrid", REALM_GRID)["parcels_only"], True)
    # Terrain mode's palette starts on Raise and chooses the tool.
    driver.present("TerrainPanel")
    assert terrain_tool(driver) == "RaiseDraw", f"started with {terrain_tool(driver)}"
    driver.click("DrawShrubs")
    wait_mode(driver, terrain_tool, "DrawShrubs")
    expect_highlight(driver, "DrawShrubs", TOOL_BUTTONS)
    driver.screenshot("terrain_panel")
    # The rotate buttons wait for a selected parcel, whose contour is drawn. The driver cannot
    # click the ground, so the plan selects through the setting.
    expect_disabled(driver, "RotateCw", True)
    driver.goto(SPOT, 20.0)
    select_parcel(driver, "playground", SPOT_PARCEL)
    expect_disabled(driver, "RotateCw", False)
    driver.present("SelectedContour")
    driver.screenshot("selected_parcel")
    terrain_cursor(driver)
    driver.click("PlayMode")
    wait_mode(driver, game_mode, "Play")
    driver.key("F2")
    wait_mode(driver, game_mode, "Edit")
    assert editor_mode(driver) == "Terrain", f"F2 returned to {editor_mode(driver)}"
    # Leaving terrain mode and returning keeps its tool.
    wait_mode(driver, terrain_tool, "DrawShrubs")
    scenery_panel(driver)
    driver.click("RealmMode")
    wait_mode(driver, editor_mode, "Realm")
    realm_panel(driver)

    driver.key("KeyT")
    driver.frames(10)
    assert driver.get_resource(INSET, at="left") == 0.0, "view still inset with the panel hidden"


def scenery_panel(driver):
    """Scenery mode's palette starts on Floor and chooses the tool; the buttons that act on a
    selection or an exemplar wait for them."""
    driver.click("SceneryMode")
    wait_mode(driver, editor_mode, "Scenery")
    driver.present("SceneryPanel")
    assert scenery_tool(driver) == "PlaceFloor", f"started with {scenery_tool(driver)}"
    driver.click("PlaceWall")
    wait_mode(driver, scenery_tool, "PlaceWall")
    expect_highlight(driver, "PlaceWall", SCENERY_BUTTONS)
    for button in ["RotateCw", "SceneryPanel/Cut", "SceneryPanel/New", "SceneryPanel/Clone"]:
        expect_disabled(driver, button, True)

    # The wall tool lists the walls, after a row that erases, as the exemplars load.
    driver.present("Exemplars/CottageWall", timeout=WAIT)
    driver.present("Exemplars/(erase)")
    # The list is longer than the panel: the filter brings the row into view.
    driver.click("Exemplars/Filter")
    driver.type("CottageWall")
    driver.absent("Exemplars/BasementWall")
    driver.click("Exemplars/CottageWall")
    expect_choice(driver, "wall", "CottageWall")
    driver.screenshot("scenery_panel")
    # The filter box's × empties it, showing every wall again.
    driver.click("Exemplars/ClearFilter")
    driver.present("Exemplars/BasementWall")
    # The actor tool lists actors, the abstract ones they extend too, so they can be edited.
    driver.click("PlaceActor")
    driver.present("Exemplars/Marcus", timeout=WAIT)
    driver.present("Exemplars/Proto.HumanoidMale")
    driver.absent("Exemplars/CottageWall")
    exemplar_properties(driver)
    text_property(driver)
    model_property(driver)
    driver.click("PlaceFixture")
    scenery_model_property(driver)
    optional_property(driver)
    collider_property(driver)
    driver.click("Exemplars/ClearFilter")
    # Returning to a tool keeps its choice, highlighted (seen in the screenshot: `Selected` is not
    # reflected, so the plan cannot read it).
    driver.click("PlaceWall")
    driver.present("Exemplars/CottageWall")
    expect_choice(driver, "wall", "CottageWall")
    # The row highlights, then scrolls into view after a layout pass.
    driver.frames(10)
    driver.screenshot("scenery_choice_kept")


def exemplar_properties(driver):
    """The chosen exemplar's properties show below the list: its own pane, then one per aspect. A
    number typed into one changes the exemplar, undoably."""
    driver.click("Exemplars/Filter")
    driver.type("FeralDogAlpha")
    driver.click("Exemplars/FeralDogAlpha")
    expect_choice(driver, "actor", "FeralDogAlpha")
    driver.present("Properties/extends")
    scale = "Properties/ActorScale/input"
    # An f32 field's input holds its shortest decimal (I39).
    loaded = {"F64": 1.2}
    edited = {"F64": 1.5}
    driver.expect(scale, NUMERIC, loaded)
    driver.screenshot("exemplar_properties")
    driver.click(scale)
    driver.key("End")
    for _ in range(8):
        driver.key("Backspace")
    driver.type("1.5")
    driver.key("Enter")
    driver.expect(scale, NUMERIC, edited)
    # Undo goes back to the value before the edit, and redo forward again. The input keeps the
    # keys, which the history waits for: the list takes them back.
    driver.click("Exemplars/FeralDogAlpha")
    undo(driver)
    driver.expect("Toast/ToastText", TEXT, "Undo: Set ActorScale")
    driver.expect(scale, NUMERIC, loaded)
    undo(driver, redo=True)
    driver.expect(scale, NUMERIC, edited)

    # Save writes the edit into the catalog, changing that line and no other. The original goes
    # back however the checks end.
    original = CHARACTERS.read_bytes()
    try:
        expect_disabled(driver, "Save", False)
        driver.click("Save")
        expect_disabled(driver, "Save", True)
        driver.absent("SaveBarrier")
        changed = [
            (before, after)
            for before, after in zip(original.decode().splitlines(), CHARACTERS.read_text().splitlines())
            if before != after
        ]
        assert changed == [('      "ActorScale": 1.2', '      "ActorScale": 1.5')], changed
        # The reload brings the edit back from the file.
        driver.expect(scale, NUMERIC, edited)
        # Undone and saved again, the file is as it was.
        driver.click("Exemplars/FeralDogAlpha")
        undo(driver)
        driver.expect(scale, NUMERIC, loaded)
        driver.click("Save")
        expect_disabled(driver, "Save", True)
        driver.absent("SaveBarrier")
        assert CHARACTERS.read_bytes() == original, "saving the undone edit changed the catalog"
    finally:
        CHARACTERS.write_bytes(original)
    driver.click("Exemplars/ClearFilter")


def text_property(driver):
    """A text field's edit is made when Enter commits it; Escape puts the text back, editing
    nothing. Saved, the edit is that one line."""
    choose(driver, "actor", "Proto.Wasp")
    armature = "Properties/Armature/armature/input"
    old = "characters/wasp.glb#Armature"
    new = "characters/wasp.glb#Rig"
    retype(driver, armature, "not an armature")
    driver.key("Escape")
    driver.frames(2)
    expect_disabled(driver, "Save", True)
    retype(driver, armature, new)
    driver.key("Enter")
    expect_disabled(driver, "Save", False)
    driver.click("Exemplars/Proto.Wasp")
    undo(driver)
    driver.expect("Toast/ToastText", TEXT, "Undo: Set Armature.armature")
    undo(driver, redo=True)
    saved_diff(
        driver,
        CHARACTERS,
        "Exemplars/Proto.Wasp",
        [f'-        "armature": "{old}"', f'+        "armature": "{new}"'],
    )


def model_property(driver):
    """A model field's button opens a picker of the character models, a row per file holding a
    row per model, with the field's model selected and its file open. A click on a file's row
    opens it; Cancel changes nothing; Save sets the field, undoably, as does a double-click.
    Saved, the edit is that one line."""
    choose(driver, "actor", "SergeantStone")
    pick = "Properties/Skin/pick"
    old = "characters/humanoid-male.glb#WithCoatHeavy"
    new = "characters/humanoid-male.glb#WithRobe"
    driver.expect(f"{pick}/model", TEXT, "humanoid-male.glb#WithCoatHeavy")
    driver.click(pick)
    current = "ModelPicker/humanoid-male/WithCoatHeavy"
    driver.present(current)
    (row,) = driver.select(current)
    driver.expect("ModelPicker/Tree", SELECTED_TREE_ITEM, row)
    driver.expect("ModelPicker/humanoid-male", EXPANDED, {})
    # Every named scene is listed, features too, but not the default scene.
    driver.present("ModelPicker/humanoid-male/HairLong")
    driver.absent("ModelPicker/humanoid-male/Scene")
    driver.screenshot("model_picker")
    # A file's row opens on a click.
    driver.click("ModelPicker/canine")
    driver.expect("ModelPicker/canine", EXPANDED, {})
    driver.click("ModelPicker/Cancel")
    driver.absent("ModelPicker")
    expect_disabled(driver, "Save", True)

    driver.click(pick)
    driver.click("ModelPicker/humanoid-male/WithRobe")
    driver.click("ModelPicker/Save")
    driver.absent("ModelPicker")
    driver.expect(f"{pick}/model", TEXT, "humanoid-male.glb#WithRobe")
    driver.click("Exemplars/SergeantStone")
    undo(driver)
    driver.expect("Toast/ToastText", TEXT, "Undo: Set Skin")
    driver.expect(f"{pick}/model", TEXT, "humanoid-male.glb#WithCoatHeavy")

    # A double-click chooses too.
    driver.click(pick)
    driver.click("ModelPicker/humanoid-male/WithRobe")
    driver.click("ModelPicker/humanoid-male/WithRobe")
    driver.absent("ModelPicker")
    driver.expect(f"{pick}/model", TEXT, "humanoid-male.glb#WithRobe")
    saved_diff(
        driver,
        CHARACTERS,
        "Exemplars/SergeantStone",
        [f'-      "Skin": "{old}",', f'+      "Skin": "{new}",'],
    )


def scenery_model_property(driver):
    """A fixture's model, in its models list, is picked the same way, from the scenery and
    terrain models. Saved, it is that one line, still a plain string."""
    choose(driver, "fixture", "CandleHolder")
    pick = "Properties/SceneryModels/Item 1/asset/pick"
    driver.expect(f"{pick}/model", TEXT, "decorations.glb#CandleHolder")
    driver.click(pick)
    current = "ModelPicker/decorations/CandleHolder"
    driver.present(current)
    (row,) = driver.select(current)
    driver.expect("ModelPicker/Tree", SELECTED_TREE_ITEM, row)
    driver.present("ModelPicker/carpentry")
    driver.present("ModelPicker/bushes")
    driver.absent("ModelPicker/canine")
    driver.click("ModelPicker/decorations/CandleStand")
    driver.click("ModelPicker/Save")
    driver.absent("ModelPicker")
    driver.expect(f"{pick}/model", TEXT, "decorations.glb#CandleStand")
    saved_diff(
        driver,
        FURNISHINGS,
        "Exemplars/CandleHolder",
        [
            '-          "asset": "scenery/models/decorations.glb#CandleHolder"',
            '+          "asset": "scenery/models/decorations.glb#CandleStand"',
        ],
    )


def optional_property(driver):
    """An optional field's × removes it and the heading's menu adds it back, with its type's
    default; each undoes. Saved, the removal is that key's line."""
    choose(driver, "fixture", "CandleHolder")
    radius = "Properties/LightSource/radius/input"
    driver.expect(radius, NUMERIC, {"F64": 2.0})
    driver.screenshot("optional_property")
    driver.click("Properties/LightSource/radius/remove")
    driver.absent(radius)
    driver.click("Exemplars/CandleHolder")
    undo(driver)
    driver.expect("Toast/ToastText", TEXT, "Undo: Remove LightSource.radius")
    driver.expect(radius, NUMERIC, {"F64": 2.0})
    # `intensity` is absent: the menu adds it at its default.
    intensity = "Properties/LightSource/intensity/input"
    driver.absent(intensity)
    driver.click("Properties/LightSource/add")
    driver.screenshot("optional_property_menu")
    driver.click("Properties/LightSource/add_intensity")
    driver.expect(intensity, NUMERIC, {"F64": 0.0})
    driver.click("Exemplars/CandleHolder")
    undo(driver)
    driver.expect("Toast/ToastText", TEXT, "Undo: Add LightSource.intensity")
    driver.absent(intensity)
    driver.click("Properties/LightSource/radius/remove")
    driver.absent(radius)
    saved_diff(driver, FURNISHINGS, "Exemplars/CandleHolder", ['-        "radius": 2,'])


def collider_property(driver):
    """A list's items: the pane's boxed plus adds one, its × removes it, each undoably. An enum's
    select switches its variant, keeping the fields the variants share. Saved, an added collider
    goes into the catalog's array, leaving the item that was there as it was."""
    choose(driver, "fixture", "CandleHolder")
    item = "Properties/SceneryColliders/Item 1"
    second = "Properties/SceneryColliders/Item 2"
    driver.expect(f"{item}/half_extents/x", NUMERIC, {"F64": 0.2})
    driver.absent(second)
    driver.click("Properties/SceneryColliders/add_item")
    driver.present(f"{second}/half_extents/y")
    driver.expect(f"{second}/half_extents/y", NUMERIC, {"F64": 1.0})
    driver.screenshot("collider_items")
    driver.click(f"{second}/remove")
    driver.absent(second)
    driver.click("Exemplars/CandleHolder")
    undo(driver)
    driver.expect("Toast/ToastText", TEXT, "Undo: Remove SceneryColliders item 2")
    driver.present(second)
    undo(driver)
    driver.expect("Toast/ToastText", TEXT, "Undo: Add SceneryColliders item 2")
    driver.absent(second)

    # A box becomes a ramp, keeping its half extents and taking a facing of 0.
    facing = f"{item}/facing/input"
    driver.absent(facing)
    driver.click(f"{item}/shape/select")
    driver.click(f"{item}/shape/select/Ramp")
    driver.expect(facing, NUMERIC, {"F64": 0.0})
    driver.expect(f"{item}/half_extents/x", NUMERIC, {"F64": 0.2})
    driver.screenshot("collider_variant")
    driver.click("Exemplars/CandleHolder")
    undo(driver)
    driver.expect("Toast/ToastText", TEXT, "Undo: Set SceneryColliders.shape")
    driver.absent(facing)

    driver.click("Properties/SceneryColliders/add_item")
    driver.present(second)
    saved_diff(
        driver,
        FURNISHINGS,
        "Exemplars/CandleHolder",
        [
            "+        },",
            "+        {",
            '+          "shape": "box",',
            '+          "half_extents": [0.5, 1, 0.5]',
        ],
    )


def choose(driver, tool, name):
    """Chooses the exemplar `name` in the current tool's list, through its filter."""
    driver.click("Exemplars/ClearFilter")
    driver.click("Exemplars/Filter")
    driver.type(name)
    driver.click(f"Exemplars/{name}")
    expect_choice(driver, tool, name)


def retype(driver, selector, text):
    """Replaces the text in the text input `selector` with `text`."""
    driver.click(selector)
    driver.key("End")
    for _ in range(60):
        driver.key("Backspace")
    driver.type(text)


def saved_diff(driver, catalog, row, expected):
    """Saves, checks that `catalog` changed by the diff lines `expected` alone, then undoes the
    edit, choosing `row` to take the keys, and saves again, leaving the file as it was. The
    original goes back however the checks end."""
    original = catalog.read_text()
    try:
        driver.click("Save")
        expect_disabled(driver, "Save", True)
        driver.absent("SaveBarrier")
        changed = [
            line
            for line in difflib.unified_diff(
                original.splitlines(), catalog.read_text().splitlines(), lineterm="", n=0
            )
            if line[:1] in "+-" and line[:3] not in ("+++", "---")
        ]
        assert changed == expected, changed
        driver.click(row)
        undo(driver)
        driver.click("Save")
        expect_disabled(driver, "Save", True)
        driver.absent("SaveBarrier")
        assert catalog.read_text() == original, "saving the undone edit changed the catalog"
    finally:
        catalog.write_text(original)


def expect_choice(driver, tool, expected):
    """Waits until scenery mode's saved choice for `tool` is `expected`."""

    def check():
        found = driver.get_resource(SCENERY_CHOICES)[tool]
        return found == expected, lambda: f"{found}"

    driver.wait(check, f"{tool} choice {expected}", {"seconds": 2})


def realm_panel(driver):
    driver.present("RealmPanel/warehouse")
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

    # Undo, from another mode, removes the point and returns to the mode of the edit; redo brings
    # it back. Before Save, whose reload would overwrite the edits (PAN57).
    driver.click("TerrainMode")
    wait_mode(driver, editor_mode, "Terrain")
    undo(driver)
    wait_mode(driver, editor_mode, "Realm")
    driver.absent("RealmPanel/warehouse-smoke")
    driver.expect("Toast/ToastText", TEXT, "Undo: Remove nav point warehouse-smoke")
    undo(driver, redo=True)
    driver.present("RealmPanel/warehouse-smoke")
    driver.expect("Toast/ToastText", TEXT, "Redo: Add nav point warehouse-smoke")

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
        driver.type(".x")
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
        rotate_contour(driver)
        # Save writes the terrain map, which the reload brings back the same size.
        driver.click("Save")
        expect_disabled(driver, "Save", True)
        driver.absent("SaveBarrier")
        driver.expect("RealmPanel/Size", TEXT, "2 × 2 parcels")

        driver.goto(WAREHOUSE, realm="smoke-realm")

        # Change Realm lists the new realm, and goes back. Its filter is the Go To dialog's.
        driver.click("ChangeRealm")
        driver.present("ChangeRealmDialog/smoke-realm")
        driver.click("ChangeRealmDialog/playground")
        driver.screenshot("change_realm")
        driver.click("ChangeRealmDialog/Go")
        driver.absent("ChangeRealmDialog")
        driver.expect("RealmPanel/Realm", TEXT, "playground")
        arrived(driver, WAREHOUSE, "Change Realm, keeping the position")
    finally:
        for file in files:
            file.unlink(missing_ok=True)


def terrain_cursor(driver):
    """Over the selected parcel, a brush tool's cursor surrounds the vertex under the pointer; a
    drag off the parcel keeps it on the parcel's edge, and off it without a drag none shows. The
    flora tools have no cursor yet. The view is centered on the parcel."""
    window, scale, middle, corner = view_points(driver)
    tool = terrain_tool(driver)
    driver.click("RaiseDraw")
    wait_mode(driver, terrain_tool, "RaiseDraw")
    x0, z0 = (n * 16 for n in SPOT_PARCEL)

    def inside(v):
        return x0 <= v[0] <= x0 + 16 and z0 <= v[1] <= z0 + 16

    def cursor():
        return driver.get("TerrainCursor", TERRAIN_CURSOR)

    move_pointer(driver, window, scale, middle)
    over = cursor()
    assert over, "no cursor over the selected parcel"
    vertices = over[0]
    assert vertices["min"] == vertices["max"] and inside(vertices["min"]), f"cursor at {over}"
    driver.screenshot("terrain_cursor")

    button(driver, window, "Pressed")
    move_pointer(driver, window, scale, corner)
    dragged = cursor()
    assert dragged, "no cursor dragged off the parcel"
    v = dragged[0]["min"]
    on_edge = v[0] in (x0, x0 + 16) or v[1] in (z0, z0 + 16)
    assert inside(v) and on_edge, f"dragged off the parcel to {v}, not its edge"
    button(driver, window, "Released")
    assert cursor() is None, f"cursor off the parcel after the drag: {cursor()}"
    driver.click(tool)
    wait_mode(driver, terrain_tool, tool)
    move_pointer(driver, window, scale, middle)
    assert cursor() is None, f"a cursor for {tool}: {cursor()}"


def view_points(driver):
    """The window, its scale, and two points in logical pixels: the middle of the view and one
    near its top-left corner."""
    row = driver.call(
        "world.query", {"data": {"components": [WINDOW]}, "filter": {"with": [WINDOW]}}
    )[0]
    resolution = row["components"][WINDOW]["resolution"]
    scale = resolution["scale_factor_override"] or resolution["scale_factor"]
    inset = driver.get_resource(INSET)
    width = resolution["physical_width"] / scale - inset["left"] - inset["right"]
    height = resolution["physical_height"] / scale - inset["top"] - inset["bottom"]
    middle = (inset["left"] + width / 2, inset["top"] + height / 2)
    corner = (inset["left"] + 20, inset["top"] + 20)
    return row["entity"], scale, middle, corner


def move_pointer(driver, window, scale, point):
    x, y = point
    driver.message(
        WINDOW_EVENT,
        {
            "CursorMoved": {
                "window": window,
                "position": [x, y],
                "physical_position": [x * scale, y * scale],
                "delta": None,
            }
        },
    )
    driver.frames(3)


def button(driver, window, state):
    """Presses or releases the primary mouse button where the pointer is."""
    driver.message(
        WINDOW_EVENT,
        {"MouseButtonInput": {"button": "Left", "state": state, "window": window}},
    )
    driver.frames(3)


def select_parcel(driver, realm, parcel):
    driver.set_resource(SELECTION, {"realm": realm, "parcel": parcel})


def rotate_contour(driver):
    """In the new realm, whose map is 2 × 2 parcels from (-1, 0): the playground's selection does
    not count here, and the rotate buttons turn a parcel selected here, undoably."""
    driver.click("TerrainMode")
    wait_mode(driver, editor_mode, "Terrain")
    expect_disabled(driver, "RotateCw", True)
    select_parcel(driver, "smoke-realm", [0, 0])
    expect_disabled(driver, "RotateCw", False)
    driver.click("RotateCw")
    undo(driver)
    driver.expect("Toast/ToastText", TEXT, "Undo: Rotate contour")
    undo(driver, redo=True)
    driver.expect("Toast/ToastText", TEXT, "Redo: Rotate contour")
    driver.click("RealmMode")
    wait_mode(driver, editor_mode, "Realm")


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


def terrain_tool(driver):
    return driver.get_resource(f"bevy_state::state::resources::State<{TERRAIN_TOOL}>")


def scenery_tool(driver):
    return driver.get_resource(f"bevy_state::state::resources::State<{SCENERY_TOOL}>")


def wait_mode(driver, read, expected):
    """Waits until `read` (`game_mode` or `editor_mode`) gives `expected`."""

    def check():
        found = read(driver)
        return found == expected, lambda: f"in {found}"

    driver.wait(check, f"the mode {expected}", {"seconds": 2})


def expect_highlight(driver, chosen, buttons=MODE_BUTTONS):
    """Of a segmented button's `buttons`, the mode selector's unless given, only `chosen` is
    highlighted."""
    driver.frames(2)
    for button in buttons:
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

    # The realm panel left the view at the warehouse, so each trip below starts away from its
    # point. A row clicked and Go are checked in the Change Realm dialog, the same chooser.

    # The filter box has focus as it opens: `tein` matches only "test-inn", by its words' starts.
    # It keeps a held arrow from the camera for the whole hold, not just its first frame. With
    # nothing selected, Enter travels to the first point shown.
    driver.key("KeyG")
    driver.present("GotoDialog/warehouse")
    driver.type("tein")
    driver.absent("GotoDialog/warehouse")
    driver.present("GotoDialog/test-inn")
    driver.screenshot("goto_dialog")
    before = driver.get("PrimaryCamera", TRANSFORM)
    hold(driver, "ArrowLeft", 0.5)
    after = driver.get("PrimaryCamera", TRANSFORM)
    assert before == after, f"a held arrow in the filter box moved the view: {before} to {after}"
    driver.key("Enter")
    arrived(driver, TEST_INN, "Enter")

    # In the filter box, Down and Up move the selection, and Enter travels to it.
    driver.key("KeyG")
    driver.present("GotoDialog/warehouse")
    driver.type("wareh")
    driver.present("GotoDialog/warehouse2")
    driver.key("ArrowDown")
    driver.key("ArrowDown")
    driver.key("ArrowUp")
    driver.key("Enter")
    arrived(driver, WAREHOUSE, "arrows and Enter")

    # A double-click on a point travels to it: the list's focus outline does not take the clicks.
    driver.key("KeyG")
    driver.present("GotoDialog/warehouse")
    driver.type("tein")
    driver.click("GotoDialog/test-inn")
    driver.click("GotoDialog/test-inn")
    arrived(driver, TEST_INN, "double-click")


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
