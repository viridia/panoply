"""Physics drawing (F3) and its Show menu, on demand: drawing every collider is slow enough (I12)
to hold up a run, so a directory run leaves it out. Run it when the drawing or the probe is
suspect.

    bevy-remote-driver plans/physics_views.py

- F3 draws the colliders; a ray cast down at the view center hits the ground, then a floor's, a
  fixture's and a tree's canopy colliders. `GroundProbe` holds where the ray hit, drawn as a white
  sphere. In `actor_capsule.png`, a cyan capsule encloses the player's actor, its bottom on the
  ground.
- The Show menu turns a collider group off, and Space toggles the checkbox just clicked, so keys
  reach widgets; Physics turns the drawing off. The menu is clicked twice: the second click lands
  only if the first left the menu open.

Settings start empty, so physics drawing starts off and the panel hidden.
"""

from bevy_remote_driver import Between

from _panoply_driver import PanoplyDriver

BIN = "panoply"
DRIVER = PanoplyDriver
TAGS = ["manual"]
VIEWS = "panoply_editor::debug_views::DebugViews"
PROBE = "panoply_editor::debug_views::GroundProbe"
PANEL = "panoply_editor::shell::EditorPanelSettings"
MOTION = "panoply_actors::actor_physics::ActorMotion"
# Forest, shoreline, water, grass and rocks.
SPOT = [110.0, 0.0, 150.0]
# Under the player's spot, `PLAYER_POSITION` in `panoply_actors`.
ACTOR_SPOT = [37.0, 0.0, 47.0]
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
    driver.caption("probe")
    probe(driver)
    driver.caption("show menu")
    show_menu(driver)
    driver.exits("Escape")


def probe(driver):
    driver.goto(SPOT, 20.0)
    assert driver.get_resource(VIEWS, at="physics") is False, "physics drawing on at start"
    assert driver.get_resource(PROBE, at="hit") is None, "probe set while drawing is off"

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

    # The player's actor, landed where it spawns, drawn with its capsule.
    driver.goto(ACTOR_SPOT, 20.0)
    driver.expect("Actor", MOTION, {"grounded": True})
    driver.screenshot("actor_capsule")


def show_menu(driver):
    driver.key("KeyT")
    driver.frames(10)
    assert driver.get_resource(PANEL, at="visible") is True, "T did not show the panel"
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
