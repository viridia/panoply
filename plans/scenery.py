"""The editor's scenery tools, over the playground.

    bevy-remote-driver plans/scenery.py

- Palette: Scenery mode starts on Floor and chooses the tool; New and Clone wait for a selection.
  The wall tool has options above its list, the fixture tool none. A tool lists its exemplars,
  the abstract ones they extend too; the filter narrows the list and its × empties it; returning
  to a tool keeps its choice.
- Walls: a click places a wall and a drag a row, undoably, rebuilding the navigation mesh; the
  wall a release would replace is hidden; `(erase)` erases one; Escape cancels a drag. Save writes
  the precinct (restored afterwards) and a new one (deleted afterwards), whose ground is navigated
  though it had no file, and which saving lists in the realm's index (restored afterwards). ] and the rotate button turn the walls, . and , step the tier. Cut hides
  what is above the tier, and shows it again.
- Fixtures: a click places one, undoably, and a drag turns it on a dial shown on every drag;
  Escape cancels a drag, ] turns the fixtures, Save writes the precinct (restored afterwards).
- Actors: hovering shows the chosen actor and its footprint; a drag turns and places it, standing
  on the ground, undoably; Save writes the precinct (restored afterwards).
- Selection: hovering outlines a fixture or an actor and a click selects it. A drag of a handle
  of the gizmo on the selection moves or turns it, undoably, and Delete removes it. The list's
  filter menu shows unlabelled instances and narrows them by kind; Rename… labels the selection
  and clears the label, undoably, and a click outside its dialog closes it, keeping the selection.

Settings start empty, so the panel starts hidden.
"""

import json

from _editor import (
    ASSETS,
    INHERITED,
    MOTION,
    NAME,
    SPOT,
    TEXT,
    WAIT,
    by_name,
    choose,
    expect_checked,
    expect_choice,
    expect_disabled,
    expect_display,
    expect_highlight,
    hidden,
    open_editor,
    restore,
    retype,
    save,
    scenery_tool,
    undo,
    wait_mode,
)
from _panoply_driver import PanoplyDriver, button, move_pointer, view_points

BIN = "panoply"
DRIVER = PanoplyDriver
TAGS = ["editor"]
BUTTON_VARIANT = "bevy_feathers::controls::button::ButtonVariant"
WALL_SETTINGS = "panoply_editor::wall_tool::WallToolSettings"
FIXTURE_SETTINGS = "panoply_editor::fixture_tool::FixtureToolSettings"
TIER = "panoply_editor::scenery_pointer::SceneryTier"
SCENERY_SELECTION = "panoply_editor::instance_selection::ScenerySelection"
OUTLINE = "panoply_editor::instance_selection::InstanceOutline"
GIZMO = "panoply_editor::instance_gizmo::GizmoActive"
NAV_MESH = "panoply_nav::mesh::PrecinctNavMesh"
SCENERY_BUTTONS = [
    "PlaceFloor", "PlaceWall", "PlaceFixture", "PlaceActor", "DrawTerrainFx", "SelectInstance",
    "Layers", "RectSelect",
]
SNAP_BUTTONS = ["SnapNormal", "SnapOffset", "SnapQuarter"]
# The playground precinct under `SPOT`, and one with no file, which the realm's bounds hold.
SPOT_PRECINCT = "scenery/precincts/playground/p001-p002.msgpack"
# The same precinct's entity, simulated while the view is at `SPOT`, so the game builds its
# navigation mesh.
SPOT_PRECINCT_ENTITY = "Precinct:playground:1:2"
EMPTY_SPOT = [160.0, 0.0, 160.0]
EMPTY_PRECINCT = "scenery/precincts/playground/p002-p002.msgpack"
# The realm's precinct index, which saving the precinct with no file lists it in.
PLAYGROUND_INDEX = "scenery/precincts/playground/index.json"
EMPTY_PRECINCT_ENTITY = "Precinct:playground:2:2"
# Inside the house in the playground's precinct (0, 0), under its upper floor on tier 2.
HOUSE = [53.0, 0.0, 41.0]


