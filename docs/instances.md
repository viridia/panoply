# Instances

How an instance (an actor, wall, fixture or relay) is named, how a reference finds it anywhere in
the world, and what of it stays in memory while its precinct is not loaded. None of it is built yet:
PAN110 builds the Ids section, PAN6 the latches, and X131 global actors, while X123 decides whether
bindings are needed. Parts marked *Settled* are decided, those marked *Provisional* hang on a choice
not yet made, the rest is the current leaning, and the last section lists what is still open.

## What "global" means

In faery, a global actor had four properties at once: any reference could find it by name, its
record stayed in memory, it belonged to no precinct, and its model was spawned only near the
viewpoint. Panoply gives these out separately, and most instances need only a name.

| Kind       | What it costs                                   | Which instances                      |
| ---------- | ----------------------------------------------- | ------------------------------------ |
| Named      | an index entry; nothing kept in memory          | actors, anything referenced or saved |
| Can travel | nothing more than being named                   | actors a quest or package sends off  |
| Travelling | a record in the global chunk; model only nearby | followers and escorts, while away    |

Every other actor is a precinct actor. Its home precinct owns it (D24), and each time that precinct
loads, the actor starts at its authored place (D80). Transient actors are as R7 describes.

## Ids

An instance's id has two parts: a **key**, generated once and never changed, and an optional
**label**, unique across the game, which text uses to name the instance (a quest's
`actor("stone")`). The Creation Kit makes the same split between FormID and EditorID, and Unreal and
Unity between a GUID and a display name.

Saves and picked references store the key, so renaming a label never breaks either. The editor shows
the label, or, for an instance without one, its exemplar and place ("Door, Tambry inn (3,4)").

- **Format.** 64 random bits within a mod's namespace, so the author's two machines and any number
  of mods can create keys without coordinating, and a diagnostic can say which mod a key came from.
  A key is never typed or shown.
- **Which instances get one.** Those D87 gives an id: actors, instances whose aspects save state,
  and instances something references. Picking an instance as a reference's target gives it a key and
  marks its precinct as changed. Most walls and fixtures have none. A key the editor creates has no
  label: D88's generated names (`wooden-chest-2`) are unique only within their precinct, and a label
  is for the author to choose.
- **The index.** A file checked in beside PAN100's precinct index, with one line per key, sorted:
  key, label, realm, precinct and kind. One line per key keeps diffs readable and merge conflicts
  rare. The editor rewrites it as it saves, and `asset_xref.py` regenerates and compares it, and
  reports references to keys that do not exist. It is checked in rather than built by scanning
  precincts at startup, because web builds cannot list folders, and the editor needs the labels of
  precincts it has not loaded. Only the editor reads it, for Go To and find references. The game
  itself needs only a map from key to live entity, kept as instances spawn and despawn.
- **Mods.** A mod ships its own index, joined with the game's, as it ships its own precinct index
  (X109). It lists the keys the mod adds, in its own precincts and in what its patches add to the
  game's, each line naming the realm and precinct. Keys are created in the mod's namespace, so two
  mods never claim one key. A mod's reference to the game's instance stores the game's key, and
  since mods only add scenery (X109), a mod never rekeys what the game owns. A mod's labels carry
  its prefix, as its aspects' names do ([modding.md](modding.md)), so two mods can each label a
  guard `stone`.
- **Migration.** Today's `SceneryInstanceId::External` names become labels, and each gets a key.

How each edit treats a key:

| Edit                                  | Key                                                    |
| ------------------------------------- | ------------------------------------------------------ |
| Place                                 | created when the instance first needs one              |
| Move, even into another precinct      | kept: a move is never a delete followed by an add      |
| Copy and paste                        | new; a copied relay's roles bind to the copies (PAN79) |
| Delete                                | removed; the editor first lists references naming it   |
| Undo a delete                         | the same key restored                                  |
| Rename a label                        | kept; `.saga` text using the old label is rewritten    |
| Copy a precinct file, or merge in git | may duplicate a key; `verify.sh` fails on duplicates   |

## Instances take their bindings from quests

*Provisional:* this section applies only if quests compose quest-side, which X123 has not decided.
World-side, each thing's own formulas name the quest, and need no bindings.

A **binding** is behavior written in a quest's file for one instance, named by its key: goals,
lines, presence, or a formula for some state of the instance, each conditioned on the quest's state.
The quest is where a binding is written, never what runs it. At runtime the instance owns the
binding, and its formulas react to the quest, as the instance's own formulas would. A stylesheet's
rule works the same way: written in the stylesheet, applied by each element it matches. Three
examples:

- *Up in Arms* holds lines for Sergeant Stone, and a presence condition that keeps him in the inn
  until the quest's next chapter is accepted ([Exemplars.md](Exemplars.md), Quest-side composition).
- A quest holds a castle portcullis's open state: open from a given stage on.
- A quest holds a realm layer's `when`: shown during its middle stages (X109).

Quests are always in memory, but the instances they name usually are not, so an instance collects
its bindings itself:

1. When the game loads its quests, it builds a reverse index from each key to every binding written
   for it, across all quests and all mods.
