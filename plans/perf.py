"""Frame rate across the playground, from its corner to its center.

    python3 crates/bevy_remote_driver/client/run.py plans/perf.py

Prints the smoothed fps and frame time at each stop; the playground's terrain covers parcels
0..16 on both axes (256 m square). Vsync is turned off for the run: with it on, a frame slightly
over the refresh interval waits a whole extra interval, so the rate jumps between 120 and 60 and
hides the real cost. A covered window runs at about half rate, so the plan takes a screenshot
first, which brings the window forward, and the window must stay in front.
"""

BIN = "panoply"
GOTO = "panoply_core::viewpoint::GotoLocation"
REALM = "playground"
STOPS = [(16, 16), (48, 48), (80, 80), (112, 112), (128, 128)]


def goto(driver, x, z):
    driver.message(GOTO, {"realm": REALM, "position": [x, 0.0, z]})


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
        # Let parcels load and the smoothed fps settle.
        driver.frames(240)
        d = driver._call("driver.diagnostics")
        print(f"at ({x}, {z}): fps {d.get('fps', 0):.0f}  frame_time {d.get('frame_time', 0):.1f} ms")
    driver.screenshot("center")
    driver.exits("Escape")
