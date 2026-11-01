# Agent instructions

Panoply is a semi-procedural RPG on Bevy main.

## Orient

1. Read `docs/roadmap.md`: the chunks to come, in order. `docs/goals.md` says what the game is
   for.
2. Check `git log --oneline -5`.
3. Read `docs/issues.md`: findings not yet routed.

The other ledgers, read by entry when cited: `docs/decisions.md` (`D<n>`, why things are as they
are), `docs/deferred.md` (`X<n>`, work put off, each with its gate), `docs/actor-requirements.md`
(`R<n>`), `docs/notes.md` (measurements, Bevy's observed behaviour, recipes) and `docs/changelog.md`
(what has landed). Print an entry by its id rather than reading the file: `scripts/show.py D24 PAN3`
(`--toc D` lists one prefix's entries).

## Tools

The scripts in `scripts/` are allowlisted in `.claude/settings.json` by absolute path, so call them
as `/Users/talin/Projects/games/panoply/scripts/<name>`, never through `python3` or a relative path,
which would prompt. A new tool lands with its allow rule in the same chunk: every tool that only
reads or diagnoses gets one, and so does a vetted mechanical rewrite (a formatter).

- `verify.sh`: the verification recipe below, in one call.
- `xref.py`: every cited id resolves, counters are current, changelog entries keep to one line.
- `show.py`: prints ledger entries by id.
- `devfmt.py --diff`: rewraps changed Markdown paragraphs to 100 columns (`--preview` to read
  first). Markdown only: the source trees are exempt, since Rust here still holds commented-out
  code. After editing `devfmt.py`, run `python3 <absolute path>/scripts/test_devfmt.py`.

## Rules

- Never commit, push, or stage. The author reviews and commits; draft the commit message: a
  short subject line (`feat:` / `fix:` / `refactor:` / `chore:` / `docs:` prefix, under about 72
  characters) ending with the chunk id, as `(PAN12)`, a blank line, then a body of bullets, one
  per change, wrapped at 72 columns.
- One chunk at a time. When it lands, in the same chunk: delete its roadmap entry, add its
  changelog line (the instructions are in a comment at the top of the file), and record new
  decisions, deferrals and issues.
- Every open item has a destination: a chunk in the roadmap, a deferral with a gate, or an
  issue.
- Propose structural changes (new dependencies, replacing a subsystem) with a recommendation
  before writing code.
- Prefer Bevy's built-in replacements over third-party crates where one exists. Check the
  source Panoply builds, Cargo's checkout of the commit in `Cargo.lock`
  (`~/.cargo/git/checkouts/bevy-*/<short commit>/`), and its migration guides in
  `_release-content/migration-guides/`, rather than guessing APIs. `../bevy` is the author's
  working checkout, often on a branch.

## Verify

- `scripts/verify.sh`: cross-references, Markdown wrapping of changed paragraphs, and
  `cargo clippy --workspace`. A warning is a failure.
- Runtime: shader and pipeline errors only appear at runtime. Run plans with the remote driver,
  `bevy-remote-driver plans` (all) or `bevy-remote-driver plans/smoke.py` (one), which launches
  the game, sends input, saves screenshots under `remote-driver/` in Cargo's target directory,
  and quits. Write a new plan in `plans/` for behaviour a chunk adds. Screenshots bring the game
  window to the front. To write a plan, read `../bevy_remote_driver/docs/writing-plans.md`; for
  the steps, `class Driver` in `../bevy_remote_driver/src/bevy_remote_driver/driver.py`, and
  Panoply's own (`goto`, `camera`) in `plans/_panoply_driver.py`.
