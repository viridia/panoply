#!/usr/bin/env python3
"""Reflow prose in source comments and markdown files to a fixed column width.

    scripts/devfmt.py [--check] [--preview] [--width N] (--diff[=REF] | --sweep) <path>...

Code formatters leave comments alone, or wrap them without understanding markdown, so a rename that
lengthens or shortens an identifier leaves every paragraph that mentions it wrapped wrong, and
fixing that by hand is one small edit per paragraph. This does the same reflow a code formatter does
for code, but for prose: it re-wraps paragraphs and leaves untouched everything with its own layout
rules — fenced code blocks, table rows, headings, blockquotes, divider lines and link definitions.

A <path> may be a file or a directory (directories are walked recursively for markdown and source
files, skipping dot-directories and build output). Given neither reporting flag, matching files are
rewritten in place, silently, unless something changed.

Two flags report instead of writing, both exiting non-zero when something would have changed — the
contract `cargo fmt --check` has. --check lists the files, which is the view for deciding what to
run over next. --preview prints a unified diff of the change itself, its hunks sized to the
paragraph that moved rather than to the minimal line change.

One of two scope flags is required; a run naming neither is refused rather than guessed at.

--diff (default ref HEAD) is the routine one: it restricts every check to paragraphs that overlap a
line `git diff` says changed against that ref, the same trick `git-clang-format` uses to stay out of
code nobody touched. <path> narrows which changed files are considered; with --diff and no paths,
every changed file in the repository is. Run from the repository root — paths from `git diff` are
repo-root-relative.

--sweep takes every paragraph in the paths it is given. Since a paragraph is refilled whether or not
its lines already fit, that rewrites far more than a diff does, which is why it has to be asked for
by name.

The source languages it knows are the rows of LANGUAGES. A file in a language it does not know, by
extension, is refused with an error rather than skipped, so a tree in a new language is never
silently left unformatted: adding the language is the fix. The other files in the same run are
still reflowed. Paths listed in EXEMPT, the project's own scripts to begin with, are skipped
whatever their language.

What it assumes:

- A line is classified by what its trimmed text starts with, not by parsing the language or
  markdown, so a line inside a string literal that starts with `//` is read as a comment and
  reflowed with everything around it. Telling the two apart needs a lexer rather than a line
  classifier. Markdown is read the same way, so an indented code block, an HTML block or a fence
  inside a list item can be misread as prose; a run that reaches one needs its output checked.
- A line with no letter or digit in it is a divider rather than prose, and is held verbatim. A line
  of punctuation meant as prose would be held too.
- A backtick span is never broken across lines, so a span longer than the width overflows it.
- A plain `//` comment is prose, the same as a doc comment. Commented-out code is reflowed as if it
  were prose, so do not point this at a tree that leaves commented-out code lying around.
- Block comments (`/* */`) are left alone entirely.
- A markdown file opening with a `---` line has YAML frontmatter, held verbatim up to the closing
  `---`. A `---` anywhere else is an ordinary line.
- A paragraph in scope is refilled whether or not its lines already fit. Editing prose shortens a
  line as often as it lengthens one, and a shortened line leaves a paragraph ragged without ever
  crossing the width, so a width test sees only half the problem. The cost is that a line broken
  short on purpose does not survive a run that reaches it.
"""

import os
import subprocess
import sys
from dataclasses import dataclass, field
from pathlib import Path

DEFAULT_WIDTH = 100


@dataclass(frozen=True)
class Language:
    """A source language whose line comments this tool reflows."""

    name: str
    extensions: tuple
    # The line-comment markers, tried in order, so one that begins with another must come first:
    # `///` before `//`. Lines with different markers are never merged into one paragraph.
    markers: tuple


# Adding a language is a row here, and a test of its comments. Check first how its doc comments mark
# structure: a line the reflow would merge into the paragraph above it, such as Doxygen's `@param`,
# needs the paragraph engine to treat it as atomic.
LANGUAGES = (Language("Rust", ("rs",), ("//!", "///", "//")),)

