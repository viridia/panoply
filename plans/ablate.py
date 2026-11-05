"""Performance ablation: where does frame time go at the playground's slowest spot?

    bevy-remote-driver plans/ablate.py

At one location, at camera distance DIST (default 20), with vsync off, measures the frame time with everything on, then with one
thing switched off at a time over BRP: water, flora, ground, three quarters of the pixels
(viewport inset), and all rendering (camera inactive). A large drop names the cost; no drop rules it out. Also prints the
render passes' CPU times, their GPU times and shader invocations where the backend records them, and the entity count. The
window must stay in front for the whole run.

    ABLATE=msaa,environment bevy-remote-driver plans/ablate.py

runs only the experiments whose names contain one of the given words, so `ABLATE=none` runs none
and stops after the baseline's report. `ABLATE=model` hides each flora model loaded, in turn, and
`ABLATE=hemlock,poplar` those models only; a run without `ABLATE` leaves these out.

    AT=townsfolk bevy-remote-driver plans/ablate.py

measures among the playground's townsfolk instead. The last experiments cannot be undone, so
each is measured on top of the one before: actors' animation off, then actors despawned, then
the environment map off.
"""

import math
import os
import time

from _panoply_driver import PanoplyDriver

BIN = "panoply"
DRIVER = PanoplyDriver
# A measurement, not a check: a directory run leaves it out.
TAGS = ["manual"]
WINDOW = "bevy_window::window::Window"
LIGHT = "bevy_light::directional_light::DirectionalLight"
VISIBILITY = "bevy_camera::visibility::Visibility"
SETTLE = 60
# Camera distance to measure at; the game's normal distance is about 20. DIST=40 overrides.
DISTANCE = float(os.environ.get("DIST", "20"))
# Forest, shoreline, water, grass and rocks in view at distance 20: a representative, near
# worst-case spot. (At distance 20, (80, 80) frames only water.)
# Townsfolk: where `plans/actors.py` views them, from their front at 45°.
LOCATIONS = {
    "forest": ("playground", [110.0, 0.0, 150.0], None),
    "townsfolk": ("playground", [145.0, 0.0, 6.0], -0.75 * math.pi),
    # Only trees, terrain and a little water, facing north-west (+x, +z).
    "trees": ("playground", [47.0, 0.0, 78.0], 0.25 * math.pi),
}
LOCATION = LOCATIONS[os.environ.get("AT", "forest")]
# Flora's node names, in the models' glTF files.
LEAF_NODES = ["leaf", "leaves"]
BARK_NODES = ["bark"]
# Flora's models, as `terrain.biomes.json` names them after the `#`.
FLORA_MODELS = [
    "buckberry", "dead1", "dead2", "dead3", "dead4", "desert1", "dreamsage", "eldenthorn",
    "fireweed", "hemlock1", "hemlock2", "hemlock3", "hemlock4", "leechgrass", "pine-arctic1",
    "pine-arctic2", "pine-arctic3", "pine-drooping", "pine-fluffy", "poplar-green",
    "poplar-green2", "poplar-green3", "poplar-ivory", "poplar-orange", "poplar-yellow", "rimeslip",
    "scragbush", "scrubthistle", "sticklebrush", "whitespear", "wracknettle",
]
HOME = "panoply_actors::precinct_actors::HomePrecinct"
SKINNED_MESH = "bevy_mesh::skinning::SkinnedMesh"
ANIMATION_GRAPH = "bevy_animation::graph::AnimationGraphHandle"


