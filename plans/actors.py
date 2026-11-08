"""Actors: the player's actor falls onto a floor in the playground and stays there, with one
skeleton (D28), plays its idle clip from the shared clips file, wears its hair and its exemplar's
colors, and holds a sword in its right hand. W walks it away from the camera, turning it to
match, with the camera following, and S backs it up. A slope steeper than 45° stops it, and it
slides down one it is dropped onto. Travel (`TravelTo`) takes it to a point, and W pre-empts it.
Its clip follows its speed: stand, walk (backward when backing up), fast walk and run. Dropped,
it plays falling, landing, then standing; it turns on the spot toward a target behind it; and
its idle is the one its aspects choose.
Precincts place their actors, facing as placed, and a precinct's actors despawn with it. Standing
actors rest (`ActorResting`), and wake when sent travelling or when a precinct's colliders go.
Fallen into a hole or off the map, the actor is put back where it last stood. Its character file
reloaded, it is rebuilt with its hair and sword.
The movement keys drive the actor only in play (`GameMode`); in editing W moves the camera. In
play the arrows do as WASD, and Go To moves the actor. Facing a chest shows its prompt, and E
opens and shuts it.

    bevy-remote-driver plans/actors.py

Looks at it from camera distance 20, mid-clip, so the outlines can be seen to
follow the pose. Checked by eye in `actor.png`: tan tunic, gold sleeves, brown hair and legs,
not the file's default materials; the hair, outlined, sits on the head. In `actor_capsule.png`,
a cyan capsule encloses the actor, its bottom on the ground. In `walked.png`, the actor stands
in the view's center facing away from the camera; in `slope.png`, at the foot of the hill's
face. In `creatures.png` (kreekin, feral dogs, bears, bandits; the alpha dog and the black bear
larger than the rest), `townsfolk.png` and `avatars.png`, each group's actors face as placed.
"""

import math
from pathlib import Path

from bevy_remote_driver import Between

from _panoply_driver import PanoplyDriver

BIN = "panoply"
DRIVER = PanoplyDriver
TRANSFORM = "bevy_transform::components::transform::Transform"
GLOBAL_TRANSFORM = "bevy_transform::components::global_transform::GlobalTransform"
MOTION = "panoply_actors::actor_physics::ActorMotion"
RESTING = "panoply_actors::actor_physics::ActorResting"
INTENT = "panoply_actors::actor_control::ActorIntent"
TRAVEL = "panoply_actors::actor_travel::TravelTo"
PLAYER = "bevy_animation::AnimationPlayer"
STATE = "panoply_actors::actor_animation::ActorAnimationState"
IDLE = "panoply_actors::actor_animation::ActorIdle"
SKELETON = "Actor/HumanoidArmature"
VIEW_READY = "panoply_editor::diagnostics::view_ready::ViewReady"
CONTAINER_STATE = "panoply_scenery::container_aspect::ContainerState"
TEXT = "bevy_ui::widget::text::Text"
SPAN = "bevy_text::text::TextSpan"
# Under the player's spot, `PLAYER_POSITION` in `panoply_actors`.
SPOT = [37.0, 0.0, 47.0]
# South of the small hill whose top is near 41, 74, on flat ground; and over its south face,
# whose ground there is about 1 m up.
HILL_FOOT = [41.0, 0.2, 77.0]
HILL_FACE = [41.0, 2.5, 74.6]
# 4 m east of `SPOT`, over the same flat ground W walks on.
TRAVEL_TARGET = [41.0, 0.0, 47.0]
# A metre east of the lip at (31.8, 47.1), past the gravestone west of `SPOT`; due west of it, in
# the hole around (23, 43); and over the empty tile beyond the map's edge.
BRINK = [32.8, 1.0, 47.1]
HOLE = [23.0, 0.0, 47.1]
OFF_THE_MAP = [-5.0, 2.0, 47.0]
WAIT = {"seconds": 60}
ASSETS = Path(__file__).resolve().parent.parent / "assets"
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


