#!/usr/bin/env python3
"""Read precinct files: what one holds, which instances match, what an edit changed.

    scripts/precinct.py show holm-overland/p010-p016                  each section's count
    scripts/precinct.py show holm-overland/p010-p016 --section actors   one section, an item a line
    scripts/precinct.py show --label inn.cookpot                      one instance, as JSON
    scripts/precinct.py show holm-overland/p010-p016 --key <uuid>
    scripts/precinct.py find --exemplar Bench [holm-overland[/p010-p016]]
    scripts/precinct.py find --aspect PortalTarget --json
    scripts/precinct.py diff holm-overland/p010-p016 [REV]            against git HEAD by default

An instance prints as a catalog entry reads: its key and label, exemplar, position, facing and
aspect overrides, keys as UUIDs and floats at f32 precision, so that a value the game wrote reads as
it does in the editor. A scenery instance's exemplar is resolved through its precinct's
`scenery_types`, so a diff shows a table reordered by a re-save as no change. A label is resolved
through the instance index (`assets/scenery/precincts/instances.json`).
"""

import argparse
import json
import math
import struct
import subprocess
import sys
import uuid
from collections import Counter

from asset_xref import INDEX, PRECINCTS, ROOT, place_of, unpack

# A precinct's sections, in the order they print. Others a file holds print after them.
SECTIONS = [
    "scenery_types",
    "floor_surfaces",
    "terrain_fx_types",
    "tiers",
    "terrain_fx",
    "actors",
    "scenery",
]
INSTANCE_SECTIONS = ("actors", "scenery")


def f32(x):
    """`x` as the shortest decimal that reads back as the same f32."""
    if not math.isfinite(x):
        return x
    try:
        bits = struct.pack(">f", x)
    except OverflowError:
        return x
    # Searched from the f32, not from `x`: two decimals may read as one f32, and an f64 nearer the
    # other would print differently from the same f32 as the game writes it.
    x = struct.unpack(">f", bits)[0]
    for digits in range(1, 10):
        shortest = float(f"{x:.{digits}g}")
        if struct.pack(">f", shortest) == bits:
            return shortest
    return x


def plain(value):
    """`value` with its floats at f32 precision and its keys (16 bytes) as UUIDs."""
    if isinstance(value, float):
        return f32(value)
    if isinstance(value, bytes):
        return str(uuid.UUID(bytes=value)) if len(value) == 16 else value.hex()
    if isinstance(value, list):
        return [plain(v) for v in value]
    if isinstance(value, dict):
        return {k: plain(v) for k, v in value.items()}
    return value


def instance(entry, types):
    """A scenery tuple (`[type, facing, position, id, aspects?]`, its id nil, a key, or a key and
    label) or an actor map, as `show` prints it."""
    if isinstance(entry, dict):
        out = {k: entry[k] for k in ("key", "label") if k in entry}
        out["exemplar"] = entry["exemplar"]
        out["position"] = entry["position"]
        out["facing"] = entry.get("facing", 0)
        aspects = entry.get("aspects")
    else:
        kind, facing, position, iid = entry[:4]
        aspects = entry[4] if len(entry) > 4 else None
        out = {}
        if iid is not None:
            key, label = iid if isinstance(iid, list) else (iid, None)
            # Files from before keys hold the loader's numbers, or faery's names, here.
            out["key" if isinstance(key, bytes) else "id"] = key
            if label is not None:
                out["label"] = label
        out["exemplar"] = types[kind] if kind < len(types) else f"(no scenery type {kind})"
        out["position"] = position
        out["facing"] = facing
    if aspects:
        out["aspects"] = aspects
    return plain(out)


def items(precinct, section):
    """The items of `section`, each as it prints."""
    values = precinct.get(section) or []
    if section in INSTANCE_SECTIONS:
        types = precinct.get("scenery_types", [])
        return [instance(entry, types) for entry in values]
    return plain(values)


def line(item):
    return item if isinstance(item, str) else json.dumps(item)


