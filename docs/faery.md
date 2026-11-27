# faery survey

Pointers into `../faery`, the earlier TypeScript/three.js engine, for areas worth reading when the
matching Panoply work starts. This is a map from a first-pass scan (2026-10-10), not the X13
catalog: entries marked *read* have been read in full, the rest are directory listings only.
Paths are relative to `../faery`.

faery is not ECS-based: take its design (what the game needs), not its structure (classes
wrapping handles and state, side maps from handles to objects). See D23.

## Layout

An npm workspace (last touched 2024). Main packages under `packages/`:

- `engine`: the core. Subsystems under `engine/src/`: `world`, `physics`, `nav`, `portals`,
  `archetypes`, `behaviors`, `quests`, `dialogue`, `items`, `skills`, `stats`, `particles`, `audio`,
  `lighting`, `materials`, `models`, `animation`, `render`, `overlays`, `player`, `scripting`. Skip
  `lib/computable`, a Solid-signal graph of derived values that change detection and systems
  replace; its subclasses (`world/structures/Computed*.ts`, `world/terrain/Computed*.ts`) hold the
  derivations worth reading.
- `game`: game rules on top of the engine: `aspects`, `sim`, `behaviors`, `goals`, `quests`,
  `systems`, `books`.
- `editor`: the real editor (`los-editor` and `los-game` are thin entry points).
- `common`, `geometry` (polygon clipping, convex partition, curves), `keys`, `schemas`,
  `reflex*` (UI and property editing), `game-ui`, `obsidian`, `server`, `genmap-cli`.
- `los-data`: realm maps (`realms/*.msgpack`), per-realm scenery, `world.json`.

Also `design/` (notes), `scripts/convert_*.mts` (the converters that produce Panoply's data; see
Migration scripts below, and X1), `artwork/`.

## Physics (*read*)

`engine/src/physics/`, about 1,600 lines. See D22 for what this decided.

- `PhysicsRealm.ts`: one rapier `World` per realm, stepped at 1/120 s. Holds the collider index
  and all queries: ray pick, shape cast, shape-list cast (placing fixtures), ray intersection with
  sensors, shape intersection.
- `defs.ts`: nine collision groups (Static, Door, Actor, Npc, Portal, Visible, Sensor, Marker,
  Pickable) as rapier membership/filter bits, and named masks (`TerrainMask`, `ActorMoveMask`,
  `PickMask`, `EditorPickMask`, ...). `ColliderIndex` maps collider handle to metadata (instance,
  archetype); this is how mouse picking finds game objects (relevant to PAN88, X14).
- `PhysicsActor.ts`: capsule body; kinematic when controlled, dynamic when not (switching
  rebuilds the body); Y rotation only; CCD on. A second, inflated capsule for picking. Movement
  uses its own shape casts, not a character controller; `unstick` pushes out of overlaps, then
  tries random offsets. Max actor size: height 1.4, radius 0.3 (fits doors).
- `PhysicsMovable.ts`: kinematic bodies that follow named nodes of an animated model (doors,
  mechanisms), with a "did you mean" warning for bad node names.
- `PhysicsHeightField.ts`: per-parcel ground as height field or trimesh; offset -0.01, likely a
  seam workaround (parcel seams let actors fall through).
- `PhysicsFloor.ts`: floor regions split by `convexPartition` and extruded by thickness into
  convex hulls. `PhysicsWalls.ts`: fixed body per structure, colliders from descriptors. Both
  have `setCutaway`, which clears the Visible bit so picking passes through cut-away walls.
- `PhysicsTrees.ts`: a capsule per tree on one fixed body per parcel. `PhysicsSlab.ts`: a slab
  that stops falls to infinity.
- `IColliderDesc.ts`, `createCollider.ts`: collider descriptors on archetypes. Shapes: box, rbox,
  sphere, cylinder, ellipsoid (convex hull), ramp (convex mesh). Types: solid, door, ladder,
  sensor, hint (guides the pathfinder without blocking), portal, marker (editor-only). Fields
  include `walkable` and a nav `margin`, so physics descriptors also feed navigation.