# Extensions of source languages missing from LANGUAGES. Meeting one is an error, where any other
# unknown extension is a file with no prose to reflow and is skipped.
UNSUPPORTED_SOURCE = frozenset(
    "c cc cpp cxx c++ h hh hpp hxx h++ inl ipp tpp ixx cppm m mm cs java kt swift go zig py rb js "
    "mjs ts tsx jsx lua sh bash zsh glsl hlsl wgsl cmake".split()
)

# Directories a walk never enters, beside every dot-directory.
SKIP_DIRS = frozenset({"target", "build", "__pycache__", "node_modules"})

# Paths under the repository root this tool leaves alone, whatever their language: the project's own
# tooling, whose comments are kept by hand, and every source tree. Panoply's Rust still holds
# commented-out code, which this would reflow as prose; plans are Python and the asset pipeline
# TypeScript, which it does not know. So in Panoply it formats Markdown only.
ROOT = Path(__file__).resolve().parent.parent
EXEMPT = ("scripts", "src", "crates", "plans", "packages", "assets", "artwork")

MARKDOWN = "markdown"
UNSUPPORTED = "unsupported"


def exempt(path):
    try:
        parts = Path(path).resolve().relative_to(ROOT).parts
    except ValueError:
        return False
    return any(parts[: len(Path(e).parts)] == Path(e).parts for e in EXEMPT)


def classify(path):
    """MARKDOWN, a Language, UNSUPPORTED, or None for a file with no prose to reflow or one under an
    EXEMPT path."""
    if exempt(path):
        return None
    ext = Path(path).suffix[1:]
    if ext == "md":
        return MARKDOWN
    for lang in LANGUAGES:
        if ext in lang.extensions:
            return lang
    if ext.lower() in UNSUPPORTED_SOURCE:
        return UNSUPPORTED
    return None


def wanted(path):
    """Whether a directory walk or a diff should hand `path` on. Unsupported source is included so
    that it reaches the error instead of disappearing."""
    return classify(path) is not None


def lines(text):
    """`text` split into lines the way Rust's `str::lines` does: a final newline ends the last line
    rather than starting an empty one, and a `\\r` before a newline is dropped."""
    if not text:
        return []
    parts = text.split("\n")
    terminated = text.endswith("\n")
    if terminated:
        parts.pop()
    last = len(parts) - 1
    return [p[:-1] if p.endswith("\r") and (i < last or terminated) else p for i, p in enumerate(parts)]