def run(driver):
    driver.goto(SPOT, 20.0)
    open_editor(driver, "Scenery")
    driver.caption("palette")
    palette(driver)
    driver.caption("walls")
    wall_tool(driver)
    driver.caption("fixtures")
    fixture_tool(driver)
    driver.caption("actors")
    driver.click("PlaceActor")
    driver.present("Exemplars/Marcus", timeout=WAIT)
    driver.present("Exemplars/Proto.HumanoidMale")
    driver.absent("Exemplars/CottageWall")
    actor_tool(driver)
    driver.caption("selection")
    select_instance(driver)
    # Returning to a tool keeps its choice, highlighted (seen in the screenshot: `Selected` is not
    # reflected, so the plan cannot read it).
    driver.click("PlaceWall")
    driver.present("Exemplars/CottageWall")
    expect_choice(driver, "wall", "CottageWall")
    # The row highlights, then scrolls into view after a layout pass.
    driver.frames(10)
    driver.screenshot("scenery_choice_kept")
    driver.exits("Escape")


def palette(driver):
    """Scenery mode's palette starts on Floor and chooses the tool; the buttons that act on a
    selection or an exemplar wait for them."""
    driver.present("SceneryPanel")
    assert scenery_tool(driver) == "PlaceFloor", f"started with {scenery_tool(driver)}"
    driver.click("PlaceWall")
    wait_mode(driver, scenery_tool, "PlaceWall")
    expect_highlight(driver, "PlaceWall", SCENERY_BUTTONS)
    for name in ["SceneryPanel/New", "SceneryPanel/Clone"]:
        expect_disabled(driver, name, True)
    # The wall tool has options above its list; the floor tool, so far, none.
    expect_display(driver, "ToolOptions", "Flex")
    expect_highlight(driver, "SnapNormal", SNAP_BUTTONS)

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


def nav_vertices(driver, precinct=SPOT_PRECINCT_ENTITY):
    """The vertices of the navigation mesh of `precinct`, or `None` while it has none."""
    entities = driver.select(precinct)
    if len(entities) != 1:
        return None
    result = driver.call(
        "world.get_components",
        {"entity": entities[0], "components": [NAV_MESH], "strict": False},
    )
    mesh = result["components"].get(NAV_MESH)
    return mesh and mesh["vertices"]


def wait_nav(driver, passes, expected, precinct=SPOT_PRECINCT_ENTITY):
    driver.wait(
        lambda: (passes(nav_vertices(driver, precinct)), "another mesh, or none"),
        expected,
        WAIT,
    )


