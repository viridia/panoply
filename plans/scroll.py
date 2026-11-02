"""Scrolling: drives the viewpoint diagonally (+x and +z, where hitches show most) across the
playground over ground never loaded, and checks that it compiles no pipelines on the way.

    bevy-remote-driver plans/scroll.py
    TRACE=1 bevy-remote-driver plans/scroll.py

Prints the average frame time; `perf.py` measures the steady cost. With `TRACE=1` the game is built with Bevy's
`trace_chrome` feature and writes `trace-*.json` (system spans per frame) in the package
directory; tracing slows every frame, so compare spikes there, not averages.
"""

import math
import os
import time

from _panoply_driver import PanoplyDriver

BIN = "panoply"
DRIVER = PanoplyDriver
FEATURES = ["bevy/trace_chrome"] if os.environ.get("TRACE") else None
# The game inherits the runner's environment; the hitch log is opt-in, and this plan reads it.
os.environ["PANOPLY_HITCH_LOG"] = "1"
DISTANCE = 20.0
START = [8.0, 0.0, 8.0]
# Facing +x and +z. ArrowUp flies the camera at 10 m/s: about 100 m in 10 s, 70 m along each
# axis.
AZIMUTH = math.pi / 4
FRAMES = 1200


def log_lines(driver, marker):
    """Lines of the game's log (written live by the runner) containing `marker`."""
    text = (driver.out_dir / "app.log").read_text(errors="replace")
    return [line for line in text.splitlines() if marker in line]


def scroll(driver, label):
    driver.goto(START, DISTANCE, AZIMUTH)
    start_frame, start_time = driver.frame(), time.monotonic()
    # Not `key(hold=...)`, which returns before the release.
    driver.press("ArrowUp")
    driver.frames(FRAMES)
    driver.release("ArrowUp")
    elapsed = time.monotonic() - start_time
    frames = driver.frame() - start_frame
    print(f"{label}: {frames} frames in {elapsed:.2f} s, {1000 * elapsed / frames:.1f} ms/frame")
    driver.screenshot(label)


def run(driver):
    driver.present("PrimaryCamera")
    # `goto` waits for startup's model warm-up (panoply_render's models::warmup) too.
    driver.goto(START, DISTANCE, AZIMUTH)
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
    driver.exits("Escape")