def reload(driver, relative):
    """Writes an asset's own bytes back over it, so the file watcher reloads it, and waits until
    the player's actor has a new body."""
    body = driver.select("Actor/ActorBody")[0]
    path = ASSETS / relative
    path.write_bytes(path.read_bytes())

    def rebuilt():
        bodies = driver.select("Actor/ActorBody")
        return bool(bodies) and bodies[0] != body, lambda: f"the body is still {body}"

    driver.wait(rebuilt, f"{relative} to reload", WAIT)


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
    # Standing, the actor stays put while its idle clip shifts its weight from foot to foot,
    # which moves the hips, and the sword, which hangs from the right hand's bone.
    driver.present("Actor/HumanoidArmature/hips")
    driver.present("weapon.R/WieldedItem/Broadsword")
    hips = driver.get("Actor/hips", TRANSFORM)
    sword = driver.get("weapon.R/WieldedItem", GLOBAL_TRANSFORM)
    driver.seconds(0.5)
    later = driver.get("Actor", MOTION)
    assert later["position"] == landed["position"], f"moved: {landed} to {later}"
    assert driver.get("Actor/hips", TRANSFORM) != hips, f"the hips did not move: {hips}"
    after = driver.get("weapon.R/WieldedItem", GLOBAL_TRANSFORM)
    assert after != sword, f"the sword did not move: {sword}"
    # Standing, it rests: the controller no longer moves it.
    wait_resting(driver, "Actor", True)
    driver.screenshot("actor")

    # F3 draws the actor's capsule with the colliders.
    driver.key("F3")
    driver.frames(10)
    driver.screenshot("actor_capsule")
    driver.key("F3")

    # In editing, W moves the free camera, not the actor.
    driver.goto(SPOT, 20.0, azimuth=math.pi / 2)
    start = driver.get("Actor", MOTION)["position"]
    driver.key("KeyW", hold=10)
    driver.frames(5)
    view = driver.get_resource(VIEW_READY, at="position")
    assert flat_distance(view, SPOT) > 0.5, f"W did not move the view: {view}"
    assert driver.get("Actor", MOTION)["position"] == start, "W moved the actor in editing"

    # In play, with the camera facing +x, W turns the actor from +z to +x and walks it there, and
    # the camera follows again (`goto` left it).
    driver.goto(SPOT, 20.0, azimuth=math.pi / 2)
    driver.mode("Play")
    driver.press("KeyW")
    driver.seconds(1.5)
    # A trot is a fast walk, its clip at its nominal rate.
    expect_gait(driver, "WalkFast", 1.0)
    driver.seconds(0.5)
    driver.release("KeyW")
    driver.frames(5)
    walked = driver.get("Actor", MOTION)
    print(f"walked: {walked}")
    assert walked["facing"] == Between(math.pi / 2 - 0.01, math.pi / 2 + 0.01), f"facing {walked}"
    # The turn takes about 0.3 s of the 2, along a curve; the steps between press and release
    # add a little to the 2.
    assert walked["position"][0] - start[0] == Between(4.0, 2.3 * TROT), f"{start} to {walked}"
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
    expect_gait(driver, "Walk", -1.0)
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
    driver.mode("Edit")
    driver.goto(HILL_FOOT, 20.0, azimuth=math.pi)
    driver.mode("Play")
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

    # Dropped from 3 m, the actor falls, lands on touchdown, and stands once the landing ends.
    place(driver, [SPOT[0], 3.0, SPOT[2]])
    driver.mode("Edit")
    driver.goto(SPOT, 20.0)
    states = []
    for _ in range(30):
        states.append(driver.get(SKELETON, STATE))
        if "Landing" in states and states[-1] == "Stand":
            break
        driver.seconds(0.1)
    print(f"states in the fall: {states}")
    seen = [state for i, state in enumerate(states) if i == 0 or states[i - 1] != state]
    assert seen[-3:] == ["Falling", "Landing", "Stand"], f"not fall, land, stand: {states}"

    # Travel to a point 4 m to the actor's side: it turns there, walks, and stops on the point.
    # Resting until then, it wakes when sent.
    driver.expect("Actor", MOTION, {"grounded": True})
    wait_resting(driver, "Actor", True)
    driver.set("Actor", TRAVEL, {"target": TRAVEL_TARGET, "speed": TROT})
    wait_resting(driver, "Actor", False)
    driver.wait(lambda: travelling(driver), "the actor to finish travelling", {"seconds": 5})
    arrived = driver.get("Actor", MOTION)
    print(f"arrived: {arrived}")
    assert flat_distance(arrived["position"], TRAVEL_TARGET) < 0.06, f"missed: {arrived}"
    # It faces the bearing to the target, which swings as millimetres of sideways offset matter
    # more near it.
    assert arrived["facing"] == Between(math.pi / 2 - 0.15, math.pi / 2 + 0.15), f"{arrived}"
    # Stopped, it crossfades back to standing.
    driver.seconds(0.5)
    expect_gait(driver, "Stand")

    # An aspect choosing combat stance (R38) makes it the idle.
    driver.set("Actor", IDLE, "CombatStance")
    driver.frames(2)
    assert driver.get(SKELETON, STATE) == "CombatStance", "idle not combat stance"
    driver.set("Actor", IDLE, "Stand")

    # Travel back at a run: the actor turns on the spot toward the target behind it (about 0.3
    # s), then runs, the run clip at its nominal rate. Checked by eye in `run.png`: a running
    # stride, not a walk. The speed ramps up as the actor turns toward the target; the
    # screenshot, which takes about 2 s while the actor runs on to it, waits for a sample at the
    # nominal rate, so the rate check never loses its sample to it.
    driver.set("Actor", TRAVEL, {"target": SPOT, "speed": RUN})
    gaits = []
    shot = False
    while not travelling(driver)[0]:
        gaits.append(gait(driver))
        state, speed = gaits[-1]
        if not shot and state == "Run" and speed == Between(0.9, 1.1):
            driver.screenshot("run")
            shot = True
        driver.seconds(0.1)
    print(f"gaits on the way: {gaits}")
    states = [state for state, _ in gaits]
    assert "Turn" in states and "Run" in states, f"did not turn, then run: {gaits}"
    assert states.index("Turn") < states.index("Run"), f"did not turn on the spot first: {gaits}"
    running = [speed for state, speed in gaits if state == "Run"]
    assert running, f"never ran: {gaits}"
    assert max(running) == Between(0.9, 1.1), f"run clip not at its nominal rate: {gaits}"

    # In play, Up pre-empts travel as W does: it ends short of its target.
    driver.mode("Play")
    driver.set("Actor", TRAVEL, {"target": TRAVEL_TARGET, "speed": WALK})
    driver.seconds(0.5)
    driver.key("ArrowUp", hold=10)
    driver.wait(lambda: travelling(driver), "Up to end the travel", {"seconds": 1})
    driver.frames(5)
    stopped = driver.get("Actor", MOTION)
    assert flat_distance(stopped["position"], TRAVEL_TARGET) > 2.0, f"travelled on: {stopped}"

    # In play, Go To moves the actor, which lands there, and the view follows it.
    driver.goto(TRAVEL_TARGET, 20.0, wait=False)

    # Until the ground there is ready, the actor has no `ActorMotion`.
    def landed_there():
        entity = driver.select("Actor")[0]
        found = driver.call(
            "world.get_components", {"entity": entity, "components": [MOTION], "strict": False}
        )["components"]
        motion = found.get(MOTION)
        there = motion is not None and flat_distance(motion["position"], TRAVEL_TARGET) < 0.1
        return there and motion["grounded"], lambda: f"{motion}"

    driver.wait(landed_there, "the actor to land at the Go To", {"seconds": 5})
    moved = driver.get("Actor", MOTION)
    print(f"went to: {moved}")
    assert flat_distance(moved["position"], TRAVEL_TARGET) < 0.1, f"not moved there: {moved}"
    driver.seconds(1)
    view = driver.get_resource(VIEW_READY, at="position")
    assert flat_distance(view, moved["position"]) < 0.3, f"view at {view}, not the actor"
    driver.mode("Edit")

    # Walked west over the lip, the actor falls into the playground's hole with
    # nothing under it, and is put back where it last stood, at the lip, its fall stopped (R27).
    place(driver, BRINK)
    driver.expect("Actor", MOTION, {"grounded": True})
    driver.set("Actor", TRAVEL, {"target": HOLE, "speed": TROT})
    fell = wait_fallen(driver)
    entity = driver.select("Actor")[0]
    driver.call("world.remove_components", {"entity": entity, "components": [TRAVEL]})
    lip = wait_put_back(driver)
    print(f"fell into the hole: {fell}; put back: {lip}")
    assert lip["position"][0] == Between(29.0, 33.0), f"not back at the lip: {lip}"
    assert lip["position"] == lip["last_stood"], f"not where it last stood: {lip}"

    # Dropped past the map's edge, where the playground has no ground, it is put back at the lip.
    place(driver, OFF_THE_MAP)
    fell = wait_fallen(driver)
    back = wait_put_back(driver)
    print(f"fell off the map: {fell}; put back: {back}")
    assert flat_distance(back["position"], lip["position"]) < 0.1, f"not at the lip: {back}"

    check_chest(driver)

    for name, (position, azimuth) in GROUPS.items():
        driver.goto(position, 20.0, azimuth=azimuth)
        driver.screenshot(name)

    # A reloaded character file rebuilds the actors made from it, hair and sword included, rather
    # than Bevy respawning its scenes in place, which panicked on the sword's.
    reload(driver, "characters/humanoid-male.glb")
    driver.present("Actor/ActorFeature/HairMediumCut")
    driver.present("weapon.R/WieldedItem/Broadsword")
    driver.frames(10)
    skeletons = driver.select("Actor/hips")
    assert len(skeletons) == 1, f"{len(skeletons)} skeletons after the reload, not one"

    # A precinct's actors despawn with it. Its colliders go too, which wakes the resting player
    # for a tick, after which it rests again: as of a later change to the realm's colliders.
    before = wait_resting(driver, "Actor", True)["collider_changes"]
    driver.call("world.despawn_entity", {"entity": driver.select(FIRST_PRECINCT)[0]})
    driver.absent("Actor:Shopkeeper")
    wait_resting(driver, "Actor", True, since=before)
    driver.exits("Escape")


