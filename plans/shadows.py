"""Shadows: a still, and a slow strafe for crawl, at the representative forest/shore spot.

    bevy-remote-driver plans/shadows.py

Saves shadows_still.png, then strafe_0..5.png taken a few frames apart while the view moves
sideways slowly. Shadow edges that shimmer or step between consecutive strafe frames are
"crawling". SHOTS=before or SHOTS=after prefixes the names, for side-by-side comparison.
"""

import os

from _panoply_driver import PanoplyDriver

BIN = "panoply"
DRIVER = PanoplyDriver
PREFIX = os.environ.get("SHOTS", "")


def name(n):
    return f"{PREFIX}_{n}" if PREFIX else n


def run(driver):
    driver.present("PrimaryCamera")
    driver.goto([110.0, 0.0, 150.0], 20.0)
    driver.screenshot(name("shadows_still"))
    # Shift+Left strafes; holding it a few frames at a time moves the view slowly.
    for i in range(6):
        driver.press("ShiftLeft")
        driver.key("ArrowLeft", hold=3)
        driver.release("ShiftLeft")
        driver.frames(10)
        driver.screenshot(name(f"strafe_{i}"))
    driver.exits("Escape")
