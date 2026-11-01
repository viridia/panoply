"""Actors: the player's actor stands on a floor in the playground, with one skeleton (D28), and
plays its idle clip from the shared clips file.

    bevy-remote-driver plans/actors.py

Looks at it from the closest camera distance (20), mid-clip, so the outlines can be seen to
follow the pose.
"""

from _panoply_driver import PanoplyDriver

BIN = "panoply"
DRIVER = PanoplyDriver
TRANSFORM = "bevy_transform::components::transform::Transform"
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

    # The idle clip shifts the actor's weight from foot to foot, which moves the hips.
    driver.present("Actor/HumanoidArmature/hips")
    before = driver.get("Actor/hips", TRANSFORM)
    driver.seconds(1)
    after = driver.get("Actor/hips", TRANSFORM)
    assert before != after, f"the hips did not move: {before}"
    driver.screenshot("actor")
    driver.exits("Escape")