def main(argv):
    check = preview = sweep = False
    width = DEFAULT_WIDTH
    diff_ref = None
    paths = []

    args = iter(argv)
    for arg in args:
        if arg == "--check":
            check = True
        elif arg == "--preview":
            preview = True
        elif arg == "--sweep":
            sweep = True
        elif arg == "--diff":
            diff_ref = "HEAD"
        elif arg.startswith("--diff="):
            diff_ref = arg[len("--diff=") :]
        elif arg == "--width":
            value = next(args, None)
            if value is None:
                print("--width needs a number", file=sys.stderr)
                return 2
            try:
                width = int(value)
            except ValueError:
                print(f"--width needs a number, got {value!r}", file=sys.stderr)
                return 2
        elif arg in ("-h", "--help"):
            print(__doc__.strip())
            return 0
        else:
            paths.append(Path(arg))

    # Repacking reaches every paragraph it is given, so a no-flag form would be a whole-file
    # rewrite rather than a tidy-up. Which paragraphs are in reach is always said out loud.
    if diff_ref is None and not sweep:
        print(
            "devfmt: name the scope — --diff for the paragraphs you changed, --sweep for every "
            "paragraph in the paths given",
            file=sys.stderr,
        )
        return 2
    if diff_ref is not None and sweep:
        print("devfmt: --diff and --sweep are alternatives; pass one", file=sys.stderr)
        return 2

    if diff_ref is not None:
        files = git_diff_changed_files(diff_ref, paths)
        if not files:
            print(f"devfmt: no changed markdown or source files against {diff_ref}")
            return 0
        touched_by_file = {}
        for file in files:
            touched = git_diff_touched_lines(diff_ref, file)
            if touched is None:
                print(f"devfmt: could not read the diff for {file}", file=sys.stderr)
                return 2
            touched_by_file[file] = touched
    else:
        if not paths:
            print_usage()
            return 2
        files = []
        for path in paths:
            collect_files(path, files)
        files = sorted(set(files))
        touched_by_file = None

    # The supported files are still reflowed, so one stray script in a diff does not block the
    # rest; the refusal is reported last, where it cannot scroll away.
    unsupported = [f for f in files if classify(f) == UNSUPPORTED]
    files = [f for f in files if classify(f) != UNSUPPORTED]

    changed_files = []
    error = False

    for file in files:
        touched = touched_by_file.get(file) if touched_by_file is not None else None
        try:
            new_contents = reflow_file(file, width, touched)
        except (OSError, UnicodeDecodeError) as e:
            print(f"devfmt: {file}: {e}", file=sys.stderr)
            error = True
            continue
        if new_contents is None:
            continue
        if check or preview:
            changed_files.append(file)
            if preview:
                old = read_text(file)
                print_preview(file, paragraph_hunks(old, new_contents))
        else:
            try:
                Path(file).write_bytes(new_contents.encode("utf-8"))
            except OSError as e:
                print(f"devfmt: {file}: {e}", file=sys.stderr)
                error = True

    if check:
        for file in changed_files:
            print(f"would reflow: {file}")
        if changed_files:
            print(f"{len(changed_files)} file(s) need reflowing (of {len(files)} checked)")

    if unsupported:
        report_unsupported(unsupported)
        return 2
    if error:
        return 1
    # Both reporting modes have `cargo fmt --check`'s contract: nothing written, non-zero when
    # something would have been.
    if (check or preview) and changed_files:
        return 1
    return 0


def print_usage():
    print(
        "usage: devfmt.py [--check] [--preview] [--width N] (--diff[=REF] | --sweep) <path>...",
        file=sys.stderr,
    )


def report_unsupported(files):
    extensions = sorted({Path(f).suffix[1:] for f in files})
    known = ", ".join(lang.name for lang in LANGUAGES)
    print(
        f"devfmt: no support for .{', .'.join(extensions)} source; these were not reflowed:",
        file=sys.stderr,
    )
    for file in files:
        print(f"  {file}", file=sys.stderr)
    print(
        f"devfmt knows Markdown and {known}. To add a language, add a row to LANGUAGES in "
        "scripts/devfmt.py, with a test of its comments in scripts/test_devfmt.py; the note above "
        "LANGUAGES says what to check first.",
        file=sys.stderr,
    )


# --------------------------------------------------------------------------------------------------
# Git integration: --diff restricts every check to paragraphs that overlap a changed line, the same
# trick `git-clang-format` uses. Both helpers fail soft (empty result) except where a missing diff
# for a file the caller already knows changed would silently widen scope instead — that case is
# treated as an error in main rather than quietly falling back to "check nothing".
# --------------------------------------------------------------------------------------------------


def git_diff_changed_files(git_ref, paths):
    """Every markdown or source file `git diff --name-only` reports as added, copied, modified or
    renamed against `git_ref`, optionally narrowed to `paths`. Deleted files are excluded — there
    is nothing left to reflow."""
    cmd = ["git", "diff", "--no-color", "--name-only", "--diff-filter=ACMR", git_ref]
    if paths:
        cmd += ["--", *map(str, paths)]
    try:
        output = subprocess.run(cmd, capture_output=True)
    except OSError:
        print("devfmt: could not run git", file=sys.stderr)
        return []
    if output.returncode != 0:
        print(f"devfmt: git diff --name-only against {git_ref} failed", file=sys.stderr)
        return []
    return [Path(p) for p in lines(output.stdout.decode("utf-8", "replace")) if wanted(p)]


