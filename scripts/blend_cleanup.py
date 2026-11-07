#!/usr/bin/env python3
"""Fix the problems blend_inventory.py reports, in .blend files that export clips or meshes.

    scripts/blend_cleanup.py                  dry run: print each change, save nothing
    scripts/blend_cleanup.py --apply ...      make the changes and save the files
    scripts/blend_cleanup.py --preview DIR    dry run, exporting each cleaned file into DIR
    scripts/blend_cleanup.py a.blend dir ...  files or directories (default artwork/characters/;
                                              a bare name is looked for there)

Runs inside Blender 5.2 or later, headless (see blendtools.py). A file holding something without a
rule here is reported and not saved. The changes, in order:

    tweak mode      left
    stash tracks    deleted when every action on them has a track of its own; an action only in a
                    stash is kept as a track or deleted by name, in STASHED, or blocks the file
    track names     a track still named NlaTrack takes its action's name, less a trailing "Action"
    off-track       an armature's active action on no track gets one, named the same way
    switches        a clip leaving unkeyed a constraint influence another clip keys (an IK/FK
                    switch) gets a constant key of the value Blender 3.5 exported it with: the
                    value the clip before it, in name order, ended with
    strip ranges    each strip covers its whole action, as Blender 3.5's exporter read it; a
                    one-frame action is left
    mesh actions    a mesh's active action and NLA tracks are removed when they do not move the mesh
                    (bone curves, stray keys on a modifier), which glTF cannot export; one that
                    moves it (a door) and is on no track gets a track named for the action, as
                    Blender 3.5 named the clip
    base colors     a Multiply Mix node feeding Base Color gets a Factor of 1, which Blender 5.2's
                    exporter needs to export its color, as 3.5's did whatever the Factor; and
                    when no Material Output targets all renderers, the one a Principled BSDF
                    feeds is made to, for the same reason (a renderer's own output still wins)
    export          Panoply's export settings are remembered in the scene, for files beside a .glb

Actions keep their names: other files link actions from humanoid-armature.blend by name.
"""

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import blendtools as bt  # noqa: E402

bt.relaunch(__file__)

import bpy  # noqa: E402

# Actions found only in a stash track, by file: "keep" makes the stash a track of that name.
STASHED = {
    "reshari.blend": {"WalkFast": "keep"},
    "dabori.blend": {"StandActionBad": "delete"},
}


def clip_name(action):
    name = action.name
    return name[: -len("Action")] if name.endswith("Action") and name != "Action" else name


