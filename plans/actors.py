"""Actors: the player's actor falls onto a floor in the playground and stays there, with one
skeleton (D28), plays its idle clip from the shared clips file, wears its hair and its exemplar's
colors, and holds a sword in its right hand. W walks it away from the camera, turning it to
match, with the camera following, and S backs it up. A slope steeper than 45° stops it, and it
slides down one it is dropped onto. Travel (`TravelTo`) takes it to a point, and W pre-empts it.

    bevy-remote-driver plans/actors.py

Looks at it from the closest camera distance (20), mid-clip, so the outlines can be seen to
follow the pose. Checked by eye in `actor.png`: tan tunic, gold sleeves, brown hair and legs,
not the file's default materials; the hair, outlined, sits on the head. In `actor_capsule.png`,
a cyan capsule encloses the actor, its bottom on the ground. In `walked.png`, the actor stands
in the view's center facing away from the camera; in `slope.png`, at the foot of the hill's
face.
"""

import math

from bevy_remote_driver import Between

from _panoply_driver import PanoplyDriver

BIN = "panoply"
DRIVER = PanoplyDriver
TRANSFORM = "bevy_transform::components::transform::Transform"
GLOBAL_TRANSFORM = "bevy_transform::components::global_transform::GlobalTransform"
MOTION = "panoply_actors::actor_physics::ActorMotion"
INTENT = "panoply_actors::actor_control::ActorIntent"
TRAVEL = "panoply_actors::actor_travel::TravelTo"
VIEW_READY = "panoply_editor::diagnostics::view_ready::ViewReady"
# Under the player's spot, `PLAYER_POSITION` in `panoply_actors`.
SPOT = [37.0, 0.0, 47.0]
# South of the small hill whose top is near 41, 74, on flat ground; and over its south face,
# whose ground there is about 1 m up.
HILL_FOOT = [41.0, 0.2, 77.0]
HILL_FACE = [41.0, 2.5, 74.6]
# 4 m east of `SPOT`, over the same flat ground W walks on.
TRAVEL_TARGET = [41.0, 0.0, 47.0]
WAIT = {"seconds": 60}
# Trot, the pace W walks at (R17), and walk, S's.
TROT = 3.0
WALK = 1.5


def flat_distance(a, b):
    return math.hypot(a[0] - b[0], a[2] - b[2])