- `design/physics.md`: one TODO, water as linear damping and reduced gravity.

## Scenery (for Next: `scenery`)

- `engine/src/world/scenery/`: `Fixture`, `Floor`, `WallTile`, `wallTileGeom`, `Cutaways`,
  `SensorSystem`, `SceneryParticles`, `TerrainFx`, `waypoints`, `markers`, `colliders`,
  `scripting/`.
- `engine/src/world/structures/`: `Structure`, `Layer`, `Tier`, `ActiveLayers`, `FloorRegions`,
  `PrecinctCache`, `PrecinctPointLights`, `createNearbyPrecincts`, `spanList`. Derived data per
  element kind, one file per product: `ComputedFixture{Models,Obstacles,Physics}`,
  `ComputedWall{Models,Obstacles,Physics}`, `ComputedFloor{Mesh,Physics,RegionObstacles}`,
  `ComputedModelGroups`, `ComputedTerrainEffects`. The pattern (each element yields render
  models, nav obstacles and colliders) may shape the scenery restoration.
- Not yet in Panoply: sensors, precinct point lights and layers.
- Cutaways (read in full): `engine/src/player/CutawayController.ts` picks cutaways each frame
  from the authored rects of the player's precinct. For each tier from the player's
  (`floor(y - TIER_OFFSET + 1.7)`) upward, it takes the rect hit by a vertical ray from the
  player, else by the ray toward the camera, each projected to that tier's height; then it adds
  every rect on those tiers that overlaps one already chosen, until nothing changes. The chosen
  rects' 3D volumes go to `world/scenery/Cutaways.ts`, which only stores them (with an optional
  instance-type mask) and signals a change; each scenery element and actor then hides itself if
  its position or bounding box is inside a cutaway volume.
- `design/gridless.md`: walls, floors, portals and tiers without a grid.

## Actors and navigation (for Next: `actors`)

- `engine/src/world/actors/`: `Actor`, `ActorArchetype`, `ActorController`, `ActorEffects`,
  `Action`, `SkinnedModel`, `animation`, `ThreatMap`, `parking`, `actorPredicates`.
- `engine/src/world/actors/goals/`: goal-tree AI (`GoalNode`, `GoalPriority`, `PrioritizedGoalList`,
  `factory`, `components/`). Not a direct port: it moves to ECS and BSN assets
  ([behavior.md](behavior.md)).
- `engine/src/nav/`: navigation mesh (`NavigationMesh`, `NavigationMeshBuilder`, `NavRegion`,
  `NavTract*`), route tasks (`NavRouteTask`, `findReachableSpot`), `NavigationController`, and a
  worker. Obstacles come from scenery (the `Computed*Obstacles` files) and terrain
  (`world/terrain/ComputedNavHeightMap`). Panoply has no nav yet.
