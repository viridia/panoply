"""Retarget an SMPL-H motion FBX (as text-to-motion services return) onto the humanoid armature.

Runs in a live Blender session with `artwork/characters/humanoid-armature.blend` open, through the
Blender MCP's `execute_blender_code`:

    exec(open("/Users/talin/Projects/games/panoply/scripts/retarget_smpl.py").read())
    result = retarget("/path/to/clip.fbx", "ZeibekikoAction")

Imports the FBX into a `SMPLImport` collection, moved 1.5 m along X to compare, and bakes every
bone of `HumanoidArmature` on every frame into a new action, made active, with NLA off so the
other clips' tracks do not mix in. The scene keeps its frame rate, which the importer changes to
the file's, and the keys are placed at it: a 30 fps clip in a 60 fps scene has a key every second
frame. It saves nothing. To keep the clip: `discard_import()` (deleting the collection by hand
leaves its actions, materials and packed textures in the file), turn NLA back on, and add the
action's NLA track like the other clips'.

The clip is turned 180 degrees about Z: SMPL faces -Y with its left at +X, the humanoid +Y with its
left at -X, so left and right keep their names. The torso, shoulders and upper limbs copy SMPL's
world rotations, corrected per bone for SMPL's T-pose against the humanoid's rest. The arms and
legs are two-bone IK chains: `handIK` and `heelIK` go where SMPL's wrists and ankles go, scaled by
limb length, and the elbow and knee poles turn with the upper limb. Height comes from the lowest
the clip's feet go, since the FBX's rest pose and its animation have different origins.

Then, to make clips that chain or loop:

    result = blend_ends("ZeibekikoAction", pose=("StandAction", 0), frames=10)
    result = blend_ends("ZeibekikoAction")  # a loop: the end blends onto the clip's own start

`blend_ends` eases every channel over the first and last `frames` keys, frames of the source
clip, onto a shared pose: a
frame of another action, so phrases that share it can follow one another, or, with no pose, the
clip's own first frame, leaving the start alone and dropping the last frame, which would repeat
the first. The hips' location is corrected along the whole
clip instead, a linear ramp, so drift is spread out rather than slid back at the seam.
"""

import math

import bpy
from mathutils import Euler, Matrix, Quaternion, Vector

TARGET = "HumanoidArmature"
SOURCE_SPACING = 1.5

# (humanoid bone, SMPL joint whose rotation drives it, SMPL joint its direction points at,
# humanoid bone whose direction that matches)
FK = [
    ("hips", "Pelvis", "Spine1", "hips"),
    ("spine", "Spine2", "Spine3", "spine"),
    ("ribs", "Spine3", "Neck", "ribs"),
    ("neck", "Neck", "Head", "neck"),
    ("head", "Head", None, "head"),
]
for _s in "LR":
    FK += [
        (f"shoulder.{_s}", f"{_s}_Collar", f"{_s}_Shoulder", f"shoulder.{_s}"),
        (f"upper_arm.{_s}", f"{_s}_Shoulder", f"{_s}_Elbow", f"upper_arm.{_s}"),
        (f"thigh.{_s}", f"{_s}_Hip", f"{_s}_Knee", f"thigh.{_s}"),
        (f"heelIK.{_s}", f"{_s}_Ankle", f"{_s}_Foot", f"foot.{_s}"),
        (f"hand.{_s}", f"{_s}_Wrist", f"{_s}_Middle1", f"hand.{_s}"),
    ]
# Bones posed from their parent's pose, parents first.
CHAIN = [
    ("spine", "hips"), ("ribs", "spine"), ("neck", "ribs"), ("head", "neck"),
    ("shoulder.L", "ribs"), ("shoulder.R", "ribs"),
    ("upper_arm.L", "shoulder.L"), ("upper_arm.R", "shoulder.R"),
    ("thigh.L", "hips"), ("thigh.R", "hips"),
]