def wall_tool(driver):
    """The wall tool, over the playground at `SPOT`: a click places a wall, and a drag a row of
    them, which undo and redo take back and put again; `(erase)` erases one. Save writes the
    precinct (restored afterwards). ] turns the walls to place, . and , step the tier, and Escape
    during a drag cancels it without quitting. Placed into a precinct with no file, a wall is saved
    into a new one (deleted afterwards), listed in the realm's precinct index (restored after).

    The precinct is simulated, so the game builds its navigation mesh with the navigation view
    off; the wall placed rebuilds it, and the precinct put back builds it as it was. The precinct
    with no file gets a mesh too, of its ground alone."""
    driver.goto(SPOT, 20.0, azimuth=0.0)
    window, scale, middle, _ = view_points(driver)
    precinct = ASSETS / SPOT_PRECINCT
    original = precinct.read_bytes()
    wait_nav(driver, lambda v: v is not None, "the simulated precinct's navigation mesh")
    built = nav_vertices(driver)

    def count():
        return element_count(driver)

    def expect_count(expected):
        expect_elements(driver, expected)

    def drag(to):
        move_pointer(driver, window, scale, middle)
        button(driver, window, "Pressed")
        move_pointer(driver, window, scale, to)
        button(driver, window, "Released")

    base = count()
    right = (middle[0] + 150, middle[1])
    try:
        drag(middle)
        expect_count(base + 1)
        wait_nav(driver, lambda v: v not in (None, built), "the mesh rebuilt around the wall")
        # Hovering over the wall just placed hides it, as a release there would replace it; off
        # it, it shows again.
        driver.frames(2)
        assert len(hidden(driver, "SceneryElement")) == 1, "the wall under the pointer shows"
        move_pointer(driver, window, scale, (middle[0], middle[1] + 200))
        driver.frames(2)
        assert not hidden(driver, "SceneryElement"), "a wall off the pointer is hidden"
        # A drag places a tile a meter, from the press to the pointer, replacing the wall the
        # click placed.
        move_pointer(driver, window, scale, middle)
        button(driver, window, "Pressed")
        move_pointer(driver, window, scale, right)
        driver.screenshot("wall_drag")
        button(driver, window, "Released")
        driver.frames(5)
        placed = count() - base
        assert placed >= 2, f"the drag placed {placed} walls"
        undo(driver)
        driver.expect("Toast/ToastText", TEXT, f"Undo: Place {placed} walls")
        expect_count(base + 1)
        undo(driver, redo=True)
        expect_count(base + placed)

        # Escape cancels a drag, and does not quit.
        move_pointer(driver, window, scale, middle)
        button(driver, window, "Pressed")
        move_pointer(driver, window, scale, right)
        driver.key("Escape")
        button(driver, window, "Released")
        driver.frames(5)
        assert count() == base + placed, "a cancelled drag placed walls"

        # `(erase)` erases the wall under the pointer.
        choose(driver, "wall", "(erase)")
        drag(middle)
        expect_count(base + placed - 1)
        choose(driver, "wall", "CottageWall")

        save(driver)
        assert precinct.read_bytes() != original, "Save did not write the precinct"
    finally:
        restore(precinct, original)
    # The watcher's reload puts the precinct back as it was, and its mesh.
    expect_count(base)
    wait_nav(driver, lambda v: v == built, "the mesh as it was before the walls")

    # ] turns the walls to place; . and , step the tier, which the tile grid shows.
    driver.key("BracketRight")
    driver.frames(2)
    assert driver.get_resource(WALL_SETTINGS, at="facing") == 1
    driver.click("RotateLeft")
    driver.frames(2)
    assert driver.get_resource(WALL_SETTINGS, at="facing") == 0
    driver.key("Period")
    driver.frames(2)
    assert tier(driver) == 1, f"tier {tier(driver)}"
    move_pointer(driver, window, scale, middle)
    driver.screenshot("wall_tier")
    driver.key("Comma")
    driver.frames(2)
    assert tier(driver) == 0, f"tier {tier(driver)}"
    cut(driver)

    # A precinct with no file gets one when saved, and a line in its realm's index.
    driver.goto(EMPTY_SPOT, 20.0, azimuth=0.0)
    window, scale, middle, _ = view_points(driver)
    created = ASSETS / EMPTY_PRECINCT
    assert not created.exists(), f"{created} exists already"
    index = ASSETS / PLAYGROUND_INDEX
    listed = index.read_text()
    assert [2, 2] not in json.loads(listed), f"{index} lists the precinct with no file"
    # Its ground is navigated though it has no file.
    wait_nav(
        driver,
        lambda v: bool(v),
        "a navigation mesh for the precinct with no file",
        EMPTY_PRECINCT_ENTITY,
    )
    base = count()
    try:
        drag(middle)
        expect_count(base + 1)
        save(driver)
        assert created.exists(), "Save did not create the precinct"
        assert [2, 2] in json.loads(index.read_text()), f"Save did not list it in {index}"
    finally:
        created.unlink(missing_ok=True)
        index.write_text(listed)
    driver.goto(SPOT, 20.0)


def cut(driver):
    """Cut hides the scenery from two tiers above the current one up, walls and floors, and
    shows it again when turned off. At the house in the playground's precinct (0, 0), whose upper
    floor is on tier 2. Checked by eye in `cut.png`: the house's ground floor, open above."""
    driver.goto(HOUSE, 20.0, azimuth=0.0)
    before = len(hidden(driver, "SceneryElement")), len(hidden(driver, "FloorRegion"))
    driver.click("SceneryPanel/Cut")
    driver.frames(2)
    driver.expect("SceneryPanel/Cut", BUTTON_VARIANT, "Primary")
    cut_away = len(hidden(driver, "SceneryElement")), len(hidden(driver, "FloorRegion"))
    print(f"hidden before Cut {before}, with it {cut_away}")
    assert cut_away[0] > before[0], f"Cut hid no walls: {cut_away}"
    assert cut_away[1] > before[1], f"Cut hid no floors: {cut_away}"
    driver.screenshot("cut")
    driver.click("SceneryPanel/Cut")
    driver.frames(2)
    driver.expect("SceneryPanel/Cut", BUTTON_VARIANT, "Normal")
    after = len(hidden(driver, "SceneryElement")), len(hidden(driver, "FloorRegion"))
    assert after == before, f"hidden after Cut {after}, before {before}"


def element_count(driver):
    """The scenery elements in the precincts, not counting a tool's preview."""
    return len(by_name(driver, "SceneryElement"))


