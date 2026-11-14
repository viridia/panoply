#!/usr/bin/env python3
"""Check that the paths assets name resolve.

    scripts/asset_xref.py           every check; exit 1 if any fails
    scripts/asset_xref.py --quiet   failures only, no summary

A precinct (`assets/scenery/precincts/<realm>/*.msgpack`) names floor surfaces, which must be files
under `assets/`, and exemplars (`file#Label`), whose catalog must exist and hold the label or an
alias of it. An exemplar catalog (`assets/exemplars/**/*.exem.json`) names, in its aspects, models as
`file.glb#Name`: the file must exist and hold a scene of that name (`SceneryModels`, `Skin`,
`ItemModel`). A `SceneryAnimations` transition's sound names a file. An `extends` must resolve as a
precinct's exemplar does. An exemplar with an `Armature` and a `Skin`, its own or inherited, needs
the armature in the Skin's file: the one `Armature.node` names, or else the file's only one; and
its clips file, `Armature.clips` or else the Skin's file, must hold clips. An exemplar's own
`Features` may turn on only the Skin's file's feature scenes (those named after a node whose extras
set `feature`), and its own `Colors` name only color slots (materials whose extras set `color_slot`)
that the Skin's scene or a feature scene uses.

A missing reference shows in the game only as an error when something loads it, so a break can sit
unseen until someone visits the precinct. Known breaks are listed in KNOWN against the ledger entry
that owns them: the check passes while they stay broken, fails on any other, and fails when a listed
one resolves, so that it is removed from the list.
"""

import argparse
import json
import struct
import sys
from collections import defaultdict
from pathlib import Path, PurePosixPath

from outlines import read_gltf

ROOT = Path(__file__).resolve().parent.parent
ASSETS = ROOT / "assets"
PRECINCTS = ASSETS / "scenery" / "precincts"
EXEMPLARS = ASSETS / "exemplars"

# Broken references the ledgers own, as {reference: ledger id}.
KNOWN = {}


def unpack(data, i=0):
    """The MessagePack value at `data[i]`, and the index after it."""
    t = data[i]
    i += 1
    if t <= 0x7F:
        return t, i
    if t >= 0xE0:
        return t - 0x100, i
    if 0x80 <= t <= 0x8F:
        return unpack_map(data, i, t & 0x0F)
    if 0x90 <= t <= 0x9F:
        return unpack_array(data, i, t & 0x0F)
    if 0xA0 <= t <= 0xBF:
        n = t & 0x1F
        return data[i : i + n].decode(), i + n
    if t in CONSTANTS:
        return CONSTANTS[t], i
    if t in SCALARS:
        fmt = SCALARS[t]
        return struct.unpack_from(fmt, data, i)[0], i + struct.calcsize(fmt)
    if t in SIZED:
        fmt, kind = SIZED[t]
        n = struct.unpack_from(fmt, data, i)[0]
        i += struct.calcsize(fmt)
        if kind == "str":
            return data[i : i + n].decode(), i + n
        if kind == "bin":
            return data[i : i + n], i + n
        if kind == "array":
            return unpack_array(data, i, n)
        return unpack_map(data, i, n)
    raise ValueError(f"unsupported MessagePack type 0x{t:02x} at {i - 1}")


CONSTANTS = {0xC0: None, 0xC2: False, 0xC3: True}
SCALARS = {
    0xCA: ">f", 0xCB: ">d",
    0xCC: ">B", 0xCD: ">H", 0xCE: ">I", 0xCF: ">Q",
    0xD0: ">b", 0xD1: ">h", 0xD2: ">i", 0xD3: ">q",
}  # fmt: skip
SIZED = {
    0xD9: (">B", "str"), 0xDA: (">H", "str"), 0xDB: (">I", "str"),
    0xC4: (">B", "bin"), 0xC5: (">H", "bin"), 0xC6: (">I", "bin"),
    0xDC: (">H", "array"), 0xDD: (">I", "array"),
    0xDE: (">H", "map"), 0xDF: (">I", "map"),
}  # fmt: skip


def unpack_array(data, i, n):
    out = []
    for _ in range(n):
        value, i = unpack(data, i)
        out.append(value)
    return out, i


def unpack_map(data, i, n):
    out = {}
    for _ in range(n):
        key, i = unpack(data, i)
        out[key], i = unpack(data, i)
    return out, i


def asset_name(path):
    return path.relative_to(ASSETS).as_posix()


