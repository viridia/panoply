#!/usr/bin/env python3
"""Report the animation data and export settings of .blend files, and what is wrong with them.

    scripts/blend_inventory.py                   every .blend under artwork/characters/
    scripts/blend_inventory.py a.blend dir ...   files, or directories
    scripts/blend_inventory.py --full ...        also every track, strip and action

Runs inside Blender, headless (see blendtools.py), and changes nothing. A problem is something that
exports wrong or confuses later edits:

    stash track          Blender's [Action Stash], made on switching actions; exported as a clip
    only in a stash      an action no other track plays, lost if its stash track is deleted
    default track name   a track still named NlaTrack, so the clip is too
    off-track action     an active action on no track, which NLA export leaves out: an armature's,
                         or a mesh's that moves it (a door)
    short strip          a strip covering less than its action, so the clip is cut short
    unkeyed switch       a clip leaving unkeyed a constraint influence (an IK/FK switch) that
                         another clip keys, so it takes whatever value Blender last evaluated
    mesh action          an action or track on a mesh that does not move it (bone curves, stray
                         keys on a modifier), so exports nothing
    tweak mode           an object left in the NLA editor's tweak mode
    partial multiply     a Multiply Mix node feeding Base Color with a Factor other than 1, whose
                         color the exporter leaves out
    stray output         a material whose Principled BSDF feeds an output for one renderer only,
                         with none for all, so the exporter finds no color
    alpha-only color     a Color Attribute node whose Alpha alone feeds a Principled BSDF, which the
                         exporter writes as a second color set, so the mesh draws opaque
    missing image        an image a material reads whose file is missing, so its texture is left out
    export setting      remembered export settings missing, or differing from Panoply's
"""

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import blendtools as bt  # noqa: E402

bt.relaunch(__file__)

import bpy  # noqa: E402


def problems(blend):
    found = []
    for obj in bpy.data.objects:
        ad = obj.animation_data
        if ad is None or not bt.is_local(obj):
            continue
        if ad.use_tweak_mode:
            found.append(f"{obj.name}: tweak mode")
        if obj.type == "MESH":
            if ad.action and not bt.animates_transform(ad.action):
                found.append(f"{obj.name}: mesh action {ad.action.name!r}")
            for track in ad.nla_tracks:
                if not any(bt.animates_transform(s.action) for s in track.strips if s.action):
                    found.append(f"{obj.name}: mesh track {track.name!r}")
            on_track = {s.action.name for t in ad.nla_tracks for s in t.strips if s.action}
            moves = ad.action and bt.animates_transform(ad.action)
            if moves and not obj.hide_render and ad.action.name not in on_track:
                found.append(f"{obj.name}: off-track action {ad.action.name!r}")
            continue
        if obj.type != "ARMATURE" or obj.hide_render:
            continue
        real = [t for t in ad.nla_tracks if not bt.is_stash(t)]
        on_real = {s.action.name for t in real for s in t.strips if s.action}
        clips = {t.name: t.strips[0].action for t in real if len(t.strips) == 1 and t.strips[0].action}
        keyed = {name: {fc.data_path for fc in bt.fcurves(a)} for name, a in clips.items()}
        switches = {p for paths in keyed.values() for p in paths if bt.is_influence(p)}
        for name, paths in keyed.items():
            if switches - paths:
                found.append(f"{obj.name}: unkeyed switch in {name!r}: {sorted(switches - paths)}")
        if ad.action and ad.action.name not in on_real:
            found.append(f"{obj.name}: off-track action {ad.action.name!r}")
        for track in ad.nla_tracks:
            actions = {s.action.name for s in track.strips if s.action}
            if bt.is_stash(track):
                lost = sorted(actions - on_real)
                found.append(
                    f"{obj.name}: stash track {track.name!r}"
                    + (f", only in a stash: {lost}" if lost else "")
                )
                continue
            if bt.is_default_name(track):
                found.append(f"{obj.name}: default track name {track.name!r}")
            if len(track.strips) != 1:
                found.append(f"{obj.name}: track {track.name!r} has {len(track.strips)} strips")
                continue
            strip = track.strips[0]
            if strip.action is None:
                found.append(f"{obj.name}: track {track.name!r} plays no action")
                continue
            start, end = strip.action.frame_range
            if end - start >= 1 and (strip.action_frame_start, strip.action_frame_end) != (
                start,
                end,
            ):
                found.append(
                    f"{obj.name}: short strip {track.name!r} "
                    f"{strip.action_frame_start:g}-{strip.action_frame_end:g} "
                    f"of {start:g}-{end:g}"
                )
    for material in bpy.data.materials:
        for node in bt.partial_multiplies(material):
            found.append(
                f"material {material.name}: partial multiply {node.name!r} "
                f"(Factor {node.inputs['Factor'].default_value:g})"
            )
        output = bt.stray_output(material)
        if output is not None:
            found.append(
                f"material {material.name}: stray output, {output.name!r} targets only "
                f"{output.target}"
            )
        for node, _ in bt.alpha_only_colors(material):
            found.append(f"material {material.name}: alpha-only color {node.name!r}")
        for image in bt.missing_images(material):
            found.append(f"material {material.name}: missing image {image.filepath!r}")
    if bt.exported_glb(blend).exists():
        stored = bpy.context.scene.get(bt.SETTINGS_KEY)
        found += bt.settings_drift(stored, bt.expected_settings(bpy.data.objects))
    return found


def full(blend):
    for obj in bpy.data.objects:
        ad = obj.animation_data
        if ad is None:
            continue
        origin = "" if bt.is_local(obj) else " (from a library)"
        render = "" if not obj.hide_render else ", not rendered"
        print(f"  {obj.name} ({obj.type}{render}){origin}")
        print(f"    active action: {ad.action.name if ad.action else None}")
        for track in ad.nla_tracks:
            muted = " muted" if track.mute else ""
            print(f"    track {track.name!r}{muted}")
            for strip in track.strips:
                action = strip.action.name if strip.action else None
                print(
                    f"      strip {strip.name!r} plays {action!r}, "
                    f"frames {strip.action_frame_start:g}-{strip.action_frame_end:g}"
                )
    print("  actions:")
    for action in bpy.data.actions:
        if action.library:
            continue
        start, end = action.frame_range
        fake = ", fake user" if action.use_fake_user else ""
        print(f"    {action.name!r}: {action.users} users{fake}, frames {start:g}-{end:g}")


def main():
    args = bt.script_args()
    show_full = "--full" in args
    files = bt.blend_files([a for a in args if a != "--full"])
    total = 0
    for blend in files:
        bpy.ops.wm.open_mainfile(filepath=str(blend))
        found = problems(blend)
        total += len(found)
        print(f"\n{blend.relative_to(bt.ROOT)}: {len(found) or 'no'} problems")
        for line in found:
            print(f"  {line}")
        if show_full:
            full(blend)
    print(f"\n{len(files)} files, {total} problems")


main()
