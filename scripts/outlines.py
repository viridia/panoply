#!/usr/bin/env python3
"""List the meshes in .glb files that Panoply draws without an outline.

    scripts/outlines.py                    every .glb under assets/
    scripts/outlines.py assets/characters  a directory, or files
    scripts/outlines.py --none ...         only meshes nothing outlines

Panoply outlines a mesh whose material's extras set `outline` to a number. faery also took it from
the node's extras, then the parent node's (an object's custom properties in Blender), before the
material's. Each line is one node and material, marked:

    node   outlined by its node or parent node: in faery, but not yet in Panoply
    skin   a skinned mesh outlined by nothing, which faery outlined by default
    none   outlined by nothing

An `outline` that is not a number opts the mesh out, and is not listed. Materials that are meant to
have no outline (eyes, leaves with outlines baked into their texture) are listed too: the script
cannot tell them apart.
"""

import json
import struct
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
GLB_MAGIC = b"glTF"
JSON_CHUNK = 0x4E4F534A


def read_gltf(path):
    """The JSON part of a .glb."""
    data = path.read_bytes()
    if data[:4] != GLB_MAGIC:
        raise ValueError("not a .glb")
    length, kind = struct.unpack_from("<II", data, 12)
    if kind != JSON_CHUNK:
        raise ValueError("first chunk is not JSON")
    return json.loads(data[20 : 20 + length])


def outline_of(extras):
    """`None` if the extras do not mention an outline, else whether they set one."""
    if not isinstance(extras, dict) or "outline" not in extras:
        return None
    value = extras["outline"]
    return isinstance(value, (int, float)) and not isinstance(value, bool)


def unoutlined(doc):
    """(mark, node name, material name) for each mesh Panoply draws without an outline."""
    nodes = doc.get("nodes", [])
    materials = doc.get("materials", [])
    parents = {child: i for i, node in enumerate(nodes) for child in node.get("children", [])}
    found = []
    for i, node in enumerate(nodes):
        if "mesh" not in node:
            continue
        by_node = outline_of(node.get("extras"))
        if by_node is None and i in parents:
            by_node = outline_of(nodes[parents[i]].get("extras"))
        for primitive in doc["meshes"][node["mesh"]]["primitives"]:
            material = materials[primitive["material"]] if "material" in primitive else {}
            by_material = outline_of(material.get("extras"))
            if by_material:
                continue
            if by_node is False or (by_node is None and by_material is False):
                continue
            mark = "node" if by_node else "skin" if "skin" in node else "none"
            entry = (mark, node.get("name", f"#{i}"), material.get("name", "(default)"))
            if entry not in found:
                found.append(entry)
    return found


def main():
    args = sys.argv[1:]
    if args and args[0] in ("-h", "--help"):
        print(__doc__.strip())
        return 0
    only_none = "--none" in args
    paths = [Path(arg) for arg in args if arg != "--none"] or [ROOT / "assets"]
    files = []
    for path in paths:
        if path.is_dir():
            files.extend(sorted(path.rglob("*.glb")))
        elif path.exists():
            files.append(path)
        else:
            print(f"{path}: no such file or directory", file=sys.stderr)
            return 1
    counts = {"node": 0, "skin": 0, "none": 0}
    for file in files:
        try:
            entries = unoutlined(read_gltf(file))
        except (ValueError, KeyError, json.JSONDecodeError) as error:
            print(f"{file}: {error}", file=sys.stderr)
            continue
        if only_none:
            entries = [entry for entry in entries if entry[0] == "none"]
        if not entries:
            continue
        shown = file.resolve()
        print(shown.relative_to(ROOT) if shown.is_relative_to(ROOT) else file)
        for mark, node, material in entries:
            counts[mark] += 1
            print(f"  {mark:5} {node:32} {material}")
    print(
        f"{counts['node']} outlined only by their node, {counts['skin']} only by faery's default "
        f"for skinned meshes, {counts['none']} by nothing"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
