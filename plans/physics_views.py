"""Physics drawing (F3), the navigation view and their Show menu, on demand: drawing every collider
is slow enough (I12) to hold up a run, so a directory run leaves it out. Run it when the drawing,
the probe or the navigation view is suspect.

    bevy-remote-driver plans/physics_views.py

- F3 draws the colliders; a ray cast down at the view center hits the ground, then a floor's, a
  fixture's and a tree's canopy colliders. `GroundProbe` holds where the ray hit, drawn as a white
  sphere. In `actor_capsule.png`, a cyan capsule encloses the player's actor, its bottom on the
  ground.
- The Show menu turns a collider group off, and Space toggles the checkbox just clicked, so keys
  reach widgets; Physics turns the drawing off. The menu is clicked twice: the second click lands
  only if the first left the menu open.
- A navigation mesh made by hand, put on the village's precinct, gets a view, hidden until
  Navigation is checked; a changed mesh replaces it, and a removed one takes it away. In
  `nav_view.png`, a row of three polygons, blue, amber (a doorway), blue: red walls along their
  sides, faint lines where they meet, and a green edge on the precinct's side at x 128.
- Navigation on builds the lair's mesh with Recast, doorways and footpaths included, and leaves
  the hand-made one alone. In `nav_recast.png`, the doorway near (50.5, 50.5) from above, amber
  between its jambs.
- Clipping under Navigation builds every mesh again with the port of faery's builder. In
  `nav_clipping.png`, the same doorway: a thin amber strip across it where the leaf closes, and
  the jambs cut back square by the agent's radius.

Settings start empty, so physics drawing and the navigation view start off and the panel hidden.
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
NAV_MESH = "panoply_nav::mesh::PrecinctNavMesh"
VISIBILITY = "bevy_camera::visibility::Visibility"
# The precinct the hand-made mesh goes on, the village, and its view. A mesh is in its precinct's
# coordinates, so the realm's x is 64 more.
PRECINCT = "Precinct:playground:1:0"
NAV_VIEW = f"{PRECINCT}/NavMeshView"
NAV_FILLS = f"{NAV_VIEW}/NavAreaFill"
# The lair, whose corner is at the origin, built by Recast once Navigation is on, and one of its
# doorways.
LAIR = "Precinct:playground:0:0"
LAIR_VIEW = f"{LAIR}/NavMeshView"
DOORWAY = [50.5, 0.0, 50.75]


def run(driver):
    driver.present("PrimaryCamera")
    driver.caption("probe")
    probe(driver)
    driver.caption("show menu")
    show_menu(driver)
    driver.caption("navigation view")
    nav_view(driver)
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


def hand_made_mesh(door="Door"):
    """Three polygons in a row along x at z 47 to 51, in the precinct's coordinates: walkable from
    x 40 to 50, a doorway (area `door`) to 51, walkable to the precinct's side at 64."""
    xs = [40.0, 50.0, 51.0, 64.0]
    vertices = [[x, 0.0, z] for x in xs for z in (47.0, 51.0)]

    def square(i, edges, area):
        # Counter-clockwise from above, from the -x side around, as `NavMesh` keeps them.
        return {
            "vertices": [2 * i, 2 * i + 1, 2 * i + 3, 2 * i + 2],
            "edges": edges,
            "area": area,
        }

    return {
        "vertices": vertices,
        "polygons": [
            square(0, ["Wall", "Wall", {"Neighbor": 1}, "Wall"], "Walkable"),
            square(1, [{"Neighbor": 0}, "Wall", {"Neighbor": 2}, "Wall"], door),
            square(2, [{"Neighbor": 1}, "Wall", {"Boundary": "PosX"}, "Wall"], "Walkable"),
        ],
    }


def nav_view(driver):
    """Follows `show_menu`, whose menu is still open."""
    assert driver.get_resource(VIEWS, at="navigation") is False, "navigation view on at start"
    driver.goto([116.0, 0.0, 49.0], 30.0)
    driver.set(PRECINCT, NAV_MESH, hand_made_mesh())
    driver.present(NAV_VIEW)
    assert driver.get(NAV_VIEW, VISIBILITY) == "Hidden", "navigation view shown while off"
    fills = driver.select(NAV_FILLS)
    assert len(fills) == 2, f"{len(fills)} fills, not one for each of the two areas"

    driver.click("Navigation")
    driver.frames(5)
    assert driver.get_resource(VIEWS, at="navigation") is True, "Navigation did not turn on"
    assert driver.get(NAV_VIEW, VISIBILITY) == "Inherited", "navigation view still hidden"
    driver.screenshot("nav_view")
    recast_mesh(driver)

    # The doorway made walkable: one view still, with one fill.
    view = driver.select(NAV_VIEW)
    driver.set(PRECINCT, NAV_MESH, hand_made_mesh(door="Walkable"))
    driver.frames(3)
    rebuilt = driver.select(NAV_VIEW)
    assert len(rebuilt) == 1 and rebuilt != view, f"views {view} became {rebuilt}"
    assert len(driver.select(NAV_FILLS)) == 1, "not one fill for one area"
    assert driver.get(NAV_VIEW, VISIBILITY) == "Inherited", "the rebuilt view is hidden"

    entity = driver.select(PRECINCT)[0]
    driver.call("world.remove_components", {"entity": entity, "components": [NAV_MESH]})
    driver.absent(NAV_VIEW)
    clipping_mesh(driver)


def recast_mesh(driver):
    """Turning Navigation on builds the meshes of the loaded precincts that have none with Recast,
    the lair among them; the hand-made one is left as it is."""
    driver.wait(
        lambda: (bool(driver.select(LAIR_VIEW)), "no view"),
        "the lair's Recast mesh",
        {"seconds": 30},
    )
    polygons = driver.get(LAIR, NAV_MESH, at="polygons")
    areas = {area: sum(1 for p in polygons if p["area"] == area) for area in ("Door", "Path")}
    print(f"lair: {len(polygons)} polygons, {areas}")
    assert areas["Door"], "the lair's mesh has no doorways"
    hand_made = driver.get(PRECINCT, NAV_MESH, at="polygons")
    assert len(hand_made) == 3, f"the hand-made mesh became {len(hand_made)} polygons"
    driver.goto(DOORWAY, 12.0)
    driver.frames(10)
    driver.screenshot("nav_recast")
    driver.goto([116.0, 0.0, 49.0], 30.0)


def clipping_mesh(driver):
    """Choosing Clipping under Navigation builds every loaded precinct's mesh again with the port of
    faery's builder, whose polygons are planar, so none carries detail triangles as Recast's do."""
    driver.click("Clipping")
    driver.frames(5)
    generator = driver.get_resource(VIEWS, at="nav_generator")
    assert generator == "Clipping", f"the generator is {generator}"

    def built():
        if not driver.select(LAIR_VIEW):
            return False, "no view"
        polygons = driver.get(LAIR, NAV_MESH, at="polygons")
        with_detail = sum(1 for p in polygons if p["detail"])
        return with_detail == 0, f"{with_detail} polygons with detail"

    driver.wait(built, "the lair's clipping mesh", {"seconds": 30})
    polygons = driver.get(LAIR, NAV_MESH, at="polygons")
    areas = {area: sum(1 for p in polygons if p["area"] == area) for area in ("Door", "Path")}
    print(f"lair, clipping: {len(polygons)} polygons, {areas}")
    assert areas["Door"], "the lair's clipping mesh has no doorways"
    assert areas["Path"], "the lair's clipping mesh has no footpaths"
    driver.goto(DOORWAY, 12.0)
    driver.frames(10)
    driver.screenshot("nav_clipping")
