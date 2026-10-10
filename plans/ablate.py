"""Performance ablation: where does frame time go at the playground's slowest spot?

    python3 crates/bevy_remote_driver/client/run.py plans/ablate.py

At one location, at camera distance DIST (default 20), with vsync off, measures the frame time with everything on, then with one
thing switched off at a time over BRP: water, flora, ground, three quarters of the pixels
(viewport inset), and all rendering (camera inactive). A large drop names the cost; no drop rules it out. Also prints the
render passes' CPU times and the entity count. The window must stay in front for the whole run.

    ABLATE=msaa,environment python3 crates/bevy_remote_driver/client/run.py plans/ablate.py

runs only the experiments whose names contain one of the given words.
"""

import os
import time

BIN = "panoply"
GOTO = "panoply_core::viewpoint::GotoLocation"
WINDOW = "bevy_window::window::Window"
LIGHT = "bevy_light::directional_light::DirectionalLight"
VISIBILITY = "bevy_camera::visibility::Visibility"
SETTLE = 60
# Camera distance to measure at; the game's normal distance is about 20. DIST=40 overrides.
DISTANCE = float(os.environ.get("DIST", "20"))
# Forest, shoreline, water, grass and rocks in view at distance 20: a representative, near
# worst-case spot. (At distance 20, (80, 80) frames only water.)
LOCATION = ("playground", [110.0, 0.0, 150.0])


def mutate(driver, entity, component, path, value):
    driver._call(
        "world.mutate_components",
        {"entity": entity, "component": component, "path": path, "value": value},
    )


def frame_time(driver, seconds=1.5):
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


def camera(driver):
    return driver._select("PrimaryCamera")[0]["entity"]


def insert(driver, entity, components):
    driver._call("world.insert_components", {"entity": entity, "components": components})


MSAA = "bevy_render::view::Msaa"
CASCADES = "bevy_light::cascade::CascadeShadowConfig"
DEPTH_PREPASS = "bevy_core_pipeline::prepass::DepthPrepass"
SHADOW_FILTER = "bevy_light::ShadowFilteringMethod"
ENV_MAP = "bevy_light::probe::EnvironmentMapLight"
GENERATED_ENV_MAP = "bevy_light::probe::GeneratedEnvironmentMapLight"


def get_cascades(driver):
    light = lights(driver)[0]
    return driver._call(
        "world.get_components", {"entity": light, "components": [CASCADES]}
    )["components"][CASCADES]


def set_cascades(driver, value):
    for light in lights(driver):
        insert(driver, light, {CASCADES: value})


def cascades(bounds, minimum_distance=8.0):
    return {"bounds": bounds, "overlap_proportion": 0.2, "minimum_distance": minimum_distance}


def set_depth_prepass(driver, on):
    if on:
        insert(driver, camera(driver), {DEPTH_PREPASS: {}})
    else:
        driver._call(
            "world.remove_components", {"entity": camera(driver), "components": [DEPTH_PREPASS]}
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
    driver.message(GOTO, {"realm": realm, "position": position, "camera_distance": DISTANCE})
    base = frame_time(driver)
    original_cascades = get_cascades(driver)
    print(f"camera distance {DISTANCE}; default cascades: {original_cascades['bounds']}")
    print(f"cascades: {original_cascades}")
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
        # Per-pixel lighting costs, switched on the camera.
        ("msaa off", lambda: insert(driver, camera(driver), {MSAA: "Off"}), lambda: insert(driver, camera(driver), {MSAA: "Sample4"})),
        # Shadow cascades fitted to what the camera sees at distance 20 (ground about 14-45 m
        # away), instead of realm.rs's 4 cascades at about 4/11/30/130 m.
        ("cascades 1 [50]", lambda: set_cascades(driver, cascades([50.0])), lambda: set_cascades(driver, original_cascades)),
        ("cascades 2 [25,50]", lambda: set_cascades(driver, cascades([25.0, 50.0])), lambda: set_cascades(driver, original_cascades)),
        ("cascades 3 [18,30,50]", lambda: set_cascades(driver, cascades([18.0, 30.0, 50.0])), lambda: set_cascades(driver, original_cascades)),
        ("depth prepass", lambda: set_depth_prepass(driver, True), lambda: set_depth_prepass(driver, False)),
        ("2x2 shadow filter", lambda: insert(driver, camera(driver), {SHADOW_FILTER: "Hardware2x2"}), lambda: insert(driver, camera(driver), {SHADOW_FILTER: "Gaussian"})),
        # Nothing rendered at all: what remains is the main world's own systems.
        ("camera inactive", lambda: set_camera_active(driver, False), lambda: set_camera_active(driver, True)),
    ]
    # ABLATE=msaa,env runs only experiments whose names contain one of the words.
    only = [w for w in os.environ.get("ABLATE", "").split(",") if w]
    if only:
        experiments = [e for e in experiments if any(w in e[0] for w in only)]
    for name, apply, undo in experiments:
        apply()
        t = frame_time(driver)
        if name == "quarter pixels":
            driver.screenshot("quarter_pixels")
        undo()
        again = frame_time(driver)
        print(f"{name}: {t:.1f} ms (saves {again - t:+.1f} ms; restored {again:.1f} ms)")

    if only and not any(w in "environment map" for w in only):
        driver.exits("Escape")
        return
    # Image-based lighting: restoring it would mean recreating its cubemap handles, so this runs
    # last and is not undone.
    for component in [ENV_MAP, GENERATED_ENV_MAP]:
        try:
            driver._call("world.remove_components", {"entity": camera(driver), "components": [component]})
        except Exception:
            pass
    t = frame_time(driver)
    reference = again if experiments else base
    print(f"environment map off: {t:.1f} ms (saves {reference - t:+.1f} ms)")
    driver.exits("Escape")
