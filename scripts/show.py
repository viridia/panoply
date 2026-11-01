#!/usr/bin/env python3
"""Print ledger entries, found by their ids.

    scripts/show.py D12        an entry, with its continuation lines
    scripts/show.py PAN3       a chunk: its roadmap entry, or its changelog line once landed
    scripts/show.py R29 X5     several at once
    scripts/show.py --toc D    each entry's first line, for one prefix: D, X, I, R or PAN

The prefixes are xref.py's `LEDGERS` and `CHUNK`. An entry runs from its id to the next line that
is blank or not indented. A gap in the numbering is a retired entry, routed elsewhere or dropped;
`git log -S` finds where it went. Any unknown id exits 1, after the others are printed.
"""

import re
import sys

from xref import CHANGELOG, CHUNK, LEDGERS, ROADMAP, ROOT, entry_def, prose

FILES = {prefix: [path] for prefix, path, _ in LEDGERS}
FILES[CHUNK] = [ROADMAP, CHANGELOG]
PREFIX = re.compile(rf"({'|'.join(FILES)})\d+[a-z]?")
WIDTH = 100


def find(anchor):
    """Return (path, first line, last line), 1-based and inclusive, or None."""
    prefix = PREFIX.fullmatch(anchor).group(1)
    for path in FILES[prefix]:
        lines = path.read_text(encoding="utf-8").split("\n")
        start = next(
            (n for n, l in prose(path) for m in [entry_def(prefix).match(l)] if m and m.group(1) == anchor),
            None,
        )
        if start is None:
            continue
        end = start
        while end < len(lines) and lines[end].startswith((" ", "\t")):
            end += 1
        return path, start, end
    return None


def main():
    args = sys.argv[1:]
    if not args:
        print(__doc__.strip(), file=sys.stderr)
        return 2
    if args[0] == "--toc":
        status = 0
        for prefix in args[1:]:
            if prefix not in FILES:
                print(f"{prefix}: not a prefix; one of {', '.join(FILES)}", file=sys.stderr)
                status = 1
                continue
            for path in FILES[prefix]:
                print(path.relative_to(ROOT))
                for n, line in prose(path):
                    if entry_def(prefix).match(line):
                        print(f"{n:>5}  {line[:WIDTH]}")
        return status
    status = 0
    for i, anchor in enumerate(args):
        if i:
            print()
        found = PREFIX.fullmatch(anchor) and find(anchor)
        if not found:
            print(f"{anchor}: no such entry", file=sys.stderr)
            status = 1
            continue
        path, start, end = found
        lines = path.read_text(encoding="utf-8").split("\n")
        print(f"{path.relative_to(ROOT)}:{start}-{end}")
        print("\n".join(lines[start - 1 : end]))
    return status


if __name__ == "__main__":
    sys.exit(main())
