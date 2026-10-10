# Agent instructions

Panoply is a semi-procedural RPG on Bevy, currently mid-migration to Bevy 0.20.

## Orient

1. Read `docs/migration.md` (the ledger): what has landed, what is next, decisions, deferrals.
2. Check `git log --oneline -5`.
3. `grep -rn "TODO(bevy-0.20)"` shows disabled code awaiting restoration.

## Rules

- Never commit, push, or stage. The author reviews and commits; draft the commit message: a
  short subject line (`feat:` / `fix:` / `refactor:` / `chore:` / `docs:` prefix, under about 72
  characters), a blank line, then a body of bullets, one per change, wrapped at 72 columns.
- One chunk per session: one module group restored or one focused change. Update the ledger in
  the same chunk (Landed row, Next, new decisions or deferrals).
- Every open item goes in the ledger: in Next, or in Deferred with a gate.
- Propose structural changes (new dependencies, replacing a subsystem) with a recommendation
  before writing code.
- Prefer Bevy's built-in replacements over third-party crates where one exists. Check the
  registry source (`~/.cargo/registry/src/*/bevy_*-0.20.0`) and the migration guides in
  `bevy-0.20.0/_release-content/migration-guides/` rather than guessing APIs.

## Verify

- `cargo clippy`: no errors, no new warnings.
- Runtime: shader and pipeline errors only appear at runtime. Run a plan with the remote driver,
  `python3 crates/bevy_remote_driver/client/run.py plans/smoke.py`, which launches the game, sends
  input, saves screenshots to `target/remote-driver/`, and quits. Write a new plan in `plans/` for
  behaviour a chunk adds. Screenshots bring the game window to the front.
