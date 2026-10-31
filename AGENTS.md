# Agent instructions

Panoply is a semi-procedural RPG on Bevy, currently mid-migration from Bevy 0.15 to Bevy main.

## Orient

1. Read `docs/migration.md` (the ledger): what has landed, what is next, decisions, deferrals.
   `docs/goals.md` says what the game is for.
2. Check `git log --oneline -5`.
3. `grep -rn "TODO(bevy-0.20)"` shows disabled code awaiting restoration.

## Rules

- Never commit, push, or stage. The author reviews and commits; draft the commit message: a
  short subject line (`feat:` / `fix:` / `refactor:` / `chore:` / `docs:` prefix, under about 72
  characters), a blank line, then a body of bullets, one per change, wrapped at 72 columns.
- One chunk at a time: one module group restored or one focused change. Update the ledger in
  the same chunk (Landed row, Next, new decisions or deferrals).
- Every open item goes in the ledger: in Next, or in Deferred with a gate.
- Propose structural changes (new dependencies, replacing a subsystem) with a recommendation
  before writing code.
- Prefer Bevy's built-in replacements over third-party crates where one exists. Check the
  source Panoply builds, Cargo's checkout of the commit in `Cargo.lock`
  (`~/.cargo/git/checkouts/bevy-*/<short commit>/`), and its migration guides in
  `_release-content/migration-guides/`, rather than guessing APIs. `../bevy` is the author's
  working checkout, often on a branch.

## Verify

- `cargo clippy`: no errors, no new warnings.
- Runtime: shader and pipeline errors only appear at runtime. Run plans with the remote driver,
  `bevy-remote-driver plans` (all) or `bevy-remote-driver plans/smoke.py` (one), which launches
  the game, sends input, saves screenshots under `remote-driver/` in Cargo's target directory,
  and quits. Write a new plan in `plans/` for behaviour a chunk adds. Screenshots bring the game
  window to the front. To write a plan, read `../bevy_remote_driver/docs/writing-plans.md`; for
  the steps, `class Driver` in `../bevy_remote_driver/src/bevy_remote_driver/driver.py`, and
  Panoply's own (`goto`, `camera`) in `plans/_panoply_driver.py`.
