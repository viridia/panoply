"""Shared code for the scripts that run inside Blender (`blend_*.py`).

Each such script is started like any other script in `scripts/`; `relaunch` then runs it again
inside Blender, headless, with the same arguments. Blender is `$BLENDER` if set, else `blender` on
`PATH`, else the macOS application.
"""

import os
import shutil
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
CHARACTERS = ROOT / "artwork" / "characters"
MAC_BLENDER = "/Applications/Blender.app/Contents/MacOS/Blender"

# The scene property in which the glTF exporter keeps its settings when "Remember Export Settings"
# is ticked; File > Export > glTF 2.0 loads them back.
SETTINGS_KEY = "glTF2ExportSettings"

# Panoply's glTF export settings. Every object that renders is exported, whether or not it is
# hidden in the viewport; objects with rendering off (foot guides, bone shapes, cameras) are not.
# Modifiers are applied, for Mirror. Each clip is an NLA track, named by the track.
EXPORT_SETTINGS = {
    "export_format": "GLB",
    "use_selection": False,
    "use_visible": False,
    "use_renderable": True,
    "use_active_collection": False,
    "use_active_scene": False,
    "export_yup": True,
    "export_apply": True,
    "export_extras": True,
    "export_texcoords": True,
    "export_normals": True,
    "export_tangents": False,
    "export_materials": "EXPORT",
    "export_vertex_color": "ACTIVE",
    "export_cameras": False,
    "export_lights": False,
    "export_skins": True,
    "export_morph": True,
    "export_draco_mesh_compression_enable": False,
    "export_animation_mode": "NLA_TRACKS",
    "export_frame_range": False,
    "export_force_sampling": True,
    "export_optimize_animation_size": True,
    "export_def_bones": False,
}


def relaunch(script):
    """Run `script` inside Blender with this process's arguments, unless already inside it."""
    try:
        import bpy  # noqa: F401
    except ImportError:
        blender = os.environ.get("BLENDER") or shutil.which("blender") or MAC_BLENDER
        args = [blender, "--background", "--factory-startup", "--python-exit-code", "1"]
        args += ["--python", str(Path(script).resolve()), "--", *sys.argv[1:]]
        os.execv(blender, args)


def script_args():
    """The arguments after Blender's `--`."""
    return sys.argv[sys.argv.index("--") + 1 :] if "--" in sys.argv else []


def blend_files(args):
    """The .blend files named, directories expanded; every character file if none. A name not
    found from the working directory is looked for in artwork/characters/."""
    paths = [Path(a) for a in args] or [CHARACTERS]
    files = []
    for path in paths:
        if not path.is_absolute():
            path = Path.cwd() / path if (Path.cwd() / path).exists() else CHARACTERS / path
        files += sorted(path.rglob("*.blend")) if path.is_dir() else [path]
    return files


def exported_glb(blend):
    """The .glb a .blend exports to, beside it with the same name."""
    return blend.with_suffix(".glb")


def require_version(major, minor):
    import bpy

    if bpy.app.version < (major, minor):
        sys.exit(f"needs Blender {major}.{minor} or later, not {bpy.app.version_string}")


def is_local(obj):
    """Whether the object belongs to this file, not linked or overridden from a library."""
    return obj.library is None and obj.override_library is None


def fcurves(action):
    """Every F-curve of an action, in all its layers, strips and slots (Blender 4.4 on)."""
    if not hasattr(action, "layers"):
        return list(action.fcurves)
    return [
        fc
        for layer in action.layers
        for strip in layer.strips
        for bag in strip.channelbags
        for fc in bag.fcurves
    ]


TRANSFORM_PATHS = {
    "location",
    "rotation_euler",
    "rotation_quaternion",
    "rotation_axis_angle",
    "scale",
}


def animates_transform(action):
    """Whether the action moves an object itself, as a door's opening does: the only animation of a
    mesh that glTF exports. Bone curves, and keys on modifiers or other properties, it leaves out."""
    if action.library and action.is_missing:
        return False
    return any(fc.data_path in TRANSFORM_PATHS for fc in fcurves(action))


def bones_only(action):
    """Whether every curve of the action animates a bone: on a mesh, such an action does nothing.
    So does an action linked from a library that no longer has it."""
    if action.library and action.is_missing:
        return True
    curves = fcurves(action)
    return bool(curves) and all(fc.data_path.startswith("pose.bones[") for fc in curves)


def is_influence(data_path):
    """Whether an F-curve keys a bone constraint's influence, as an IK/FK switch does."""
    return data_path.startswith("pose.bones[") and ".constraints[" in data_path and (
        data_path.endswith(".influence")
    )