class Assets:
    """Reads each catalog and model once, remembering why one could not be read."""

    def __init__(self):
        self.catalogs = {}
        self.models = {}
        self.slot_files = {}

    def catalog(self, path):
        """The entries of the catalog at `path` by label and alias, or a reason it has none."""
        if path not in self.catalogs:
            file = ASSETS / path
            if not file.is_file():
                self.catalogs[path] = "file not found"
            else:
                try:
                    entries = json.loads(file.read_text())
                    labels = dict(entries)
                    for entry in entries.values():
                        labels.update((alias, entry) for alias in entry.get("alias", []))
                    self.catalogs[path] = labels
                except (ValueError, AttributeError) as e:
                    self.catalogs[path] = f"unreadable: {e}"
        return self.catalogs[path]

    def model(self, path):
        """The scene names, node names, armature names and clip count of the `.glb` at `path`, or
        a reason it has none. An armature is a node, not a bone itself, holding a skin's root
        bone."""
        if path not in self.models:
            file = ASSETS / path
            if not file.is_file():
                self.models[path] = "file not found"
            else:
                try:
                    gltf = read_gltf(file)
                    names = lambda key: {x["name"] for x in gltf.get(key, []) if "name" in x}
                    nodes = gltf.get("nodes", [])
                    bones = {j for skin in gltf.get("skins", []) for j in skin["joints"]}
                    armatures = {
                        node.get("name")
                        for i, node in enumerate(nodes)
                        if i not in bones and bones & set(node.get("children", []))
                    }
                    clips = len(gltf.get("animations", []))
                    self.models[path] = (names("scenes"), names("nodes"), armatures, clips)
                except ValueError as e:
                    self.models[path] = f"unreadable: {e}"
        return self.models[path]

    def resolve(self, reference, base=""):
        """The catalog and entry the exemplar `reference` names, or a reason it does not
        resolve. `base` is the catalog that names it, against which a reference with no file
        (`#Label`) or a relative one resolves."""
        file, _, label = reference.partition("#")
        if not label:
            return "no #label"
        if not file:
            file = base
        elif base:
            file = str(PurePosixPath(base).parent / file)
        labels = self.catalog(file)
        if isinstance(labels, str):
            return f"{file}: {labels}"
        if label not in labels:
            return f"no exemplar {label} in {file}"
        return file, labels[label]

    def exemplar_problem(self, reference, base=""):
        """Why the exemplar `reference` does not resolve, or `None`."""
        found = self.resolve(reference, base)
        return found if isinstance(found, str) else None

    def aspect(self, entry, source, name):
        """The aspect `name` of `entry`, in the catalog `source`, its own or the nearest one it
        inherits through `extends`, whole, as the game resolves it; `None` if it has none."""
        seen = set()
        while name not in entry.get("aspects", {}):
            if "extends" not in entry or id(entry) in seen:
                return None
            seen.add(id(entry))
            found = self.resolve(entry["extends"], source)
            if isinstance(found, str):
                return None
            source, entry = found
        return entry["aspects"][name]

    def armature_problem(self, skin_file, node, clips_file):
        """Why an `Armature` of `node` (or `None`, for the file's only one) and `clips_file` does
        not resolve against a Skin in `skin_file`, or `None`."""
        found = self.model(skin_file)
        if isinstance(found, str):
            return f"{skin_file}: {found}"
        armatures = found[2]
        if node is None and len(armatures) != 1:
            return f"no Armature.node, and {skin_file} has armatures {sorted(armatures)}"
        if node is not None and node not in armatures:
            return f"no armature {node} in {skin_file}"
        clips = self.model(clips_file)
        if isinstance(clips, str):
            return f"{clips_file}: {clips}"
        if clips[3] == 0:
            return f"no clips in {clips_file}"
        return None

    def slots(self, path):
        """The feature scenes of the character file at `path`, and the color slot materials each of
        its scenes uses, by scene, or a reason it has none. A feature is a scene named after a node
        in it whose extras set `feature`; a color slot, a material whose extras set `color_slot`."""
        if path not in self.slot_files:
            file = ASSETS / path
            try:
                gltf = read_gltf(file)
            except (OSError, ValueError) as e:
                self.slot_files[path] = f"unreadable: {e}"
                return self.slot_files[path]
            nodes, meshes = gltf.get("nodes", []), gltf.get("meshes", [])
            materials = gltf.get("materials", [])
            features, slots = set(), {}
            for scene in gltf.get("scenes", []):
                stack, used = list(scene.get("nodes", [])), set()
                while stack:
                    node = nodes[stack.pop()]
                    stack += node.get("children", [])
                    # The scene made from a feature's object, not the file's own, which holds all.
                    marked = (node.get("extras") or {}).get("feature")
                    if marked and node.get("name") == scene.get("name"):
                        features.add(scene.get("name"))
                    for primitive in meshes[node["mesh"]]["primitives"] if "mesh" in node else []:
                        material = materials[primitive["material"]] if "material" in primitive else {}
                        if (material.get("extras") or {}).get("color_slot"):
                            used.add(material.get("name"))
                slots[scene.get("name")] = used
            self.slot_files[path] = (features, slots)
        return self.slot_files[path]

    def file_problem(self, path):
        return None if (ASSETS / path).is_file() else "file not found"

    def model_problem(self, reference, kind):
        """Why `file.glb#Name` does not resolve to a scene or node (`kind`), or `None`."""
        file, _, name = reference.partition("#")
        found = self.model(file)
        if isinstance(found, str):
            return f"{file}: {found}"
        if kind is None:
            return None
        if not name:
            return f"no #{kind}"
        scenes, nodes, _, _ = found
        if name not in (scenes if kind == "scene" else nodes):
            return f"no {kind} {name} in {file}"
        return None