def prompt(driver):
    return driver.get("InteractionPrompt", TEXT) + driver.get("InteractionPromptCaption", SPAN)


def check_chest(driver):
    """Facing the chest nearest `SPOT`, the prompt offers it and E opens and shuts it; walking away
    clears the prompt. Checked by eye in `chest_shut.png` and `chest_open.png`: the lid raised."""
    rows = driver.call(
        "world.query",
        {
            "data": {"components": [CONTAINER_STATE, GLOBAL_TRANSFORM]},
            "filter": {"with": [CONTAINER_STATE]},
        },
    )
    assert rows, "no containers near the player"

    def translation(row):
        # An affine: three axes, then the translation.
        return row["components"][GLOBAL_TRANSFORM][9:]

    row = min(rows, key=lambda row: flat_distance(translation(row), SPOT))
    chest, center = row["entity"], translation(row)
    # West of it, facing east (+x), inside the entry's 1 m.
    stand = [center[0] - 0.8, center[1] + 0.2, center[2]]
    driver.goto(stand, 8.0)
    place(driver, stand)
    driver.expect("Actor", MOTION, {"grounded": True})
    motion = driver.get("Actor", MOTION)
    motion.update(facing=math.pi / 2)
    driver.set("Actor", MOTION, motion)
    driver.mode("Play")
    driver.frames(5)
    print(f"chest at {center}, actor {driver.get('Actor', MOTION)}")
    assert prompt(driver) == "[E] Open", f"prompt {prompt(driver)!r}"
    driver.screenshot("chest_shut")

    def state():
        return driver.call(
            "world.get_components", {"entity": chest, "components": [CONTAINER_STATE]}
        )["components"][CONTAINER_STATE]

    driver.key("KeyE")
    driver.frames(5)
    assert state() == {"open": True}, f"E did not open it: {state()}"
    driver.seconds(2)
    driver.screenshot("chest_open")
    driver.key("KeyE")
    driver.frames(5)
    assert state() == {"open": False}, f"E did not shut it: {state()}"

    motion = driver.get("Actor", MOTION)
    motion.update(facing=-math.pi / 2)
    driver.set("Actor", MOTION, motion)
    driver.frames(5)
    assert prompt(driver) == "", f"prompt not cleared: {prompt(driver)!r}"
    driver.mode("Edit")


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
    wait_resting(driver, "Actor:Shopkeeper", True)
    driver.seconds(1)
    facing = driver.get("Actor:Shopkeeper", MOTION)["facing"]
    assert facing == Between(2.18, 2.21), f"shopkeeper facing {facing}"
    # In the next precinct east, `ActorScale` draws the black bear 1.3 times its model's size.
    scale = driver.get("Actor:BlackBear/ActorBody", TRANSFORM)["scale"]
    assert all(s == Between(1.299, 1.301) for s in scale), f"bear scale {scale}"


