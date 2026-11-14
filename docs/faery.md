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
- Not yet in Panoply: cutaways, sensors, precinct point lights, tiers and layers.
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
- `engine/src/world/actors/goals/`: goal-tree AI (`GoalNode`, `GoalPriority`,
  `PrioritizedGoalList`, `factory`, `components/`). Not a direct port: it moves to ECS and BSN
  assets (X17).
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

- `game/src/aspects/`: book, combat, container, conversant, door, follower, mechanisms, music,
  shop, sign, stairs.
- `game/src/sim/`: damage, damage types, effectors, predicates, side effects, skills.
- `game/src/systems/`: `GameClock`, `RealmLighting` (see X12), `MusicSystem`, `OceanSounds`,
  `ParticleDamage`, `PlayerDeath`.
- `design/`: `combat.md`, `dialog.md`, `items.md`, `talents.md`, `game-save.md`, `Scripting.md`,
  `scripting/predicates.md`, `Particles.md`, `UI.md`, `Bootstrapping.md`, `missing.md`;
  `archive/SAGA.md` (see X7).
- `TODO.md` at the root (21 KB): the old engine's own open items.

## Catalog entries (from the author, not yet read in the code)

Books, dialogues and the other catalog kinds are not exemplars. They share a base type with
exemplars, `IMetadata`: anything loaded from JSON and saved. A file may hold entries of several
kinds (dialogues and remarks together), but never exemplars. The files are grouped by region, which
has nothing to do with loading: every catalog is loaded all the time.

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
| `convert_precincts.mts`      | `scenery/<realm>/*.msgpack`      | `scenery/precincts/<realm>/*.msgpack` | overwrites|
| `convert_realm_maps.mts`     | `realms/*.msgpack`               | `terrain/maps/*.terrain`              | frozen    |
| `convert_terrain_groups.mts` | `world.json`                     | `terrain/terrain.groups.json`         | frozen    |

The frozen two warn and exit: they convert everything they read, and the editor edits their outputs
(PAN37-PAN40). The archetype converter merges (D56): it adds the catalogs, exemplars, top-level
fields and aspects Panoply lacks, and keeps those it has, so hand edits survive (`CollisionSize`,
PAN3f; humanoids' `"Run": "RunMale"`, PAN3i; the cottage roof ramps, PAN3j). `--dry-run` reports
without writing; `--replace <Aspect>` rewrites that aspect wherever it differs. An exemplar deleted
in Panoply returns on the next run, so a deletion is made in the converter. From `meta/regional/` it
takes actors only; the rest is dialogue, quests and books. What it does not yet convert, it logs. It
writes each catalog back whole as `JSON.stringify` lays it out, so a run reformats every catalog
that keeps short arrays on one line (eight, the wall and fixture catalogs among them), even when it
adds nothing; `--dry-run` writes nothing.

The precinct converter overwrites, which is safe while nothing in Panoply edits a precinct (X69). It
resolves each exemplar name against Panoply's catalogs: a name its catalog lacks moves to the one
catalog that has it (faery's precincts named `wall-tiles` for `furnishings`' `Font`), and a name no
catalog has is dropped with its instances. Precincts holding only tiers in faery's older layout
(`walls` and `floors`, which Panoply does not read) are skipped, and their output deleted.

The realm map converter also wrote `terrain/maps/underworld.terrain`, a 3x3 placeholder the three.js
version needed for a floors-only realm; it was deleted (see PAN84). X1 needs the archetype converter
(or a successor) to emit BSN.

Conversions baked into Panoply's data: wall and fixture facings from quarter turns to degrees
(`* 90`); actor facings from radians to degrees, turned a quarter (faery's 0 faced +x, Panoply's
+z), and written even when 0; tier `level -= 2` and floor type index `-= 1`; instances without an
`iid` numbered in sequence; terrain effect `road` renamed `cobbles`; model paths rewritten (`a:b` to
`a.glb#b`, folders moved, `Boulder1` to `Boulder.1` and similar); floor types become
`scenery/floors/<kebab-case>.floor.ron` (the `.floor.ron` files are not made by these scripts;
`floor-tiles.json` is skipped).

Dropped (the scripts log most of them), by the module that would want them. Some are features
Panoply will not carry over because it has a better way; which is which is undecided per item.

- Actor archetypes: `equipSlots` (with X42), `skills` (with combat), `gender` (with dialogue's
  pronouns; clips already differ by prototype through `ClipNames`), `imposterHeight`/
  `imposterRadius` (three.js level of detail, not carried over), `portraitHeight`/`portraitOffset`
  (with dialogue portraits). `scale` was restored by hand as the `ActorScale` aspect (PAN3l).
  Aspects: `combatant` (some with `healthMax`, `attackPower`), `enemy` (with `loot` tables, e.g.
  `loot:Wolf`), `conversant`, `shopkeeper`, `follower`, `aggressive`, `territorial`.
- Actor instances in precincts: only `combatant` is kept. Dropped: `enemy`, `territorial`, and
  script hooks (`demo:wolf`, `heroncreek:cwParty`, `tambry:*`, ...).
- Items (`items`, `quest-items`, `weapons`, `books`, `creature-parts`, `common`): `type`, `model`
  (as `ItemModel`) and `equipSlot` (as `EquipSlot`, `weaponR` renamed to the bone's `weapon.R`) are
  kept; icon, weapon stats, document content and quest hooks, weight, price, stack size and sounds
  are dropped.
- Wall and fixture aspects: `door`, `stairs`, `ladder`, `toggleButton`, `pushButton`,
  `pressurePlate`, `controlledOpenable`, `wall:autoDoor`, `container` (`openDelay`,
  `containerType`), `sign` (`frame`), `book`, `soundSource` (`sound`, distances), `particles`
  (effects, damage), `music`, `encounter:encounter`, `waymark`, `waypoints`, editor markers
  (`circle`, `rect`), `fixture:waterFx`. Kept: `Portal`, `LightSource`, models, colliders,
  marks, wall size.
- Scenery instance props: `Group` and `PortalTarget` are kept (Panoply registers neither yet);
  per-instance aspects (`triggerGroup`, `wall:directDoor`, `wall:flipFlopDoor`,
  `playground:spearTrap2`, `playground:proximityDoor`), `inventory` and per-instance `facing`
  are dropped.
- Terrain effects: `TerrainBlocked` (`blocked`). Precincts: `layers`.