def git_diff_touched_lines(git_ref, file):
    """The line numbers `file` changed on, in the working-tree version, against `git_ref`. None
    means the git command itself failed — distinct from an empty result, which is a real "no
    changed lines" answer (e.g. a rename with no content change)."""
    try:
        output = subprocess.run(
            ["git", "diff", "--no-color", "-U0", git_ref, "--", str(file)], capture_output=True
        )
    except OSError:
        return None
    if output.returncode != 0:
        return None
    return parse_diff_touched_lines(output.stdout.decode("utf-8", "replace"))


def parse_diff_touched_lines(diff):
    """Parses `@@ -a,b +c,d @@` hunk headers out of a `git diff -U0` patch into the set of touched
    lines in the new (working-tree) file. A hunk that only deletes (`d` is 0) has no line of its own
    in the new file, but the deletion still changed the paragraph around it, so both neighbors of
    the deletion point are marked instead."""
    touched = set()
    for line in lines(diff):
        if not line.startswith("@@ "):
            continue
        new_side = next((tok for tok in line[3:].split() if tok.startswith("+")), None)
        if new_side is None:
            continue
        start_str, _, count_str = new_side[1:].partition(",")
        try:
            start, count = int(start_str), int(count_str or "1")
        except ValueError:
            continue
        if count == 0:
            touched.update((start, max(start - 1, 0)))
        else:
            touched.update(range(start, start + count))
    return touched


def read_text(path):
    # Bytes, so that Python's newline translation cannot turn a CRLF file into an LF one.
    return Path(path).read_bytes().decode("utf-8")


def reflow_file(path, width, touched):
    """Reads `path`, reflows it if it is a kind this tool understands, and returns the new contents
    if they differ from what is on disk. None means either the file did not need any change, or it
    is not markdown or a known language. `touched`, when given, restricts reflowing to paragraphs
    that overlap one of those line numbers — see the module doc's --diff section."""
    kind = classify(path)
    if kind is None or kind == UNSUPPORTED:
        return None
    source = read_text(path)

    # Preserve CRLF and a missing final newline, so a Windows-authored file or a file someone
    # deliberately left without a trailing newline does not pick up a spurious diff.
    crlf = "\r\n" in source
    normalized = source.replace("\r\n", "\n") if crlf else source
    had_trailing_newline = normalized.endswith("\n")

    if kind == MARKDOWN:
        result = reflow_markdown(normalized, width, touched)
    else:
        result = reflow_source(normalized, kind, width, touched)

    if had_trailing_newline and not result.endswith("\n"):
        result += "\n"
    elif not had_trailing_newline and result.endswith("\n"):
        result = result[:-1]
    if crlf:
        result = result.replace("\n", "\r\n")

    return None if result == source else result


def reflow_markdown(text, width, touched):
    """Reflows a markdown file, holding any YAML frontmatter verbatim. The body is reflowed from its
    real line number so that --diff scoping still lines up with what `git diff` reported."""
    frontmatter, body, body_start = split_frontmatter(text)
    return frontmatter + reflow_paragraphs(body, width, "", body_start, touched)


def split_frontmatter(text):
    """Splits a leading YAML frontmatter block off `text`, returning it verbatim, the body after it,
    and the line number the body starts on. Frontmatter is recognized only at the very start of the
    file, which is what keeps a `---` thematic break further down from opening one."""
    if not text.startswith("---\n"):
        return "", text, 1
    end = len("---\n")
    for line_no, line in enumerate(lines(text[end:]), 2):
        end = min(end + len(line) + 1, len(text))
        if line.rstrip() == "---":
            return text[:end], text[end:], line_no + 1
    return "", text, 1


