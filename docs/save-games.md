# Save games

Panoply aims at open worlds larger than fit in memory, built from precincts that page in and out
(see the ledger, D24). Much of the world is mutable, so a save game must capture changes across the
whole world without writing the whole world. This doc records the design, from the author's essay
[Minimizing save games with structural
sharing](https://medium.com/machine-words/minimizing-save-games-with-structural-sharing-6365222def15),
and what it asks of the engine. Nothing here is implemented yet; the old three.js engine never
finished it either.

## Design

The world's mutable state is split into **chunks** that can be written independently: one per
precinct, plus one for the player character (several, if parts of it change at different rates)
and one for world-wide state such as weather and time of day. Most play is local, so between two
saves most chunks are unchanged, and saves share them rather than copying them.

- A chunk's id is its area id plus a version number. Chunks are immutable once written; a change
  writes a new version.
- A **root index** maps each area id to the chunk id (area plus version) of its current state. A
  saved game is a root index plus what the load dialog shows: a name, the character, a
  timestamp, and the game time played.
- Two root indexes are kept in memory: **prev**, the last game loaded or saved (empty for a new
  game), and **next**, the current state of the world.
- **Eviction:** when a precinct leaves the cache, its state is compared with the version in
  prev. If it changed, a new chunk version is written and its id goes into next. Next itself is
  not written yet. So the game writes chunks in the background as the player travels, whether or
  not they save.
- **Save:** flush every cached precinct, as for eviction; write next; next becomes prev, and a
  copy of it becomes the new next.
- **Load:** read the save's root index as prev; clear the precinct cache entirely; reload
  precincts around the player, applying each one's chunk version from prev over its authored
  asset.
- **Orphans:** quitting without saving leaves chunks no root index points to, and deleting a save
  deletes only its root index, since other saves may share its chunks. A mark-and-sweep pass over
  all root indexes removes orphans.

A quest with far-reaching effects is the exception to "only nearby state changes", and is handled
as a special case (for example through the global chunk).

## When it saves

Saving is automatic: the game saves when it finds a moment convenient, such as between interactions
with no transition under way, and the player never has to. A save the player keeps is a bookmark of
progress, a chapter or a quest's stage, not a snapshot of an instant.

## Animation

No save holds a pose or an animation part-way through. What a precinct loads with, from a save or
back from the cache, follows from these rules, and content is built so that the jump they make
rarely matters.

- **The logical state saves, and loads at its rest pose.** A lowered drawbridge loads lowered; a
  one-shot transition does not replay.
- **Open and shut does not save.** Doors and chests close by themselves once the player is done with
  them, and while a chest is open its inventory has the player's attention, so the game does not
  save then. A door saved open loads shut; an actor standing in its doorway is freed as one spawned
  inside solid scenery is (PAN3q).
- **A loop takes its phase from the world clock**, time modulo its period, so it saves nothing, and
  a windmill does not jump when its precinct leaves the cache and comes back.
- **Consequences follow the change of logical state,** not an animation's end, so loading at the
  rest pose skips none of them.
- **Carriers.** A thing that carries an actor (a lift, a moving platform) is the case these rules
  serve worst: a looping one resumes at its phase, but a lift loaded at rest mid-ride leaves its
  rider in the air. Carriers are rare, so they are the exception: a carrier saves its position along
  its path, and loads there. That needs motion of its own, a position along a path that it advances
  and restores, not a Bevy animation clip.

## Chunk format

A precinct's chunk is MessagePack: a map from an instance's key (D87) to that instance's entries,
each a map from an aspect's name to the state that aspect saved. A patch that moves a keyed instance
to another precinct, or takes its key, would strand its entries; X132 keeps them.

```text
{ <key>: { "Lock": { locked: false } }, ... }
```

- **Aspects save their own state.** `Aspect` has `save`, which writes an entry from the entity's
  components, and `restore`, which applies one; both do nothing by default. The unit is the aspect,
  not the component: `Lock` saves `LockState`, which it inserted, and an aspect may combine several
  components in one entry. Restoring runs after the aspects attach, over the state they built from
  the asset.
- **Entries are keyed by `Aspect::name()`,** not by type path, which is long and changes whenever
  code moves between modules. Names are checked unique across every registered aspect at startup; a
  mod's aspects carry its prefix ([modding.md](modding.md)). A renamed aspect keeps its old names as
  aliases.
- **An entry is written when its state differs from the authored state** (D74): the aspect is the
  authored state, and the state play changes is a component apart from it. A chest unlocked and
  locked again writes nothing, and an entry restored from a save is written again only while it
  still differs from the asset.
- **Payloads are serde structs written with field names** (`rmp_serde::to_vec_named`; `to_vec`
  writes structs as arrays, so adding a field would shift every field after it). Not reflection:
  Bevy's reflect deserializer rejects a field it does not know, so a save from before a field was
  removed would fail to load.
- **State with no aspect,** such as an actor's home when a quest moves it, goes under one built-in
  entry of the instance.

Old saves survive engine changes by the presence or absence of keys, as in protobuf, with no version
number:

| Since the save               | On load                                                |
| ---------------------------- | ------------------------------------------------------ |
| Aspect added                 | No entry: the instance keeps its authored state        |
| Aspect gone, or name unknown | Entry ignored                                          |
| Field added                  | The field's serde default                              |
| Field removed                | Ignored                                                |
| Field's type changed         | The entry fails to decode: logged, authored state kept |
| Aspect renamed               | Found by its alias                                     |

The chunk is read as untyped MessagePack values (`rmpv`) and each entry decoded on its own, so an
entry that fails loses only itself. A missing field takes its type's default, not the authored
value: a `LockState` saved before `locked` existed would read as unlocked. Patching the live
component field by field would fix that, if it ever matters.

## What it asks of Panoply

- **Persisted state is a delta over the authored asset.** A precinct's chunk holds only what changed
  in play (an NPC killed, a door unlocked, a chest looted), keyed by instance key, which stays
  stable across edits of the authored precinct; internal ids do not (D87).
- **Collecting a precinct's state works on a live precinct,** not only at despawn, because a save
  flushes every cached precinct. Eviction is "collect state, then despawn"; save is "collect
  state" for every precinct. So this is an operation both call, not an observer on despawn.
- **Change detection replaces the comparison with prev.** A precinct is marked dirty when any
  persisted component on it or its descendants changes (Bevy change detection); only dirty
  precincts are written.
- **The precinct cache can be cleared** on load, and precincts rebuild cleanly from nothing.
- **Actors belong to a home precinct** (a relationship whose despawn is linked), so their state is
  part of that precinct's chunk, except their places: a precinct that loads again starts its actors
  where they were authored, and their goals move them on (D80). Followers are global, persisted in
  the player or global chunks, places included. Random monsters are transient and not persisted.
- **Parcels hold no persisted state.** Parcels repeat across the world; precincts are unique to a
  place. State tied to a world location, such as picked herbs (ledger X14), goes in the
  precinct's chunk.