class Cleanup:
    def __init__(self, blend):
        self.blend = blend
        self.changes = []
        self.blockers = []

    def change(self, text):
        self.changes.append(text)

    def block(self, text):
        self.blockers.append(text)

    def run(self):
        for obj in bpy.data.objects:
            ad = obj.animation_data
            if ad is None or not bt.is_local(obj):
                continue
            if ad.use_tweak_mode:
                ad.use_tweak_mode = False
                self.change(f"{obj.name}: left tweak mode")
            if obj.type == "MESH":
                self.mesh(obj, ad)
            elif obj.type == "ARMATURE" and not obj.hide_render:
                self.armature(obj, ad)
        for material in bpy.data.materials:
            for node in bt.partial_multiplies(material):
                factor = node.inputs["Factor"]
                self.change(
                    f"material {material.name}: {node.name!r} Factor "
                    f"{factor.default_value:g} set to 1"
                )
                factor.default_value = 1.0
            output = bt.stray_output(material)
            if output is not None:
                self.change(
                    f"material {material.name}: {output.name!r} targets all renderers, "
                    f"not {output.target}"
                )
                output.target = "ALL"
        if bt.exported_glb(self.blend).exists():
            expected = bt.expected_settings(bpy.data.objects)
            if bt.settings_drift(bpy.context.scene.get(bt.SETTINGS_KEY), expected):
                bpy.context.scene[bt.SETTINGS_KEY] = expected
                anim = "with" if expected["export_animations"] else "without"
                self.change(f"remembered Panoply's export settings, {anim} animation")

    def mesh(self, obj, ad):
        """A mesh's clips move the mesh itself (a door); anything else on it exports nothing."""
        if ad.action and not bt.animates_transform(ad.action):
            self.change(f"{obj.name}: cleared mesh action {ad.action.name!r}, which moves nothing")
            ad.action = None
        for track in list(ad.nla_tracks):
            actions = [s.action for s in track.strips if s.action]
            if not any(bt.animates_transform(a) for a in actions):
                self.change(f"{obj.name}: deleted mesh track {track.name!r}, which moves nothing")
                ad.nla_tracks.remove(track)
            elif bt.is_stash(track):
                self.block(f"{obj.name}: stash track {track.name!r} moves the mesh")
        on_track = {s.action.name for t in ad.nla_tracks for s in t.strips if s.action}
        # A mesh with rendering off is not exported, so its action is no clip.
        if ad.action and not obj.hide_render and ad.action.name not in on_track:
            # Named for the action, as Blender 3.5 named such a clip.
            name = ad.action.name
            if name in {t.name for t in ad.nla_tracks}:
                self.block(f"{obj.name}: off-track {name!r} would need a track of that name, taken")
            else:
                track = ad.nla_tracks.new()
                track.name = name
                track.strips.new(name, int(ad.action.frame_range[0]), ad.action)
                self.change(f"{obj.name}: new track {name!r} for its off-track action")
        self.strip_ranges(obj, ad)

    def switches(self, obj, ad):
        """Key, in each clip that leaves it unkeyed, any constraint influence another clip keys."""
        from bpy_extras import anim_utils

        tracks = [t for t in ad.nla_tracks if len(t.strips) == 1 and t.strips[0].action]
        keyed = {
            t.name: {fc.data_path for fc in bt.fcurves(t.strips[0].action)} for t in tracks
        }
        switches = sorted(
            {p for paths in keyed.values() for p in paths if bt.is_influence(p)}
        )
        # Blender 3.5 exported clips in name order, and an unkeyed influence kept the value the
        # clip before it ended with; the first clip took the constraint's own value.
        ordered = sorted(tracks, key=lambda t: t.name)
        last = {}
        for path in switches:
            bone, constraint = bt.influence_target(path)
            last[path] = obj.pose.bones[bone].constraints[constraint].influence
        planned = []
        for track in ordered:
            action = track.strips[0].action
            missing = [p for p in switches if p not in keyed[track.name]]
            if missing:
                planned.append((track, {p: last[p] for p in missing}))
            end = action.frame_range[1]
            for fc in bt.fcurves(action):
                if fc.data_path in last:
                    last[fc.data_path] = fc.evaluate(end)
        for track, values in planned:
            strip = track.strips[0]
            bag = anim_utils.action_ensure_channelbag_for_slot(strip.action, strip.action_slot)
            start = strip.action.frame_range[0]
            for path, value in values.items():
                curve = bag.fcurves.new(path, index=0)
                curve.keyframe_points.insert(start, value)
            shown = ", ".join(
                f"{bt.influence_target(p)[0]} {v:.2f}" for p, v in sorted(values.items())
            )
            self.change(f"{obj.name}: clip {track.name!r} keyed unkeyed influences: {shown}")

    def armature(self, obj, ad):
        rules = STASHED.get(self.blend.name, {})
        real = [t for t in ad.nla_tracks if not bt.is_stash(t)]
        on_real = {s.action.name for t in real for s in t.strips if s.action}
        names = {t.name for t in real}
        for track in [t for t in ad.nla_tracks if bt.is_stash(t)]:
            lost = sorted({s.action.name for s in track.strips if s.action} - on_real)
            if not lost:
                self.change(f"{obj.name}: deleted {track.name!r}")
                ad.nla_tracks.remove(track)
            elif len(lost) == 1 and rules.get(lost[0]) == "keep":
                self.change(f"{obj.name}: {track.name!r} kept as track {lost[0]!r}")
                track.name = lost[0]
                names.add(lost[0])
            elif all(rules.get(name) == "delete" for name in lost):
                self.change(f"{obj.name}: deleted {track.name!r} and with it {lost}")
                ad.nla_tracks.remove(track)
            else:
                self.block(f"{obj.name}: {track.name!r} holds {lost}, on no other track")
        for track in ad.nla_tracks:
            if bt.is_default_name(track) and len(track.strips) == 1 and track.strips[0].action:
                name = clip_name(track.strips[0].action)
                if name in names:
                    self.block(f"{obj.name}: {track.name!r} would be {name!r}, already a track")
                    continue
                self.change(f"{obj.name}: renamed track {track.name!r} to {name!r}")
                track.name = name
                names.add(name)
        if ad.action and ad.action.name not in on_real:
            name = clip_name(ad.action)
            if name in names:
                self.block(f"{obj.name}: off-track {ad.action.name!r} would be {name!r}, taken")
            else:
                track = ad.nla_tracks.new()
                track.name = name
                track.strips.new(name, int(ad.action.frame_range[0]), ad.action)
                self.change(f"{obj.name}: new track {name!r} for off-track {ad.action.name!r}")
        self.switches(obj, ad)
        self.strip_ranges(obj, ad)

    def strip_ranges(self, obj, ad):
        for track in ad.nla_tracks:
            if len(track.strips) != 1 or track.strips[0].action is None:
                self.block(f"{obj.name}: track {track.name!r} is not one strip with an action")
                continue
            strip = track.strips[0]
            start, end = strip.action.frame_range
            have = (strip.action_frame_start, strip.action_frame_end)
            if end - start >= 1 and have != (start, end):
                self.change(
                    f"{obj.name}: strip {track.name!r} {have[0]:g}-{have[1]:g} "
                    f"set to its action's {start:g}-{end:g}"
                )
                strip.action_frame_start, strip.action_frame_end = start, end


def main():
    args = bt.script_args()
    apply = "--apply" in args
    preview = None
    if "--preview" in args:
        at = args.index("--preview")
        preview = Path(args[at + 1]).resolve()
        args = args[:at] + args[at + 2 :]
    if apply or preview:
        bt.require_version(5, 2)
    files = bt.blend_files([a for a in args if a != "--apply"])
    blocked = 0
    for blend in files:
        bpy.ops.wm.open_mainfile(filepath=str(blend))
        cleanup = Cleanup(blend)
        cleanup.run()
        print(f"\n{blend.relative_to(bt.ROOT)}: {len(cleanup.changes) or 'no'} changes")
        for line in cleanup.changes:
            print(f"  {line}")
        for line in cleanup.blockers:
            print(f"  BLOCKED {line}")
        if cleanup.blockers:
            blocked += 1
        elif apply and cleanup.changes:
            bpy.ops.wm.save_mainfile()
            print("  saved")
        if preview and not cleanup.blockers and bt.exported_glb(blend).exists():
            preview.mkdir(parents=True, exist_ok=True)
            bt.export(preview / bt.exported_glb(blend).name)
            print(f"  exported to {preview}")
    mode = "applied" if apply else "dry run, nothing saved"
    print(f"\n{len(files)} files, {blocked} blocked; {mode}")


main()
