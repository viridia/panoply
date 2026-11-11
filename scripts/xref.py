#!/usr/bin/env python3
"""Check that the ledgers' cross-references resolve.

    scripts/xref.py           every check; exit 1 if any fails
    scripts/xref.py --quiet   failures only, no summary

Each numbered ledger owns a prefix, so a reference resolves without knowing where it is written:
`D12` is an entry of docs/decisions.md, `X5` of docs/deferred.md, `I1` of docs/issues.md, `R29` of
docs/actor-requirements.md. A chunk, `PAN<n>`, is open in docs/roadmap.md or landed in
docs/changelog.md. An id after a possessive (`the driver's X9`, `arbor_day's X48`) belongs to
another repository and is skipped.

A ledger numbered from a counter carries a `**Next: <prefix><n>.**` line, which must exceed every
number the ledger has used, in the working tree or in its git history: an entry routed out leaves a
gap that is never reissued.

Each changelog entry is one line, within WIDTH columns, and cites no decision, deferral or issue.

The tables below are this script's configuration; show.py reads them too.
"""

import argparse
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
DOCS = ROOT / "docs"
ROADMAP = DOCS / "roadmap.md"
CHANGELOG = DOCS / "changelog.md"

# Ledgers whose entries are list items opening `- **<prefix><n>**`, as (prefix, file, whether it
# carries a `**Next: <prefix><n>.**` counter).
LEDGERS = [
    ("D", DOCS / "decisions.md", True),
    ("X", DOCS / "deferred.md", True),
    ("I", DOCS / "issues.md", True),
    ("R", DOCS / "actor-requirements.md", False),
]

# Chunks: open ones are roadmap entries opening `- **PAN<n> <title>`, landed ones changelog entries
# opening `- **PAN<n>**`. Lettered parts (`PAN7a`) share their parent's number.
CHUNK = "PAN"

# Source files whose comments cite ids, as (directory, glob).
CODE = [("src", "*.rs"), ("crates", "*.rs"), ("plans", "*.py"), ("scripts", "*.py")]

WIDTH = 100

POSSESSIVE = re.compile(r"'s $")
CHUNK_ID = rf"{CHUNK}\d+[a-z]?"


def entry_def(prefix):
    if prefix == CHUNK:
        return re.compile(rf"^- \*\*({CHUNK_ID})\b")
    return re.compile(rf"^- \*\*({prefix}\d+)\*\*")


def cite(prefix):
    return re.compile(rf"\b{CHUNK_ID}\b" if prefix == CHUNK else rf"\b{prefix}\d+\b")


def counter(prefix):
    return re.compile(rf"^\*\*Next: {prefix}(\d+)\.\*\*")


def number(anchor):
    return int(re.search(r"\d+", anchor).group())


def prose(path):
    """Yield (line number, text), skipping fenced code blocks. Of a Rust file, only the comments:
    its code has identifiers that look like ids (SAGA's `I32` type)."""
    fenced = False
    rust = path.suffix == ".rs"
    for n, line in enumerate(path.read_text(encoding="utf-8").split("\n"), 1):
        if rust:
            start = line.find("//")
            if start >= 0:
                yield n, line[start:]
            continue
        if line.lstrip().startswith("```"):
            fenced = not fenced
            continue
        if not fenced:
            yield n, line


def defined(path, pattern):
    """{id: line number of its first definition}, and the ids defined more than once."""
    seen, dupes = {}, []
    for n, line in prose(path):
        if m := pattern.match(line):
            if m.group(1) in seen:
                dupes.append((n, m.group(1)))
            seen.setdefault(m.group(1), n)
    return seen, dupes


def highest_ever(path, pattern):
    """The highest number `pattern` has matched in `path`, in the working tree or any commit."""
    lines = path.read_text(encoding="utf-8").split("\n")
    log = subprocess.run(
        ["git", "log", "-p", "--format=", "--", str(path.relative_to(ROOT))],
        cwd=ROOT,
        capture_output=True,
        text=True,
    )
    # A diff line carries a one-character prefix.
    lines += [l[1:] for l in log.stdout.split("\n") if l[:1] in "+- "]
    return max((number(m.group(1)) for l in lines for m in [pattern.match(l)] if m), default=0)


def sources():
    md = sorted(ROOT.glob("*.md")) + sorted(DOCS.glob("*.md"))
    # This script's docstring cites example ids.
    code = sorted(
        p for d, glob in CODE for p in (ROOT / d).rglob(glob) if p != Path(__file__).resolve()
    )
    return md, code


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--quiet", action="store_true", help="failures only")
    args = ap.parse_args()

    fails = []

    def fail(path, n, msg):
        fails.append(f"{path.relative_to(ROOT)}:{n}  {msg}")

    # Every id each prefix defines, with the file to name in a failure.
    known = {}
    for prefix, path, _ in LEDGERS:
        ids, dupes = defined(path, entry_def(prefix))
        known[prefix] = (set(ids), path.relative_to(ROOT))
        for n, i in dupes:
            fail(path, n, f"{i} is defined more than once")
    chunks = set()
    for path in (ROADMAP, CHANGELOG):
        ids, dupes = defined(path, entry_def(CHUNK))
        chunks |= set(ids)
        for n, i in dupes:
            fail(path, n, f"{i} is defined more than once")
    # A chunk landed in lettered parts is cited by its parent id too.
    chunks |= {i.rstrip("abcdefghijklmnopqrstuvwxyz") for i in chunks}
    known[CHUNK] = (chunks, "the roadmap or the changelog")

    md, code = sources()
    for path in md + code:
        for n, line in prose(path):
            if line.startswith("**Next: "):
                continue
            for prefix, (ids, where) in known.items():
                for m in cite(prefix).finditer(line):
                    if m.group() not in ids and not POSSESSIVE.search(line[: m.start()]):
                        fail(path, n, f"{m.group()} has no entry in {where}")

    counted = [(prefix, path, [path]) for prefix, path, c in LEDGERS if c]
    counted.append((CHUNK, ROADMAP, [ROADMAP, CHANGELOG]))
    for prefix, path, histories in counted:
        found = next(((n, int(m.group(1))) for n, l in prose(path) for m in [counter(prefix).match(l)] if m), None)
        if found is None:
            fail(path, 1, f"has no **Next: {prefix}<n>.** counter")
            continue
        n, value = found
        used = max(highest_ever(p, entry_def(prefix)) for p in histories)
        if value <= used:
            fail(path, n, f"Next: {prefix}{value} is stale; {prefix}{used} has been used")

    ledger_cite = re.compile(r"\b[DXI]\d+\b")
    lines = CHANGELOG.read_text(encoding="utf-8").split("\n")
    comment = False
    for n, line in enumerate(lines, 1):
        if line.startswith("<!--"):
            comment = True
        if comment:
            comment = "-->" not in line
            continue
        if not line.startswith("- "):
            continue
        if len(line) > WIDTH:
            fail(CHANGELOG, n, f"entry is {len(line)} columns; the limit is {WIDTH}")
        if n < len(lines) and lines[n].startswith(" ") and lines[n].strip():
            fail(CHANGELOG, n, "entry runs onto a second line")
        if m := ledger_cite.search(line):
            fail(CHANGELOG, n, f"entry cites {m.group()}; the decision names the chunk instead")

    for line in fails:
        print(line)
    if not args.quiet:
        entries = ", ".join(f"{len(ids)} {prefix}" for prefix, (ids, _) in known.items())
        print(f"\n{len(md)} documents, {len(code)} source files | {entries} | {len(fails)} problem(s)")
    return 1 if fails else 0


if __name__ == "__main__":
    sys.exit(main())