def collect_files(path, out):
    """Walks `path`, appending every markdown and source file found to `out`. A bare file is
    appended directly regardless of extension — the caller decides what to do with it."""
    path = Path(path)
    if path.is_file():
        out.append(path)
        return
    if not path.exists():
        print(f"devfmt: cannot read {path}", file=sys.stderr)
        return
    if not path.is_dir():
        return
    try:
        entries = sorted(os.scandir(path), key=lambda e: e.name)
    except OSError:
        print(f"devfmt: cannot read directory {path}", file=sys.stderr)
        return
    for entry in entries:
        if entry.name.startswith(".") or entry.name in SKIP_DIRS:
            continue
        child = path / entry.name
        if child.is_dir():
            collect_files(child, out)
        elif wanted(child):
            out.append(child)


# --------------------------------------------------------------------------------------------------
# Source: strip the comment marker off each comment line, reflow the prose that results the same way
# markdown prose is reflowed, then put the marker back.
# --------------------------------------------------------------------------------------------------


def reflow_source(source, lang, width, touched):
    out = []
    run = []  # contents of the comment lines in the current run
    run_prefix = ""  # "<indent><marker> " shared by every line in the run
    run_start_line = 1

    for line_no, line in enumerate(lines(source), 1):
        parsed = parse_comment_line(line, lang.markers)
        if parsed is not None and parsed[0] == run_prefix:
            run.append(parsed[1])
        elif parsed is not None:
            flush_comment_run(run, run_prefix, run_start_line, width, touched, out)
            run_prefix, run, run_start_line = parsed[0], [parsed[1]], line_no
        else:
            flush_comment_run(run, run_prefix, run_start_line, width, touched, out)
            run = []
            out.append(line + "\n")
    flush_comment_run(run, run_prefix, run_start_line, width, touched, out)
    return "".join(out)


def parse_comment_line(line, markers):
    """Splits `line` into ("<indent><marker> ", rest) if it is a comment line, where `marker` is
    whichever of `markers` matches first. `rest` keeps whatever whitespace followed the marker, so a
    blank comment line and an already-indented continuation both round-trip."""
    trimmed = line.lstrip()
    indent = line[: len(line) - len(trimmed)]
    for marker in markers:
        if trimmed.startswith(marker):
            rest = trimmed[len(marker) :]
            return f"{indent}{marker} ", rest[1:] if rest.startswith(" ") else rest
    return None


def flush_comment_run(run, prefix, start_line, width, touched, out):
    """Reflows one run of same-prefix comment lines, starting at file line `start_line`, and appends
    the result to `out`. A run with a changed prefix or a code line in between was already split
    into separate runs by the caller."""
    if not run:
        return
    reflowed = reflow_paragraphs("\n".join(run), width, prefix, start_line, touched)
    for line in lines(reflowed):
        # A blank comment line reflows to just the bare prefix; trim the trailing space so `///`
        # round-trips as `///` rather than `/// `.
        if line.rstrip() == prefix.rstrip() and line.endswith(" "):
            out.append(prefix.rstrip() + "\n")
        else:
            out.append(line + "\n")


# --------------------------------------------------------------------------------------------------
# Paragraph reflow, shared by markdown files and by the prose extracted from source comments.
# `base_indent` is prepended to every emitted line — the comment marker for source, or an empty
# string for a markdown file (markdown carries its own indentation as part of the text).
# --------------------------------------------------------------------------------------------------


@dataclass
class ParaLine:
    """One line as it originally appeared (`raw`, everything after `base_indent`, reproduced
    verbatim when the paragraph is out of scope) alongside the text that feeds a rewrap (`rewrap`,
    the marker-stripped words), and the file line number it came from, for --diff scoping."""

    raw: str
    rewrap: str
    line_no: int


