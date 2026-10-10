"""Tone mapping comparison: the same view under several `Tonemapping` modes.

    python3 crates/bevy_remote_driver/client/run.py plans/tonemapping.py

Saves one screenshot per mode (tonemap_<mode>.png) at playground (80, 80), which has grass,
water and trees in view. Bevy's default, TonyMcMapface, desaturates bright colors; three.js, which
the original version used, applies no tone mapping by default.
"""

BIN = "panoply"
GOTO = "panoply_core::viewpoint::GotoLocation"
TONEMAPPING = "bevy_render::view::Tonemapping"
MODES = ["TonyMcMapface", "None", "KhronosPbrNeutral", "AgX"]


def run(driver):
    driver.present("PrimaryCamera")
    camera = driver._select("PrimaryCamera")[0]["entity"]
    driver.message(
        GOTO, {"realm": "playground", "position": [80.0, 0.0, 80.0], "camera_distance": 40.0}
    )
    driver.frames(120)
    for mode in MODES:
        driver._call(
            "world.insert_components", {"entity": camera, "components": {TONEMAPPING: mode}}
        )
        # Switching modes respecializes pipelines; give them time to compile.
        driver.frames(60)
        driver.screenshot(f"tonemap_{mode}")
    driver.exits("Escape")
