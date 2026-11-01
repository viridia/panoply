"""Actors: the player's actor stands on a floor in the playground, with one skeleton (D28).

    bevy-remote-driver plans/actors.py

Looks at it from the closest camera distance (20).
"""

from _panoply_driver import PanoplyDriver

BIN = "panoply"
DRIVER = PanoplyDriver
# The player's spot, `PLAYER_POSITION` in `src/actors/mod.rs`.
SPOT = [58.0, 0.0, 26.62]
WAIT = {"seconds": 60}


def run(driver):
    driver.present("PrimaryCamera")
    # The body scene: its mesh is named for the skin.
    driver.present("Actor/ActorBody/WithTunic", timeout=WAIT)
    # Waits for warm-up too, whose instances of the character file have skeletons of their own.
    driver.goto(SPOT, 20.0)
    skeletons = driver.select("hips")
    assert len(skeletons) == 1, f"{len(skeletons)} skeletons, not one"
    driver.screenshot("actor")
    driver.exits("Escape")
