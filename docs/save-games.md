# Save games

Panoply aims at open worlds larger than fit in memory, built from precincts that page in and
out (see the ledger, D24). Much of the world is mutable, so a save game must capture changes
across the whole world without writing the whole world. This note records the design, from the
author's essay [Minimizing save games with structural
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

## What it asks of Panoply

- **Persisted state is a delta over the authored asset.** A precinct's chunk holds only what
  changed in play (an NPC killed, a door opened, a chest looted), keyed by `SceneryInstanceId`.
  Instance ids must therefore stay stable across edits of the authored precinct.
- **Collecting a precinct's state works on a live precinct,** not only at despawn, because a save
  flushes every cached precinct. Eviction is "collect state, then despawn"; save is "collect
  state" for every precinct. So this is an operation both call, not an observer on despawn.
- **Change detection replaces the comparison with prev.** A precinct is marked dirty when any
  persisted component on it or its descendants changes (Bevy change detection); only dirty
  precincts are written.
- **The precinct cache can be cleared** on load, and precincts rebuild cleanly from nothing.
- **Actors belong to a home precinct** (a relationship whose despawn is linked), so their state is
  part of that precinct's chunk. Followers are global, persisted in the player or global chunks.
  Random monsters are transient and not persisted.
- **Parcels hold no persisted state.** Parcels repeat across the world; precincts are unique to a
  place. State tied to a world location, such as picked herbs (ledger X14), goes in the
  precinct's chunk.
