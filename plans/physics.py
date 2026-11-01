"""Physics debug views and the editor shell: F3 draws the colliders, a ray cast down at the view
center hits the ground, then a floor's, a fixture's and a tree's canopy colliders, T shows the
editor panel beside the view, and its Show menu turns views off.

    bevy-remote-driver plans/physics.py

Settings start empty, so physics drawing starts off, F3 turns it on, and the panel starts
hidden. `GroundProbe` holds where the ray hit, drawn as a white sphere. The Show menu is clicked
twice: the second click lands only if the first left the menu open.
"""

from bevy_remote_driver import Between

from _panoply_driver import PanoplyDriver

BIN = "panoply"
DRIVER = PanoplyDriver
VIEWS = "panoply_editor::debug_views::DebugViews"
PROBE = "panoply_editor::debug_views::GroundProbe"
PANEL = "panoply_editor::shell::EditorPanelSettings"
INSET = "panoply_core::viewport::ViewportInset"
SPOT = [110.0, 0.0, 150.0]
TERRAIN_GROUP = 1
FLOOR_GROUP = 2
FIXTURE_GROUP = 8
CANOPY_GROUP = 0x20
# (label, spot, expected top, group): a parquet floor on tier 0 (top at TIER_OFFSET), a walnut
# table (box of half-height 0.42 on the floor at 0), and a tree's cell, whose canopy is 2 m over
# ground at 0. Trunks are under the canopy, out of the probe's reach.
SPOTS = [
    ("floor", [58.0, 0.0, 26.62], 0.02, FLOOR_GROUP),
    ("table", [42.23, 0.0, 58.71], 0.84, FIXTURE_GROUP),
    ("canopy", [108.5, 0.0, 146.5], 2.0, CANOPY_GROUP),
]


def run(driver):
    driver.present("PrimaryCamera")
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
    for label, spot, top, group in SPOTS:
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
    driver.click("Physics")
    driver.frames(5)
    assert driver.get_resource(VIEWS, at="physics") is False, "Physics did not turn off"

    driver.key("KeyT")
    driver.frames(10)
    assert driver.get_resource(INSET, at="left") == 0.0, "view still inset with the panel hidden"
    driver.exits("Escape")
