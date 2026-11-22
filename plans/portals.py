"""Portals: the view through an aperture, at the playground's mine entrance, whose far end is its
pair 26 m east, and at the stone portal near the origin, open from both sides.

    bevy-remote-driver plans/portals.py

Saves `mine_portal.png` with the view through the portal, `mine_portal_gizmos.png` with the Show
menu's Portals on, and `mine_no_portal.png` with the portal camera switched off: compared with
`mine_portal.png`, the main view's shadows must not change, since the portal camera's shadow pass
reuses the primary camera's shadow map layers. `stone_from_south.png` and `stone_from_north.png`
look through the stone portal from either side.
"""

from math import pi

from _panoply_driver import PanoplyDriver

BIN = "panoply"
DRIVER = PanoplyDriver

CAMERA = "bevy_camera::camera::Camera"
DEBUG_VIEWS = "panoply_editor::debug_views::DebugViews"
PORTAL_CAMERA = "PortalCamera:playground.mine.1"

# The ground under the aperture of the mine entrance `playground.mine.1` (precinct p001-p002), open
# from +x, whose target is its pair `playground.mine.2`.
MINE = [67.5, 0.0, 163.0]
# Facing -x, so the camera is on the aperture's open side.
FACING_WEST = 3 * pi / 2

# The ground under the aperture of the stone portal `playground.portal` (p000-p000), open from both
# sides, its aperture facing -z; its target is across the playground at (45.5, 0.75, 35.5).
STONE = [21.5, -1.98, 3.5]
STONE_CAMERA = "PortalCamera:playground.portal"


def run(driver):
    driver.caption("mine portal")
    driver.present("PrimaryCamera")
    driver.goto(MINE, 11.0, FACING_WEST)
    driver.present(PORTAL_CAMERA)
    # Its pair, open toward the camera but behind it, has no camera.
    driver.absent("PortalCamera:playground.mine.2", {"frames": 0})
    driver.frames(4)
    driver.screenshot("mine_portal")

    driver.set_resource(DEBUG_VIEWS, True, at="portals")
    driver.frames(2)
    driver.screenshot("mine_portal_gizmos")
    driver.set_resource(DEBUG_VIEWS, False, at="portals")

    # The same view without the portal camera's passes, its shadow pass among them.
    driver.set(PORTAL_CAMERA, CAMERA, False, at="is_active")
    driver.frames(4)
    driver.screenshot("mine_no_portal")

    driver.caption("facing away")
    # From the closed side, the portal shows nothing, and its view ends.
    driver.goto(MINE, 11.0, pi / 2)
    driver.absent(PORTAL_CAMERA)

    driver.caption("stone portal")
    # Open from both sides: a view from each.
    for azimuth, name in [(0.0, "stone_from_south"), (pi, "stone_from_north")]:
        driver.goto(STONE, 11.0, azimuth)
        driver.present(STONE_CAMERA)
        # The far end loads once the portal is near; ViewReady does not wait for it.
        driver.frames(30)
        driver.screenshot(name)
    driver.exits("Escape")