2. When an instance spawns, because its precinct has loaded, it looks up its key and attaches the
   bindings it finds, joining them to its own aspects: Stone takes the lines and the presence
   condition, and the portcullis the formula for its open state.
3. Those formulas read only quest state, which is always loaded, so they never wait on anything.
4. When the precinct unloads, the bindings go with the instance. The quest never held a reference to
   it, so nothing in the quest changes.

So a quest never acts on an instance, and never writes its state. Some consequences:

- State a binding's formula decides saves nothing: when the portcullis spawns again, its formula
  says whether it is open.
- Naming an actor in a quest does not make it global. Stone stays in his precinct.
- Two mods that each give the same guard a quest both take effect, since neither edits the guard.
- The editor lists the bindings naming a selected instance, as PAN79 lists the relays that name one.

## Global actors

*Settled:* an actor is global while it travels a long way from its home precinct, as a follower or
an escorted character does. A character who appears in several places is several instances, each
with its own presence condition (R85). That is cheaper than one actor walking between places, and
none of those instances is global.

Nothing is simulated out of the player's sight, and away from the player there is no loaded ground
to walk on. So a global actor far from the player is always standing still, as a follower told to
wait is. Content is built so that its absence never shows, as presence conditions hide actors who
are not in a scene.

A global actor's place and state are saved, so a game saved during an escort restores the escorted
character beside the player.

### Becoming global, and going home

Some native goal types are travelling goals, such as `Follow` or `Escort`. An actor becomes global
the first time one of its travelling goals becomes active. It stays global while that goal is
inactive: a follower told to wait somewhere is still a follower. It stops being global when a
`Release` goal becomes active, whose `when` says the actor is no longer needed in its role
(`Release { when: stage >= Delivered }`), and it then returns to its home precinct:

- Its home owns it again (D24). Its place no longer saves: the next time its home loads, it starts
  at its authored place (D80), and the home's authored instance is used again, not the snapshot.
- Its saved aspect state, such as its inventory, stays in its roster record, marked as gone home,
  until its home next loads. As the home spawns its actors, each one with such a record takes its
  state from it, and the record is dropped; the home's next eviction saves that state in its own
  chunk. So nothing writes the chunk of a precinct that is not loaded.
- If its home is far away, it does not walk there: the despawn latch keeps it until it is off
  screen, and then it is gone. An `Exit` goal can send it to the nearest place out of the player's
  view, if there is one, rather than leaving it standing until the player leaves (X127).

A dismissed follower is the simple case. Removing them from the party makes their `Release` active;
they stay where they stand until the player is out of sight, and the next time their town loads they
are at their authored place there, with their state, ready to rejoin. Rejoining makes `Follow`
active, and they are global again.

An actor never moves to another home. A character who stays where they were taken is a second
instance placed there, present from that stage on (R85). The two share no state, which the designer
allows for. An escort, for example:

1. A prisoner has two instances: one in the prison, and one at home, whose presence starts at the
   quest's delivered stage. Freed, the prison instance starts `Escort`, and becomes global.
2. At the prisoner's home the quest reaches its delivered stage, and the prison instance's `Release`
   becomes active. Its home is the prison, where its presence has ended with the quest, so going
   home only drops its record.
3. It is on screen, so the latch keeps it beside the player until the player leaves.
4. When the player comes back, the home instance greets them, with thanks and perhaps the next
   quest. It appears only once its place is out of view (Actors far from home), so the two are never
   seen together.

Which actors can travel is known before play, from the roles and packages that hold travelling
goals, so no author has to mark one. The editor shows a badge on such an actor, with the reason
("can travel: *Up in Arms*, role escort, `Follow`"). `verify.sh` reports how many such actors there
are and how large their definitions are, the measurement X115 asks for.

### Near and far

Near the player, a global actor is **embodied**, as a precinct actor in a loaded precinct is: it has
its goal trees and its model, and holds handles to its exemplar and scripted aspects.

Far from the player, it is a **kernel**: an entity holding only its key, realm, place, facing, home,
aspect state and goal state. It drops its goal trees, its model and its handles. Goal trees are
saved and rebuilt through X17's read-out and write-back, the path hot reload uses, so coming back
into range is the same spawn a precinct load does. Formulas can still read a kernel's state. They
cannot read its aspects' config, which only attaching an aspect inserts (D107), and a formula that
tries suspends.

Two alternatives were rejected:

- **Keeping its definitions loaded**, in X115's global set. The only gain is that a missing exemplar
  is found at startup rather than when the actor comes back into range.
- **Keeping its goal trees, paused.** No precinct actor has a dormant state like that, and a hot
  reload, a mod change or a jump in time would leave the paused trees stale.

### Its record

When an actor becomes global, its record takes a copy of its `ActorInstanceData`. The record needs
the exemplar and aspect overrides to embody the actor while its home precinct is not loaded. It adds
the realm, which a precinct would otherwise supply, and the actor's play state. An edit to the
instance in its precinct reaches a travelling actor only when it goes home, or through hot reload.

### `ActorRoster`