- Physics readiness (read in full; Panoply's is D67):
  `TerrainController.isPhysicsReady(realm, position, radius)` grows the point by the radius into a
  rectangle and is true when every parcel and precinct it overlaps is ready. A parcel must be cached
  with `isPhysicsOk`. A precinct must be cached, simulated (near the camera) and `isPhysicsReady`,
  which `world/structures/Structure.ts` memoizes as: fixture colliders built, and every tier's
  physics up to date and enabled. Each frame, `ActorController` activates the physics of a simulated
  actor in the active realm where its area is ready and deactivates it elsewhere; the actor keeps
  updating either way.
- `engine/src/player/`: `PlayerActionController`, `CutawayController`, `GameUiState`.
- `engine/src/animation/AnimationDriver.ts`.

## Portals (for Next: `portals`, D21)

- `engine/src/portals/`: `ActivePortal`, `ActivePortalMaterial`, `ActivePortalSystem`,
  `IPortal`, `portalAspect`. Small; background for the rewrite.
- `game/src/systems/FollowerTeleport.ts`: followers crossing with the player.

## Editor (for the Scenery and Editor threads)

- Screenshots, in [faery-screenshots/](faery-screenshots/), each mode at its route: a bar of mode
  buttons atop the panel, realm (`/map`), terrain (`/plots`), scenery, catalogs and play, then save
  (disabled while nothing is unsaved) and the Show menu (▼).
  - `realm.png`: map size, realm and view position; the map size control (eight arrows that grow or
    shrink each side, turned with the camera); Create and Change Realm; nav points with Go, + and −.
  - `terrain.png`: the terrain tools as a grid of buttons; terrain groups and biomes, each list with
    its own + − ▲ ▼; the terrain plot palette.
  - `scenery-select.png`, `scenery-walls.png`, `scenery-actors.png`: the scenery tools as a grid of
    buttons, the selected tool's palette with a search box, New and Clone; for actors, the selected
    exemplar's properties below.
  - `catalogs-quests.png`: catalog kinds (quests, dialogues, remarks, books, particles, items, item
    tables, spawn tables, cut scenes), the chosen kind's entries, New, Clone and search.
  - `play.png`: the realm's actors, the selected one's goal tree, then its state.
  - The property editor: `properties-skill.png` (× removes an optional field, Add lists the absent
    ones, an array collapsed to "1 items" with − +); `scenery-fixture.png` (nested structs and
    arrays, joined by tree lines); `scenery-actor-inherited.png` (fields inherited through `extends`
    in dim italics, where × reverts an overridden field to the inherited value; a color field opened
    to its wheel). Nested structs and arrays were its weakest part. `scenery-add-aspect.png`: the
    exemplar's Add menu, its absent fields then "aspects…", which opens `scenery-choose-aspect.png`,
    a checklist of aspects grouped by package, with Cancel, Clear and Save. `choose-model.png`: a
    model field's picker, a dialog listing "(no model)" then the models grouped under their `.glb`
    files, each group a disclosure row, with Cancel and Save.
  - `catalogs-dialogue.png`: the dialogue state machine editor, which replaced the 3D view while a
    dialogue entry was selected: a box per state holding its intros and options, each in the
    property editor, with lines from a `go` field to the state it names.
- `editor/src/tools/`: 15 tools, each a subclass of `TerrainTool` (pointer picking against terrain
  and scenery, hover and drag), with `mutations.ts` and `ToolState.ts`. Most are modal, made by the
  mode or submode button selected:
  - Terrain (`view/PlotEditPanel.tsx`): `TerrainContouringTool`, `FloraPlacementTool`.
  - Floors (`view/modes/PlaceFloorMode.tsx`): `DrawFloorTool`, `EditFloorTool`, and `StampFloorTool`
    in four shapes (1, 2, 3 cells, octagon).
  - Scenery (`view/SceneryEditPanel.tsx`): `PlaceWallTool`, `PlaceTerrainFxTool`,
    `PlaceFixtureTool`, `PlaceActorTool`, `SelectSceneryRectTool`, `SceneryInteractTool`,
    `SelectStructTool`.
  - Selected instance (`view/modes/SelectSceneryInstanceMode.tsx`): `MoveInstanceTool`,
    `RotateInstanceTool`.

  One is situational: `PlaceWaypointTool`, made by a button beside a field in the property editor
  (`WaypointFieldEditor`, `LocationFieldEditor`, `Location2FieldEditor`).
- `editor/src/reflex-ui-faery/customFieldEditors.tsx`: the property editor picks a field's widget
  from its type descriptor, by faery's own reflection: a `subtype` annotation on a plain type (a
  string's `model`, `realm`, `rgb`, `resourceKey`, `instanceKey`, `textureKey`, `sfx`, `sceneRole`,
  `instanceGroup`; a nullable string's `dialogueState`; a `Vector3`'s `waypoint`; a record's
  `stageExitMap`, a union's `stagePredicate`, a tuple's `stageRange`, a struct's `predicate` or
  `sideEffect`), or a named custom type (`ColorMap`, `FeatureSelection`, `Location`, `Location2`,
  `InstanceRef`, `Inventory`, `SoundRef`). In Rust most annotations become newtypes (D45).
- `editor/src/`: `builders`, `controls`, `view`, `helpers`, `reflex-ui-three`,
  `reflex-ui-faery`.
