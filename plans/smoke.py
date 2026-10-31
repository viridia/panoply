"""Smoke test: the game starts, the viewpoint moves, strafes and zooms, and Escape quits.

    bevy-remote-driver plans/smoke.py

Screenshots land in target/remote-driver/panoply/smoke/. Movement is checked by comparing them,
since the viewpoint is a resource rather than a component a step can `expect`.
"""

from _panoply_driver import PanoplyDriver

BIN = "panoply"
DRIVER = PanoplyDriver


def run(driver):
    driver.present("PrimaryCamera")
    # Let the realms load and the camera settle on the playground.
    driver.frames(60)
    driver.screenshot("start")

    # Press, wait, release: `key(hold=...)` returns before the release, and a screenshot is not
    # input, so it would not wait for it.
    driver.press("ArrowUp")
    driver.frames(60)
    driver.release("ArrowUp")
    driver.screenshot("forward")

    # Shift+Arrow strafes rather than turns.
    driver.press("ShiftLeft")
    driver.key("ArrowLeft", hold=30)
    driver.release("ShiftLeft")
    driver.screenshot("strafed")

    driver.press("ArrowRight")
    driver.frames(30)
    driver.release("ArrowRight")
    driver.screenshot("turned")

    # Zoom in two steps and back out. (Plans run with their own settings, so this never touches
    # the developer's saved zoom.)
    driver.key("Equal")
    driver.key("Equal")
    driver.seconds(0.4)
    driver.screenshot("zoomed")
    driver.key("Minus")
    driver.key("Minus")

    driver.exits("Escape")