def mutate(driver, entity, component, path, value):
    driver.call(
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


def render_diagnostics(driver, samples=30):
    """Each `render/` diagnostic averaged over `samples` readings, since a reading is the latest
    frame's."""
    readings = {}
    for _ in range(samples):
        driver.frames(4)
        for diagnostic in driver.call("diagnostics.get", {})["diagnostics"]:
            path, value = diagnostic["path"], diagnostic["value"]
            if path.startswith("render/") and value is not None:
                readings.setdefault(path, []).append(value)
    return {path: sum(values) / len(values) for path, values in readings.items()}


def window_pixels(driver):
    window = driver.call(
        "world.get_components", {"entity": driver.window, "components": [WINDOW]}
    )["components"][WINDOW]["resolution"]
    return window["physical_width"] * window["physical_height"]


def opaque_summary(driver):
    """The main opaque pass's GPU time and shading, for one experiment's line; empty where the
    backend does not record them."""
    average = render_diagnostics(driver)
    name = "render/main_opaque_pass_3d"
    ms = average.get(f"{name}/elapsed_gpu")
    if ms is None:
        return ""
    fragments = average.get(f"{name}/fragment_shader_invocations", 0.0)
    vertices = average.get(f"{name}/vertex_shader_invocations", 0.0)
    pixels = window_pixels(driver)
    return f"; opaque pass {ms:.2f} ms, {fragments / pixels:.2f} fragments/pixel, {vertices:,.0f} vertices"


def gpu_report(driver):
    """Prints each render pass's GPU time and shader invocations. Bevy records these on Vulkan and
    DX12 only, so on Metal this says so and prints nothing else."""
    average = render_diagnostics(driver)
    passes = sorted(
        (
            (path.removesuffix("/elapsed_gpu"), value)
            for path, value in average.items()
            if path.endswith("/elapsed_gpu")
        ),
        key=lambda row: -row[1],
    )
    if not passes:
        print("GPU: no timestamps on this backend")
        return
    pixels = window_pixels(driver)
    print(f"GPU, ms per frame:{sum(ms for _, ms in passes):.2f} in all; window {pixels} pixels")
    for name, ms in passes:
        fragments = average.get(f"{name}/fragment_shader_invocations")
        vertices = average.get(f"{name}/vertex_shader_invocations")
        counts = ""
        if fragments is not None:
            counts += f"  {fragments / pixels:.2f} fragments per pixel"
        if vertices is not None:
            counts += f"  {vertices:,.0f} vertices"
        print(f"  {name}: {ms:.2f} ms{counts}")


def set_visible(driver, name, visible):
    for entity in driver.select(name):
        mutate(driver, entity, VISIBILITY, "", "Inherited" if visible else "Hidden")


def with_component(driver, component):
    rows = driver.call("world.query", {"data": {"components": []}, "filter": {"with": [component]}})
    return [row["entity"] for row in rows]


def set_actors_visible(driver, visible):
    """The precincts' actors: their bodies, outlines and features, in every pass."""
    for entity in with_component(driver, HOME):
        mutate(driver, entity, VISIBILITY, "", "Inherited" if visible else "Hidden")


def stop_animation(driver):
    """Without a graph, Bevy skips a skeleton's bones: no clips sampled, no bones moved."""
    for entity in with_component(driver, ANIMATION_GRAPH):
        driver.call("world.remove_components", {"entity": entity, "components": [ANIMATION_GRAPH]})


def despawn_actors(driver):
    for entity in with_component(driver, HOME):
        driver.call("world.despawn_entity", {"entity": entity})


def remove_environment_map(driver):
    for component in [ENV_MAP, GENERATED_ENV_MAP]:
        try:
            driver.call("world.remove_components", {"entity": driver.camera(), "components": [component]})
        except Exception:
            pass


def lights(driver):
    rows = driver.call("world.query", {"data": {"components": []}, "filter": {"with": [LIGHT]}})
    return [row["entity"] for row in rows]


def set_shadows(driver, on):
    for light in lights(driver):
        mutate(driver, light, LIGHT, ".shadow_maps_enabled", on)


CAMERA = "bevy_camera::camera::Camera"


INSET = "panoply_core::viewport::ViewportInset"


def set_inset(driver, fraction):
    """Insets the 3D view by `fraction` of the window's logical size, from the right and bottom."""
    window = driver.call(
        "world.get_components", {"entity": driver.window, "components": [WINDOW]}
    )["components"][WINDOW]["resolution"]
    scale = window.get("scale_factor", 2.0)
    width = window["physical_width"] / scale
    height = window["physical_height"] / scale
    driver.call(
        "world.mutate_resources",
        {"resource": INSET, "path": "", "value": {
            "left": 0.0, "top": 0.0, "right": width * fraction, "bottom": height * fraction,
        }},
    )


def insert(driver, entity, components):
    driver.call("world.insert_components", {"entity": entity, "components": components})


MSAA = "bevy_render::view::Msaa"
CASCADES = "bevy_light::cascade::CascadeShadowConfig"
DEPTH_PREPASS = "bevy_core_pipeline::prepass::DepthPrepass"
SHADOW_FILTER = "bevy_light::ShadowFilteringMethod"
ENV_MAP = "bevy_light::probe::EnvironmentMapLight"
GENERATED_ENV_MAP = "bevy_light::probe::GeneratedEnvironmentMapLight"


def get_cascades(driver):
    light = lights(driver)[0]
    return driver.call(
        "world.get_components", {"entity": light, "components": [CASCADES]}
    )["components"][CASCADES]


def set_cascades(driver, value):
    for light in lights(driver):
        insert(driver, light, {CASCADES: value})


def cascades(bounds, minimum_distance=8.0):
    return {"bounds": bounds, "overlap_proportion": 0.2, "minimum_distance": minimum_distance}


def set_depth_prepass(driver, on):
    if on:
        insert(driver, driver.camera(), {DEPTH_PREPASS: {}})
    else:
        driver.call(
            "world.remove_components", {"entity": driver.camera(), "components": [DEPTH_PREPASS]}
        )


def set_camera_active(driver, active):
    mutate(driver, driver.camera(), CAMERA, ".is_active", active)


def run(driver):
    driver.present("PrimaryCamera")
    mutate(driver, driver.window, WINDOW, ".present_mode", "AutoNoVsync")
    driver.frames(30)
    driver.screenshot("focus")
    realm, position, azimuth = LOCATION
    driver.goto(position, DISTANCE, azimuth, realm=realm)
    base = frame_time(driver)
    original_cascades = get_cascades(driver)
    print(f"camera distance {DISTANCE}; default cascades: {original_cascades['bounds']}")
    print(f"cascades: {original_cascades}")
    driver.screenshot("location")

    diagnostics = {
        d["path"]: d["value"]
        for d in driver.call("diagnostics.get", {})["diagnostics"]
        if d["value"] is not None
    }
    print(f"entities: {diagnostics.get('entity_count', 0):.0f}")
    print(f"outlines: {len(driver.select('Outline'))}")
    print(f"precinct actors: {len(with_component(driver, HOME))}")
    print(f"skinned meshes: {len(with_component(driver, SKINNED_MESH))}")
    passes = sorted(
        ((k, v) for k, v in diagnostics.items() if k.startswith("render/") and k.endswith("elapsed_cpu")),
        key=lambda kv: -kv[1],
    )
    for key, value in passes[:12]:
        print(f"  {key}: {value:.2f} ms")
    print(f"baseline: {base:.1f} ms")
    gpu_report(driver)

    def hide_all(names):
        for name in names:
            set_visible(driver, name, False)

    def show_all(names):
        for name in names:
            set_visible(driver, name, True)

    experiments = [
        # The models' own node names: every tree and shrub is opaque bark and alpha-masked leaves.
        ("leaves hidden", lambda: hide_all(LEAF_NODES), lambda: show_all(LEAF_NODES)),
        ("bark hidden", lambda: hide_all(BARK_NODES), lambda: show_all(BARK_NODES)),
        # Switching shadow_maps_enabled off at runtime panics bevy_pbr 0.20's render thread
        # ("Failed to get directional light visible entities for cascade"), so there is no
        # shadows experiment; hiding flora removes the trees from the shadow pass as well.
        ("water hidden", lambda: set_visible(driver, "Water", False), lambda: set_visible(driver, "Water", True)),
        ("outlines hidden", lambda: set_visible(driver, "Outline", False), lambda: set_visible(driver, "Outline", True)),
        ("flora hidden", lambda: set_visible(driver, "Flora", False), lambda: set_visible(driver, "Flora", True)),
        ("ground hidden", lambda: set_visible(driver, "Ground", False), lambda: set_visible(driver, "Ground", True)),
        ("actors hidden", lambda: set_actors_visible(driver, False), lambda: set_actors_visible(driver, True)),
        # A quarter of the pixels: the 3D view shrunk to half width and half height by the
        # editor's viewport inset. (Resizing the window over BRP sets width and height on
        # separate frames, which wgpu rejects; the scale-factor override has no effect.)
        ("quarter pixels", lambda: set_inset(driver, 0.5), lambda: set_inset(driver, 0.0)),
        # Per-pixel lighting costs, switched on the camera.
        ("msaa off", lambda: insert(driver, driver.camera(), {MSAA: "Off"}), lambda: insert(driver, driver.camera(), {MSAA: "Sample4"})),
        # Shadow cascades fitted to what the camera sees at distance 20 (ground about 14-45 m
        # away), instead of realm.rs's 4 cascades at about 4/11/30/130 m.
        ("cascades 1 [50]", lambda: set_cascades(driver, cascades([50.0])), lambda: set_cascades(driver, original_cascades)),
        ("cascades 2 [25,50]", lambda: set_cascades(driver, cascades([25.0, 50.0])), lambda: set_cascades(driver, original_cascades)),
        ("cascades 3 [18,30,50]", lambda: set_cascades(driver, cascades([18.0, 30.0, 50.0])), lambda: set_cascades(driver, original_cascades)),
        ("depth prepass", lambda: set_depth_prepass(driver, True), lambda: set_depth_prepass(driver, False)),
        ("2x2 shadow filter", lambda: insert(driver, driver.camera(), {SHADOW_FILTER: "Hardware2x2"}), lambda: insert(driver, driver.camera(), {SHADOW_FILTER: "Gaussian"})),
        # Nothing rendered at all: what remains is the main world's own systems.
        ("camera inactive", lambda: set_camera_active(driver, False), lambda: set_camera_active(driver, True)),
    ]
    # ABLATE=msaa,env runs only experiments whose names contain one of the words.
    only = [w for w in os.environ.get("ABLATE", "").split(",") if w]
    if only:
        # One experiment per flora model, only when asked for, since each costs seconds. A model
        # hidden is hidden everywhere it is loaded, whether or not it is in view.
        for model in FLORA_MODELS:
            count = len(driver.select(model))
            if count and any(w in f"model {model}" for w in only):
                experiments.append(
                    (
                        f"model {model} ({count} loaded)",
                        lambda model=model: set_visible(driver, model, False),
                        lambda model=model: set_visible(driver, model, True),
                    )
                )
        experiments = [e for e in experiments if any(w in e[0] for w in only)]
    for name, apply, undo in experiments:
        apply()
        t = frame_time(driver)
        gpu = opaque_summary(driver)
        if name == "quarter pixels":
            driver.screenshot("quarter_pixels")
        undo()
        again = frame_time(driver)
        print(f"{name}: {t:.1f} ms (saves {again - t:+.1f} ms; restored {again:.1f} ms){gpu}")

    # Not undone, so they run last, each on top of the ones before. Restoring animation or actors
    # would mean rebuilding them, and image-based lighting its cubemap handles.
    permanent = [
        ("animation off", lambda: stop_animation(driver)),
        ("actors despawned", lambda: despawn_actors(driver)),
        ("environment map off", lambda: remove_environment_map(driver)),
    ]
    if only:
        permanent = [p for p in permanent if any(w in p[0] for w in only)]
    reference = again if experiments else base
    for name, apply in permanent:
        apply()
        t = frame_time(driver)
        print(f"{name}: {t:.1f} ms (saves {reference - t:+.1f} ms)")
        reference = t
    driver.exits("Escape")