def expect_elements(driver, expected):
    def check():
        found = element_count(driver)
        return found == expected, lambda: f"{found} scenery elements"

    driver.wait(check, f"{expected} scenery elements", {"seconds": 5})


def tier(driver):
    value = driver.get_resource(TIER)
    return value[0] if isinstance(value, list) else value


def fixture_tool(driver):
    """The fixture tool, over the playground at `SPOT`: a click places the chosen fixture, which
    undo and redo take back and put again. A drag turns it toward the pointer, on a dial with the
    facing beside the pointer; the dial shows again on the next drag. Escape cancels a drag, ]
    turns the fixtures to place, and Save writes the precinct (restored afterwards)."""
    driver.click("PlaceFixture")
    wait_mode(driver, scenery_tool, "PlaceFixture")
    expect_display(driver, "ToolOptions", "None")
    driver.present("Exemplars/FlowerPot", timeout=WAIT)
    choose(driver, "fixture", "FlowerPot")
    driver.goto(SPOT, 20.0, azimuth=0.0)
    window, scale, middle, _ = view_points(driver)
    right = (middle[0] + 150, middle[1])
    precinct = ASSETS / SPOT_PRECINCT
    original = precinct.read_bytes()

    def dial_shown():
        return driver.get("FacingDial", INHERITED)

    def press_and_drag(to):
        move_pointer(driver, window, scale, middle)
        button(driver, window, "Pressed")
        move_pointer(driver, window, scale, to)
        driver.frames(3)

    base = element_count(driver)
    try:
        # A press shows the dial, a sign that a drag would turn the fixture; a click places it.
        press_and_drag(middle)
        assert dial_shown(), "no dial on the press"
        button(driver, window, "Released")
        expect_elements(driver, base + 1)
        undo(driver)
        driver.expect("Toast/ToastText", TEXT, "Undo: Place FlowerPot")
        expect_elements(driver, base)
        undo(driver, redo=True)
        expect_elements(driver, base + 1)

        # Dragged to the right of the view, which looks north, the fixture faces east.
        press_and_drag(right)
        assert dial_shown(), "no dial while dragging"
        driver.expect("FacingReadout", TEXT, "270°")
        driver.screenshot("fixture_drag")
        button(driver, window, "Released")
        expect_elements(driver, base + 2)
        assert not dial_shown(), "the dial stays after the drag"

        # The dial shows again on the next drag, which Escape cancels without quitting.
        press_and_drag(right)
        assert dial_shown(), "no dial on the second drag"
        driver.key("Escape")
        button(driver, window, "Released")
        driver.frames(5)
        assert element_count(driver) == base + 2, "a cancelled drag placed a fixture"

        save(driver)
        assert precinct.read_bytes() != original, "Save did not write the precinct"
    finally:
        restore(precinct, original)
    expect_elements(driver, base)

    # ] turns the fixtures to place a quarter turn right.
    driver.key("BracketRight")
    driver.frames(2)
    facing = driver.get_resource(FIXTURE_SETTINGS, at="facing")
    assert facing == 270.0, f"facing {facing}"


def actor_tool(driver):
    """The actor tool, over the playground at `SPOT`: hovering shows the chosen actor and its
    footprint; a drag turns it toward the pointer and places it, an actor that stands on the
    ground, which undo and redo take back and put again. Save writes the precinct (restored
    afterwards)."""
    choose(driver, "actor", "Marcus")
    driver.goto(SPOT, 20.0, azimuth=0.0)
    window, scale, middle, _ = view_points(driver)
    right = (middle[0] + 150, middle[1])
    precinct = ASSETS / SPOT_PRECINCT
    original = precinct.read_bytes()

    def marcuses(with_=()):
        return len(by_name(driver, "Actor:Marcus", with_=with_))

    def expect_marcuses(expected, with_=()):
        def check():
            found = marcuses(with_)
            return found == expected, lambda: f"{found} Marcuses"

        driver.wait(check, f"{expected} Marcuses", {"seconds": 5})

    base = marcuses()
    standing = marcuses([MOTION])
    try:
        move_pointer(driver, window, scale, middle)
        driver.frames(3)
        assert driver.get("ActorFootprint", INHERITED), "no footprint on hover"
        driver.present("ActorPreview", timeout=WAIT)
        assert driver.get("ActorPreview", INHERITED), "no preview on hover"
        button(driver, window, "Pressed")
        move_pointer(driver, window, scale, right)
        driver.frames(3)
        assert driver.get("FacingDial", INHERITED), "no dial while dragging"
        driver.expect("FacingReadout", TEXT, "270°")
        # The preview turns with the drag, before the release places the actor.
        facing = driver.get("ActorPreview", MOTION)["facing"]
        assert abs(facing - 4.712389) < 1e-3, f"preview facing {facing}"
        driver.screenshot("actor_drag")
        button(driver, window, "Released")
        expect_marcuses(base + 1)
        # It gets physics: the ground under it is found.
        expect_marcuses(standing + 1, [MOTION])
        undo(driver)
        driver.expect("Toast/ToastText", TEXT, "Undo: Place Marcus")
        expect_marcuses(base)
        undo(driver, redo=True)
        expect_marcuses(base + 1)

        save(driver)
        assert precinct.read_bytes() != original, "Save did not write the precinct"
    finally:
        restore(precinct, original)
    expect_marcuses(base)
    driver.click("Exemplars/ClearFilter")


