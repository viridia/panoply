# Bevy 0.20 migration ledger

The project was last fully working on Bevy 0.15. Rather than port everything at once, all modules
were disabled except a minimal core, and are being restored one group (a "chunk") at a time.
Disabled code stays in place, marked `TODO(bevy-0.20)`; `grep -rn "TODO(bevy-0.20)"` lists what
remains. Excluded crates are listed under `[workspace] exclude` in the root `Cargo.toml`.

## Landed

| Commit    | Chunk                                                                                   |
| --------- | --------------------------------------------------------------------------------------- |
| `6f0a2db` | Checkpoint of the abandoned 0.16-dev WIP port.                                          |
| `6a04afa` | Minimal core on 0.20: `panoply_core`, `world`, `view`, `diagnostics`, window settings.  |
| `0f32a52` | `reflect_types`, `materials`, `models`; shaders ported to WESL; infinite grid.          |
| `dd242ce` | This ledger and `AGENTS.md`; removed `bevy_saga`.                                       |
| `c46a218` | Input via `bevy_action_map` (`src/input.rs`); removed `panoply_core::pan_rotate`.       |
| `c46a218` | `bevy_remote_driver` forked in; `plans/smoke.py`.                                       |
| `cd25a24` | `panoply_exemplar`; `plans/exemplars.py` checks loading and `extends`.                  |
| `2ac5f1b` | Terrain: `panoply_terrain` + `src/terrain`, shaders to WESL; `plans/terrain.py`.        |
| `2ac5f1b` | Editor-only location overlay and zoom; `GotoLocation` message; `plans/perf.py`.         |
| `a3ef8f2` | Perf plans; dedicated `OutlineMaterial` on child entities; isolated plan settings.      |
| `143a84d` | Hemisphere ambient light; shadow cascades fitted to camera distance; 2x2 filter.        |
| `5139647` | Parcels outside the view are hidden (D19); `plans/parcels.py`.                          |
| (pending) | Parcel show set (D20); random flora Y rotation, restored from the three.js version.     |
| (pending) | Physics stays on rapier (D22, X16); `docs/faery.md`, a map of the three.js engine.      |
| (pending) | `scenery` without actors (D24): tier entities, D11 wait, floor outlines; `plans/scenery.py`. |

## Next

