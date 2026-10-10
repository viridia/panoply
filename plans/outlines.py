"""Outlines: the test objects near the origin, close up.

    python3 crates/bevy_remote_driver/client/run.py plans/outlines.py

Looks at `spawn_material_test`'s objects: an outlined cube at (-4, 4), the camping model at
(4, 0) and the skinned humanoid at (8, 0), from the closest camera distance (20).
"""

BIN = "panoply"
GOTO = "panoply_core::viewpoint::GotoLocation"


CAMERA = "bevy_camera::camera::Camera"


def run(driver):
    driver.present("PrimaryCamera")
    # Black outlines don't show against the default black background, so clear to light grey.
    camera = driver._select("PrimaryCamera")[0]["entity"]
    driver._call(
        "world.mutate_components",
        {
            "entity": camera,
            "component": CAMERA,
            "path": ".clear_color",
            "value": {"Custom": {"Srgba": {"red": 0.8, "green": 0.8, "blue": 0.8, "alpha": 1.0}}},
        },
    )
    driver.frames(60)
    driver.message(
        GOTO, {"realm": "playground", "position": [2.0, 0.0, 2.0], "camera_distance": 20.0}
    )
    driver.frames(30)
    driver.screenshot("outlines")
    driver.exits("Escape")