def partial_multiplies(material):
    """Multiply Mix nodes feeding a Principled BSDF's Base Color with a Factor other than 1.

    Blender 3.5's exporter read such a node as color times texture whatever its Factor; 5.2's
    reads it only at a Factor of 1, and otherwise exports neither."""
    if material.library or not material.node_tree or material.is_grease_pencil:
        return []
    found = []
    for node in material.node_tree.nodes:
        if node.bl_idname != "ShaderNodeMix" or node.data_type != "RGBA":
            continue
        if node.blend_type != "MULTIPLY":
            continue
        factor = node.inputs["Factor"]
        if factor.is_linked or factor.default_value == 1:
            continue
        feeds_base = any(
            link.to_node.bl_idname == "ShaderNodeBsdfPrincipled"
            and link.to_socket.name == "Base Color"
            for output in node.outputs
            for link in output.links
        )
        if feeds_base:
            found.append(node)
    return found


def stray_output(material):
    """The Material Output a Principled BSDF feeds, when it targets one renderer and no output
    targets all of them.

    Blender 5.2's exporter then finds no color, where 3.5's read the Principled BSDF."""
    if material.library or not material.node_tree or material.is_grease_pencil:
        return None
    outputs = [n for n in material.node_tree.nodes if n.bl_idname == "ShaderNodeOutputMaterial"]
    if any(o.target == "ALL" for o in outputs):
        return None
    fed = [
        o
        for o in outputs
        if any(l.from_node.bl_idname == "ShaderNodeBsdfPrincipled" for l in o.inputs["Surface"].links)
    ]
    return fed[0] if len(fed) == 1 else None


def alpha_only_colors(material):
    """(Color Attribute node, Principled BSDF) pairs where the node feeds the BSDF's Alpha and
    nothing reads its Color.

    Blender 5.2's exporter does not handle that case: it writes a white COLOR_0 and the attribute
    as COLOR_1, which engines ignore, so the mesh draws opaque. Blender 3.5 wrote the attribute as
    COLOR_0."""
    if material.library or not material.node_tree or material.is_grease_pencil:
        return []
    found = []
    for node in material.node_tree.nodes:
        if node.bl_idname != "ShaderNodeVertexColor" or node.outputs["Color"].links:
            continue
        for link in node.outputs["Alpha"].links:
            if link.to_node.bl_idname == "ShaderNodeBsdfPrincipled" and link.to_socket.name == "Alpha":
                found.append((node, link.to_node))
    return found


def missing_images(material):
    """The images a material's Image Texture nodes read whose files are missing and not packed.

    The exporter leaves such a texture out, so the material exports without it."""
    import bpy

    if material.library or not material.node_tree or material.is_grease_pencil:
        return []
    found = []
    for node in material.node_tree.nodes:
        image = node.image if node.bl_idname == "ShaderNodeTexImage" else None
        if image is None or image.source != "FILE" or image.packed_file or image in found:
            continue
        if not Path(bpy.path.abspath(image.filepath)).exists():
            found.append(image)
    return found


def influence_target(data_path):
    """The bone and constraint names in `pose.bones["b"].constraints["c"].influence`."""
    parts = data_path.split('"')
    return parts[1], parts[3]


STASH = "[Action Stash]"


def is_stash(track):
    """Blender's own track, made when the Action Editor switches an object to another action."""
    return track.name.startswith(STASH)


def is_default_name(track):
    """A track still named as Blender made it."""
    return track.name == "NlaTrack" or track.name.startswith("NlaTrack.")


def animates(blend_objects):
    """Whether a file's export carries clips: a local object that renders has tracks, an armature
    (a character) or a mesh (a door)."""
    return any(
        obj.type in ("ARMATURE", "MESH")
        and is_local(obj)
        and not obj.hide_render
        and obj.animation_data
        and any(not is_stash(t) for t in obj.animation_data.nla_tracks)
        for obj in blend_objects
    )


def expected_settings(blend_objects):
    """Panoply's export settings for one file."""
    return EXPORT_SETTINGS | {"export_animations": animates(blend_objects)}


def export(out):
    """Export the open file to `out` with its remembered settings, as File > Export would."""
    import bpy

    stored = bpy.context.scene.get(SETTINGS_KEY)
    if stored is None:
        sys.exit(f"{bpy.data.filepath}: no remembered export settings; run blend_cleanup.py")
    result = bpy.ops.export_scene.gltf(filepath=str(out), **dict(stored))
    if result != {"FINISHED"}:
        sys.exit(f"{bpy.data.filepath}: export failed: {result}")


def settings_drift(stored, expected):
    """Lines naming the remembered export settings that are missing or differ from Panoply's."""
    stored = dict(stored) if stored is not None else {}
    unset = [key for key in expected if key not in stored]
    lines = [
        f"export setting {key} is {stored[key]!r}, not {want!r}"
        for key, want in expected.items()
        if key in stored and stored[key] != want
    ]
    if unset:
        lines.append(f"export settings not remembered: {len(unset)} of {len(expected)}")
    return lines