@dataclass
class Paragraph:
    """`marker` is a list item's bullet or number, exactly as written, so `- ` and `12. ` each hang
    continuation lines under their own width; None for a plain paragraph. `indent` is the
    whitespace between `base_indent` and the text, taken from the paragraph's first line and reused
    on every line it emits. Dropping it would pull an indented continuation paragraph out to column
    0, escaping the list item it belonged to, and flatten a nested bullet into a top-level one."""

    marker: str
    indent: str
    lines: list = field(default_factory=list)


def reflow_paragraphs(text, width, base_indent, start_line, touched):
    out = []
    in_fence = False
    para = None

    # `text` arrives with `base_indent` already removed. Stripping it again would eat a doc comment
    # inside a doc example, whose content itself starts with `/// `.
    for line_no, content in enumerate(lines(text), start_line):
        trimmed = content.lstrip()
        indent = content[: len(content) - len(trimmed)]

        if in_fence:
            flush_paragraph(para, width, base_indent, touched, out)
            para = None
            out.append(base_indent + content + "\n")
            if trimmed.startswith("```"):
                in_fence = False
            continue

        opens_fence = trimmed.startswith("```")
        # A line with no letter or digit is a divider or a thematic break rather than prose, and a
        # rewrap would pull the following sentence's first word up onto the end of it.
        atomic = (
            opens_fence
            or not trimmed
            or not any(c.isalnum() for c in trimmed)
            or trimmed.startswith("|")
            or trimmed.startswith("#")
            # A blockquote's `>` would be refilled as a word, landing mid-line; held, it overflows.
            or trimmed.startswith(">")
            or is_link_definition(trimmed)
        )

        if atomic:
            flush_paragraph(para, width, base_indent, touched, out)
            para = None
            out.append(base_indent + content + "\n")
            in_fence = opens_fence
        elif (marker := list_marker(trimmed)) is not None:
            flush_paragraph(para, width, base_indent, touched, out)
            para = Paragraph(marker, indent, [ParaLine(content, trimmed[len(marker) :], line_no)])
        else:
            line = ParaLine(content, trimmed, line_no)
            if para is not None:
                para.lines.append(line)
            else:
                para = Paragraph(None, indent, [line])
    flush_paragraph(para, width, base_indent, touched, out)
    return "".join(out)


def flush_paragraph(para, width, base_indent, touched, out):
    if para is None:
        return

    in_scope = touched is None or any(l.line_no in touched for l in para.lines)

    # A paragraph outside the diff's reach is never touched, even if it happens to violate the width
    # — that debt was there before this edit and is not this run's to fix. Inside the reach it is
    # refilled whether or not its lines already fit, because an edit shortens a line as often as it
    # lengthens one and a shortened line leaves the paragraph ragged without ever crossing the
    # width. That half of the problem a width test cannot see, so scope rather than width is what
    # bounds the blast radius.
    if not in_scope:
        for line in para.lines:
            out.append(base_indent + line.raw + "\n")
        return

    first_prefix = cont_prefix = base_indent + para.indent
    if para.marker is not None:
        first_prefix += para.marker
        cont_prefix += " " * len(para.marker)

    units = wrap_units(" ".join(l.rewrap for l in para.lines))
    if not units:
        return

    first_avail = max(width - len(first_prefix), 1)
    cont_avail = max(width - len(cont_prefix), 1)

    wrapped = []
    current = ""
    avail = first_avail
    for unit in units:
        if not current:
            current = unit
        elif len(current) + 1 + len(unit) <= avail:
            current += " " + unit
        else:
            wrapped.append(current)
            avail = cont_avail
            current = unit
    wrapped.append(current)

    for i, line in enumerate(wrapped):
        out.append((first_prefix if i == 0 else cont_prefix) + line + "\n")