def sections_of(precinct):
    return SECTIONS + [s for s in precinct if s not in SECTIONS]


def precinct_files(scope=None):
    """The precinct files `scope` names: a realm, `realm/precinct`, or every one."""
    if scope is None:
        return sorted(PRECINCTS.glob("*/*.msgpack"))
    realm, _, name = scope.removesuffix(".msgpack").partition("/")
    if name:
        file = PRECINCTS / realm / f"{name}.msgpack"
        if not file.is_file():
            sys.exit(f"no precinct {realm}/{name}")
        return [file]
    files = sorted((PRECINCTS / realm).glob("*.msgpack"))
    if not files:
        sys.exit(f"no realm {realm}")
    return files


def read(file):
    precinct, _ = unpack(file.read_bytes())
    return precinct


def name_of(file):
    return f"{file.parent.name}/{file.stem}"


def instances(file):
    """`(section, instance)` for each instance in the precinct `file`."""
    precinct = read(file)
    for section in INSTANCE_SECTIONS:
        for item in items(precinct, section):
            yield section, item


def indexed(label):
    """The precinct file and key the instance index gives `label`."""
    for key, name, realm, coords, _ in json.loads(INDEX.read_text()):
        if name == label:
            for file in sorted((PRECINCTS / realm).glob("*.msgpack")):
                if place_of(file) == (realm, coords):
                    return file, key
            sys.exit(f"{label}: the index places it in {realm} {coords}, which has no file")
    sys.exit(f"{label}: not in {INDEX.relative_to(ROOT)}")


def show(args):
    if args.label:
        file, key = indexed(args.label)
        if args.precinct and precinct_files(args.precinct) != [file]:
            sys.exit(f"{args.label} is in {name_of(file)}")
        files = [file]
    else:
        if args.precinct is None and not args.key:
            sys.exit("show needs a precinct, --label or --key")
        files = precinct_files(args.precinct)
        key = args.key
    if key:
        for file in files:
            for section, item in instances(file):
                if item.get("key") == key:
                    print(f"# {name_of(file)} {section}")
                    print(json.dumps(item, indent=2))
                    return 0
        sys.exit(f"no instance with key {key}")
    precinct = read(files[0])
    if args.section:
        if args.section not in precinct:
            sys.exit(f"{name_of(files[0])} has no section {args.section}")
        for item in items(precinct, args.section):
            print(line(item))
        return 0
    for section in sections_of(precinct):
        print(f"{section} {len(precinct.get(section) or [])}")
    return 0


def find(args):
    if not (args.exemplar or args.label or args.aspect):
        sys.exit("find needs --exemplar, --label or --aspect")
    found = 0
    for file in precinct_files(args.precinct):
        for section, item in instances(file):
            if args.exemplar and args.exemplar not in item["exemplar"]:
                continue
            if args.label and args.label not in item.get("label", ""):
                continue
            if args.aspect and args.aspect not in item.get("aspects", {}):
                continue
            found += 1
            if args.json:
                print(f"# {name_of(file)} {section}")
                print(json.dumps(item, indent=2))
            else:
                who = item.get("label") or item.get("key") or "(no key)"
                aspects = " ".join(item.get("aspects", {}))
                print(f"{name_of(file)} {section} {who} {item['exemplar']} {aspects}".rstrip())
    print(f"{found} instances")
    return 0


def at_revision(file, rev):
    """The precinct `file` at git `rev`, or `None` if it has none there."""
    path = file.relative_to(ROOT).as_posix()
    result = subprocess.run(
        ["git", "-C", str(ROOT), "show", f"{rev}:{path}"], capture_output=True
    )
    if result.returncode != 0:
        message = result.stderr.decode().strip()
        if "does not exist" in message or "exists on disk, but not in" in message:
            return None
        sys.exit(message)
    precinct, _ = unpack(result.stdout)
    return precinct