def gait(driver):
    """The actor's animation state, and the playback speed of the clip with the most weight on
    its skeleton."""
    active = driver.get(SKELETON, PLAYER)["active_animations"]
    entries = active.items() if isinstance(active, dict) else active
    _, animation = max(entries, key=lambda entry: entry[1]["weight"])
    return driver.get(SKELETON, STATE), animation["speed"]


def expect_gait(driver, state, speed=None):
    found = gait(driver)
    print(f"gait: {found}")
    assert found[0] == state, f"state {found[0]}, not {state}: {found}"
    if speed is not None:
        assert found[1] == Between(speed - 0.1, speed + 0.1), f"speed {found[1]}, not {speed}"


def resting(driver, selector):
    """The `ActorResting` of the actor `selector` matches, or None."""
    entity = driver.select(selector)[0]
    found = driver.call(
        "world.get_components", {"entity": entity, "components": [RESTING], "strict": False}
    )["components"]
    return found.get(RESTING)


def wait_resting(driver, selector, expected, since=None):
    """Waits until the actor `selector` matches rests, or does not, and returns its
    `ActorResting`, or None. With `since`, a resting actor must have come to rest after that many
    collider changes."""

    def check():
        found = resting(driver, selector)
        done = (found is not None) == expected
        if found is not None and since is not None:
            done = done and found["collider_changes"] > since
        return done, lambda: f"{found} at {driver.get(selector, MOTION)}"

    state = "to rest" if expected else "not to rest"
    driver.wait(check, f"{selector} {state}", {"seconds": 5})
    return resting(driver, selector)


def wait_fallen(driver):
    """Waits until the actor has fallen half a metre below ground level (0), and returns its
    motion."""

    def check():
        motion = driver.get("Actor", MOTION)
        return motion["position"][1] < -0.5, lambda: f"{motion}"

    driver.wait(check, "the actor to fall", {"seconds": 5})
    return driver.get("Actor", MOTION)


def wait_put_back(driver):
    """Waits until the actor stands on the ground again, and returns its motion."""

    def check():
        motion = driver.get("Actor", MOTION)
        return motion["grounded"] and motion["position"][1] > -0.5, lambda: f"{motion}"

    driver.wait(check, "the actor to be put back", {"seconds": 5})
    return driver.get("Actor", MOTION)


def travelling(driver):
    """For `wait`: done once the actor has no `TravelTo`."""
    entity = driver.select("Actor")[0]
    found = driver.call(
        "world.get_components", {"entity": entity, "components": [TRAVEL], "strict": False}
    )["components"]
    return TRAVEL not in found, lambda: f"{found} at {driver.get('Actor', MOTION)}"
