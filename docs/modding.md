# Modding

Panoply is the engine. A game is a package built on it, and a mod is a package built on a game.
Legacy of Storms, the first game, is such a package: its skill trees, combat abilities, quests and
dialogues are content on the engine's primitives, which are Rust.

## Why

- **The game can be sold while the engine stays open source.** Its content lives in a private
  repository.
- **Story content stays private**, so the public repository spoils nothing.
- **Other games of the genre can use the engine.** Thematic content (skills, spells) stays out of
  it.

## Packages

- **A game** is a Rust crate and an asset folder, loaded as an asset source of its own. It may add
  primitives: aspects, systems, native memos, SAGA host functions.
- **A mod**, DLC included, is assets and SAGA only. Rust has no stable ABI, and loading compiled
  plugins is unsafe and breaks with each compiler version, so a mod combines the primitives its
  engine and game provide.

The reasons above need only games: a private crate and asset folder, with no mod system.

## What a mod can do

A mod adds assets and formulas, attaches and configures aspects, and overrides other packages'
exemplars through `extends` (D27). It cannot add systems, rendering, widgets or long-running
scripts. SAGA has no file or network access, so a mod is sandboxed by construction. Panoply is not a
general-purpose engine: even the Creation Kit's scope is more than it aims for.

## Where logic lives

- **Centralized**, as in Bethesda's Creation Kit: Papyrus scripts are source files of functions and
  event handlers declared by name, with variables of persistent state, and assets reference the
  names they export.
- **Distributed:** small formulas inside the assets they belong to. The world is the computer,
  driven by the interaction of objects in the simulation, as a spreadsheet is driven by its cells.
  On a pinball table each bumper's action is simple and the ball's path is not. A migrating bird
  keeps no calendar; it notices the days growing shorter.

Panoply is distributed, with a centralized library: formulas sit at their call sites in assets, and
the functions they share are named and defined in `.saga` files and ruleset assets, as combat
resolution is in [saga.md](saga.md)'s ice storm.

The two models keep different things together: a formula beside the object it drives, or beside the
logic related to it. Distributed loses all of a quest's logic in one place. The editor gets it back
with find references: formulas are compiled and typed, so the flags, stats and functions each one
reads are known without running it.

Scripts hold no memory but the world's ([saga.md](saga.md)). So there are no persistent script
instances, which in Papyrus bloat saves and outlive the mods that made them.

## What mods need

faery's aspects show how much data a mod declares. Its built-in aspects were registered in
`packages/game/src/aspects/` and the engine; its mods were the modules in
`packages/game/src/behaviors/`, loaded by `module:name` id. The type is `IAspectType`
(`packages/engine/src/archetypes/Aspect.ts`), and it declared two kinds of field:

- **`config`:** parameters of one attachment, stored with the aspect where it is attached, usually
  the archetype. Read-only at runtime and not saved, they say what kind of thing this is: a
  container's `containerType`, a combatant's `healthMax`.
- **`properties`:** instance variables, merged into one flat namespace on the instance, each a live
  signal. A realm file sets them per placement (`instanceVars`), and a save holds only what changed
  since the realm loaded. `enumerable: false` made one runtime only (`health`), `hidden` kept one
  out of the editor (timers), and a `formulas` entry made one a reactive cell, not saved. The flat
  namespace served as an interface: door, container and enemy all declare `open`, and
  `controlledOpenable` reads `active` from any kind of button.

Of the 24 aspects loaded as scripts:

- 15, one per character, declare no fields: only goals, interactions and presence.
- 7 logic doors declare instance references (`source`, `source2`), a `toggle`, and a formula for
  `open`. Their wiring is X63's.
- `encounter`, a spawner with 8 fields, and `waterFx`, with 2, are the only ones with data of their
  own. Both belong in the engine: a spawner serves any game of the genre, and water is
  presentation.

The 28 built-in aspects use few field types: bool, float, bounded int, string, string enum, color,
vector, curve, resource key, instance reference, and arrays of a few built-in structs.

So faery's mods needed:

- **A formula bound to a field of a native aspect**, as the logic doors' `open` is. X17 needs
  formulas in assets for the same reason.
- **References to instances** by stable id.
- **State any aspect can read.** `open` and `active` become small native components that formulas
  read.
- **Behavior:** goals, interactions and presence. Characters' scripts are most of a mod, and they
  are code, not data.

None declared a type of its own. Panoply needs no `config` and `properties` split either: an aspect
attached in an exemplar and overridden per instance (PAN43) covers where a value is set, and runtime
state (`health`, timers, `open`) is components outside the asset, saved as X18 plans.

## Types a mod declares

faery was written in JavaScript, so its editor treated built-in and modded aspects alike: one
property inspector served `portal`, which was built in, and `territorial` and `combatant`, which
were mods. Panoply's editor inspects aspects through reflection (PAN28). Bevy's new reflection,
`bevy_reflect2` ([bevy#26356](https://github.com/bevyengine/bevy/pull/26356)), is simpler, faster
and lighter than the old, but it reflects only Rust types, with no dynamic structs, so an aspect
declared in SAGA would need a layer above it. That layer is Bevy's own plan: the PR moves dynamic
data into a system of its own, built on reflection, which Bevy has yet to write.

Mods declare no aspect types until one needs to (X64). A per-instance variable a mod needs goes in a
generic variables aspect, a map from names to SAGA values, as X53's named flags and stats are. SAGA
itself needs no reflection for a struct it declares: it lays structs out in slots
([saga.md](saga.md), Values).

What packages need once there are several (ids, precedence, a removed mod's data, versions) is X65.
WESL's imports are the model for its ids.