def same_key(item):
    """What makes two items the same for a diff: ints read as floats, since a re-save may write
    `0` as `0.0`, and an empty list as no field, since the game skips writing one."""

    def floats(value):
        if isinstance(value, bool) or not isinstance(value, (int, list, dict)):
            return value
        if isinstance(value, int):
            return float(value)
        if isinstance(value, list):
            return [floats(v) for v in value]
        return {k: floats(v) for k, v in value.items() if v != []}

    return json.dumps(floats(item), sort_keys=True)


def identity(section, item):
    """What matches an item across revisions: an instance's key, a tier's level, or `None`."""
    if section in INSTANCE_SECTIONS:
        return item.get("key")
    if section == "tiers":
        return item.get("level")
    return None


def diff_section(section, before, after):
    """The lines a diff prints for one section."""
    out = []
    if section == "terrain_fx":
        if before != after:
            changed = sum(a != b for a, b in zip(before, after))
            out.append(f"  ~ {changed} values differ in the first {min(len(before), len(after))}")
        return out
    old = {identity(section, i): i for i in before if identity(section, i) is not None}
    new = {identity(section, i): i for i in after if identity(section, i) is not None}
    for ident, item in old.items():
        if ident not in new:
            out.append(f"  - {line(item)}")
    for ident, item in new.items():
        if ident not in old:
            out.append(f"  + {line(item)}")
    for ident in old:
        if ident in new and same_key(old[ident]) != same_key(new[ident]):
            out.append(f"  ~ {ident}")
            out.append(f"    before {line(old[ident])}")
            out.append(f"    after  {line(new[ident])}")
    rest_old = Counter(same_key(i) for i in before if identity(section, i) is None)
    rest_new = Counter(same_key(i) for i in after if identity(section, i) is None)
    shown = {same_key(i): i for i in before + after}
    for k, n in (rest_old - rest_new).items():
        out += [f"  - {line(shown[k])}"] * n
    for k, n in (rest_new - rest_old).items():
        out += [f"  + {line(shown[k])}"] * n
    return out


def diff(args):
    realm, _, name = args.precinct.removesuffix(".msgpack").partition("/")
    if not name:
        sys.exit("diff needs realm/precinct")
    file = PRECINCTS / realm / f"{name}.msgpack"
    before = at_revision(file, args.rev)
    after = read(file) if file.is_file() else None
    if before is None and after is None:
        sys.exit(f"no precinct {args.precinct}, here or at {args.rev}")
    before, after = before or {}, after or {}
    changed = False
    for section in sections_of({**before, **after}):
        old, new = items(before, section), items(after, section)
        lines = diff_section(section, old, new)
        counts = f"{len(old)}" if len(old) == len(new) else f"{len(old)} -> {len(new)}"
        print(f"{section} {counts}{'' if lines else ', same'}")
        for text in lines:
            print(text)
        changed |= bool(lines)
    return 1 if changed else 0


def main():
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    commands = parser.add_subparsers(dest="command", required=True)

    p = commands.add_parser("show", help="each section's count, a section, or an instance")
    p.add_argument("precinct", nargs="?", help="realm/precinct, as holm-overland/p010-p016")
    p.add_argument("--section", help="print this section, an item a line")
    p.add_argument("--label", help="print the instance with this label, found through the index")
    p.add_argument("--key", help="print the instance with this key")
    p.set_defaults(run=show)

    p = commands.add_parser("find", help="instances that match every filter given")
    p.add_argument("precinct", nargs="?", help="a realm, or realm/precinct; every one if omitted")
    p.add_argument("--exemplar", help="exemplars containing this text")
    p.add_argument("--label", help="labels containing this text")
    p.add_argument("--aspect", help="instances overriding this aspect")
    p.add_argument("--json", action="store_true", help="each instance as show prints it")
    p.set_defaults(run=find)

    p = commands.add_parser("diff", help="what changed since a git revision; exit 1 if anything")
    p.add_argument("precinct", help="realm/precinct")
    p.add_argument("rev", nargs="?", default="HEAD", help="git revision, HEAD by default")
    p.set_defaults(run=diff)

    args = parser.parse_args()
    return args.run(args)


if __name__ == "__main__":
    sys.exit(main())