def retarget(fbx_path, action_name):
    scene = bpy.context.scene
    if bpy.context.mode != "OBJECT":
        bpy.ops.object.mode_set(mode="OBJECT")
    coll = bpy.data.collections.new("SMPLImport")
    scene.collection.children.link(coll)
    bpy.context.view_layer.active_layer_collection = (
        bpy.context.view_layer.layer_collection.children[coll.name]
    )
    before = set(bpy.data.objects.keys())
    actions_before = set(bpy.data.actions.keys())
    fps = (scene.render.fps, scene.render.fps_base)
    bpy.ops.import_scene.fbx(filepath=fbx_path)
    # The importer sets the scene to the file's frame rate, which would stretch every other clip.
    source_fps = scene.render.fps / scene.render.fps_base
    scene.render.fps, scene.render.fps_base = fps
    step = (fps[0] / fps[1]) / source_fps
    coll["imported_actions"] = sorted(set(bpy.data.actions.keys()) - actions_before)
    new = [bpy.data.objects[n] for n in set(bpy.data.objects.keys()) - before]
    src = next(o for o in new if o.type == "ARMATURE")
    src_action = src.animation_data.action
    first, last = (int(f) for f in src_action.frame_range)

    ours = bpy.data.objects[TARGET]
    ad = ours.animation_data or ours.animation_data_create()
    previous = {"action": ad.action.name if ad.action else None, "use_nla": ad.use_nla}
    ad.action = None
    ad.use_nla = False

    turn = Matrix.Rotation(math.pi, 4, "Z")
    OB, PB = ours.data.bones, ours.pose.bones

    def s_mat(name, rest=False):
        bone = src.data.bones[name].matrix_local if rest else src.pose.bones[name].matrix
        loc, rot, _ = (turn @ src.matrix_world @ bone).decompose()
        return loc, rot.to_matrix()

    def s_pos(name, rest=False):
        return s_mat(name, rest)[0]

    def direction(bone):
        b = OB[bone]
        return (b.tail_local - b.head_local).normalized()

    ground = math.inf
    for f in range(first, last + 1):
        scene.frame_set(f)
        ground = min(ground, s_pos("L_Foot").z, s_pos("R_Foot").z)
    scene.frame_set(first)
    p1 = s_pos("Pelvis")
    standing = s_pos("Pelvis", True).z - s_pos("L_Foot", True).z
    pelvis_ref = Vector((p1.x, p1.y, ground + standing))

    def limb_ratio(a, b, sa, sb):
        return (OB[a].head_local - OB[b].tail_local).length / (
            s_pos(sa, True) - s_pos(sb, True)
        ).length

    s_leg = limb_ratio("thigh.L", "shin.L", "L_Hip", "L_Ankle")
    s_arm = limb_ratio("upper_arm.L", "forearm.L", "L_Shoulder", "L_Wrist")

    # Per bone: ours(f) = smpl(f) @ C, where C makes ours rest when SMPL's bone points along
    # ours at rest.
    C = {}
    for o, s, toward, od in FK:
        align = Matrix.Identity(3)
        if toward:
            d = (s_pos(toward, True) - s_pos(s, True)).normalized()
            align = d.rotation_difference(direction(od)).to_matrix()
        C[o] = (align @ s_mat(s, True)[1]).inverted() @ OB[o].matrix_local.to_3x3()

    def rest_rel(bone):
        b = OB[bone]
        return b.parent.matrix_local.inverted() @ b.matrix_local

    def basis(bone, pose, parent_pose=None):
        if OB[bone].parent is None:
            return OB[bone].matrix_local.inverted() @ pose
        return rest_rel(bone).inverted() @ parent_pose.inverted() @ pose

    def at(position, rot3):
        return Matrix.Translation(position) @ rot3.to_4x4()

    baked = {pb.name: [] for pb in PB}
    worst = {"leg_stretch": 0.0, "arm_stretch": 0.0, "heel_z_min": math.inf}
    for f in range(first, last + 1):
        scene.frame_set(f)
        for pb in PB:
            pb.matrix_basis = Matrix.Identity(4)
        R = {o: s_mat(s)[1] @ C[o] for o, s, *_ in FK}
        pose = {"hips": at(OB["hips"].head_local + s_leg * (s_pos("Pelvis") - pelvis_ref), R["hips"])}
        out = {"hips": basis("hips", pose["hips"])}
        for b, p in CHAIN:
            pose[b] = at((pose[p] @ rest_rel(b)).translation, R[b])
            out[b] = basis(b, pose[b], pose[p])
        for s in "LR":
            for upper, lower, ik, pole, sj, mj, ej, scale, key in [
                (f"thigh.{s}", f"shin.{s}", f"heelIK.{s}", f"kneeIK.{s}",
                 f"{s}_Hip", f"{s}_Knee", f"{s}_Ankle", s_leg, "leg_stretch"),
                (f"upper_arm.{s}", f"forearm.{s}", f"handIK.{s}", f"elbowIK.{s}",
                 f"{s}_Shoulder", f"{s}_Elbow", f"{s}_Wrist", s_arm, "arm_stretch"),
            ]:
                root = pose[upper].translation
                end = root + scale * (s_pos(ej) - s_pos(sj))
                mid = root + scale * (s_pos(mj) - s_pos(sj))
                reach = OB[upper].length + OB[lower].length
                worst[key] = max(worst[key], (end - root).length / reach)
                end_rot = R[ik] if ik in R else OB[ik].matrix_local.to_3x3()
                out[ik] = basis(ik, at(end, end_rot))
                turned = R[upper] @ OB[upper].matrix_local.to_3x3().inverted()
                pole_at = mid + turned @ (OB[pole].head_local - OB[lower].head_local)
                out[pole] = basis(pole, at(pole_at, OB[pole].matrix_local.to_3x3()))
                if ik.startswith("heelIK"):
                    worst["heel_z_min"] = min(worst["heel_z_min"], end.z)
        for name, m in out.items():
            PB[name].matrix_basis = m
        # The hands hang from the IK-solved forearms, so they wait for the solve.
        bpy.context.view_layer.update()
        for s in "LR":
            forearm = PB[f"forearm.{s}"].matrix
            hand = at(forearm.translation, R[f"hand.{s}"])
            m = rest_rel(f"hand.{s}").inverted() @ forearm.inverted() @ hand
            PB[f"hand.{s}"].matrix_basis = m.to_quaternion().to_matrix().to_4x4()
        for pb in PB:
            q = pb.matrix_basis.to_quaternion()
            prev = baked[pb.name][-1][1] if baked[pb.name] else None
            if pb.rotation_mode == "QUATERNION":
                if prev is not None and q.dot(prev) < 0:
                    q.negate()
                rot = q
            else:
                rot = q.to_euler(pb.rotation_mode, *([prev] if prev is not None else []))
            baked[pb.name].append((pb.location.copy(), rot))

    action = bpy.data.actions.new(action_name)
    action.use_fake_user = True
    ad.action = action
    for pb in PB:
        quat = pb.rotation_mode == "QUATERNION"
        channels = [("location", 3, 0), ("rotation_quaternion" if quat else "rotation_euler", 4 if quat else 3, 1)]
        for path, count, k in channels:
            for i in range(count):
                fc = action.fcurve_ensure_for_datablock(ours, f'pose.bones["{pb.name}"].{path}', index=i)
                fc.keyframe_points.add(len(baked[pb.name]))
                co = []
                for t, e in enumerate(baked[pb.name]):
                    co += [float(t * step), float(e[k][i])]
                fc.keyframe_points.foreach_set("co", co)
                for kp in fc.keyframe_points:
                    kp.interpolation = "LINEAR"
                fc.update()
    for pb in PB:
        pb.matrix_basis = Matrix.Identity(4)
    src.location.x += SOURCE_SPACING
    scene.frame_start, scene.frame_end = 0, round((last - first) * step)
    scene.frame_set(0)
    return {
        "action": action.name,
        "frames": last - first + 1,
        "source_fps": source_fps,
        "key_step": step,
        "previous": previous,
        "rest_heel_z": round(OB["heelIK.L"].head_local.z, 3),
        **{k: round(v, 3) for k, v in worst.items()},
    }


