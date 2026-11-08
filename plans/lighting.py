"""Lighting: stills at a village spot and the forest/shore spot, for judging art direction (X12).

    bevy-remote-driver plans/lighting.py

Saves lighting_village.png (playground 48.5, 54.8: buildings, props, cobbles),
lighting_shore.png (110, 150: water, trees), lighting_bluff.png (197, 9, facing +x+z: waves
breaking on a bluff), lighting_ocean.png (189, 62: open water only), lighting_far.png (the
shore spot at camera distance 40: texture shimmer and mipmaps), lighting_snow.png (demorun
190, 220: snow meeting tundra, across parcel edges), lighting_glade.png (demorun 226, 223: grass
amid snow), lighting_biomes.png (holm-overland 992, 891 at distance 80: many parcels' biomes
blending) and lighting_marheim.png (holm-overland 499, 701: the palace's relief on faces turned
from the sun). Environment variables try
other values without a rebuild, through the reflected resources:

    SUN=1500        Sunlight::illuminance (lux)
    AMBIENT=600     HemisphereLight::brightness
    TONEMAP=AgX     camera Tonemapping (Linear by default)
    SSAO=off        removes ScreenSpaceAmbientOcclusion from the camera
    SSAO=msaa       removes it and SMAA, and turns on 4x MSAA in both cameras
    SSAO_RADIUS=1.5 ScreenSpaceAmbientOcclusion::radius (m)
    SHOTS=before    prefixes the names, for side-by-side comparison
"""

import math
import os

from _panoply_driver import PanoplyDriver

BIN = "panoply"
DRIVER = PanoplyDriver
# A gallery, judged by eye: a directory run leaves it out.
TAGS = ["manual"]
SUNLIGHT = "panoply_core::sunlight::Sunlight"
HEMISPHERE = "panoply_terrain::hemisphere::HemisphereLight"
TONEMAPPING = "bevy_render::view::Tonemapping"
SSAO = "bevy_pbr::ssao::ScreenSpaceAmbientOcclusion"
SMAA = "bevy_anti_alias::smaa::Smaa"
MSAA = "bevy_render::view::Msaa"
PREFIX = os.environ.get("SHOTS", "")
# Name: (realm, position, azimuth, camera distance). Azimuth 0 faces +z, PI/4 faces between +x
# and +z.
SPOTS = {
    "village": ("playground", [48.5, 0.0, 54.8], None, 20.0),
    "shore": ("playground", [110.0, 0.0, 150.0], None, 20.0),
    "bluff": ("playground", [197.0, 0.0, 9.0], math.pi / 4, 20.0),
    "ocean": ("playground", [189.0, 0.0, 62.0], None, 20.0),
    "far": ("playground", [110.0, 0.0, 150.0], None, 40.0),
    "snow": ("demorun", [190.3, 0.0, 220.1], None, 20.0),
    "glade": ("demorun", [225.7, 0.0, 222.6], None, 20.0),
    "biomes": ("holm-overland", [992.0, 0.0, 891.1], None, 80.0),
    "marheim": ("holm-overland", [499.29, 0.2, 700.93], None, 20.0),
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
    if os.environ.get("SSAO") == "off":
        driver.call("world.remove_components", {"entity": driver.camera(), "components": [SSAO]})
    if os.environ.get("SSAO") == "msaa":
        driver.call(
            "world.remove_components", {"entity": driver.camera(), "components": [SSAO, SMAA]}
        )
        for camera in (driver.camera(), driver.select("HudCamera")[0]):
            driver.call(
                "world.insert_components", {"entity": camera, "components": {MSAA: "Sample4"}}
            )
    if "SSAO_RADIUS" in os.environ:
        driver.call(
            "world.mutate_components",
            {
                "entity": driver.camera(),
                "component": SSAO,
                "path": ".radius",
                "value": float(os.environ["SSAO_RADIUS"]),
            },
        )
    for spot, (realm, position, azimuth, distance) in SPOTS.items():
        driver.goto(position, distance, azimuth, realm=realm)
        driver.screenshot(name(f"lighting_{spot}"))
    driver.exits("Escape")
