"""Lighting: stills at a village spot and the forest/shore spot, for judging art direction (X12).

    python3 crates/bevy_remote_driver/client/run.py plans/lighting.py

Saves lighting_village.png (playground 48.5, 54.8: buildings, props, cobbles),
lighting_shore.png (110, 150: water, trees) and lighting_bluff.png (197, 9, facing +x+z: waves
breaking on a bluff). Environment variables try other values without a rebuild, through the
reflected resources:

    SUN=1500        Sunlight::illuminance (lux)
    AMBIENT=600     HemisphereLight::brightness
    TONEMAP=AgX     camera Tonemapping (Linear by default)
    SHOTS=before    prefixes the names, for side-by-side comparison
"""

import math
import os

BIN = "panoply"
GOTO = "panoply_core::viewpoint::GotoLocation"
SUNLIGHT = "panoply_core::sunlight::Sunlight"
HEMISPHERE = "panoply_terrain::hemisphere::HemisphereLight"
TONEMAPPING = "bevy_render::view::Tonemapping"
PREFIX = os.environ.get("SHOTS", "")
# Name: (position, azimuth). Azimuth 0 faces +z, PI/4 faces between +x and +z.
SPOTS = {
    "village": ([48.5, 0.0, 54.8], None),
    "shore": ([110.0, 0.0, 150.0], None),
    "bluff": ([197.0, 0.0, 9.0], math.pi / 4),
}


def name(n):
    return f"{PREFIX}_{n}" if PREFIX else n


def mutate(driver, resource, path, value):
    driver._call("world.mutate_resources", {"resource": resource, "path": path, "value": value})


def run(driver):
    driver.present("PrimaryCamera")
    if "SUN" in os.environ:
        mutate(driver, SUNLIGHT, ".illuminance", float(os.environ["SUN"]))
    if "AMBIENT" in os.environ:
        mutate(driver, HEMISPHERE, ".brightness", float(os.environ["AMBIENT"]))
    if "TONEMAP" in os.environ:
        camera = driver._select("PrimaryCamera")[0]["entity"]
        driver._call(
            "world.insert_components",
            {"entity": camera, "components": {TONEMAPPING: os.environ["TONEMAP"]}},
        )
    for spot, (position, azimuth) in SPOTS.items():
        goto = {"realm": "playground", "position": position, "camera_distance": 20.0}
        if azimuth is not None:
            goto["azimuth"] = azimuth
        driver.message(GOTO, goto)
        # Room for parcels and scenery to spawn and pipelines to compile.
        driver.frames(120)
        driver.screenshot(name(f"lighting_{spot}"))
    driver.exits("Escape")
