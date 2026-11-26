#!/usr/bin/env python3
"""Give marker fixtures their faery values as instance overrides (PAN102a). Run once, then delete.

    scripts/migrate_marker_values.py           print what it would do
    scripts/migrate_marker_values.py --apply   rewrite the precincts

faery's precinct converter dropped the `properties` of marker instances: an area's `radius` or
`halfSizeX` and `halfSizeZ`, and a waypoint list's points. This reads them from faery's precincts
and gives each matching Panoply instance a `MarkerArea` or `MarkerPath` of its own, matched by
exemplar and position. A path's points become offsets from the fixture; faery's were in the
realm's coordinates. Each precinct file is spliced in place: a matched scenery tuple gains a fifth
element, its aspects, and everything else keeps its bytes.
"""

import argparse
import struct
import sys
from pathlib import Path

import asset_xref as xref
from migrate_instance_keys import header, map_header, pack_str

FAERY = xref.ROOT.parent / "faery" / "packages" / "los-data" / "scenery"
PRECINCT_SIZE = 64
# The largest distance, in metres, at which a faery instance is taken for a Panoply one: Panoply
# keeps positions as f32.
NEAR = 1e-3
# The exemplars with marker aspects. Others share the field names: a spawner's `radius` is where
# it spawns.
MARKERS = {
    "CircularRegion", "RectangularRegion", "MusicCircularRegion", "MusicRectangularRegion",
    "WaterCurrent", "RiverCurrent", "WaypointsList",
}  # fmt: skip

FIXEXT = {0xD4: 1, 0xD5: 2, 0xD6: 4, 0xD7: 8, 0xD8: 16}
EXT = {0xC7: ">B", 0xC8: ">H", 0xC9: ">I"}


def unpack(data, i=0):
    """`xref.unpack`, reading faery's extension types (its vectors) as the value they hold."""
    t = data[i]
    if t in FIXEXT or t in EXT:
        if t in FIXEXT:
            n, j = FIXEXT[t], i + 1
        else:
            n = struct.unpack_from(EXT[t], data, i + 1)[0]
            j = i + 1 + struct.calcsize(EXT[t])
        return unpack(data[j + 1 : j + 1 + n])[0], j + 1 + n
    if 0x80 <= t <= 0x8F or t in (0xDE, 0xDF):
        n, j = header(data, i, "map")
        out = {}
        for _ in range(n):
            key, j = unpack(data, j)
            out[key], j = unpack(data, j)
        return out, j
    if 0x90 <= t <= 0x9F or t in (0xDC, 0xDD):
        n, j = header(data, i, "array")
        out = []
        for _ in range(n):
            value, j = unpack(data, j)
            out.append(value)
        return out, j
    return xref.unpack(data, i)


def pack_f32(x):
    return b"\xca" + struct.pack(">f", x)


def pack_vec3(v):
    return b"\x93" + b"".join(pack_f32(c) for c in v)


def pack_array(items):
    n = len(items)
    head = bytes([0x90 | n]) if n < 16 else b"\xdc" + struct.pack(">H", n)
    return head + b"".join(items)


def pack_map(pairs):
    return map_header(len(pairs)) + b"".join(pack_str(k) + v for k, v in pairs)


def area(props):
    if "radius" in props:
        return [("shape", pack_str("circle")), ("radius", pack_f32(props["radius"]))]
    if "halfSizeX" in props:
        return [
            ("shape", pack_str("rect")),
            ("half_x", pack_f32(props["halfSizeX"])),
            ("half_z", pack_f32(props["halfSizeZ"])),
        ]
    return None


def faery_markers(file, origin):
    """faery's marker values in one precinct, as [(exemplar name, position, aspects, summary)]."""
    data = unpack(file.read_bytes())[0]
    structure = data.get("structure") or {}
    types = structure.get("fixtureTypes") or []
    found = []
    for fixture in structure.get("nfixtures") or []:
        schematic, _, position, props = (fixture + [None])[:4]
        props = (props or {}).get("properties") or {}
        name = types[schematic].split(":")[-1]
        if name not in MARKERS:
            continue
        shape = area(props)
        if shape:
            summary = {k: props[k] for k in ("radius", "halfSizeX", "halfSizeZ") if k in props}
            found.append((name, position, [("MarkerArea", pack_map(shape))], summary))
        elif "waypoints" in props:
            at = [origin[0] + position[0], position[1], origin[1] + position[2]]
            points = [[p - a for p, a in zip(wp["position"], at)] for wp in props["waypoints"]]
            path = pack_map([("points", pack_array([pack_vec3(p) for p in points]))])
            summary = {"points": [[round(c, 3) for c in p] for p in points]}
            found.append((name, position, [("MarkerPath", path)], summary))
    return found


def near(a, b):
    return all(abs(x - y) <= NEAR for x, y in zip(a, b))


def migrate(file, markers):
    """The file's bytes with `markers` spliced in, and the names of those matched."""
    data = file.read_bytes()
    types = xref.unpack(data)[0].get("scenery_types", [])
    out = bytearray()
    n, i = header(data, 0, "map")
    out += data[:i]
    matched = []
    for _ in range(n):
        key, j = xref.unpack(data, i)
        out += data[i:j]
        if key != "scenery":
            _, i = xref.unpack(data, j)
            out += data[j:i]
            continue
        count, i = header(data, j, "array")
        out += data[j:i]
        for _ in range(count):
            length, k = header(data, i, "array")
            exemplar, m = xref.unpack(data, k)
            _, m = xref.unpack(data, m)
            position, m = xref.unpack(data, m)
            end = m
            for _ in range(length - 3):
                _, end = xref.unpack(data, end)
            name = types[exemplar].split("#")[-1]
            match = next(
                (mk for mk in markers if mk[0] == name and near(mk[1], position)), None
            )
            if match is None:
                out += data[i:end]
            else:
                assert length == 4, f"{file}: {name} at {position} has aspects already"
                out += bytes([0x95]) + data[k:end] + pack_map(match[2])
                markers.remove(match)
                matched.append((name, position, match[3]))
            i = end
    assert i == len(data), f"{file}: {len(data) - i} bytes left over"
    return bytes(out), matched


def main():
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--apply", action="store_true", help="write the files")
    args = parser.parse_args()

    written, total, missed = [], 0, 0
    for source in sorted(FAERY.glob("*/*.msgpack")):
        target = xref.PRECINCTS / source.parent.name / source.name
        coords = [int(c[1:]) * (1 if c[0] == "p" else -1) for c in source.stem.split("-")]
        origin = [c * PRECINCT_SIZE for c in coords]
        markers = faery_markers(source, origin)
        if not markers:
            continue
        if not target.exists():
            print(f"{target.relative_to(xref.PRECINCTS)}: missing, {len(markers)} markers lost")
            missed += len(markers)
            continue
        data, matched = migrate(target, markers)
        name = target.relative_to(xref.PRECINCTS).with_suffix("").as_posix()
        for exemplar, position, summary in matched:
            at = ", ".join(f"{c:.2f}" for c in position)
            print(f"{name}  {exemplar} ({at})  {summary}")
        for exemplar, position, _, _ in markers:
            print(f"{name}  {exemplar} at {position}: no Panoply instance")
        total += len(matched)
        missed += len(markers)
        written.append((target, data))
    print(f"{total} instances given values, {missed} not matched")
    if missed:
        return 1
    if not args.apply:
        print("dry run: --apply writes the files")
        return 0
    for target, data in written:
        target.write_bytes(data)
    print(f"wrote {len(written)} precincts")
    return 0


if __name__ == "__main__":
    sys.exit(main())