1. Exemplar catalogs load once per label. Bevy 0.20 starts a separate load of the whole file
   for each `file#Label` requested before the file has loaded (`bevy_asset` `load_internal`,
   `HandleLoadingMode::Force`; Bevy issue #10549). The playground's first precinct loads
   `wall-tiles.exem.json` 128 times and `furnishings` 34. Fix in `panoply_exemplar`: load each
   catalog root first, then take the labeled handles once it has loaded.
2. Upgrade `rapier3d` from 0.22 to the current release (D22). Used directly, not through
   `bevy_rapier`, so not tied to a Bevy version. Integration is still small:
   `panoply_core::realm_physics` (`RealmPhysics`, one rapier world per realm), a fixed rigid body
   per parcel (`parcel_cache.rs`, `Parcel::physics`), and ground colliders (`ground_mesh.rs`,
   `Parcel::terrain_collider`); also the editor's terrain thumbnail. Cheaper now than after
   actors are restored, and picks up any parry fixes before actors depend on them.
3. `actors`. Risk: in the three.js version actors fell through seams between parcel ground
   colliders. Ground here is a trimesh per parcel; parry's `FIX_INTERNAL_EDGES` flags likely
   cover only edges within one shape, not between colliders. Write a plan that walks an actor
   across parcel edges and corners. If it falls: make shared edge vertices bit-identical,
   overlap parcel colliders by a cell, enable CCD, and cast with a skin. Actors also need
   navigation, which Panoply lacks; faery's is in `engine/src/nav` (see [faery.md](faery.md)).
   Precincts carry actors (playground `p000-p000` has 13): restore the `actors` field in
   `PrecinctAsset` (now skipped, `TODO(bevy-0.20)`), and spawn them from the actors module with
   a home-precinct relationship whose despawn is linked (D24), not from scenery.
4. `portals`: a rewrite, not a restoration (D21).
5. `editor` (see X3).

When `actors` is back, remove the `TEMP(bevy-0.20)` items from `main.rs`
(`spawn_material_test`, `ExemplarCheck`) and `plans/exemplars.py`. Until the modules that
register them are restored, unknown aspect types are skipped with a warning (D25).

## Decisions

- **D1** Build against crates.io Bevy releases, not git main.
- **D2** Settings use `bevy_settings` (`SettingsGroup` resources, `settings.toml`).
  `bevy_prefs_lite` and `bevy_basic_prefs` are abandoned. Window position falls back to winit's
  `outer_position()` via `bevy::winit::WINIT_WINDOWS` (a stopgap API, Bevy issue #17667).
- **D3** Shaders are WESL (`.wesl`); naga_oil is gone from Bevy. Material bindings use
  `@group(constants::MATERIAL_BIND_GROUP)`. Bevy modules are `bevy_pbr::render::*` etc.
- **D4** Exemplars stay as the composition mechanism for now. Keep the `panoply_exemplar` API the
  boundary, so that scenery/actors depend on "entity with aspects", not on exemplar internals.
- **D5** GLTF models spawn via `WorldAssetRoot`; per-model render layers are copied to descendants
  in an observer on `WorldInstanceReady` (`models::copy_model_render_layers`).
- **D6** Removed the "inline" asset source (`InlineAssetReader` / `InlineAssetParams`); it had
  been unused since `d0d2b9b`.
- **D7** Asset preprocessing happens in `packages/build-assets/pipeline.ts`, run by `overrun` (a
  TypeScript build orchestrator in the style of grunt/gulp), not in Bevy's asset processor.
  Bevy's processor handles 1:1 transformations well, but not merging or splitting files, which
  this pipeline needs. In watch mode (`npm run build:watch`), overrun notices Blender exports,
  writes the processed output into `assets/`, and Bevy hot-reloads it.
- **D8** `crates/bevy_saga` was a failed experiment, never integrated into the game, and has
  been removed (along with `assets/scripts/` and `src/scripting.rs`). Compiling to wasmtime is a
  dead end: its support for garbage-collected types is too immature to use. Scripting and
  modding will build on the SAGA prototypes in `bevy_reactor` instead, a bytecode VM interpreter
  with a possible later move to something like Cranelift (see X7).
- **D9** Input goes through `bevy_action_map` (actions and contexts in `src/input.rs`).
  `panoply_core` exposes the `Viewpoint` API (`rotate`, `move_local`) and has no input
  dependency.
- **D10** Automated runtime checks use `crates/bevy_remote_driver`, forked from
  `bevy_action_map` at `f641559` (where it lives only for convenience of development). The plugin
  is inert unless `BEVY_REMOTE_DRIVER_PORT` is set. Plans live in `plans/`, and run with
  `python3 crates/bevy_remote_driver/client/run.py plans/<plan>.py`. Fork changes: the client
  can launch a binary (`BIN`), not only an example; an `exits` step for a key that quits; and a
  `message` step that writes any reflected message, e.g. `panoply_core::GotoLocation` to
  teleport the viewpoint.
- **D11** Layering: `panoply_core` (realm, both grid sizes) <- `panoply_terrain` <- scenery.
  `PRECINCT_SIZE` lives in `panoply_core::metrics`, which removed terrain's only import from
  scenery. Scenery pushes terrain effects into parcels through `ParcelTerrainFx`, defined in
  `panoply_terrain`. At runtime, ground meshes and flora still wait for scenery: a parcel spawns
  with `RebuildParcelTerrainFx`, and only scenery's `terrain_fx_map` clears it, once the
  precinct's effects are written. This is deliberate (no ground before its roads and holes).
- **D12** WESL has no in-file `#define`. Shader-local feature toggles (`water.wesl`) are `const`
  bools with `if` branches, which fold at compile time.
- **D13** The map editor and its aids are development-only, behind the `editor` Cargo feature
  (on by default, not shipped). Zoom and the location overlay are editor-only; in the game the
  camera distance is fixed (`CAMERA_DISTANCE` in `main.rs`).
- **D14** Outlines are central to the art style (art nouveau, specifically Mucha). They use
  normal extrusion (an inverted hull), not jump flooding; this is an aesthetic choice. Terrain
  contours have no outlines; actors and scenery do. Flora is a special case: leaf textures have
  outlines baked in, and trunks use the outline material. Each outline is a child entity that
  shares the mesh, with a dedicated `OutlineMaterial`. Details in [outlines.md](outlines.md).
- **D15** Under the remote driver (`BEVY_REMOTE_DRIVER_PORT` set), the game uses its own settings
  (`org.viridia.panoply.driven`), so plans never change the developer's saved settings. Plans that
  measure set the camera distance through `GotoLocation::camera_distance`.
- **D16** Ambient light is a hemisphere light (`panoply_terrain::HemisphereLight`: sky color,
  ground color, brightness), as in the three.js version, not a camera-wide `EnvironmentMapLight`
  (about 4 ms a frame at any cubemap size). The ground shader applies it per pixel in place of
  Bevy's ambient term; other materials get `GlobalAmbientLight` at the average color. It is
  reflected, so it can be tuned live over BRP.
- **D17** Normal camera distance is about 20 (not final): close enough to read characters and
  NPCs, far enough to navigate. Performance targets assume it: a rock-solid 120 fps, with terrain
  leaving room for scenery and actors. Measured at playground (110, 150), a forest/shore spot.
- **D18** Shadows: one cascade fitted to the camera distance, 0.4x to 2.5x (8-50 m at 20), refitted
  on zoom (`panoply_core::viewpoint::fit_shadow_cascades`); `ShadowFilteringMethod::Hardware2x2`.
  Crisper shadows, as the painterly style wants, and 1.7 ms cheaper than Bevy's default four
  cascades with Gaussian filtering. A depth prepass costs 0.5 ms more, so rendering stays plain
  forward.
- **D19** The parcel cache (LRU, 128 parcels) keeps parcels outside the view alive, so it holds
  about as many entities at distance 20 as at 40. `spawn_parcels` now hides them
  (`Visibility::Hidden` from `Parcel::visible`), and parcel children (ground, water, flora) use
  `Visibility::Inherited`, never `Visible`, which would ignore the parent.
- **D20** Loaded is not shown. The load rectangle stays generous (prefetch for scrolling), but a
  loaded parcel is shown only if its bounds intersect the frustum of an active camera whose
  render layers include the parcel's realm, so portal cameras count without special cases.
  Bounds: the parcel's 16 m square, its contour's height range plus 4 m for flora (tallest
  model 4.45 m at scale 0.8), 2 m slack, swept along each shadow-casting directional light in
  the realm (sun, moon) by the shadow length, clamped for a sun below about 15 degrees.
  `parcel_cache::parcel_show_bounds`. Scenery taller than 4 m will need its own allowance.
- **D21** Portals will be rewritten from scratch: Bevy has changed a lot since the old code, and
  third-party portal crates are unlikely to handle multiple realms on separate render layers.
  Survey them for technique anyway, and propose the design before writing code. Constraints:
  portals are small (a double doorway, about 2 m square), so a portal camera should get an
  off-axis projection fitted to the portal opening; then its `Frustum` is the tight one, and
  D20 shows only what can be seen through the opening. `spawn_parcels` loads only around the
  main viewpoint (its "including portals" TODO); a portal view needs its own load region, the
  ground under that small frustum out to a distance cap.
- **D22** Physics stays on `rapier3d`, not `avian3d`. Realms need separate physics worlds (one
  per realm, `RealmPhysics`), all within one Bevy ECS world, so actors can cross portals without
  duplicating resources. Avian has one physics world per app; multiple worlds is open as avian
  issue #935 (needs design; an unmerged prototype gave each world its own `Collisions` and
  `SpatialQueryPipeline` as components). See X16. The three.js version's physics, read in full,
  is summarized in [faery.md](faery.md).
- **D23** Panoply's modules were written before a full grasp of ECS, and the three.js version
  was not ECS at all, so parts may be mis-architected. When restoring a module, first review it
  for ECS fit (state in resources or caches that belongs on entities, objects passed between
  systems that should be components, hand-kept hierarchies that relationships now cover,
  polling where observers or change detection fit) and recommend keeping or restructuring it;
  restructuring is proposed before code. From the three.js version take the design, not the
  structure.
- **D24** A precinct is the unit of authored content: one asset holds all of a 64 m area's
  scenery, floors, terrain effects and actors (everything but terrain). The goal is open worlds
  larger than fit in memory, built by a solo creator: walls, floors and fixtures are placed on a
  grid (an homage to isometric tile games, with perspective and looser quantization), and
  floors are extruded polygons that the editor stamps and merges like tiles, with free vertex
  editing when wanted. So actors are owned by their home precinct and despawn when it leaves
  the cache; most NPCs stay near their spawn point. Followers (global asset) and random
  monsters (transient, per encounter) are the exceptions. Ownership is a relationship whose
  despawn is linked, replacing `Precinct.actors`. Within a precinct, each tier (a floor level
  with its own cutaways) is an entity, a child of the precinct, with its floor regions as its
  children. Persisted play state follows the same unit (X18).
- **D25** An aspect type no plugin has registered is skipped with a warning
  (`panoply_exemplar::aspect_list`), not an error that fails the whole catalog or precinct, so
  modules can be restored one at a time (the wall-tile catalog uses portals' `Portal`). Saving
  such an asset drops the skipped aspects; see X20.

## Deferred

- **X1** Migrate exemplars to BSN. *Gate:* BSN assets released, and BSN supports shared
  (non-instanced) templates. Includes updating the exemplar migration script, which lives in the
  `faery` repo (the earlier TypeScript/three.js engine), to emit BSN.
- **X2** Synthesized shared assets (successor to D6). *Gate:* a restored module needs them.
  Consider deterministic `AssetId`s from hashed params instead of a fake asset source.
- **X3** Editor UI toolkit: `bevy_ui_widgets` / `bevy_feathers`, or `bevy_reactor` as a git
  dependency (not on crates.io; prototype). *Gate:* editor restoration. Also replace
  `bevy_mod_picking` with `bevy_picking`, `@PreferencesGroup` with `SettingsGroup`.
- **X4** Replace `PropagateRenderLayers` with Bevy's `Propagate<RenderLayers>`. *Gate:* scenery,
  actors and flora restored (they are the callers).
- **X5** `bevy-inspector-egui` (pinned bevy 0.15). *Gate:* a compatible release is needed.
- **X6** Revisit asset bundling. The GLTF layout carries three.js-era assumptions, made to cut
  HTTP requests: small models (e.g. flora) are combined into one file, and large Blender
  catalogs are never split. Under Bevy, per-model files and labeled sub-assets may suit better.
  *Gate:* restoring terrain/flora/scenery, or a change to the asset pipeline (see D7).
- **X7** Scripting and modding using the SAGA language from `bevy_reactor` (SAGA: "Scripts for
  Animated Graphic Adventures"). *Gate:* the `bevy_reactor` SAGA prototype is ready to integrate.
- **X8** Horizontal trackpad scroll rotates the viewpoint through a bespoke `MouseWheel` reader
  (`input::scroll_rotate_viewpoint`). *Gate:* `bevy_action_map` supports scroll input.
- **X9** Finish splitting terrain and scenery into crates: move the parcel systems from
  `src/terrain` into `panoply_terrain`, and scenery into its own `panoply_scenery`. Flora's only
  tie to the app is `models::PropagateRenderLayers` (see X4). *Gate:* scenery restored.
- **X10** Reflections for shiny objects: `LightProbe` volumes with `EnvironmentMapLight` (using
  `skybox_diffuse_32.ktx2` and `skybox_specular.ktx2`) where shiny objects are, instead of on the
  camera. *Gate:* scenery or actors with shiny materials restored.
- **X11** Water: apply the hemisphere light, or tie water's sky colors to it. *Gate:* author's
  direction.
- **X12** Art direction for lighting: painterly (pre-Raphaelite, art nouveau); shadows currently
  too dark (tune sun vs. hemisphere brightness), and since D18's sharper shadows the sunlight
  reads brighter and harsher. Underground realms a little more gothic, still
  painterly (`RealmLighting::Interior`). Day/night cycle: sun shadows fade out in the evening
  before they grow long; the moon casts shadows only at night. Night uses the cinematic blue
  filter: red and green illumination reduced, blue brighter (artistic, not realistic).
  Outlines read a little thinner than in the old engine, and the dark shadows make them hard
  to see; compare the old engine's extrusion (units, any distance scaling) with
  `outline.wesl` when tuning. Some models still put `outline` on objects instead of materials,
  which the game ignores; the author is moving it in Blender.
  *Gate:* scenery restored, for whole scenes to judge.
- **X13** Sweep the old JavaScript/three.js engine (`../faery`) for design elements worth
  carrying over. It was far more complete than this version; it was abandoned for jankiness,
  not design. About 300k lines (TypeScript, GLSL) in 14 packages (`game`, `los-game`,
  `los-editor`, `genmap-cli`, `server`, `game-ui`, ...) plus `design/` docs. Needs its own
  session or several; produce a written catalog, not code. *Gate:* the Bevy 0.20 port is done.
  A first-pass map, with pointers per Next item, is [faery.md](faery.md); read the relevant
  section before restoring a module, and add to it rather than rescanning.
- **X14** Herb picking: picking a herb hides that instance for a while, then it regrows. Pick by
  grid cell (at most one herb per 1 m cell; a ray hit gives the cell), not by entity. Picked
  cells live in a per-realm resource keyed by world cell with a regrow time, outside the parcel
  so eviction does not regrow them (and in the save game if they must survive a reload). Flora
  placement skips picked cells. With merged flora (X15), hide an instance by collapsing its
  vertex range in the parcel's herb mesh. *Gate:* `bevy_picking` (X3) and gameplay input.
- **X15** Merge flora into per-parcel meshes. Flora costs about 1.5 ms at distance 20 and about
  5 ms at 40 (Notes); hiding and the show set (D19, D20) cut little, so the cost is the shown
  flora entities. Plan: cache each model once as a template (part meshes, transforms, material
  key) in an `Arc`; `gen_flora`'s async task bakes placements (`Mesh::transformed_by`) and
  merges (`Mesh::merge`), like ground meshes. Trunks (solid color, no UVs) merge into one
  vertex-colored mesh per parcel with one outline child; leaves group by texture; herbs get
  their own mesh with per-cell vertex ranges (X14). Flora outlines need their own
  `OutlineMaterial`, width times the mean flora scale (0.65), since the shader extrudes in
  mesh-local space. Drop a hidden parcel's merged meshes and rebuild on show (about 2-3 MB per
  parcel). Removes about 5,000 transform-only scene entities and flora's
  `PropagateRenderLayers` use (X4). *Gate:* scenery restored and a full scene measured, to see
  whether the headroom is needed.
- **X16** Revisit `avian3d` (D22). *Gate:* avian merges multiple physics worlds (issue #935)
  with game entities kept in one ECS world, per-world spatial queries, and moving a body
  between worlds (an actor crossing a portal); and a release supports the Bevy in use.
- **X17** Actor goals (the three.js version's goal-tree AI, `engine/src/world/actors/goals`)
  move to ECS and BSN assets: each goal's hierarchy of elements becomes a hierarchy of entities
  and components. Most reactions pair a condition with an effect. Conditions are reactive
  formulas; an effect is usually a Rust type with parameters, but can be a formula. So SAGA
  (X7) needs a way to embed formulas in assets. *Gate:* BSN assets with shared (singleton, non-instanced) scenes, as for X1,
  and SAGA formulas in assets.
- **X18** Save games with structural sharing: one chunk of play changes per precinct, written on
  eviction; a save is only a root index of chunk versions. Design and what it asks of the
  engine in [save-games.md](save-games.md). X14's picked cells would live in the precinct chunk.
  *Gate:* actors restored (the first content with real mutable state).
- **X19** Cutaways: when the player is under a ceiling, scenery above them is hidden, including
  actors on those floors. Rects are authored per tier (`TierSer.cutaways`); choosing them and the
  3D test against element and actor bounds follow the three.js version ([faery.md](faery.md),
  Scenery). Tier entities (D24) carry their rects; elements need bounds. Hidden elements use
  `Visibility::Hidden` so descendants inherit it (D19). *Gate:* scenery and actors restored.
- **X20** Keep unknown aspects lossless (D25): hold each as an opaque value that saves back
  unchanged, so the editor cannot drop data a disabled module owns. *Gate:* editor restored.
- **X21** Scenery follow-ups, each small: a show set for precincts like D20 (scenery taller
  than D20's 4 m allowance also needs its own); `LightSource` point lights per precinct
  (registered, not yet acted on; candles in the old engine); hot reload of `FloorSurface`
  assets (floors do not rebuild when one changes); `SceneryColliders` into rapier (with item 2
  of Next, or actors). *Gate:* scenery restored; take each when it is needed.

## Notes

- Performance at playground (80, 80), camera distance 40, M1 Pro, from `plans/ablate.py`
  (2026-10-10), in the developer's window: 13.7 ms/frame. (Plans now run in the default window
  size, where the same spot measures 15.6 ms; compare runs only with each other.) Camera off:
  3.3 ms, so about 10.4 ms is rendering. A quarter of the pixels saves 5.6 ms, so over half the
  rendering cost is per-pixel. Hiding ground saves 4.5 ms,
  flora 4.3, water 2.8. Water's fragment-wave loop is only about 0.6 ms.
- At distance 20, playground (110, 150), `plans/ablate.py` (2026-10-10): 8.4 ms before D18,
  6.7 ms after. Camera off (CPU floor): 3.3 ms. Flora 2.5, quarter pixels 2.2, ground 1.7, water
  1.4, MSAA 1.1.
- Hiding off-view parcels (D19), same spot and plan: 6.8 ms before, 6.7 after (noise); 8,717
  entities either way. `plans/parcels.py` found 64 of 94 cached parcels out of view there, left
  over from the start location; they cost little, so per-entity cost is in view.
- Flora culling there (scratch probe of `ViewVisibility`, which counts shadow views too): about
  195 of 1,179 trees on screen, but before D20 all 2,194 flora meshes in shown parcels were
  drawn into the shadow map, since Bevy bounds the cascade with a sphere that covered the whole
  loaded area. D20 shows 16 of 30 loaded parcels (1,620 flora meshes): 6.7 to 6.6 ms at
  distance 20, 10.7 to 10.3 ms at 40. Flora costs 1.5 ms at 20 and about 5 ms at 40. Of 8,717
  entities only 3,534 have meshes; most of the rest are flora scene roots and glTF nodes that
  only carry transforms. Flora Y rotation uses noise seed 8; the three.js version used 6, which
  here picks the biome along z.
- CPU profile there (samply, 2026-10-10): no `panoply` system is significant. The floor is Bevy's
  per-entity work over about 8,700 entities, run on the compute pool (about 9.6 ms CPU per frame
  across 4 threads): sun shadow visibility culling, camera culling, material specialization
  checks, instance batching, light/cluster preparation, transform propagation, and executor
  overhead. Render thread about 2.2 ms per frame, mostly command submission. Fewer entities is
  the lever; flora (several mesh entities per tree, plus outline children) dominates.
- Profiling recipe: run the game under `samply record --save-only --unstable-presymbolicate` with
  the driver's environment (`BEVY_REMOTE_DRIVER_PORT`, `CARGO_MANIFEST_DIR`,
  `DYLD_FALLBACK_LIBRARY_PATH` = `rustc --print target-libdir` plus `target/debug/deps`), drive it
  with the client's `Driver` class to the spot, and weight samples by `threadCPUDelta`, skipping
  stacks whose leaf is a wait call.
- Lighting costs at the same spot (`ABLATE=... plans/ablate.py`): image-based lighting (the
  camera's `EnvironmentMapLight`) about 4 ms, independent of cubemap size; MSAA 4x 3.4 ms; Gaussian
  vs 2x2 shadow filtering 1.3 ms. `GeneratedEnvironmentMapLight` re-filters every frame (35 ms).
  `skybox_diffuse.ktx2` was 1024x1024x6 with no mips; `skybox_diffuse_32.ktx2` is a 32x32 box
  filter of it, visually identical, 0.6 ms cheaper.
- Measure frame time by counting frames over wall time without polling: the driver's
  diagnostics return the latest single frame, and polling every 5 ms disturbs the app.
- Bevy 0.20 issues seen: (1) switching `DirectionalLight::shadow_maps_enabled` off at runtime
  panics the render thread ("Failed to get directional light visible entities for cascade",
  `bevy_pbr` `render/light.rs`); (2) an entity with two `MeshMaterial3d` components (e.g. an
  extended material alongside `StandardMaterial`) fails wgpu validation in the shadow pass;
  `models` now puts outlines on a child entity instead; (3) `RenderDiagnosticsPlugin` records
  only CPU pass times on Metal.

- The water shows a fine square grid (visible before and after D16): waves texture tiling or
  mesh seams; not investigated.
- `ExemplarData`'s `Serialize` impl is `todo!()`; serializing an exemplar panics.
- Scenery content: playground `p000-p000` holds 1,048 scenery instances, 13 actors and 6 tiers,
  a usable test map for scenery performance (X15); the other playground precincts hold less.
  Overland has one precinct here; the old engine's overland rooms were not migrated.
- Floor regions have identity transforms: their outlines are in precinct coordinates.
- With `dynamic_linking`, run via `cargo run`; the bare binary can't find `libstd`.
