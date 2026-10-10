"""Performance ablation: where does frame time go at the playground's slowest spot?

    python3 crates/bevy_remote_driver/client/run.py plans/ablate.py

At one location, with vsync off, measures the frame time with everything on, then with one
thing switched off at a time over BRP: water, flora, ground, three quarters of the pixels
(viewport inset), and all rendering (camera inactive). A large drop names the cost; no drop rules it out. Also prints the
render passes' CPU times and the entity count. The window must stay in front for the whole run.
"""

import time

BIN = "panoply"
GOTO = "panoply_core::viewpoint::GotoLocation"
WINDOW = "bevy_window::window::Window"
LIGHT = "bevy_light::directional_light::DirectionalLight"
VISIBILITY = "bevy_camera::visibility::Visibility"
SETTLE = 180
LOCATION = ("playground", [80.0, 0.0, 80.0])


def mutate(driver, entity, component, path, value):
    driver._call(
        "world.mutate_components",
        {"entity": entity, "component": component, "path": path, "value": value},
    )


def frame_time(driver, seconds=3.0):
    """Average milliseconds per frame over `seconds`, after letting a change settle.

    The driver's diagnostics report the latest single frame, which is too noisy to compare, so
    this counts frames over wall time instead, without polling the app in between.
    """
    driver.frames(SETTLE)
    f0, t0 = driver.frame(), time.monotonic()
    time.sleep(seconds)
    f1, t1 = driver.frame(), time.monotonic()
    return (t1 - t0) * 1000 / max(1, f1 - f0)


def set_visible(driver, name, visible):
    for match in driver._select(name):
        mutate(driver, match["entity"], VISIBILITY, "", "Inherited" if visible else "Hidden")


def lights(driver):
    rows = driver._call("world.query", {"data": {"components": []}, "filter": {"with": [LIGHT]}})
    return [row["entity"] for row in rows]


def set_shadows(driver, on):
    for light in lights(driver):
        mutate(driver, light, LIGHT, ".shadow_maps_enabled", on)


CAMERA = "bevy_camera::camera::Camera"


INSET = "panoply::view::viewport::ViewportInset"


def set_inset(driver, fraction):
    """Insets the 3D view by `fraction` of the window's logical size, from the right and bottom."""
    window = driver._call(
        "world.get_components", {"entity": driver.window, "components": [WINDOW]}
    )["components"][WINDOW]["resolution"]
    scale = window.get("scale_factor", 2.0)
    width = window["physical_width"] / scale
    height = window["physical_height"] / scale
    driver._call(
        "world.mutate_resources",
        {"resource": INSET, "path": "", "value": {
            "left": 0.0, "top": 0.0, "right": width * fraction, "bottom": height * fraction,
        }},
    )


def set_camera_active(driver, active):
    for match in driver._select("PrimaryCamera"):
        mutate(driver, match["entity"], CAMERA, ".is_active", active)


def run(driver):
    driver.present("PrimaryCamera")
    mutate(driver, driver.window, WINDOW, ".present_mode", "AutoNoVsync")
    driver.frames(30)
    driver.screenshot("focus")
    realm, position = LOCATION
    driver.message(GOTO, {"realm": realm, "position": position, "camera_distance": 40.0})
    base = frame_time(driver)
    driver.screenshot("location")

    diagnostics = driver._call("driver.diagnostics")
    print(f"entities: {diagnostics.get('entity_count', 0):.0f}")
    print(f"outlines: {len(driver._select('Outline'))}")
    passes = sorted(
        ((k, v) for k, v in diagnostics.items() if k.startswith("render/") and k.endswith("elapsed_cpu")),
        key=lambda kv: -kv[1],
    )
    for key, value in passes[:12]:
        print(f"  {key}: {value:.2f} ms")
    print(f"baseline: {base:.1f} ms")

    experiments = [
        # Switching shadow_maps_enabled off at runtime panics bevy_pbr 0.20's render thread
        # ("Failed to get directional light visible entities for cascade"), so there is no
        # shadows experiment; hiding flora removes the trees from the shadow pass as well.
        ("water hidden", lambda: set_visible(driver, "Water", False), lambda: set_visible(driver, "Water", True)),
        ("outlines hidden", lambda: set_visible(driver, "Outline", False), lambda: set_visible(driver, "Outline", True)),
        ("flora hidden", lambda: set_visible(driver, "Flora", False), lambda: set_visible(driver, "Flora", True)),
        ("ground hidden", lambda: set_visible(driver, "Ground", False), lambda: set_visible(driver, "Ground", True)),
        # A quarter of the pixels: the 3D view shrunk to half width and half height by the
        # editor's viewport inset. (Resizing the window over BRP sets width and height on
        # separate frames, which wgpu rejects; the scale-factor override has no effect.)
        ("quarter pixels", lambda: set_inset(driver, 0.5), lambda: set_inset(driver, 0.0)),
        # Nothing rendered at all: what remains is the main world's own systems.
        ("camera inactive", lambda: set_camera_active(driver, False), lambda: set_camera_active(driver, True)),
    ]
    for name, apply, undo in experiments:
        apply()
        t = frame_time(driver)
        if name == "quarter pixels":
            driver.screenshot("quarter_pixels")
        undo()
        again = frame_time(driver)
        print(f"{name}: {t:.1f} ms (saves {again - t:+.1f} ms; restored {again:.1f} ms)")
    driver.exits("Escape")