- `design/Editor.md`: tool organization, map mode, mods and add-ons.
- Debug views: a "Show" menu in the editor panel (`editor/src/ControlPalette.tsx`), each a flag
  saved in `lib/UIPrefs.ts`: Smoothed Terrain Grid, Scenery Bounds, Structure Grid, Colliders
  (shapes from archetypes' collider descriptors), Debug Physics (rapier's debug render,
  `engine/src/overlays/DebugPhysicsOverlay.ts`), Cutaway Rects, Portals, Marks, Instance IDs,
  Navigation Mesh, Pathfind Routes, and Expanded Edit Panel.

## Game rules (later)

- `game/src/aspects/`: book, combat, container, conversant, door, follower, mechanisms, music, shop,
  sign, stairs (Aspects, below).
- `game/src/sim/`: combat (Combat, below), and two registries scripts called: `predicates.ts`
  (`isFollowing`) and `sideEffects.ts` (`shop`, `rewardCoin`, `rewardItem`, `startFollowing`), as
  host functions and host commands are in SAGA.
- `game/src/systems/`: `GameClock`, `RealmLighting` (see X12), `MusicSystem`, `OceanSounds`,
  `ParticleDamage`, `PlayerDeath`.
- `design/`: `combat.md`, `dialog.md`, `items.md`, `talents.md`, `game-save.md`, `Scripting.md`,
  `scripting/predicates.md`, `Particles.md`, `UI.md`, `Bootstrapping.md`, `missing.md`;
  `archive/SAGA.md` (see X7).
- `TODO.md` at the root (21 KB): the old engine's own open items.

## Combat (*read* 2026-11-25)

`game/src/sim/`, to be redesigned rather than ported. Its wounds (`IWound`, `damageTypes.ts`) are an
abandoned idea: each hit added a wound effect ("Minor Cut") that healed by ticks, and health was the
adjusted max less every wound's strength (`updateActorHealth`).

- A skill (`skills.ts`) is data: caption, icon, group (attack, defend, utility), an `enabled` test
  (the weapon class equipped), targeting and range, duration, clips, a delay before its effect, a
  method, and a list of effects, each a vector with a strength, damage type and duration.
- A method (`skillMethods.ts`: `meleeAttack`, `rangeAttack`, `parry`, `block`, `wait`) builds a
  damage event, `{ source, target, damage: [], resist: ['parry', 'block'] }`, and has each of the
  skill's vectors (`skillVectors.ts`: `weaponDamage`, `naturalDamage`, `poison`, `venom`) push a
  damage entry onto it: type, base, adjusted, and how it applies. Weapon damage is the weapon's
  times the vector's strength times the combatant's `attackPower`.
- The pipeline (`applyDamage`, `damage.ts`): each of the attacker's active effects runs `onAttack`,
  then each of the defender's runs `onDefend`; either may scale entries' `adjusted` or cancel the
  event by returning false. Order is attacker's effects, then defender's, each in list order.
  `parry` (`effectors.ts`) is such an effect: a defend skill adds it for 2 s, and on a parryable
  event a roll cancels the blow, scales it down, or fails, and the parry ends.
- After the pipeline, damage above zero adds threat toward the source, plays the take-damage clip,
  and puts the player into combat mode, targeting the source if nothing is targeted. At zero health
  the actor plays `dying`, and the player's focus on it clears.
- Poison is two effects, so that each can be dispelled apart: a damage-over-time effect that each
  tick adds to a damage effect, which heals on its own. Curing the poison leaves the damage; healing
  the damage leaves the poison running.
- Damage types: impact, cutting, piercing, fire, frost, poison, acid, vitality, spirit.
- In play (`faery-screenshots/combat.png`, from the author): a first click on an opponent, under a
  sword cursor, does not attack; it focuses the target, marked by a pulsing ring on the ground, and
  enters combat mode, which shows the combat bar of the skills available. A second click or
  interaction on the focused target performs the default attack: a fixed slot, or the last plain
  attack used, the author does not recall which. A nameplate, its health bar, floats over a targeted
  actor below full health, and gives the name on hover; not every actor has one, which would
  clutter. The player's portrait carries a health bar and the statuses on it.

## Aspects (*read*, *counted* 2026-11-20)

