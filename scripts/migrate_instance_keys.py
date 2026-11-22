#!/usr/bin/env python3
"""Give precinct instances keys, and the story's names as labels (PAN110). Run once, then delete.

    scripts/migrate_instance_keys.py           print what it would do: the labels, and counts
    scripts/migrate_instance_keys.py --apply   rewrite the precincts and write instances.json

Each precinct file is spliced in place: only a scenery tuple's 4th element and an actor's `iid`
change, so everything else keeps its bytes. A scenery instance gets a key if its exemplar chain
has an aspect with state, or it keeps a label; every actor gets one. Its 4th element becomes nil,
the key, or `[key, label]`; an actor's `iid` becomes `key` and `label` fields. faery's other names,
and its integer ids, are dropped.
"""

import argparse
import json
import struct
import sys
import uuid
from collections import Counter

import asset_xref as xref

STATEFUL_ASPECTS = ("Lock", "Container")

# The names kept as labels: Tambry's characters and their furnishings, and the music cues and
# entrances. `?` marks those for the author to decide.
KEPT = {
    "adokka.balar", "adokka.bed.1", "adokka.bed.2", "adokka.bed.3", "adokka.bed.4",
    "adokka.serela", "alc.bed", "alc.couch", "alc.writing.chair.1", "alc.writing.chair.2",
    "alcaldesa", "bsmith.anvil", "bsmith.forge", "bsmith.gs.stool", "bsmith.gs.wheel",
    "chair.stone", "inn.bed.1", "inn.bed.2", "inn.bed.3", "inn.bed.4", "inn.bench.1",
    "inn.bench.2", "inn.bench.3", "inn.bench.4", "inn.bench.5", "inn.bench.6", "inn.cookpot",
    "inn.worktable.0", "inn.worktable.1", "ivy", "ivy.bed", "ivy.counter", "ivy.sweep",
    "ivy.writing.chair", "philo.aynesworth", "philo.bed", "philo.chair", "philo.sweep",
    "philo.table", "poul.bed", "ryba.poul", "ryba.sigrid", "serela.sweep", "sgt.stone",
    "sigrid.bed", "sigrid.sweep", "sigrid.writing.chair",
    "tambry.music", "heroncreek-music", "cave.kreekin.entrance", "cave.kreekin.exit",
    "necropolis.outside",
}  # fmt: skip
UNDECIDED = {"phoebe", "spooksy", "spooksy.explore", "stephan", "sgtstone"}

# Labels written by hand, by precinct and old name: the portals `plans/portals.py` names, and the
# three music regions faery named alike.
RENAMED = {
    ("playground/p001-p002", "meo0"): "playground.mine.1",
    ("playground/p001-p002", "meo1"): "playground.mine.2",
    ("playground/p000-p000", "sp0"): "playground.portal",
    ("holm-overland/p016-p013", "music.necropolis"): "music.necropolis",
    ("holm-underground/p016-p013", "music.necropolis"): "music.necropolis.2",
    ("holm-underground/p017-p013", "music.necropolis"): "music.necropolis.3",
}


def label_for(precinct, name):
    if not isinstance(name, str):
        return None
    if (precinct, name) in RENAMED:
        return RENAMED[(precinct, name)]
    return name if name in KEPT or name in UNDECIDED else None


def header(data, i, kind):
    """The length of the array or map (`kind`) at `data[i]`, and the index after its header."""
    t = data[i]
    if kind == "array" and 0x90 <= t <= 0x9F or kind == "map" and 0x80 <= t <= 0x8F:
        return t & 0x0F, i + 1
    fmt = {"array": {0xDC: ">H", 0xDD: ">I"}, "map": {0xDE: ">H", 0xDF: ">I"}}[kind].get(t)
    if fmt is None:
        raise ValueError(f"no {kind} at {i}")
    return struct.unpack_from(fmt, data, i + 1)[0], i + 1 + struct.calcsize(fmt)


def map_header(n):
    return bytes([0x80 | n]) if n < 16 else b"\xde" + struct.pack(">H", n)


def pack_str(text):
    raw = text.encode()
    if len(raw) < 32:
        return bytes([0xA0 | len(raw)]) + raw
    return b"\xd9" + bytes([len(raw)]) + raw


def pack_key(key):
    return b"\xc4\x10" + key.bytes


def pack_id(key, label):
    if key is None:
        return b"\xc0"
    if label is None:
        return pack_key(key)
    return b"\x92" + pack_key(key) + pack_str(label)