def wrap_units(text):
    """The units a greedy fill may not break: whitespace-separated words, except that words inside a
    backtick span are held together as one. A span containing a space otherwise splits across lines,
    and the fragment left at the head of the second line can itself read as a list marker on a later
    run over the same file.

    A unit longer than the width overflows its line, the same trade a long URL already gets. An odd
    number of backticks in the paragraph means they cannot be paired, so the span rule is dropped
    rather than guessed at — one stray backtick would otherwise glue the rest of the paragraph into
    a single unit."""
    if text.count("`") % 2 == 1:
        return text.split()

    units = []
    open_span = False
    for word in text.split():
        if units and open_span:
            units[-1] += " " + word
        else:
            units.append(word)
        if word.count("`") % 2 == 1:
            open_span = not open_span
    return units


def list_marker(trimmed):
    """Whether `trimmed` opens a markdown list item, and if so, the exact marker text ("- ",
    "12. ", ...) so a continuation line can hang-indent under it."""
    for bullet in ("- ", "* ", "+ "):
        if trimmed.startswith(bullet):
            return bullet
    digits = len(trimmed) - len(trimmed.lstrip("0123456789"))
    if digits and trimmed[digits:].startswith(". "):
        return trimmed[:digits] + ". "
    return None


def is_link_definition(trimmed):
    """A reference-style link definition, `[label]: target`. These cannot be reflowed without
    breaking the reference, so they are passed through untouched like a table row."""
    if not trimmed.startswith("["):
        return False
    close = trimmed.find("]", 1)
    return close != -1 and trimmed[close + 1 :].startswith(":")


# --------------------------------------------------------------------------------------------------
# Preview: a unified diff of what a run would write. The hunks are the size of the region the reflow
# rewrote rather than the minimal line change — a reflowed paragraph has every one of its lines
# changed, so a minimal diff of it is noise to read.
#
# No diff algorithm is involved. Every path here emits a blank line verbatim and no paragraph ever
# produces one, so the blank lines of the old text and the new correspond one to one, and the text
# between a pair of them can be compared directly. Dropping the lines that already match at each end
# of such a segment is what narrows a hunk to the part that moved.
# --------------------------------------------------------------------------------------------------


@dataclass
class Hunk:
    """One hunk, ready to print: where it starts in each text (1-based, counting its context) and
    how many lines it covers there, with `body` already carrying the ' '/'-'/'+' prefixes."""

    old_start: int
    new_start: int
    old_count: int
    new_count: int
    body: list


@dataclass
class Region:
    """One changed region, as 0-based half-open line ranges into the old and new texts."""

    old_from: int
    old_to: int
    new_from: int
    new_to: int


def paragraph_hunks(old, new):
    """Where `new` differs from `old`, aligned on the blank lines they share. Empty when they
    match."""
    old_lines, new_lines = lines(old), lines(new)

    regions = []
    i = j = 0
    while True:
        oe, ne = next_blank(old_lines, i), next_blank(new_lines, j)
        narrow_into(old_lines[i:oe], new_lines[j:ne], i, j, regions)
        if oe >= len(old_lines) and ne >= len(new_lines):
            break
        i, j = oe + 1, ne + 1

    # A hunk carries one line of context on each side, since `git apply` refuses a patch whose
    # hunks have none unless told it was generated at -U0. Two regions with a single line between
    # them — the usual spacing of two paragraphs that both moved — would each claim that line, and
    # overlapping hunks are refused just as firmly, so they become one hunk holding it as interior
    # context instead.
    hunks = []
    start = 0
    while start < len(regions):
        end = start
        while end + 1 < len(regions) and regions[end + 1].old_from <= regions[end].old_to + 1:
            end += 1
        hunks.append(materialize(regions[start : end + 1], old_lines, new_lines))
        start = end + 1
    return hunks


def next_blank(text_lines, start):
    return next((k for k in range(start, len(text_lines)) if not text_lines[k].strip()), len(text_lines))


