"""Frame rate across the playground, from its corner to its center, and among its townsfolk.

    bevy-remote-driver plans/perf.py

Prints the average frame time at each stop and appends them as a line to `docs/perf.csv`, whose
header says how to read it. The playground's terrain covers parcels 0..16 on both axes (256 m
square). Vsync is turned off for the run: with it on, a frame slightly over the refresh interval
waits a whole extra interval, so the rate jumps between 120 and 60 and hides the real cost. A
covered window runs at about half rate, so the plan takes a screenshot first, which brings the
window forward, and the window must stay in front.

Environment variables measure the primary camera without parts of its post-processing, or with
other SSAO quality, and the line records them in its settings column:

    SSAO=off        removes ScreenSpaceAmbientOcclusion and the depth and normal prepasses it
                    requires
    SSAO=prepass    removes ScreenSpaceAmbientOcclusion and keeps the prepasses
    SMAA=off        removes Smaa, leaving no antialiasing
    SSAO_QUALITY=Low
                    ScreenSpaceAmbientOcclusion::quality_level: Low (4 samples a pixel), Medium
                    (8), High (18, the default) or Ultra (54)
"""

import csv
import datetime
import math
import os
import socket
import subprocess
import time
from pathlib import Path

from _panoply_driver import PanoplyDriver

BIN = "panoply"
DRIVER = PanoplyDriver
# A measurement, not a check: a directory run leaves it out.
TAGS = ["manual"]
REALM = "playground"
WINDOW = "bevy_window::window::Window"
SSAO = "bevy_pbr::ssao::ScreenSpaceAmbientOcclusion"
PREPASSES = [
    "bevy_core_pipeline::prepass::DepthPrepass",
    "bevy_core_pipeline::prepass::NormalPrepass",
]
SMAA = "bevy_anti_alias::smaa::Smaa"
# Each variable's values, and the camera components each value removes.
SETTINGS = {
    "SSAO": {"off": [SSAO, *PREPASSES], "prepass": [SSAO]},
    "SMAA": {"off": [SMAA]},
}
QUALITIES = ["Low", "Medium", "High", "Ultra"]
ROOT = Path(__file__).resolve().parent.parent
LOG = ROOT / "docs" / "perf.csv"
# Each stop's column in the log, its position, camera distance and azimuth. The townsfolk are seen
# as `plans/actors.py` sees them.
STOPS = [
    ("16_16", [16.0, 0.0, 16.0], 40.0, None),
    ("48_48", [48.0, 0.0, 48.0], 40.0, None),
    ("80_80", [80.0, 0.0, 80.0], 40.0, None),
    ("112_112", [112.0, 0.0, 112.0], 40.0, None),
    ("128_128", [128.0, 0.0, 128.0], 40.0, None),
    ("townsfolk", [145.0, 0.0, 6.0], 20.0, -0.75 * math.pi),
]


def git(*args):
    return subprocess.run(
        ["git", "-C", str(ROOT), *args], capture_output=True, text=True, check=True
    ).stdout.strip()


def settings():
    """The variables set, as `NAME=value` joined by spaces; the components they remove; and the
    SSAO quality level, or None for the build's own."""
    names, removed = [], []
    for name, values in SETTINGS.items():
        value = os.environ.get(name)
        if value is None:
            continue
        if value not in values:
            raise ValueError(f"{name}={value}: expected one of {', '.join(values)}")
        names.append(f"{name}={value}")
        removed += values[value]
    quality = os.environ.get("SSAO_QUALITY")
    if quality is not None:
        if quality not in QUALITIES:
            raise ValueError(f"SSAO_QUALITY={quality}: expected one of {', '.join(QUALITIES)}")
        if SSAO in removed:
            raise ValueError("SSAO_QUALITY has no SSAO to set with SSAO=off or SSAO=prepass")
        names.append(f"SSAO_QUALITY={quality}")
    return " ".join(names), removed, quality


def run(driver):
    driver.present("PrimaryCamera")
    setting, removed, quality = settings()
    if removed:
        driver.call(
            "world.remove_components", {"entity": driver.camera(), "components": removed}
        )
    if quality is not None:
        driver.call(
            "world.mutate_components",
            {
                "entity": driver.camera(),
                "component": SSAO,
                "path": ".quality_level",
                "value": quality,
            },
        )
    driver.call(
        "world.mutate_components",
        {
            "entity": driver.window,
            "component": WINDOW,
            "path": ".present_mode",
            "value": "AutoNoVsync",
        },
    )
    driver.frames(60)
    driver.screenshot("start")
    times = []
    for name, position, distance, azimuth in STOPS:
        driver.goto(position, distance, azimuth, realm=REALM)
        # Average over wall time: the driver's diagnostics report the latest single frame, which
        # is too noisy to compare.
        f0, t0 = driver.frame(), time.monotonic()
        time.sleep(3.0)
        f1, t1 = driver.frame(), time.monotonic()
        ms = (t1 - t0) * 1000 / max(1, f1 - f0)
        times.append(ms)
        print(f"{name}: {ms:.2f} ms/frame ({1000 / ms:.0f} fps)")
    driver.screenshot("townsfolk")
    window = driver.call(
        "world.get_components", {"entity": driver.window, "components": [WINDOW]}
    )["components"][WINDOW]["resolution"]
    log(window, setting, times)
    driver.exits("Escape")


def log(window, setting, times):
    row = [
        datetime.date.today().isoformat(),
        socket.gethostname().split(".")[0],
        git("rev-parse", "--short", "HEAD"),
        "yes" if git("status", "--porcelain") else "no",
        f"{window['physical_width']}x{window['physical_height']}",
        setting,
    ] + [f"{ms:.2f}" for ms in times]
    with LOG.open("a", newline="") as file:
        csv.writer(file).writerow(row)
    print(f"logged to {LOG}: {row}")
