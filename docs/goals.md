# Goals

What Panoply is for. For how far the earlier engine got, see `faery.md`.

## The game

- A shipped game, on Steam, itch.io or another storefront.
- Innovative where it can be, not another Ultima or Wizardry.

## Story

- The idea is decades old. Hundreds of pages of story notes and quests exist, in HackMD and
  (an older version) in a Claude project.
- That is not nearly enough for a game this size. Filling the map with no story deserts takes
  about three times the design material there is now.
- LLM-designed quests come out formulaic and obvious, lacking both inspired novelty and
  deviousness. Quest design is the author's.

## Combat

- No fast reflexes required, but not strictly turn-based either. The hybrid timing and the
  curated skill sets (see `faery.md`) are experimental and open to revision.
- Turn-based combat feels artificial and arbitrary. Monsters waiting for the player is just as
  artificial, but the hope is that players accept it as a cinematic convention, like opponents
  taking turns in a martial arts film. Waiting monsters don't freeze; they idle.
- A brainstorm of film conventions that could become mechanics is in [combat.md](combat.md).

## Fun risks

- Whether a design is fun is known only once it is built and played, and many promising designs turn
  out tedious. The riskiest parts are built early and cheaply, and kept cheap to change: tuning is
  an edit to an asset, not a rebuild.
- The risks are combat, the skill trees and the virtue system ([virtues.md](virtues.md)). Quests and
  exploration follow established formulas.
- Combat comes first: the skill trees and virtues show only through it, so until combat is fun,
  nothing learned about them holds.

## Party

- Undecided. Followers have their own personalities and ambient dialogue, and must be
  recruited. Possible models: Dragon Age, Might and Magic.
- The world is open, so content can't assume the player's party: a quest must work with or
  without followers.

## Scale

The author's estimates for the finished world, for designs to size against:

- holm-overland has room for 841 precincts.
- About 10% of them are dense, each with about a dozen actors, a dozen doors and a dozen containers.
- Most instances are local: nothing outside their precinct names them.
- Every actor has goals or dialogue. Actors point to dialogue as they point to meshes.
- About 10% of actors take part in a quest.

## Art

- Panoply simulates genre conventions, not reality. Its art is a cinematic language of its own: the
  outlines, the art-nouveau palette and UI, the simplified character designs, the Bollywood combat.
  Retro is one item in the palette, not the theme.
- Motion and the camera follow the same rule, verisimilitude over realism: they are judged by how
  they read on screen.
- The homage is to the late-90s and early-2000s engines, built without their limits on CPU and
  memory, not to the limits themselves: no pixelated art, no deliberately low frame rate.
- 120 fps is the aim, because smooth motion feels good. It is polish, not play, which is never timed
  finely: a heavy moment may slow down, but below about 12 fps it stops being watchable.
- Characters and scenery are good enough at a distance. Close-up portraits are the weak spot.

## Modding

- The goal is new story content from modders, as in Skyrim. That needs a scripting language.
- Can be dropped if it proves too difficult.

## First playable: "Up in Arms"

The starter quest, mostly complete in faery. The steps, and what each needs:

1. Talk to NPCs and get the quest: dialogue.
2. Travel to Heron Creek Armory: navigation.
3. Explore the abandoned keep, with a few hostile monsters: combat, enemy AI.
4. Open a door with a button: a puzzle element.
5. Meet the antagonist: dialogue.
6. Fight the antagonist: combat.
7. Loot the body: items, containers.
8. Bring the quest item back to the quest-giver: quests, item exchange.

In faery the dialogue and item exchange worked; none of the combat did.