Global actors belong to **`ActorRoster`** (a proposed name). It owns its actors through the same
relationship a precinct uses (D24), shares the precinct's operation that collects state for saving,
and saves to the global chunk ([save-games.md](save-games.md)). It is not a precinct without
scenery: it has no place, owns no catalogs, gains and loses members in play, and is play state, not
an authored asset with saved changes over it. Its only authored part is the player (X46), the one
actor with no home precinct. A companion who starts in the party is a precinct actor, placed in some
town, whose `Follow` is active from the start, so that dismissing them sends them somewhere.

## Quest state

*Settled:* a quest's state is one index into a sequence of stages, each numbered and optionally
named, and saving a quest saves its index. A condition tests the index against spans, `start..end`,
by number or name, as faery's quest stage predicate did. A branching quest is flattened into one
sequence that jumps within itself, so each choice is its own span of stages. How the player
persuades the official in [Exemplars.md](Exemplars.md)'s third quest is such a span, which later
quests test. Objectives completed in any order are subquests, each with its own entry and hints in
the quest log, and the main quest names them by id. So the stage is all a quest exposes.

Each quest marks its final stages as completing or failing. The quest log reads them, and so does a
parent quest, whose condition is that its subquests are all in completing stages.

A restartable quest's failing stage lets the player accept it again from whoever offered it, and
accepting resets the index to the start. This is the only way an index goes backwards. Bindings
follow the index, so what the quest gave the world reverts with it, and plays again. What play
changed directly does not revert: the player's inventory, and state saved in precincts, such as a
death or a forced lock. A restartable quest is written so that restarting never needs either undone.
Nothing may assume an index only rises: a trigger that fires once on passing a stage would miss the
second time through.

## Global state has global owners

There is no blackboard of ownerless flags. Every value that outlives its precinct belongs to an
owner that is itself global.

| State                           | Owner                                        |
| ------------------------------- | -------------------------------------------- |
| Quest stages                    | the quest                                    |
| Travelling actors               | `ActorRoster`                                |
| Party membership                | the player                                   |
| Inventory, stats, map knowledge | the player chunk                             |
| Time of day, weather            | the world                                    |
| Realm layers                    | none: their `when` reads quest stages (X109) |
| Reputation, crime               | a faction, in a later game; none in this one |

## Actors far from home

A precinct actor that strays out of range of its home does no harm out of sight: if its home is
evicted, it despawns, and next time starts at its authored place (D80). What must not happen is an
actor despawning in view, such as a guard who chases the player two precincts away while his home
precinct leaves the cache.

- *Settled:* **the despawn latch.** When a precinct is evicted, its actors on screen stay until they
  leave the view, and then despawn, as R10's presence latch keeps an actor until it is off screen.
- *Settled:* **the appearing latch.** An actor whose presence becomes true appears only once its
  place is out of view, or as its precinct loads, as realm layers switch only out of view (X109).
  R10 latches only disappearing, so this is its other half.
- **The authoring check.** The editor and `verify.sh` warn when an actor's goals name a target
  beyond its home precinct and that precinct's neighbours: a patrol path, a fixture it uses, a
  waymark, or the area a `Wander` goal's tether fixture defines. This catches a patrol drawn too
  wide.
- **Pathing.** Routes are planned only over loaded navigation meshes (R8), so a goal whose route
  would leave them fails, and another goal wins (R50).
- **The leash** (R93): a territorial actor more than 20 m from home clears its threat and returns,
  which keeps a chase near its home precinct. The rule belongs to the aspect, computed from the
  actor's home and a radius, so nothing has to be placed beside each monster.

## Open

- **A cell a relay shares across precincts.** A lever in one precinct that opens a gate in another
  must be remembered while the gate's precinct is not loaded, in a cell kept in memory and saved in
  the global chunk. Two ways to own it:
  - A precinct relay owns the cell and exports it. The puzzle's wiring stays in the precincts that
    hold the puzzle. The index lists each exported cell's name and type, for PAN79's chooser, and a
    cell has its type's default value until something writes it.
  - Only global owners hold cells. The index lists only locations, and a puzzle spanning two
    precincts has a relay in each, plus its cell in a global file.

  Leaning to the second, until a puzzle needs the first.
- **Reading an instance that is not loaded.** Leaning: a formula reading one suspends, as it does
  now ([saga.md](saga.md), Values), and nothing more is offered. A fact needed across the world goes
  through quest state, set while its source is loaded. The alternative, keeping each named
  instance's last values in memory, is the residency this design avoids.
- **Labels in text, or picked.** A quest names actors in `.saga` text by label, which the compiler
  checks and the editor completes from the index, while relays' fields are picked. Or a quest's
  bindings are picked too, stored as data beside its code, as a relay's are.
- **One identity across appearances** (R85): one key with several placements, presence choosing
  among them, and state they share in the global chunk. Deferred until a quest needs variables that
  a character's appearances share.
- **Precedence**, when a binding and a relay both set one fixture's state.
- **A failed escort.** Whether an escorted character can be killed. If not, as R85 has it for quest
  NPCs, an escort fails some other way, and restarting never needs anyone brought back to life.
