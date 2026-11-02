"""Actors: the player's actor stands on a floor in the playground, with one skeleton (D28),
plays its idle clip from the shared clips file, wears its hair and its exemplar's colors, and
holds a sword in its right hand.

    bevy-remote-driver plans/actors.py

Looks at it from the closest camera distance (20), mid-clip, so the outlines can be seen to
follow the pose. Checked by eye in `actor.png`: tan tunic, gold sleeves, brown hair and legs,
not the file's default materials; the hair, outlined, sits on the head.
"""

from _panoply_driver import PanoplyDriver

BIN = "panoply"
DRIVER = PanoplyDriver
TRANSFORM = "bevy_transform::components::transform::Transform"
GLOBAL_TRANSFORM = "bevy_transform::components::global_transform::GlobalTransform"
# The player's spot, `PLAYER_POSITION` in `src/actors/mod.rs`.
SPOT = [58.0, 0.0, 26.62]
WAIT = {"seconds": 60}


def run(driver):
    driver.present("PrimaryCamera")
    # The body scene: its mesh is named for the skin.
    driver.present("Actor/ActorBody/WithTunic", timeout=WAIT)
    driver.present("Actor/ActorFeature/HairMediumCut")
    # Waits for warm-up too, whose instances of the character file have skeletons of their own.
    driver.goto(SPOT, 20.0)
    # The hair's copy of the armature is gone, its mesh bound to the body's bones.
    skeletons = driver.select("hips")
    assert len(skeletons) == 1, f"{len(skeletons)} skeletons, not one"

    # The idle clip shifts the actor's weight from foot to foot, which moves the hips.
    driver.present("Actor/HumanoidArmature/hips")
    before = driver.get("Actor/hips", TRANSFORM)
    driver.seconds(1)
    after = driver.get("Actor/hips", TRANSFORM)
    assert before != after, f"the hips did not move: {before}"

    # The sword hangs from the right hand's bone, so the idle clip moves it too.
    driver.present("weapon.R/WieldedItem/Broadsword")
    before = driver.get("weapon.R/WieldedItem", GLOBAL_TRANSFORM)
    driver.seconds(1)
    after = driver.get("weapon.R/WieldedItem", GLOBAL_TRANSFORM)
    assert before != after, f"the sword did not move: {before}"
    driver.screenshot("actor")
    driver.exits("Escape")
