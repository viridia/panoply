"""Hot reload: an edit to an embedded shader shows in the running game (D32, editor builds).

    python3 crates/bevy_remote_driver/client/run.py plans/hot_reload.py

Saves hot_reload_before.png, then rewrites water.wesl to return solid magenta, waits for the
reload, and saves hot_reload_after.png (the water should be magenta). The shader is always
restored, and the plan waits again so the game ends with the original.
"""

from pathlib import Path

BIN = "panoply"
GOTO = "panoply_core::viewpoint::GotoLocation"
SHADER = Path(__file__).resolve().parents[1] / "crates/panoply_terrain/src/shaders/water.wesl"
ORIGINAL = "    return tone_mapping(color, view.color_grading);\n"
MAGENTA = "    return vec4(1.0, 0.0, 1.0, 1.0);\n"


def run(driver):
    driver.present("PrimaryCamera")
    driver.message(GOTO, {"realm": "playground", "position": [189.0, 0.0, 62.0], "camera_distance": 20.0})
    driver.frames(120)
    driver.screenshot("hot_reload_before")
    source = SHADER.read_text()
    assert ORIGINAL in source, "water.wesl no longer ends with the expected return"
    try:
        SHADER.write_text(source.replace(ORIGINAL, MAGENTA))
        # The watcher debounces, then the shader and its pipelines recompile.
        driver.frames(240)
        driver.screenshot("hot_reload_after")
    finally:
        SHADER.write_text(source)
    driver.frames(240)
    driver.exits("Escape")