class Migration:
    def __init__(self):
        self.assets = xref.Assets()
        self.stateful = {}
        self.index = []  # [key, label, realm, [x, y], kind]
        self.counts = Counter()

    def has_state(self, reference):
        if reference not in self.stateful:
            found = self.assets.resolve(reference)
            self.stateful[reference] = not isinstance(found, str) and any(
                self.assets.aspect(found[1], found[0], aspect) is not None
                for aspect in STATEFUL_ASPECTS
            )
        return self.stateful[reference]

    def precinct(self, file):
        """The file's bytes, migrated."""
        name = file.relative_to(xref.PRECINCTS).with_suffix("").as_posix()
        realm, coords = name.split("/")
        place = [int(c[1:]) * (1 if c[0] == "p" else -1) for c in coords.split("-")]
        data = file.read_bytes()
        out = bytearray()
        n, i = header(data, 0, "map")
        out += data[:i]
        types = xref.unpack(data)[0].get("scenery_types", [])
        for _ in range(n):
            key, j = xref.unpack(data, i)
            out += data[i:j]
            if key == "scenery":
                i = self.scenery(data, j, out, name, types, realm, place)
            elif key == "actors":
                i = self.actors(data, j, out, name, realm, place)
            else:
                _, i = xref.unpack(data, j)
                out += data[j:i]
        assert i == len(data), f"{name}: {len(data) - i} bytes left over"
        return bytes(out)

    def scenery(self, data, i, out, name, types, realm, place):
        n, j = header(data, i, "array")
        out += data[i:j]
        i = j
        for _ in range(n):
            length, j = header(data, i, "array")
            exemplar, k = xref.unpack(data, j)
            for _ in range(2):
                _, k = xref.unpack(data, k)
            out += data[i:k]
            old, m = xref.unpack(data, k)
            label = label_for(name, old)
            key = uuid.uuid4() if label or self.has_state(types[exemplar]) else None
            out += pack_id(key, label)
            self.record(key, label, realm, place, "scenery", name, old)
            end = m
            for _ in range(length - 4):
                _, end = xref.unpack(data, end)
            out += data[m:end]
            i = end
        return i

    def actors(self, data, i, out, name, realm, place):
        n, j = header(data, i, "array")
        out += data[i:j]
        i = j
        for _ in range(n):
            fields, j = header(data, i, "map")
            pairs, old = [], None
            for _ in range(fields):
                field, k = xref.unpack(data, j)
                value, end = xref.unpack(data, k)
                if field == "iid":
                    old = value
                else:
                    pairs.append(data[j:end])
                j = end
            label = label_for(name, old)
            key = uuid.uuid4()
            pairs.append(pack_str("key") + pack_key(key))
            if label:
                pairs.append(pack_str("label") + pack_str(label))
            out += map_header(len(pairs)) + b"".join(pairs)
            self.record(key, label, realm, place, "actor", name, old)
            i = j
        return i

    def record(self, key, label, realm, place, kind, name, old):
        self.counts["keyed" if key else "unkeyed"] += 1
        if isinstance(old, str):
            self.counts["named"] += 1
            self.counts["labelled" if label else "dropped"] += 1
        if label:
            self.index.append([str(key), label, realm, place, kind])
            mark = " ?" if old in UNDECIDED else ""
            renamed = f" (was {old})" if old != label else ""
            print(f"{label}{mark}  {kind}, {name}{renamed}")


def index_json(entries):
    lines = [json.dumps(entry, separators=(",", ":")) for entry in sorted(entries)]
    return "[\n  " + ",\n  ".join(lines) + "\n]\n" if lines else "[]\n"


def main():
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--apply", action="store_true", help="write the files")
    args = parser.parse_args()

    migration = Migration()
    files = sorted(xref.PRECINCTS.glob("*/*.msgpack"))
    migrated = [(file, migration.precinct(file)) for file in files]
    labels = Counter(entry[1] for entry in migration.index)
    repeated = [label for label, count in labels.items() if count > 1]
    if repeated:
        print(f"labels on more than one instance: {', '.join(sorted(repeated))}")
        return 1
    c = migration.counts
    print(
        f"{c['keyed']} instances keyed, {c['unkeyed']} not; of {c['named']} names, "
        f"{c['labelled']} labels, {c['dropped']} dropped; ? marks those to decide"
    )
    if not args.apply:
        print("dry run: --apply writes the files")
        return 0
    for file, data in migrated:
        file.write_bytes(data)
    (xref.PRECINCTS / "instances.json").write_text(index_json(migration.index))
    print(f"wrote {len(migrated)} precincts and instances.json")
    return 0


if __name__ == "__main__":
    sys.exit(main())
