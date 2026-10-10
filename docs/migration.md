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
| (pending) | Parcels outside the view are hidden (D19); `plans/parcels.py`.                          |

## Next

1. Merge flora into per-parcel combined meshes: once the GLTFs load, bake each placement's
   transform into its meshes (`Mesh::transformed_by`) and concatenate by material
   (`Mesh::merge`), one entity per (parcel, material) plus `OutlineMaterial` children for trunks.
   Drops flora's `PropagateRenderLayers` use (X4). Aim: forest spot at distance 20 under about
   5 ms, leaving scenery room. Hiding off-view parcels (D19) did not move frame time (Notes), so
   the cost is the in-view flora entities.
2. `scenery` (see D11). Its `FloorOutline` still builds the old extended outline material; switch
   it to `OutlineMaterial`. Port `assets/shaders/lib/*` to WESL if anything still imports it.
3. `actors`, `portals`.
4. `editor` (see X3).

When `scenery` and `actors` are back, remove the `TEMP(bevy-0.20)` items from `main.rs`
(`spawn_material_test`, `ExemplarCheck`, `release_parcel_terrain_fx`) and
`plans/exemplars.py`. Until then, only exemplar
catalogs without aspects can load, since the aspect types are registered by those modules.

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
  painterly (`RealmLighting::Interior`). *Gate:* scenery restored, for whole scenes to judge.
- **X13** Sweep the old JavaScript/three.js engine (`../faery`) for design elements worth
  carrying over. It was far more complete than this version; it was abandoned for jankiness,
  not design. About 300k lines (TypeScript, GLSL) in 14 packages (`game`, `los-game`,
  `los-editor`, `genmap-cli`, `server`, `game-ui`, ...) plus `design/` docs. Needs its own
  session or several; produce a written catalog, not code. *Gate:* the Bevy 0.20 port is done.

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
- `assets/shaders/outline.wgsl` is an unused duplicate of `src/materials/outline.wesl`.
- With `dynamic_linking`, run via `cargo run`; the bare binary can't find `libstd`.
