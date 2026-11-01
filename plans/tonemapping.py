"""Tone mapping comparison: the same view under several `Tonemapping` modes.

    bevy-remote-driver plans/tonemapping.py

Saves one screenshot per mode (tonemap_<mode>.png) at playground (80, 80), which has grass,
water and trees in view. Bevy's default, TonyMcMapface, desaturates bright colors; three.js, which
the original version used, applies no tone mapping by default.
"""

from _panoply_driver import PanoplyDriver

BIN = "panoply"
DRIVER = PanoplyDriver
TONEMAPPING = "bevy_render::view::Tonemapping"
MODES = ["TonyMcMapface", "None", "KhronosPbrNeutral", "AgX"]


def run(driver):
    driver.present("PrimaryCamera")
    camera = driver.camera()
    driver.goto([80.0, 0.0, 80.0], 40.0)
    for mode in MODES:
        driver.call(
            "world.insert_components", {"entity": camera, "components": {TONEMAPPING: mode}}
        )
        # Switching modes respecializes pipelines; give them time to compile.
        driver.frames(60)
        driver.screenshot(f"tonemap_{mode}")
    driver.exits("Escape")
