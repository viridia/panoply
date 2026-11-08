# SAGA

SAGA is Panoply's scripting language, and the only code a mod can ship ([modding.md](modding.md)).
Scripts are for what the runtime gives them, not for syntax: behaviors that hot reload, and logic
that ships outside the engine. Most scripts are like spreadsheet formulas: short, not long-running,
holding no memory but the world's. The game's logic emerges from many of them working together. This
doc records the design; nothing here is implemented yet (X7).

## Early customers

- **Dialogue enabling conditions:** whether a dialogue option is shown.
- **Quest stage advancement.**
- **Character goals** (X17).
- **Actor presence:** whether an actor exists at all, as by quest stage.

Dialogue conditions, like interactions' `when` (PAN21), are evaluated on demand, when the options
are listed. Quest stages, goals and presence react to change.

The first customer is the chest's `when` (PAN45): its "Open" and "Close" are offered by whether its
`ContainerState` is open, formulas in `Proto.Container`. An on-demand formula needs no cells or
reaction passes, and PAN21's native action already has the effect, so SAGA only returns a boolean.
The second is reactive: X63's door opened by a button, its `open` a formula over its source's
`active`.

## The prototype

`../bevy_reactor_3/crates/bevy_reactor_formulae` is the starting point, to be extracted into a
workspace crate, `panoply_saga`, without the rest of `bevy_reactor`. It has a Rust-like language, a
PEG parser, basic blocks, a type inference solver, and a bytecode interpreter on a stack VM. From
`bevy_reactor` it needs only `TrackingScope` (`crates/bevy_reactor/src/tracking_scope.rs`), which
comes along, less its unused cleanups.

What it already gets right:

- A VM is built per evaluation, so its heap of temporaries is freed when the call returns.
- The VM holds `&World` only. A script's result is its effect, applied by the host as a command.
- Host reads of components record dependencies in the tracking scope.

What it gets wrong: every non-primitive value is reflected. A `Vec3` returned by a host function is
a `Box<dyn PartialReflect>` on the VM's heap, cloned back out by downcast when passed to another;
`load_field` finds a field's type by a chain of runtime downcasts the type solver already resolved.

## Values

- **Value types are flattened into stack slots.** A struct whose fields are all primitives or
  value types is laid out from its `TypeInfo` when registered with the host: `Vec3` is 3 slots,
  `Quat` 4, `Transform` 10. Field access is a slot offset fixed at compile time. Calls, returns and
  local indices count slots, not values.
- **Components are read through compiled accessors.** `self.transform.translation.x` is one
  instruction: the entity on the stack, an accessor built at registration that tracks the
  component and copies the fields read onto the stack.
- **Native functions take and return slots** through a trait (`from_slots`, `to_slots`),
  implemented for the glam and Bevy types that matter, with a reflection fallback.