def gizmo_handle(driver, window, scale, start, step):
    """Moves the pointer from `start` by `step`, in logical pixels, until it is over a handle of
    the selection's gizmo; returns the point and the handle."""
    x, y = start
    for _ in range(40):
        x, y = x + step[0], y + step[1]
        move_pointer(driver, window, scale, (x, y))
        hovered = driver.get_resource(GIZMO)["hovered"]
        if hovered is not None:
            return (x, y), hovered
    raise AssertionError(f"no gizmo handle from {start} by {step}")


def select_instance(driver):
    """Selection mode, over the playground at `SPOT`: hovering a fixture outlines it and a click
    selects it, its pane showing it has no name. An actor is selected by a click too. The list's
    filter menu shows unlabelled instances, the pot among them, and narrows them by kind to actors,
    where the actor is chosen, then renamed through Rename…, whose box starts with the name;
    a click outside the dialog closes it and keeps the selection, and undo takes the rename back.
    The precinct is restored afterwards."""
    driver.click("PlaceFixture")
    wait_mode(driver, scenery_tool, "PlaceFixture")
    choose(driver, "fixture", "FlowerPot")
    driver.goto(SPOT, 20.0, azimuth=0.0)
    window, scale, middle, _ = view_points(driver)
    below = (middle[0], middle[1] + 100)
    precinct = ASSETS / SPOT_PRECINCT
    original = precinct.read_bytes()

    def click_at(point):
        move_pointer(driver, window, scale, point)
        button(driver, window, "Pressed")
        button(driver, window, "Released")

    def selected():
        return driver.get_resource(SCENERY_SELECTION)

    try:
        # A flower pot in the middle of the view, and an actor below it, along the strip of open
        # ground between the trees and the water: among the trees, the actor would stand on the
        # canopy, over the ground the click placed it on.
        base = element_count(driver)
        click_at(middle)
        expect_elements(driver, base + 1)
        driver.click("PlaceActor")
        choose(driver, "actor", "Marcus")
        click_at(below)

        driver.click("SelectInstance")
        wait_mode(driver, scenery_tool, "SelectInstance")
        move_pointer(driver, window, scale, middle)
        assert driver.get("InstanceOutline:Hovered", OUTLINE)["polygons"], "no outline on hover"
        click_at(middle)
        assert selected()["instance"] is not None, "the click selected nothing"
        driver.expect("InstanceLabel", TEXT, "none")
        driver.expect("InstancePrecinct", TEXT, "playground (1, 2)")
        driver.screenshot("instance_selected")

        # The gizmo: right of the pot, the pointer meets one of its handles, and a drag of it
        # moves or turns the pot, as one edit. Delete removes the pot, leaving nothing selected,
        # and undo puts it back.
        point, handle = gizmo_handle(driver, window, scale, middle, (8, 0))
        verb = "Turn" if handle == "Ring" else "Move"
        button(driver, window, "Pressed")
        assert driver.get_resource(GIZMO)["held"] == handle, f"the press did not take {handle}"
        move_pointer(driver, window, scale, (point[0] + 60, point[1] - 60))
        driver.screenshot("gizmo_drag")
        button(driver, window, "Released")
        assert selected()["instance"] is not None, "the drag changed the selection"
        undo(driver)
        driver.expect("Toast/ToastText", TEXT, f"Undo: {verb} FlowerPot")
        driver.key("Delete")
        expect_elements(driver, base)
        assert selected()["instance"] is None, "the deleted pot is still selected"
        undo(driver)
        driver.expect("Toast/ToastText", TEXT, "Undo: Delete FlowerPot")
        expect_elements(driver, base + 1)

        # The actor is outlined on hover once it stands there with its collider, and a click
        # selects it.
        move_pointer(driver, window, scale, below)

        def actor_hovered():
            polygons = driver.get("InstanceOutline:Hovered", OUTLINE)["polygons"]
            return bool(polygons), lambda: "no outline"

        driver.wait(actor_hovered, "the actor outlined on hover", {"seconds": 5})
        click_at(below)
        clicked = selected()["instance"]
        driver.screenshot("actor_selected")

        # The list holds labelled instances only, until the filter menu's Labeled is turned off;
        # then it holds every instance, the flower pot, which has no key, among them, by exemplar
        # and place.
        def unlabelled(prefix):
            return [r for r in by_name(driver, prefix) if ", " in r["components"][NAME]]

        assert not unlabelled("Marcus"), "an unlabelled actor listed with Labeled on"
        driver.click("Exemplars/InstanceFilter")
        expect_checked(driver, "Exemplars/Labeled", True)
        driver.click("Exemplars/Labeled")
        expect_checked(driver, "Exemplars/Labeled", False)
        driver.screenshot("instance_filter")
        pots = unlabelled("FlowerPot")
        assert pots, "no flower pot in the list with Labeled off"
        driver.click(f"Exemplars/{pots[-1]['components'][NAME]}")
        driver.wait(
            lambda: (selected()["instance"] not in (None, clicked), lambda: str(selected())),
            "the flower pot chosen in the list",
            {"seconds": 5},
        )

        # Wall and Fixture off narrow the list to actors; choosing the actor there selects the one
        # the click did.
        driver.click("Exemplars/InstanceFilter")
        expect_checked(driver, "Exemplars/Actor", True)
        driver.click("Exemplars/Wall")
        driver.click("Exemplars/Fixture")
        expect_checked(driver, "Exemplars/Fixture", False)
        driver.absent(f"Exemplars/{pots[-1]['components'][NAME]}")
        rows = unlabelled("Marcus")
        assert rows, "no actor in the list"
        row = rows[-1]["components"][NAME]
        driver.click(f"Exemplars/{row}")
        driver.expect("InstanceLabel", TEXT, "none")
        actor = selected()["instance"]
        assert actor == clicked, f"the click selected {clicked}, the list {actor}"

        # Back to the filter's defaults, which leave the actor out again.
        driver.click("Exemplars/InstanceFilter")
        driver.click("Exemplars/Wall")
        driver.click("Exemplars/Fixture")
        driver.click("Exemplars/Labeled")
        expect_checked(driver, "Exemplars/Labeled", True)
        driver.absent(f"Exemplars/{row}")
        # Focus elsewhere closes the menu, whose popup hangs over the pane's Rename….
        driver.click("Exemplars/Filter")
        driver.expect("Exemplars/Labeled", INHERITED, False)

        # A click outside the dialog closes it, and the press does not reach the view.
        driver.click("RenameInstance")
        driver.present("TextEntryDialog")
        click_at(middle)
        driver.absent("TextEntryDialog")
        assert selected()["instance"] == actor, "a click outside the dialog changed the selection"

        # A label replaces the row, and the dialog's × clears it again.
        driver.click("RenameInstance")
        driver.present("TextEntryDialog")
        retype(driver, "TextEntryDialog/Entry", "plan-guard")
        driver.click("TextEntryDialog/Ok")
        driver.expect("InstanceLabel", TEXT, "plan-guard")
        driver.present("Exemplars/plan-guard")
        driver.click("RenameInstance")
        driver.present("TextEntryDialog")
        driver.click("TextEntryDialog/Clear")
        driver.click("TextEntryDialog/Ok")
        driver.expect("InstanceLabel", TEXT, "none")
        driver.absent("Exemplars/plan-guard")
        undo(driver)
        driver.expect("Toast/ToastText", TEXT, "Undo: Clear label plan-guard")
        driver.expect("InstanceLabel", TEXT, "plan-guard")
        undo(driver)
        driver.expect("Toast/ToastText", TEXT, "Undo: Label plan-guard")

        save(driver)
    finally:
        restore(precinct, original)
    expect_elements(driver, base)