def place(driver, position):
    """Moves the actor to `position`, falling from there."""
    motion = driver.get("Actor", MOTION)
    motion.update(position=position, previous=position, vertical_speed=0.0, grounded=False)
    driver.set("Actor", MOTION, motion)


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

    # Spawned 2 m over ground level (0), the actor gets physics once the ground is built, falls,
    # lands, and stays there.
    driver.expect("Actor", MOTION, {"grounded": True})
    landed = driver.get("Actor", MOTION)
    print(f"landed: {landed}")
    # The controller keeps a 1 cm gap.
    assert landed["position"][1] == Between(0.0, 0.05), f"not on the ground: {landed}"
    driver.seconds(1)
    later = driver.get("Actor", MOTION)
    assert later["position"] == landed["position"], f"moved: {landed} to {later}"

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

    # F3 draws the actor's capsule with the colliders.
    driver.key("F3")
    driver.frames(10)
    driver.screenshot("actor_capsule")
    driver.key("F3")

    # With the camera facing +x, W turns the actor from +z to +x and walks it there, and the
    # camera follows again (`goto` left it).
    driver.goto(SPOT, 20.0, azimuth=math.pi / 2)
    start = driver.get("Actor", MOTION)["position"]
    driver.press("KeyW")
    driver.seconds(2)
    driver.release("KeyW")
    driver.frames(5)
    walked = driver.get("Actor", MOTION)
    print(f"walked: {walked}")
    assert walked["facing"] == Between(math.pi / 2 - 0.01, math.pi / 2 + 0.01), f"facing {walked}"
    # The turn takes about 0.3 s of the 2, along a curve.
    assert walked["position"][0] - start[0] == Between(4.0, 2 * TROT), f"{start} to {walked}"
    assert walked["position"][2] - start[2] == Between(0.0, 1.0), f"{start} to {walked}"
    assert walked["grounded"], f"not grounded: {walked}"
    # The camera trails the actor and catches up once it stops, to within its slack (0.25 m).
    driver.seconds(2)
    view = driver.get_resource(VIEW_READY, at="position")
    assert flat_distance(view, walked["position"]) < 0.3, f"view at {view}, not the actor"

    driver.screenshot("walked")

    # S backs the actor toward the camera at a walk, still facing away from it.
    driver.press("KeyS")
    driver.seconds(1)
    driver.release("KeyS")
    driver.frames(5)
    backed = driver.get("Actor", MOTION)
    print(f"backed: {backed}")
    assert abs(backed["facing"] - walked["facing"]) < 1e-3, f"turned: {backed}"
    assert backed["position"][0] < walked["position"][0], f"not backward: {backed}"
    back = flat_distance(walked["position"], backed["position"])
    assert back == Between(0.5 * WALK, 1.2 * WALK), f"backed {back} m: {backed}"
    assert driver.get("Actor", INTENT)["speed"] == 0.0, "still moving with no key held"

    # The hill's south face rises from about 0.1 at z 75 to 1.8 at z 74, too steep to walk up
    # (R24): W toward it stops the actor at its foot.
    place(driver, HILL_FOOT)
    driver.goto(HILL_FOOT, 20.0, azimuth=math.pi)
    driver.press("KeyW")
    highest = -math.inf
    for _ in range(8):
        driver.seconds(0.25)
        highest = max(highest, driver.get("Actor", MOTION)["position"][1])
    driver.release("KeyW")
    driver.frames(5)
    foot = driver.get("Actor", MOTION)
    print(f"at the hill: {foot}, highest {highest:.2f}")
    assert highest < 0.6, f"climbed the slope to {highest}: {foot}"
    assert foot["position"][2] == Between(74.0, 76.0), f"not stopped at the slope: {foot}"
    driver.screenshot("slope")

    # Dropped onto the face, the actor slides down it rather than standing there.
    place(driver, HILL_FACE)
    driver.seconds(2)
    slid = driver.get("Actor", MOTION)
    print(f"slid: {slid}")
    assert slid["position"][1] < 0.3, f"stayed on the slope: {slid}"
    assert slid["position"][2] > HILL_FACE[2], f"did not slide down the face: {slid}"

    # Travel to a point 4 m to the actor's side: it turns there, walks, and stops on the point.
    place(driver, SPOT)
    driver.goto(SPOT, 20.0)
    driver.expect("Actor", MOTION, {"grounded": True})
    driver.set("Actor", TRAVEL, {"target": TRAVEL_TARGET, "speed": TROT})
    driver.wait(lambda: travelling(driver), "the actor to finish travelling", {"seconds": 5})
    arrived = driver.get("Actor", MOTION)
    print(f"arrived: {arrived}")
    assert flat_distance(arrived["position"], TRAVEL_TARGET) < 0.06, f"missed: {arrived}"
    # It faces the bearing to the target, which swings as millimetres of sideways offset matter
    # more near it.
    assert arrived["facing"] == Between(math.pi / 2 - 0.15, math.pi / 2 + 0.15), f"{arrived}"

    # W pre-empts travel: it ends short of its target.
    driver.set("Actor", TRAVEL, {"target": SPOT, "speed": WALK})
    driver.seconds(0.5)
    driver.key("KeyW", hold=10)
    driver.wait(lambda: travelling(driver), "W to end the travel", {"seconds": 1})
    driver.frames(5)
    stopped = driver.get("Actor", MOTION)
    assert flat_distance(stopped["position"], SPOT) > 2.0, f"travelled on: {stopped}"
    driver.exits("Escape")


def travelling(driver):
    """For `wait`: done once the actor has no `TravelTo`."""
    entity = driver.select("Actor")[0]
    found = driver.call(
        "world.get_components", {"entity": entity, "components": [TRAVEL], "strict": False}
    )["components"]
    return TRAVEL not in found, lambda: f"{found} at {driver.get('Actor', MOTION)}"