def narrow_into(old, new, old_at, new_at, out):
    """Appends the parts of one aligned span that actually differ. Lines that already match at either
    end are dropped; then the longest run of lines the two sides still share splits what is left,
    and each side of it is narrowed again.

    Trimming the ends alone is not enough. A blank-delimited segment of a source file routinely
    holds a doc comment, the code it documents and a second comment, and a change at each end would
    drag the untouched code between them into the hunk — thirty lines shown as removed and re-added.
    `old_at`/`new_at` are where this span begins in its own text."""
    shorter = min(len(old), len(new))
    head = 0
    while head < shorter and old[head] == new[head]:
        head += 1
    if head == len(old) and head == len(new):
        return
    tail = 0
    while tail < shorter - head and old[len(old) - 1 - tail] == new[len(new) - 1 - tail]:
        tail += 1
    old_lo, old_hi = head, len(old) - tail
    new_lo, new_hi = head, len(new) - tail

    # Trimming has already established that the two sides disagree at both ends of what is left, so
    # any shared run lies strictly inside it and both halves are smaller than what we started with.
    shared = longest_shared_run(old[old_lo:old_hi], new[new_lo:new_hi])
    if shared is not None:
        oi, ni, length = shared
        old_mid, new_mid = old_lo + oi, new_lo + ni
        narrow_into(old[old_lo:old_mid], new[new_lo:new_mid], old_at + old_lo, new_at + new_lo, out)
        narrow_into(
            old[old_mid + length : old_hi],
            new[new_mid + length : new_hi],
            old_at + old_mid + length,
            new_at + new_mid + length,
            out,
        )
        return

    out.append(Region(old_at + old_lo, old_at + old_hi, new_at + new_lo, new_at + new_hi))


def longest_shared_run(old, new):
    """The longest run of consecutive identical lines `old` and `new` share, as an offset into each
    and its length. None when they have no line in common. Taking the longest first is what keeps an
    incidental one-line match — a bare `///`, a lone brace — from being preferred to the real
    one."""
    best = None
    for a in range(len(old)):
        for b in range(len(new)):
            if old[a] != new[b]:
                continue
            length = 1
            while a + length < len(old) and b + length < len(new) and old[a + length] == new[b + length]:
                length += 1
            if best is None or length > best[2]:
                best = (a, b, length)
    return best


def materialize(group, old_lines, new_lines):
    """Builds the printable hunk covering `group`, which is one region or several that had to merge.
    The context lines come from the old text; every one of them is a line both texts agree on, so
    which side they are read from does not matter."""
    first, last = group[0], group[-1]
    lead = int(first.old_from > 0 and first.new_from > 0)
    trail = int(last.old_to < len(old_lines) and last.new_to < len(new_lines))

    body = []
    if lead:
        body.append(" " + old_lines[first.old_from - 1])
    for n, region in enumerate(group):
        body += ["-" + l for l in old_lines[region.old_from : region.old_to]]
        body += ["+" + l for l in new_lines[region.new_from : region.new_to]]
        if n + 1 < len(group):
            body += [" " + l for l in old_lines[region.old_to : group[n + 1].old_from]]
    if trail:
        body.append(" " + old_lines[last.old_to])

    return Hunk(
        old_start=first.old_from + 1 - lead,
        new_start=first.new_from + 1 - lead,
        old_count=last.old_to + trail - (first.old_from - lead),
        new_count=last.new_to + trail - (first.new_from - lead),
        body=body,
    )


def diff_span(start, count):
    """A unified-diff line span. A side with no lines at all is written as the line it follows with a
    count of zero; a hunk carrying context never reaches that, but one at the very start of a file
    can."""
    return (max(start - 1, 0), 0) if count == 0 else (start, count)


def print_preview(path, hunks):
    if not hunks:
        return
    print(f"--- a/{path}")
    print(f"+++ b/{path}")
    for hunk in hunks:
        old_start, old_count = diff_span(hunk.old_start, hunk.old_count)
        new_start, new_count = diff_span(hunk.new_start, hunk.new_count)
        print(f"@@ -{old_start},{old_count} +{new_start},{new_count} @@")
        for line in hunk.body:
            print(line)


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
