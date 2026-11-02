"""Actors: the player's actor falls onto a floor in the playground and stays there, with one
skeleton (D28), plays its idle clip from the shared clips file, wears its hair and its exemplar's
colors, and holds a sword in its right hand. W walks it away from the camera, turning it to
match, with the camera following, and S backs it up. A slope steeper than 45° stops it, and it
slides down one it is dropped onto. Travel (`TravelTo`) takes it to a point, and W pre-empts it.
Its clip follows its speed: stand, walk (backward when backing up), fast walk and run.
Precincts place their actors, facing as placed, and a precinct's actors despawn with it.

    bevy-remote-driver plans/actors.py

Looks at it from the closest camera distance (20), mid-clip, so the outlines can be seen to
follow the pose. Checked by eye in `actor.png`: tan tunic, gold sleeves, brown hair and legs,
not the file's default materials; the hair, outlined, sits on the head. In `actor_capsule.png`,
a cyan capsule encloses the actor, its bottom on the ground. In `walked.png`, the actor stands
in the view's center facing away from the camera; in `slope.png`, at the foot of the hill's
face. In `creatures.png` (kreekin, feral dogs, bears, bandits; the alpha dog and the black bear
larger than the rest), `townsfolk.png` and `avatars.png`, each group's actors face as placed.
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
PLAYER = "bevy_animation::AnimationPlayer"
# The skeleton's animation graph: the root, then a node per clip in the order
# `build_actor_graph` adds them.
STAND_NODE, WALK_NODE, WALK_FAST_NODE, RUN_NODE = 1, 2, 3, 4
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
HOME = "panoply_actors::precinct_actors::HomePrecinct"
FIRST_PRECINCT = "Precinct:playground:0:0"
# Groups of precinct actors, a wide variety in a small area each, and the azimuth to see them
# from. The creatures and townsfolk stand in ranks along z, facing +x, so they are seen from
# their front at 45° (facing north-east); the townsfolk do not all fit in the view.
GROUPS = {
    "creatures": ([124.0, 0.0, 22.0], -0.75 * math.pi),
    "townsfolk": ([145.0, 0.0, 6.0], -0.75 * math.pi),
    "avatars": ([8.0, 0.0, 8.0], 0.0),
}
# Trot, the pace W walks at (R17), and walk, S's.
TROT = 3.0
WALK = 1.5
RUN = 5.0


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
    skeletons = driver.select("Actor/hips")
    assert len(skeletons) == 1, f"{len(skeletons)} skeletons, not one"
    check_precinct_actors(driver)

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
    driver.seconds(1.5)
    # A trot is a fast walk, its clip at its nominal rate.
    expect_gait(driver, WALK_FAST_NODE, 1.0)
    driver.seconds(0.5)
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
    driver.seconds(0.7)
    # The walk, played backward.
    expect_gait(driver, WALK_NODE, -1.0)
    driver.seconds(0.3)
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
    # Stopped, it crossfades back to standing.
    driver.seconds(0.5)
    expect_gait(driver, STAND_NODE)

    # Travel back at a run: the run clip, at its nominal rate. Checked by eye in `run.png`: a
    # running stride, not a walk. The speed ramps up as the actor turns toward the target; the
    # screenshot, which takes about 2 s while the actor runs on to it, waits for a sample at the
    # nominal rate, so the rate check never loses its sample to it.
    driver.set("Actor", TRAVEL, {"target": SPOT, "speed": RUN})
    gaits = []
    shot = False
    while not travelling(driver)[0]:
        gaits.append(gait(driver))
        node, speed = gaits[-1]
        if not shot and node == RUN_NODE and speed == Between(0.9, 1.1):
            driver.screenshot("run")
            shot = True
        driver.seconds(0.1)
    print(f"gaits on the way: {gaits}")
    running = [speed for node, speed in gaits if node == RUN_NODE]
    assert running, f"never ran: {gaits}"
    assert max(running) == Between(0.9, 1.1), f"run clip not at its nominal rate: {gaits}"

    # W pre-empts travel: it ends short of its target.
    driver.set("Actor", TRAVEL, {"target": TRAVEL_TARGET, "speed": WALK})
    driver.seconds(0.5)
    driver.key("KeyW", hold=10)
    driver.wait(lambda: travelling(driver), "W to end the travel", {"seconds": 1})
    driver.frames(5)
    stopped = driver.get("Actor", MOTION)
    assert flat_distance(stopped["position"], TRAVEL_TARGET) > 2.0, f"travelled on: {stopped}"

    for name, (position, azimuth) in GROUPS.items():
        driver.goto(position, 20.0, azimuth=azimuth)
        driver.screenshot(name)

    # A precinct's actors despawn with it.
    driver.call("world.despawn_entity", {"entity": driver.select(FIRST_PRECINCT)[0]})
    driver.absent("Actor:Shopkeeper")
    driver.exits("Escape")


def check_precinct_actors(driver):
    """The playground's first precinct places its 13 actors, each where and as it says."""
    precinct = driver.select(FIRST_PRECINCT)[0]

    def homed():
        rows = driver.call(
            "world.query", {"data": {"components": [HOME]}, "filter": {"with": [HOME]}}
        )
        here = [row for row in rows if row["components"][HOME] == precinct]
        return len(here) == 13, lambda: f"{len(here)} of {len(rows)} actors homed there"

    driver.wait(homed, "the first precinct's 13 actors", WAIT)
    # Shekku's exemplar extends another, which gives it its skin.
    driver.present("Actor:Shekku/ActorBody", timeout=WAIT)
    # Standing, the shopkeeper keeps the facing the precinct gives it (125.9°).
    driver.expect("Actor:Shopkeeper", MOTION, {"grounded": True}, timeout=WAIT)
    driver.seconds(1)
    facing = driver.get("Actor:Shopkeeper", MOTION)["facing"]
    assert facing == Between(2.18, 2.21), f"shopkeeper facing {facing}"
    # In the next precinct east, `ActorScale` draws the black bear 1.3 times its model's size.
    scale = driver.get("Actor:BlackBear/ActorBody", TRANSFORM)["scale"]
    assert all(s == Between(1.299, 1.301) for s in scale), f"bear scale {scale}"


def gait(driver):
    """The animation graph node with the most weight on the actor's skeleton, and its playback
    speed."""
    active = driver.get("Actor/HumanoidArmature", PLAYER)["active_animations"]
    entries = active.items() if isinstance(active, dict) else active
    node, animation = max(entries, key=lambda entry: entry[1]["weight"])
    return int(node), animation["speed"]


def expect_gait(driver, node, speed=None):
    found = gait(driver)
    print(f"gait: {found}")
    assert found[0] == node, f"node {found[0]}, not {node}: {found}"
    if speed is not None:
        assert found[1] == Between(speed - 0.1, speed + 0.1), f"speed {found[1]}, not {speed}"


def travelling(driver):
    """For `wait`: done once the actor has no `TravelTo`."""
    entity = driver.select("Actor")[0]
    found = driver.call(
        "world.get_components", {"entity": entity, "components": [TRAVEL], "strict": False}
    )["components"]
    return TRAVEL not in found, lambda: f"{found} at {driver.get('Actor', MOTION)}"
