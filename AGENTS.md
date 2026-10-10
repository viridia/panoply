# Agent instructions

Panoply is a semi-procedural RPG on Bevy, currently mid-migration to Bevy 0.20.

## Orient

1. Read `docs/migration.md` (the ledger): what has landed, what is next, decisions, deferrals.
2. Check `git log --oneline -5`.
3. `grep -rn "TODO(bevy-0.20)"` shows disabled code awaiting restoration.

## Rules

- Never commit, push, or stage. The author reviews and commits; draft the commit message in the
  repo's style (`feat:` / `fix:` / `refactor:` / `chore:` clauses on one line).
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
- `cargo run`: shader and pipeline errors only appear at runtime, so run the app and check the
  log. For visual changes, take a screenshot.