def check_precincts(assets, report):
    for file in sorted(PRECINCTS.glob("*/*.msgpack")):
        source = asset_name(file)
        precinct, _ = unpack(file.read_bytes())
        for path in precinct.get("floor_surfaces", []):
            report(source, path, assets.file_problem(path))
        exemplars = precinct.get("scenery_types", []) + precinct.get("terrain_fx_types", [])
        exemplars += [actor["exemplar"] for actor in precinct.get("actors", [])]
        for reference in exemplars:
            report(source, reference, assets.exemplar_problem(reference))


# The aspects that name models, as (aspect, field or None for the aspect's own value, what the
# name after `#` must be in the file: "scene", "node", or None for the file alone).
MODEL_FIELDS = [
    ("SceneryModels", "asset", "scene"),
    ("Skin", None, "scene"),
    ("ItemModel", None, "scene"),
]


def check_exemplars(assets, report):
    for file in sorted(EXEMPLARS.glob("**/*.exem.json")):
        source = asset_name(file)
        for label, entry in json.loads(file.read_text()).items():
            where = f"{source}#{label}"
            if "extends" in entry:
                reference = entry["extends"]
                report(where, reference, assets.exemplar_problem(reference, source))
            aspects = entry.get("aspects", {})
            for aspect, field, kind in MODEL_FIELDS:
                if aspect not in aspects:
                    continue
                values = aspects[aspect]
                values = values if isinstance(values, list) else [values]
                for value in values:
                    reference = value if field is None else value.get(field)
                    if reference is not None:
                        report(where, reference, assets.model_problem(reference, kind))
            for animation in aspects.get("SceneryAnimations", []):
                for transition in animation.get("transitions", []):
                    if "sound" in transition:
                        reference = transition["sound"]["asset"]
                        report(where, reference, assets.file_problem(reference))
            armature = assets.aspect(entry, source, "Armature")
            skin = assets.aspect(entry, source, "Skin")
            if armature is not None and skin is not None:
                skin_file = skin.partition("#")[0]
                node = armature.get("node")
                clips_file = armature.get("clips", skin_file)
                reference = f"{skin_file}#{node or '(its armature)'}, clips {clips_file}"
                report(where, reference, assets.armature_problem(skin_file, node, clips_file))
            if skin is not None and ("Features" in aspects or "Colors" in aspects):
                check_slots(assets, report, where, skin, aspects)


def check_slots(assets, report, where, skin, aspects):
    """Each feature an exemplar's own `Features` turns on must be a feature scene of its Skin's
    file, and each of its own `Colors` a color slot of the Skin's scene or of a feature scene."""
    skin_file, _, body = skin.partition("#")
    found = assets.slots(skin_file)
    if isinstance(found, str):
        return  # The Skin's own check reports the file.
    features, slots = found
    for feature, enabled in aspects.get("Features", {}).items():
        if enabled:
            problem = None if feature in features else f"no feature scene {feature} in {skin_file}"
            report(where, f"Features.{feature}", problem)
    colorable = set(slots.get(body, set())).union(*(slots[name] for name in features))
    for color in aspects.get("Colors", {}):
        problem = None if color in colorable else f"no color slot {color} in {skin} or its features"
        report(where, f"Colors.{color}", problem)


def main():
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--quiet", action="store_true", help="failures only, no summary")
    args = parser.parse_args()

    assets = Assets()
    broken = defaultdict(dict)  # (reference, problem): sources, in order, without repeats
    seen = set()
    count = 0

    def report(source, reference, problem):
        nonlocal count
        count += 1
        seen.add(reference)
        if problem is not None:
            broken[(reference, problem)][source] = None

    check_precincts(assets, report)
    check_exemplars(assets, report)

    failures = []
    for (reference, problem), sources in sorted(broken.items()):
        if reference in KNOWN:
            continue
        sources = list(sources)
        shown = ", ".join(sources[:3]) + (f" and {len(sources) - 3} more" if len(sources) > 3 else "")
        failures.append(f"{reference}: {problem}; named by {shown}")
    broken_references = {reference for reference, _ in broken}
    for reference, owner in sorted(KNOWN.items()):
        if reference not in seen:
            failures.append(f"{reference}: listed against {owner}, but nothing names it now")
        elif reference not in broken_references:
            failures.append(f"{reference}: listed against {owner}, but it resolves now")

    for failure in failures:
        print(failure)
    if not args.quiet:
        known = len(broken_references & KNOWN.keys())
        print(f"{count} references checked, {known} known breaks, {len(failures)} failures")
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