The mechanism is `engine/src/archetypes/Aspect.ts` and `Instance.ts`; the aspects are in
`game/src/aspects/`, `game/src/behaviors/` and a few engine folders (`portals`, `audio`, `lighting`,
`particles`, `world/scenery/markers.ts`). The counts are over `los-data`: 659 archetypes in
`meta/archetypes/` and `meta/regional/`, and every precinct.

- An aspect had two sets of fields. `config` was set on an archetype; a child archetype or an
  instance overrode it only by restating the whole aspect. `properties` were per instance: the
  precinct wrote each one's starting value (`instanceVars` on actors, `properties` on scenery), play
  changed it, and a save kept it only where it differed from that value.
- Properties were one flat namespace per instance, so aspects composed by field name:
  `wall:directDoor`'s formula for `open` drove the `door` aspect's `open`. The UI did the same: a
  chest's "Open" set `open` and showed the container screen after the aspect's `openDelay` (a
  timeout, `engine/src/player/GameUiState.ts`), and the screen, shown while `open` held, closed by
  writing `open = false`, which closed the lid too
  (`game-ui/src/modals/container/ItemContainer.tsx`).
- An instance could add aspects (`setAspect` copies the archetype's map first) but not remove an
  inherited one: `removeAspect` only drops an override.
- No field was set at both stages. What instances set was start state (`open` 339 times, 3 of them
  true; `locked` 45, all false; `active`), references (`portalTarget` 24, `content` 12, `source` 10,
  the spawners' tables, Poul's `stock`) and placement geometry (`radius`, `halfSizeX`/`halfSizeZ`,
  `waypoints`, `track`). No instance changed an aspect's config but two Kreekin given `enemy.loot`
  (Migration scripts). `key` was never set, so no content had a chest or door that a given key
  opens.
- Aspects whose fields spanned both stages: `portal` (shape on the archetype, target on the
  instance), `container` (type and delay; key and start state), `sign` and `book` (frame; content)
  and `encounter:encounter` (interval; what it spawns).
- Instances added aspects 23 times: the 17 per-character behaviors (`tambry:poul` and the rest),
  which have no fields and name precinct fixtures in code (`'poul.bed'`, `'bsmith.forge'`), even
  where the character has an archetype of its own; and 6 logic doors and groups (`wall:directDoor`,
  `wall:flipFlopDoor`, `playground:spearTrap2`, `playground:proximityDoor`, `triggerGroup`).
- Archetypes restated an inherited aspect 17 times, all actor combat (`combatant`, `enemy`).
  `FeralDogAlpha` restates `combatant` as `{}`, which drops `Proto.FeralDog`'s `healthMax: 50` for
  the default 100; whether that was meant is unknown.
- 57 actors carry `movementSpeed`, `fireResist` and `poisonResist` of 0, which look like defaults
  the editor wrote out.

## Catalog entries (from the author, not yet read in the code)

Books, dialogues and the other catalog kinds are not exemplars. They share a base type with
exemplars, `IMetadata`: anything loaded from JSON and saved. A file may hold entries of several
kinds (dialogues and remarks together), but never exemplars. The files are grouped by region, which
has nothing to do with loading: every catalog is loaded all the time.

In the code, floors and terrain effects are archetypes (`FloorArchetype`, `TerrainFxArchetype` in
`engine/src/world/scenery/`), but with fixed fields and no aspects, and nothing instances them.

A book holds the text of a sign, book, pamphlet or anything else the player reads: the background
art it is shown on, optionally fonts, and its paragraphs as JSON. Whether the paragraphs carry rich
text is unknown; Macondo, the usual font, has no styles. The layout (one page or two facing pages)
is hard-coded in the engine, not chosen by the book.

## How far faery got

From the author's notes (2026-10-30), not from reading the code.

Combat:

- The game design was very incomplete. The concept: a deck-building game without the deck. Before
  an encounter you curate your skills into a smaller set. Which of those are available in a
  given round is partly random and partly situational.
- Timing mixes real time and turns. In combat mode the game runs in real time while you keep
  acting. If you pause, the monsters eventually run out of action points and pause too.
- Combat skills were a sketch, and skill trees were very incomplete.
- Ranged weapons, physical and magical, were not done. Missiles (arrows, fireballs) need
  rethinking: they can't just be particles.
- Monster types needed their own combat strategies and behaviors. Enemy swarming was buggy:
  opponents piled up trying to reach the player.
- No game balancing.

World and movement:

- Navigation maps and pathfinding were mostly done, but navigation was a bit janky.
- Ladders and vertical travel were not done, neither pathfinding nor physics.
- Portals were buggy.
- Water currents and drowning were not done.
- Vehicles were undecided. The original Faery had a raft, a turtle and a golden swan to ride.
- Random encounters were incomplete.

Traps and puzzles were partly done. The aim was to build more and more elaborate puzzles out of
combinations of stock elements, wired together with reactive formulas.

- Done: push buttons, toggle buttons, trip plates, automatic doors, spears that slide up from
  the floor (the converter drops their aspects; see Migration scripts).
- Not done: sliding blocks, stones you push into place, for example in front of a fireball
  emitter to block its missile.
- Other mechanics were still to be researched.

Story and content:

- Quests worked. Dialogues mostly worked. Signs and placards worked.
- Phasing (scenery and characters change with the active quest stage) was started, but editing
  support was poor.
- Situational music was partly done.

UI and art:

- No save games.
- Shopkeeper and bargaining UI not done, but the artwork exists.
- Close-up portraits looked bad (the art, not the code).

Scripting and modding were all JavaScript, so nothing carries over to Bevy.

## Migration scripts (*read*, *run* 2026-11-07)

`scripts/convert_*.mts` turn `packages/los-data` into Panoply assets, writing straight into
`../panoply/assets`. Node runs them without a build step, from any directory:
`node /Users/talin/Projects/games/faery/scripts/<name>.mts`. Run the archetypes before the
precincts, which resolve names against the exemplars.

| Script                       | Reads (`los-data/`)              | Writes (`assets/`)                    | Runs      |
| ---------------------------- | -------------------------------- | ------------------------------------- | --------- |
| `convert_archetypes.mts`     | `meta/archetypes/`, `regional/`  | `exemplars/**/*.exem.json`            | merges    |
| `convert_precincts.mts`      | `scenery/<realm>/*.msgpack`      | `scenery/precincts/<realm>/*.msgpack` | frozen    |
| `convert_realm_maps.mts`     | `realms/*.msgpack`               | `terrain/maps/*.terrain`              | frozen    |
| `convert_terrain_groups.mts` | `world.json`                     | `terrain/terrain.groups.json`         | frozen    |

The frozen three warn and exit: they convert everything they read, and the editor edits their
outputs (PAN37-PAN40, PAN94). The precinct converter runs again once it merges (X69). The archetype
converter merges only the aspects a run names (D56), on exemplars Panoply already has, and never
adds a catalog, an exemplar or a top-level field: `--aspect <Aspect>` adds that aspect where an
exemplar lacks it, `--replace <Aspect>` rewrites it wherever it differs, and `--dry-run` reports
without writing. A run naming nothing refuses. Hand edits survive (`CollisionSize`, PAN3f;
humanoids' `"Run": "RunMale"`, PAN3i; the cottage roof ramps, PAN3j). From `meta/regional/` it takes
actors only; the rest is dialogue, quests and books. What it does not yet convert, it logs. A
catalog the run changes is written back whole as `JSON.stringify` lays it out, which reformats one
that keeps short arrays on one line (eight, the wall and fixture catalogs among them).

The precinct converter overwrites, which is safe while nothing in Panoply edits a precinct (X69). It
resolves each exemplar name against Panoply's catalogs: a name its catalog lacks moves to the one
catalog that has it (faery's precincts named `wall-tiles` for `furnishings`' `Font`), and a name no
catalog has is dropped with its instances. Precincts holding only tiers in faery's older layout
(`walls` and `floors`, which Panoply does not read) are skipped, and their output deleted.

The realm map converter also wrote `terrain/maps/underworld.terrain`, a 3x3 placeholder for a realm
that has no scenery in faery either; the map and the realm were deleted (PAN84). X1 needs the
archetype converter (or a successor) to emit BSN.

Conversions baked into Panoply's data: wall and fixture facings from quarter turns to degrees
(`* 90`); actor facings from radians to degrees, turned a quarter (faery's 0 faced +x, Panoply's
+z), and written even when 0; tier `level -= 2` and floor type index `-= 1`; instances without an
`iid` numbered in sequence (to stop: internal ids are not saved, D87); terrain effect `road` renamed
`cobbles`; model paths rewritten (`a:b` to `a.glb#b`, folders moved, `Boulder1` to `Boulder.1` and
similar); floor types become `scenery/floors/<kebab-case>.floor.ron` (the `.floor.ron` files are not
made by these scripts; `floor-tiles.json` is skipped).

Dropped (the scripts log most of them), by the module that would want them. Some are features
Panoply will not carry over because it has a better way; which is which is undecided per item.

- Actor archetypes: `equipSlots` (with X42), `skills` (with combat), `gender` (with dialogue's
  pronouns; clips already differ by prototype through `ClipNames`), `imposterHeight`/
  `imposterRadius` (three.js level of detail, not carried over), `portraitHeight`/`portraitOffset`
  (with dialogue portraits). `scale` was restored by hand as the `ActorScale` aspect (PAN3l).
  Aspects: `combatant` (some with `healthMax`, `attackPower`), `enemy` (with `loot` tables, e.g.
  `loot:Wolf`), `conversant`, `shopkeeper`, `follower`, `aggressive`, `territorial`.
- Actor instances in precincts: `combatant` is kept on two playground Kreekin, by mistake: it
  belongs on their exemplar, so the converter is to drop it. Dropped: `enemy` (`loot`),
  `territorial`; behavior hooks on 16 actors (`demo:wolf`, `heroncreek:cwParty`, `tambry:*`, ...),
  every per-character behavior faery had; `inventory` (32 actors); `instanceVars.stock` (Poul's
  shop); `healthMax` (9 actors in `demorun`); inline `goals`, the patrol routes of five guards in
  `demorun/p002-p002`.
- Items (`items`, `quest-items`, `weapons`, `books`, `creature-parts`, `common`): `type`, `model`
  (as `ItemModel`) and `equipSlot` (as `EquipSlot`, `weaponR` renamed to the bone's `weapon.R`) are
  kept; icon, weapon stats, document content and quest hooks, weight, price, stack size and sounds
  are dropped.
- Wall and fixture aspects: `door`, `stairs`, `ladder`, `toggleButton`, `pushButton`,
  `pressurePlate`, `controlledOpenable`, `wall:autoDoor`, `container` (`openDelay`,
  `containerType`), `sign` (`frame`), `book`, `soundSource` (`sound`, distances), `particles`
  (effects, damage), `music`, `encounter:encounter`, `fixture:waterFx`. Kept: `Portal`,
  `LightSource`, models, colliders, marks, wall size, and the markers (`waymark`, `circle`, `rect`,
  `waypoints`, as `MarkerPin`, `MarkerArea` and `MarkerPath`, PAN102a; `showOrientation` dropped).
- Scenery instance props: `Group` and `PortalTarget` are kept (Panoply registers neither yet);
  per-instance aspects (`triggerGroup`, `wall:directDoor`, `wall:flipFlopDoor`,
  `playground:spearTrap2`, `playground:proximityDoor`), `inventory` (4 containers) and per-instance
  `facing` are dropped, as are the `properties` other than `portalTarget`: `source` (logic wiring,
  10), `open: true` (3), `content` (11 signs, naming regional text), the spawners' `spawnTable`,
  `spawnCount`, `spawnCountMax`, `cooldown`, `radius` and `spawn`, the music regions' `track`, and
  `takeable` (1). The regions' `halfSizeX`, `halfSizeZ` and `radius` and the `waypoints` were
  restored afterwards as instance overrides (PAN102a's `migrate_marker_values.py`). `aIndex` and
  `pos`, on 135 `demorun` instances, look like stale editor fields.
- Terrain effects: `TerrainBlocked` (`blocked`). Precincts: `layers`.
