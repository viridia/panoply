"""Shadow crawl: a slow strafe at the representative forest/shore spot.

    bevy-remote-driver plans/shadows.py

Saves strafe_0..5.png taken a few frames apart while the view moves sideways slowly. Shadow edges
that shimmer or step between consecutive frames are "crawling". The still is `lighting_shore.png`
in `lighting.py`. SHOTS=before or SHOTS=after prefixes the names, for side-by-side comparison.
"""

import os

from _panoply_driver import PanoplyDriver

BIN = "panoply"
DRIVER = PanoplyDriver
# A gallery, judged by eye: a directory run leaves it out.
TAGS = ["manual"]
PREFIX = os.environ.get("SHOTS", "")


def name(n):
    return f"{PREFIX}_{n}" if PREFIX else n


def run(driver):
    driver.present("PrimaryCamera")
    driver.goto([110.0, 0.0, 150.0], 20.0)
    # Shift+Left strafes; holding it a few frames at a time moves the view slowly.
    for i in range(6):
        driver.press("ShiftLeft")
        driver.key("ArrowLeft", hold=3)
        driver.release("ShiftLeft")
        driver.frames(10)
        driver.screenshot(name(f"strafe_{i}"))
    driver.exits("Escape")
