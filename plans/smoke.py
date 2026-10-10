"""Smoke test: the game starts, the viewpoint moves and strafes, and Escape quits.

    python3 crates/bevy_remote_driver/client/run.py plans/smoke.py

Screenshots land in target/remote-driver/panoply/smoke/. Movement is checked by comparing them,
since the viewpoint is a resource rather than a component a step can `expect`.
"""

BIN = "panoply"


def run(driver):
    driver.present("PrimaryCamera")
    # Let the realms load and the camera settle on the playground.
    driver.frames(60)
    driver.screenshot("start")

    driver.key("ArrowUp", hold=60)
    driver.screenshot("forward")

    # Shift+Arrow strafes rather than turns.
    driver.press("ShiftLeft")
    driver.key("ArrowLeft", hold=30)
    driver.release("ShiftLeft")
    driver.screenshot("strafed")

    driver.key("ArrowRight", hold=30)
    driver.screenshot("turned")

    driver.exits("Escape")