def discard_import():
    """Removes what `retarget` imported: the `SMPLImport` collection, its objects, the actions the
    import made, and, by purging orphans, their meshes, materials and packed textures."""
    coll = bpy.data.collections.get("SMPLImport")
    if coll is None:
        return {"removed": None}
    actions = list(coll.get("imported_actions", []))
    objects = [o.name for o in coll.all_objects]
    for name in objects:
        bpy.data.objects.remove(bpy.data.objects[name])
    bpy.data.collections.remove(coll)
    for name in actions:
        if name in bpy.data.actions:
            bpy.data.actions.remove(bpy.data.actions[name])
    bpy.data.orphans_purge(do_local_ids=True, do_linked_ids=False, do_recursive=True)
    return {
        "objects": objects,
        "actions": actions,
        "packed_images_left": [i.name for i in bpy.data.images if i.packed_file],
    }


def _fcurves(action):
    """An action's F-curves by (data path, index)."""
    found = {}
    for layer in action.layers:
        for strip in layer.strips:
            for bag in strip.channelbags:
                for fc in bag.fcurves:
                    found[(fc.data_path, fc.array_index)] = fc
    return found


def _smoothstep(u):
    u = min(max(u, 0.0), 1.0)
    return u * u * (3 - 2 * u)


def blend_ends(action_name, pose=None, frames=10):
    curves = _fcurves(bpy.data.actions[action_name])
    if pose:
        source, at = _fcurves(bpy.data.actions[pose[0]]), pose[1]
    else:
        source, at = curves, None

    # Group the channels by (data path) into vectors, so rotations blend as rotations.
    groups = {}
    for (path, i), fc in curves.items():
        groups.setdefault(path, {})[i] = fc
    rest = {"location": 0.0, "rotation_euler": 0.0, "rotation_quaternion": None, "scale": 1.0}

    # The hips' drift is spread across the whole clip, as a ramp of their location. The IK
    # targets and poles, parentless like the hips, take its horizontal part, or the body is
    # dragged away from its feet; not its vertical part, which would sink them into the floor.
    bones = bpy.data.objects[TARGET].data.bones
    hips_path = 'pose.bones["hips"].location'
    drift = None
    if hips_path in groups:
        hfc = [groups[hips_path][i] for i in range(3)]
        first, end = hfc[0].keyframe_points[0], hfc[0].keyframe_points[-1]
        h0 = [fc.keyframe_points[0].co[1] for fc in hfc]
        h1 = [fc.keyframe_points[-1].co[1] for fc in hfc]
        if pose:
            goal = [source[(hips_path, i)].evaluate(at) if (hips_path, i) in source else 0.0 for i in range(3)]
            start, finish = Vector(goal) - Vector(h0), Vector(goal) - Vector(h1)
        else:
            start, finish = Vector((0, 0, 0)), Vector(h0) - Vector(h1)
        to_armature = bones["hips"].matrix_local.to_quaternion()
        start, finish = to_armature @ start, to_armature @ finish
        start.z = finish.z = 0.0
        f0, span = first.co[0], end.co[0] - first.co[0]
        drift = lambda frame: start.lerp(finish, (frame - f0) / span)

    moved = {}
    for path, by_index in groups.items():
        prop = path.rsplit(".", 1)[1]
        count = len(by_index)
        fcs = [by_index[i] for i in range(count)]
        n = len(fcs[0].keyframe_points)
        last = n - 1
        values = [[fc.keyframe_points[t].co[1] for fc in fcs] for t in range(n)]
        bone = path.split('"')[1]
        if drift and prop == "location" and bone != "hips" and bones[bone].parent is None:
            to_local = bones[bone].matrix_local.to_quaternion().inverted()
            for t in range(n):
                shift = to_local @ drift(fcs[0].keyframe_points[t].co[0])
                values[t] = [v + s for v, s in zip(values[t], shift)]

        def target_value(i, frame_of_clip):
            fc = source.get((path, i))
            if fc is None:
                if prop == "rotation_quaternion":
                    return 1.0 if i == 0 else 0.0
                return rest.get(prop, 0.0)
            return fc.evaluate(at if at is not None else frame_of_clip)

        target = [target_value(i, fcs[i].keyframe_points[0].co[0]) for i in range(count)]
        start_target = target if pose else values[0]

        def toward(v, goal, w):
            if prop == "rotation_quaternion":
                q, g = Quaternion(v), Quaternion(goal)
                if q.dot(g) < 0:
                    g.negate()
                return list(q.slerp(g, w))
            if prop == "rotation_euler":
                g = Euler(goal)
                g.make_compatible(Euler(v))
                goal = list(g)
            return [a + (b - a) * w for a, b in zip(v, goal)]

        out = [list(v) for v in values]
        if path == 'pose.bones["hips"].location':
            d0 = [a - b for a, b in zip(start_target, values[0])]
            d1 = [a - b for a, b in zip(target, values[last])]
            for t in range(n):
                u = t / last
                out[t] = [v + (1 - u) * a + u * b for v, a, b in zip(values[t], d0, d1)]
        else:
            for t in range(n):
                if pose and t < frames:
                    out[t] = toward(values[t], start_target, _smoothstep(1 - t / frames))
                if t > last - frames:
                    out[t] = toward(out[t], target, _smoothstep((t - (last - frames)) / frames))
        change = max(abs(a - b) for row_v, row_o in zip(values, out) for a, b in zip(row_v, row_o))
        moved[path] = round(change, 3)
        for i, fc in enumerate(fcs):
            for t in range(n):
                fc.keyframe_points[t].co[1] = out[t][i]
            fc.update()
    if not pose:
        # The last frame now equals the first, which the loop plays next: drop it, or the seam
        # holds that pose for two frames.
        for fc in curves.values():
            fc.keyframe_points.remove(fc.keyframe_points[-1])
            fc.update()
    biggest = sorted(moved.items(), key=lambda kv: -kv[1])[:6]
    return {"action": action_name, "pose": pose, "frames": frames, "largest_changes": biggest}
