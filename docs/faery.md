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
  `archetypes`, `behaviors`, `quests`, `dialogue`, `items`, `skills`, `stats`, `particles`,
  `audio`, `lighting`, `materials`, `models`, `animation`, `render`, `overlays`, `player`,
  `scripting`.
- `game`: game rules on top of the engine: `aspects`, `sim`, `behaviors`, `goals`, `quests`,
  `systems`, `books`.
- `editor`: the real editor (`los-editor` and `los-game` are thin entry points).
- `common`, `geometry` (polygon clipping, convex partition, curves), `keys`, `schemas`,
  `reflex*` (UI and property editing), `game-ui`, `obsidian`, `server`, `genmap-cli`.
- `los-data`: realm maps (`realms/*.msgpack`), per-realm scenery, `world.json`.

Also `design/` (notes), `scripts/convert_*.mts` (the converters that produce Panoply's data; see
X1), `artwork/`.

## Physics (*read*)

`engine/src/physics/`, about 1,600 lines. See D22 for what this decided.

- `PhysicsRealm.ts`: one rapier `World` per realm, stepped at 1/120 s. Holds the collider index
  and all queries: ray pick, shape cast, shape-list cast (placing fixtures), ray intersection with
  sensors, shape intersection.
- `defs.ts`: nine collision groups (Static, Door, Actor, Npc, Portal, Visible, Sensor, Marker,
  Pickable) as rapier membership/filter bits, and named masks (`TerrainMask`, `ActorMoveMask`,
  `PickMask`, `EditorPickMask`, ...). `ColliderIndex` maps collider handle to metadata (instance,
  archetype); this is how mouse picking finds game objects (relevant to X3, X14).
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
- `engine/src/player/`: `PlayerActionController`, `CutawayController`, `GameUiState`.
- `engine/src/animation/AnimationDriver.ts`.

## Portals (for Next: `portals`, D21)

- `engine/src/portals/`: `ActivePortal`, `ActivePortalMaterial`, `ActivePortalSystem`,
  `IPortal`, `portalAspect`. Small; background for the rewrite.
- `game/src/systems/FollowerTeleport.ts`: followers crossing with the player.

## Editor (for Next: `editor`, X3)

- `editor/src/tools/`: 17 tools: floor draw/edit/stamp, wall, fixture, actor, waypoint, flora
  placement, terrain, terrain contouring, terrain effects, move/rotate instance, select
  (structure, scenery rectangle), scenery interact. `mutations.ts`, `ToolState.ts`.
- `editor/src/`: `builders`, `controls`, `view`, `helpers`, `reflex-ui-three`,
  `reflex-ui-faery`.
- `design/Editor.md`: tool organization, map mode, mods and add-ons.

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
