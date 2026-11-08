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

The rest of `docs/` are design docs, each owning one subsystem's design and cited by section
(`saga.md`, `modding.md`, `combat.md`), and references, which map something outside Panoply
(`faery.md`, the old engine's code).

## Tools

The scripts in `scripts/` are allowlisted in `.claude/settings.json` by absolute path, so call them
as `/Users/talin/Projects/games/panoply/scripts/<name>`, never through `python3` or a relative path,
which would prompt. A new tool lands with its allow rule in the same chunk: every tool that only
reads or diagnoses gets one, and so does a vetted mechanical rewrite (a formatter).

- `verify.sh`: the verification recipe below, in one call.
- `xref.py`: every cited id resolves, counters are current, changelog entries keep to one line.
- `asset_xref.py`: every path a precinct or exemplar names resolves, known breaks listed against
  their ledger entries.
- `show.py`: prints ledger entries by id.
- `outlines.py`: lists the meshes in `assets/` drawn without an outline, and whether faery outlined
  them (`--none` for those nothing outlines).
- `blend_inventory.py`: what is wrong with `.blend` files' clips, materials and export settings
  (D55). It and the other `blend_*.py` run themselves inside Blender, headless.
- `blend_export.py`: exports `.blend` files with their remembered settings (`--to DIR` elsewhere).
- `blend_cleanup.py`: repairs what the inventory reports; a dry run unless `--apply`, which saves
  `.blend` files, so it has no allow rule.
- `glb_compare.py`: what a re-export changed, clip by clip and pose by pose, against git `HEAD`.
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

- `scripts/verify.sh`: cross-references in the ledgers and in `assets/`, Markdown wrapping of
  changed paragraphs, and `cargo clippy --workspace`. A warning is a failure.
- Runtime: shader and pipeline errors only appear at runtime. The remote driver's plans, in
  `plans/`, launch the game, send input, save screenshots under `remote-driver/` in Cargo's target
  directory, and quit. To write a plan, read `../bevy_remote_driver/docs/writing-plans.md`; for the
  steps, `class Driver` in `../bevy_remote_driver/src/bevy_remote_driver/driver.py`, and Panoply's
  own (`goto`, `camera`) in `plans/_panoply_driver.py`.
- Each plan file is a launch, and each launch's window takes focus from the author, who works during
  a run and takes a window closing for the run's end. Test a chunk's new behaviour in the existing
  plan it fits; a new file only for a new area.
- Run the driver as `/Users/talin/.local/bin/bevy-remote-driver` (the Bash tool's `PATH` lacks
  `~/.local/bin`), given the absolute path of a plan or of `plans/`: it builds the workspace that
  holds the plan, and a relative path resolves against the working directory, which may be another
  repo's. Never two runs at once: plans compete for the window and its timing.
- A chunk's end, in order: `scripts/verify.sh`; then stop and hand the build to the author to try;
  when they report back, one run of `/Users/talin/Projects/games/panoply/plans`, every plan and not
  only the chunk's, approved once; then one of `plans/perf.py`, which the directory run skips, and
  whose line in `docs/perf.csv` goes in the chunk's commit; the commit message last. While
  iterating, run one plan by its path.
