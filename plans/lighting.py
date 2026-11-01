"""Lighting: stills at a village spot and the forest/shore spot, for judging art direction (X12).

    bevy-remote-driver plans/lighting.py

Saves lighting_village.png (playground 48.5, 54.8: buildings, props, cobbles),
lighting_shore.png (110, 150: water, trees), lighting_bluff.png (197, 9, facing +x+z: waves
breaking on a bluff), lighting_ocean.png (189, 62: open water only) and lighting_far.png (the
shore spot at camera distance 40: texture shimmer and mipmaps). Environment variables try
other values without a rebuild, through the reflected resources:

    SUN=1500        Sunlight::illuminance (lux)
    AMBIENT=600     HemisphereLight::brightness
    TONEMAP=AgX     camera Tonemapping (Linear by default)
    SHOTS=before    prefixes the names, for side-by-side comparison
"""

import math
import os

from _panoply_driver import PanoplyDriver

BIN = "panoply"
DRIVER = PanoplyDriver
SUNLIGHT = "panoply_core::sunlight::Sunlight"
HEMISPHERE = "panoply_terrain::hemisphere::HemisphereLight"
TONEMAPPING = "bevy_render::view::Tonemapping"
PREFIX = os.environ.get("SHOTS", "")
# Name: (position, azimuth, camera distance). Azimuth 0 faces +z, PI/4 faces between +x and +z.
SPOTS = {
    "village": ([48.5, 0.0, 54.8], None, 20.0),
    "shore": ([110.0, 0.0, 150.0], None, 20.0),
    "bluff": ([197.0, 0.0, 9.0], math.pi / 4, 20.0),
    "ocean": ([189.0, 0.0, 62.0], None, 20.0),
    "far": ([110.0, 0.0, 150.0], None, 40.0),
}


def name(n):
    return f"{PREFIX}_{n}" if PREFIX else n


def mutate(driver, resource, path, value):
    driver.call("world.mutate_resources", {"resource": resource, "path": path, "value": value})


def run(driver):
    driver.present("PrimaryCamera")
    if "SUN" in os.environ:
        mutate(driver, SUNLIGHT, ".illuminance", float(os.environ["SUN"]))
    if "AMBIENT" in os.environ:
        mutate(driver, HEMISPHERE, ".brightness", float(os.environ["AMBIENT"]))
    if "TONEMAP" in os.environ:
        camera = driver.camera()
        driver.call(
            "world.insert_components",
            {"entity": camera, "components": {TONEMAPPING: os.environ["TONEMAP"]}},
        )
    for spot, (position, azimuth, distance) in SPOTS.items():
        driver.goto(position, distance, azimuth)
        driver.screenshot(name(f"lighting_{spot}"))
    driver.exits("Escape")
