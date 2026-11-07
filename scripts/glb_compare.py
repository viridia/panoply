#!/usr/bin/env python3
"""Compare .glb files: what a re-export changed.

    scripts/glb_compare.py FILE ...           each file against its version at git HEAD
    scripts/glb_compare.py --pair OLD NEW     two files, or two directories (paired by file name)

Reports, for each pair: clips added or removed; for clips in both, a change of length and the
largest difference of any bone's pose, sampled 120 times a second; channels listed twice; nodes
added or removed; mesh primitives whose vertex or index counts or attributes changed; materials and
extras that changed. Unchanged things are not listed. A pose difference above 0.0001 is marked.
"""

import bisect
import json
import struct
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SAMPLE_RATE = 120
TOLERANCE = 1e-4
COMPONENTS = {"SCALAR": 1, "VEC2": 2, "VEC3": 3, "VEC4": 4}


class Glb:
    def __init__(self, data, label):
        self.label = label
        self.size = len(data)
        length, _ = struct.unpack_from("<II", data, 12)
        self.doc = json.loads(data[20 : 20 + length])
        rest = 20 + length
        self.binary = b""
        if rest < len(data):
            blength, _ = struct.unpack_from("<II", data, rest)
            self.binary = data[rest + 8 : rest + 8 + blength]

    def list(self, key):
        return self.doc.get(key, [])

    def accessor(self, index):
        acc = self.doc["accessors"][index]
        view = self.doc["bufferViews"][acc["bufferView"]]
        n = COMPONENTS[acc["type"]]
        offset = view.get("byteOffset", 0) + acc.get("byteOffset", 0)
        stride = view.get("byteStride", 4 * n)
        if acc["componentType"] != 5126:
            raise ValueError(f"{self.label}: animation accessor {index} is not float")
        return [struct.unpack_from(f"<{n}f", self.binary, offset + k * stride) for k in range(acc["count"])]

    def node_name(self, index):
        return self.doc["nodes"][index].get("name", f"#{index}")

    def clips(self):
        return {a.get("name", f"#{i}"): a for i, a in enumerate(self.list("animations"))}

    def channels(self, anim):
        """(node, path) -> list of (times, values, interpolation); a list, to catch doubles."""
        result = {}
        for channel in anim["channels"]:
            sampler = anim["samplers"][channel["sampler"]]
            key = (self.node_name(channel["target"]["node"]), channel["target"]["path"])
            times = [t[0] for t in self.accessor(sampler["input"])]
            values = self.accessor(sampler["output"])
            interp = sampler.get("interpolation", "LINEAR")
            result.setdefault(key, []).append((times, values, interp))
        return result


def sample(track, time):
    times, values, interp = track
    if time <= times[0] or len(times) == 1:
        return values[0]
    if time >= times[-1]:
        return values[-1]
    k = bisect.bisect_right(times, time) - 1
    if interp == "STEP":
        return values[k]
    if interp == "CUBICSPLINE":
        raise ValueError("cubic spline sampling is not supported")
    f = (time - times[k]) / (times[k + 1] - times[k])
    return tuple(a + (b - a) * f for a, b in zip(values[k], values[k + 1]))


def difference(path, a, b):
    d = max(abs(x - y) for x, y in zip(a, b))
    if path == "rotation":  # q and -q are the same rotation
        d = min(d, max(abs(x + y) for x, y in zip(a, b)))
    return d


def duration(channels):
    return max((tracks[0][0][-1] for tracks in channels.values()), default=0.0)


def compare_clip(old, new, old_anim, new_anim):
    """Lines describing how one clip differs, or none."""
    oc, nc = old.channels(old_anim), new.channels(new_anim)
    lines = []
    doubled = sum(1 for tracks in nc.values() if len(tracks) > 1)
    if doubled:
        lines.append(f"{doubled} channels listed twice")
    missing = sorted(set(oc) - set(nc))
    added = sorted(set(nc) - set(oc))
    if missing:
        lines.append(f"{len(missing)} channels gone, as {missing[0]}")
    if added:
        lines.append(f"{len(added)} channels new, as {added[0]}")
    od, nd = duration(oc), duration(nc)
    if abs(od - nd) > 1e-6:
        lines.append(f"length {od * 60:.1f} -> {nd * 60:.1f} frames")
    end = max(od, nd)
    worst, where = 0.0, None
    for key in set(oc) & set(nc):
        for i in range(int(end * SAMPLE_RATE) + 1):
            t = i / SAMPLE_RATE
            d = difference(key[1], sample(oc[key][0], t), sample(nc[key][0], t))
            if d > worst:
                worst, where = d, (key, t)
    if worst > TOLERANCE:
        (node, path), t = where
        lines.append(f"pose differs by {worst:.4f} ({node} {path} at {t * 60:.1f} frames) <<<")
    return lines


