"""Frame rate across the playground, from its corner to its center.

    python3 crates/bevy_remote_driver/client/run.py plans/perf.py

Prints the average frame time at each stop; the playground's terrain covers parcels
0..16 on both axes (256 m square). Vsync is turned off for the run: with it on, a frame slightly
over the refresh interval waits a whole extra interval, so the rate jumps between 120 and 60 and
hides the real cost. A covered window runs at about half rate, so the plan takes a screenshot
first, which brings the window forward, and the window must stay in front.
"""

import time

BIN = "panoply"
GOTO = "panoply_core::viewpoint::GotoLocation"
REALM = "playground"
STOPS = [(16, 16), (48, 48), (80, 80), (112, 112), (128, 128)]


def goto(driver, x, z):
    driver.message(GOTO, {"realm": REALM, "position": [x, 0.0, z], "camera_distance": 40.0})


def run(driver):
    driver.present("PrimaryCamera")
    driver._call(
        "world.mutate_components",
        {
            "entity": driver.window,
            "component": "bevy_window::window::Window",
            "path": ".present_mode",
            "value": "AutoNoVsync",
        },
    )
    driver.frames(60)
    driver.screenshot("start")
    for x, z in STOPS:
        goto(driver, x, z)
        # Let parcels load, then average over wall time: the driver's diagnostics report the
        # latest single frame, which is too noisy to compare.
        driver.frames(240)
        f0, t0 = driver.frame(), time.monotonic()
        time.sleep(3.0)
        f1, t1 = driver.frame(), time.monotonic()
        ms = (t1 - t0) * 1000 / max(1, f1 - f0)
        print(f"at ({x}, {z}): {ms:.2f} ms/frame ({1000 / ms:.0f} fps)")
    driver.screenshot("center")
    driver.exits("Escape")