- **References name things by stable id** (a fixture in the actor's precinct, the player), resolved
  to an `Entity` when read, as X17's cursors name definition nodes.
- **Absent is `Option`.** A field nobody set, or a reference with no target, is `None`; the type
  solver makes scripts handle it. `??` is sugar for `unwrap_or`, and `?.` for mapping to a field.
  There is no null.
- **Not loaded is not absent.** Reading a reference whose target's precinct is not loaded suspends
  the evaluation, as React's Suspense does, and the formula's dependency on the reference queues it
  again when the target loads. Scripts never see it; the host applies the fallback of each kind of
  consumer:
  - a reactive cell keeps its last value, so a door stays as its save left it;
  - an on-demand condition counts as false: the interaction or dialogue option is not offered;
  - an effect is discarded whole, and since its writes are buffered, it leaves no partial change.

  A formula that could decide without the missing part (an OR whose other input is true) holds
  instead. Kleene's three-valued logic, with unknown a value that propagates through every operator,
  would decide it, at the cost of a third case in every operator that leaks into scripts as SQL's
  NULL does. Not until a puzzle needs it.
- **Opaque values stay boxed or borrowed:** handles, names, strings, collections. The per-call heap
  bounds them.
- **Enums are not flattened.** Scripts see colors as `LinearRgba`, converted at the boundary. An
  enum scripts need is wrapped by the host in a newtype with access methods of its own.
- **Sets of things are native aggregates, not collections.** "The hostiles near X" is asked through
  functions like `count_within`, `best_cluster` and `lowest_health`, as a spreadsheet's `COUNTIF`,
  so scripts need neither loops over entities nor heap-allocated lists.

## Formulas and effects

- A **formula** (a condition, a computed value) is read-only and returns a value.
- An **effect** may also write, but its writes are commands buffered in the invocation context
  (`damage(target, 5)`), applied between passes. Writes are values, never borrowed data.
- Which host functions a script may call depends on its kind, checked at compile time.
- **Randomness** is for effects and on-demand formulas only: a roll in a reactive formula would
  change its value on every pass. Rolls draw from a seeded generator, so a fight replays.
- **Interceptors** rewrite the buffer before it applies: damage absorption, resistance and reaction
  abilities are filters over pending commands, not skills reaching into each other.
- Entry points take parameters: an ability's `on_hit(caster, target)` runs once per target.
- Inside a script, locals and loops are ordinary and mutable.

## Memos

Most predicates call a memo rather than reading noisy state: `proximity(subject, object, min,
max)` reads transforms that change every frame, but returns a boolean that rarely does;
`isHourBetween(a, b)` reads a `GameHour` resource that changes once per game hour, not the clock.
faery organized its predicates this way (`design/scripting/predicates.md` in faery).

- A memo is a cell: a value, the tick it last changed, and its own tracking scope. Its tick
  advances only when its value changes, so readers compare ticks as Bevy's change detection does.
- Native memos are formulas written in Rust; script formulas are cells too, compiled.
- An actor's cells live beside its goal cursors (X17), indexed by call site. They go when the
  cursor goes, so they need no entity, relation or reference count. Two call sites with the same
  arguments compute twice.
- A tracking scope depends on components, resources and memo cells.

## When reactions run

Bevy has no mutation hooks: a hook on every `&mut` would cost every writer
(`bevy_ecs/src/component/mod.rs`, on `Mutability`). Polling each dependency's ticks, as
`bevy_reactor` did once per frame, is random access per dependency per pass. Instead, changes are
noticed on the read side at the world boundary and pushed inside the graph:

- **World to cells: scans.** A system per watched component type runs a `Changed<C>` query and
  looks up the dependent cells in a reverse index from `(Entity, ComponentId)`. Components scripts
  watch that change rarely (health, inventory, quest and door state) take
  `#[component(summary_tick)]`, so the scan skips tables unchanged since its last run.
- **Cell to cell: push.** A memo whose value changes queues its dependents.
- **Effects: push.** Effects' writes go through the buffer, so each pass knows what it wrote and
  queues the dependents.
- **Time: deadlines.** A status's expiry, a cooldown, a combat clock's turn: each registers a
  deadline with a timer queue, which queues its dependents when it passes. Nothing reads the clock
  every frame.

Pass one, at one point in the schedule after gameplay systems, starts from the scans. Later passes
run the queue only, memos before formulas, until it is empty, with `run_reactions`'s divergence
limit.

## Worked example: an ice storm

An enemy wizard casts Ice Storm on the party. **Asset** is content, **formula** is SAGA, **native**
is Panoply's Rust. Combat timing follows faery's combat clocks (`design/combat.md` in faery).

1. **Beforehand.** *Asset:* `IceStorm`, ground-targeted, radius 4, range 15, costing 20 mana and 3 s
   of combat clock, with a cooldown, cast animations, particles, a timeline and an effect formula.
   `Chilled`, a status whose modifier slows movement by 30% for 6 s. The wizard's archetype grants
   Ice Storm, as a skill tree node grants the player an ability (X53).
2. **The wizard decides.** *Native:* its fight goal (X17) gets a turn when its personal clock falls
   behind the global clock; until then it idles in a fighting stance. *Formula, on demand:* each
   ability it knows scores itself, by a default in the ability asset that a monster's strategy may
   override: `count_within(hostiles(self), best_cluster(hostiles(self), 4.0), 4.0) * 2`, less a
   penalty when mana is short. *Native:* the aggregates.
3. **It commits.** *Native:* its clock advances 3 s and the action locks. The executor walks it
   into range, faces the target and starts the wind-up. The frost circle shows on the ground before
   the storm lands: the audience sees the blow coming.
4. **The timeline.** *Native:* the channel animation at 0 s, particles (X38) at 1.2 s, the hit at
   1.5 s.
5. **The hit.** *Native:* finds the actors in the circle. *Formula, effect:* `on_hit(caster,
   target)` runs per target, buffering `damage(target, Frost, roll(8, 14) * caster.stat(Intellect) /
   10)` and `apply(target, Chilled, 6.0)`. *Native, with formulas from the ruleset asset:*
   interceptors rewrite the buffer: absorption from a talent lowers the damage, frost resistance (a
   derived-stat cell, already current) scales it, and a party member's evade reaction leaps clear.
6. **Application and reactions.** *Native:* the buffer applies; `Health` changes, `Chilled`
   attaches with an expiry deadline. *Formula, reactive:* on the next pass the victims' speed cells
   recompute and locomotion reads them; `badly_hurt(ally)` flips for one victim, waking the healer's
   goal; threat from the damage retargets the party's AI. *Native:* hit reactions (X47), damage
   numbers, a frost tint from the status. A death wakes any quest condition watching it.
7. **Six seconds later.** *Native:* `Chilled`'s deadline passes; the modifier goes and the speed
   cells recompute. The global clock moves on to the party's turn.

Only steps 2, 5 and 6 run formulas, each a line or two; everything with timing or animation is
native.

The timeline is data in abstract time: the hit is at 1.5 s because the ability asset says so, and
the animation and particles follow it as presentation. So combat resolves without presentation,
which a headless balance simulator needs ([combat.md](combat.md)).

## Editor support

- **Syntax highlighting:** a VS Code TextMate grammar in `tools/vscode-saga/`, installed as a local
  extension, for `.saga` files (the prototype's loader uses `.fmod`).
- **Formulas embedded in assets** (dialogue conditions, X17's goals) are highlighted by an injection
  grammar into the asset format, once that format is settled.
- **Semantic tokens and diagnostics** need a language server over `panoply_saga`'s parser and type
  solver; a chunk of its own, when scripts are long enough to want it.

## Open

- Spatial predicates ("any enemy within 10 m") depend on a set of entities. First answer: depend on
  the spatial index as a whole, and let the memo's unchanged result stop the ripple.
- Sharing memos across call sites and actors, if measured cost asks for it.
- Untagged slots: `StackValue`'s tag repeats what the type solver proved.

## Order of work

1. Extract `panoply_saga`, its tests passing, with `.saga` as its extension and the syntax grammar
   (PAN45).
2. Formulas embedded in assets and compiled component accessors, for the chest's `when` (PAN45).
3. Value types in slots.
4. Cells, memos and the reaction passes, with `proximity` and an hour predicate as the first native
   memos, for X63's button and door; measure with `plans/perf.py`.
