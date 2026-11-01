"""Physics debug views and the editor shell: F3 draws the colliders, a ray cast down at the view
center hits the ground, T shows the editor panel beside the view, and its Show menu turns views
off.

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
    driver.screenshot("colliders")

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