def primitives(glb):
    result = {}
    for mesh in glb.list("meshes"):
        for i, prim in enumerate(mesh["primitives"]):
            material = prim.get("material")
            mname = glb.doc["materials"][material].get("name") if material is not None else None
            acc = glb.doc["accessors"]
            result[(mesh.get("name"), i, mname)] = (
                acc[prim["attributes"]["POSITION"]]["count"],
                acc[prim["indices"]]["count"] if "indices" in prim else None,
                tuple(sorted(prim["attributes"])),
                len(prim.get("targets", [])),
            )
    return result


def named(glb, key):
    return {item.get("name", f"#{i}"): item for i, item in enumerate(glb.list(key))}


def compare(old, new):
    lines = []
    og, ng = old.doc["asset"].get("generator"), new.doc["asset"].get("generator")
    if og != ng:
        lines.append(f"generator {og} -> {ng}")
    if old.size != new.size:
        lines.append(f"size {old.size:,} -> {new.size:,} bytes")

    on, nn = named(old, "nodes"), named(new, "nodes")
    for label, names in (("nodes gone", set(on) - set(nn)), ("nodes new", set(nn) - set(on))):
        if names:
            lines.append(f"{label}: {sorted(names)}")
    for name in sorted(set(on) & set(nn)):
        if on[name].get("extras") != nn[name].get("extras"):
            lines.append(f"node {name} extras {on[name].get('extras')} -> {nn[name].get('extras')}")

    op, np_ = primitives(old), primitives(new)
    for key in sorted(set(op) | set(np_), key=str):
        if op.get(key) != np_.get(key):
            lines.append(f"mesh {key[0]} primitive {key[1]} ({key[2]}): {op.get(key)} -> {np_.get(key)}")

    om, nm = named(old, "materials"), named(new, "materials")
    for name in sorted(set(om) | set(nm)):
        if om.get(name) != nm.get(name):
            if name not in nm or name not in om:
                lines.append(f"material {name} {'gone' if name not in nm else 'new'}")
            else:
                keys = sorted(k for k in set(om[name]) | set(nm[name]) if om[name].get(k) != nm[name].get(k))
                lines.append(f"material {name} changed: {keys}")

    oclips, nclips = old.clips(), new.clips()
    gone, new_clips = sorted(set(oclips) - set(nclips)), sorted(set(nclips) - set(oclips))
    if gone:
        lines.append(f"clips gone: {gone}")
    if new_clips:
        lines.append(f"clips new: {new_clips}")
    same = 0
    for name in sorted(set(oclips) & set(nclips)):
        found = compare_clip(old, new, oclips[name], nclips[name])
        if found:
            lines.append(f"clip {name}: {'; '.join(found)}")
        else:
            same += 1
    if oclips or nclips:
        lines.append(f"{same} of {len(nclips)} clips unchanged")
    return lines


def at_head(path):
    rel = path.resolve().relative_to(ROOT)
    shown = subprocess.run(
        ["git", "-C", str(ROOT), "show", f"HEAD:{rel}"], capture_output=True, check=False
    )
    if shown.returncode != 0:
        return None
    return Glb(shown.stdout, f"HEAD:{rel}")


def report(label, old, new):
    print(f"\n{label}")
    if old is None:
        print("  not at HEAD")
        return
    for line in compare(old, new) or ["no change"]:
        print(f"  {line}")


def main():
    args = sys.argv[1:]
    if args[:1] == ["--pair"]:
        if len(args) != 3:
            sys.exit(__doc__)
        old, new = Path(args[1]), Path(args[2])
        if old.is_dir() and new.is_dir():
            for new_path in sorted(new.glob("*.glb")):
                old_path = old / new_path.name
                if not old_path.exists():
                    print(f"\n{new_path.name}\n  no counterpart in {old}")
                    continue
                report(new_path.name, Glb(old_path.read_bytes(), str(old_path)), Glb(new_path.read_bytes(), str(new_path)))
        elif old.is_file() and new.is_file():
            report(str(new), Glb(old.read_bytes(), str(old)), Glb(new.read_bytes(), str(new)))
        else:
            sys.exit("--pair takes two files or two directories")
    elif args:
        for path in map(Path, args):
            report(str(path), at_head(path), Glb(path.read_bytes(), str(path)))
    else:
        sys.exit(__doc__)


if __name__ == "__main__":
    main()
