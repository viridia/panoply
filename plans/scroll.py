"""Scrolling: drives the viewpoint diagonally (+x and +z, where hitches show most) across the
playground twice along the same line, first over ground never loaded, then again with everything
cached, to separate load hitches from steady cost.

    python3 crates/bevy_remote_driver/client/run.py plans/scroll.py
    TRACE=1 python3 crates/bevy_remote_driver/client/run.py plans/scroll.py

Prints the average frame time of each pass. With `TRACE=1` the game is built with Bevy's
`trace_chrome` feature and writes `trace-*.json` (system spans per frame) in the package
directory; tracing slows every frame, so compare spikes there, not averages.
"""

import math
import os
import time

BIN = "panoply"
FEATURES = ["bevy/trace_chrome"] if os.environ.get("TRACE") else None
GOTO = "panoply_core::viewpoint::GotoLocation"
DISTANCE = 20.0
START = [8.0, 0.0, 8.0]
# Facing +x and +z. W walks at 10 m/s: about 100 m in 10 s, 70 m along each axis.
AZIMUTH = math.pi / 4
FRAMES = 1200


def log_lines(driver, marker):
    """Lines of the game's log (written live by the runner) containing `marker`."""
    text = (driver.out_dir / "app.log").read_text(errors="replace")
    return [line for line in text.splitlines() if marker in line]


def scroll(driver, label):
    driver.message(
        GOTO,
        {
            "realm": "playground",
            "position": START,
            "camera_distance": DISTANCE,
            "azimuth": AZIMUTH,
        },
    )
    # Let loading around the start settle before moving.
    driver.seconds(1)
    start_frame, start_time = driver.frame(), time.monotonic()
    driver.key("KeyW", hold=FRAMES)
    elapsed = time.monotonic() - start_time
    frames = driver.frame() - start_frame
    print(f"{label}: {frames} frames in {elapsed:.2f} s, {1000 * elapsed / frames:.1f} ms/frame")
    driver.screenshot(label)


def run(driver):
    driver.present("PrimaryCamera")
    driver.present("Ground", timeout={"frames": 600, "seconds": 30})
    # Model warm-up (models::warmup) runs at startup; wait for it to finish.
    driver.absent("ModelWarmup", timeout={"frames": 1200, "seconds": 60})
    before = len(log_lines(driver, "Pipeline created"))
    scroll(driver, "cold")
    # The hitch log (editor builds) logs each pipeline created after startup. Walking into new
    # ground should need none: warm-up compiled them all.
    # The screenshot step makes its own pipeline; that one is the driver's, not the game's.
    created = [
        line
        for line in log_lines(driver, "Pipeline created")[before:]
        if "screenshot" not in line
    ]
    for line in created:
        print(line.split("Pipeline created: ")[-1][:160])
    assert not created, f"{len(created)} pipelines created during the cold walk"
    scroll(driver, "warm")
    driver.exits("Escape")
