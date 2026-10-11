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
| (pending) | Scroll hitches: pipeline warm-up, scene budgets (D26); hitch log; `plans/scroll.py`.     |
| (pending) | Exemplar catalogs load once per file (D27): `ExemplarServer`; same-file `extends`.       |
| (pending) | Removed the no-op `draco()` pass from `pipeline.ts` (D28).                              |
| (pending) | Lighting as in the three.js version (D30): linear tone mapping, `Sunlight`; `plans/lighting.py`. |
| (pending) | Water, first steps (D31): colors as three.js displayed them, alpha, seabed under layer and tint. |
| (pending) | Embedded shaders hot-reload in editor builds (`embedded_watcher`, D32).                  |
| (pending) | Terrain textures as KTX2 with mipmaps from `pipeline.ts`; tiling samplers, no `fract` (D33). |

## Next

0. Move to Bevy main instead of 0.20 (supersedes D1 once done). Panoply will not ship before
   0.21 (BSN assets), and main has what the editor needs: split pane, tabs, tree view, the
   `bevy_inspector` crate (may close X5), and `BEVY_SETTINGS_DIR`, which lets driver plans
   start from a fresh settings directory (may retire D15's separate identity).
   - **How Bevy is named.** Every repo in the family names it exactly
     `git = "https://github.com/bevyengine/bevy.git", branch = "main"`. Cargo treats a
     `branch` and a `rev` as different sources even at the same commit, so a `rev` here would
     build a second Bevy beside the dependencies'. Each workspace's `Cargo.lock` holds the
     commit; bump with `cargo update` deliberately, one chunk per bump, all repos together.
   - **Dependencies, done (2026-10-11).** `bevy_action_map`: `main` is on Bevy main (chunk 196),
     0.1.x stays on `release/0.1` (tag `v0.1.2`). `bevy_immediate` (X3): fork
     `viridia/bevy_immediate`, branch `bevy-main` (`57d724e`), whose `main` mirrors upstream;
     breaks were `NumberInputValue` (now `bevy_ui_widgets::NumericValue`) and
     `SystemParam::init_access` (no world argument; returns a `Result`). Keep the branch to
     those changes and rebase it on upstream; offer it upstream; drop it at 0.21.
   - **Order.** (a) Extract `bevy_remote_driver` into its own repo: done 2026-10-11 as
     `../bevy_remote_driver` (`viridia/bevy_remote_driver`), the union of both copies, with its
     own ledger. `bevy_action_map` keeps its copy (it is published, and others may run its tests).
     Panoply keeps `crates/bevy_remote_driver` until (d), then depends on the repo by git and
     runs plans as `python3 ../bevy_remote_driver/client/run.py plans/smoke.py`. (b) A shared VS Code multi-root workspace (panoply,
     arbor_day, bevy_action_map, bevy_immediate, bevy_remote_driver, seedling for reference),
     with `rust-analyzer.linkedProjects` per `Cargo.toml` and `../bevy` not linked. Consider
     a shared `CARGO_TARGET_DIR`. (c) The `arbor_day` shell (X28), as X3's `bevy_immediate`
     spike. (d) This item in Panoply.
   - **In Panoply.** Expect breakage from WESL 0.6, the window event split, `DisplayTarget` on
     `Window`, and the observer `Trigger` and `SystemParam` changes. Update AGENTS.md (API
     source becomes `../bevy` at the locked commit, not the registry) and add a decision
     replacing D1.
1. Upgrade `rapier3d` from 0.22 to the current release (D22). Used directly, not through
   `bevy_rapier`, so not tied to a Bevy version. Integration is still small:
   `panoply_core::realm_physics` (`RealmPhysics`, one rapier world per realm), a fixed rigid body
   per parcel (`parcel_cache.rs`, `Parcel::physics`), and ground colliders (`ground_mesh.rs`,
   `Parcel::terrain_collider`); also the editor's terrain thumbnail. Cheaper now than after
   actors are restored, and picks up any parry fixes before actors depend on them. Also fix:
   `realm_physics_system` runs in `FixedUpdate` (64 Hz by default) but rapier's default
   `IntegrationParameters` step 1/60 s, so physics would run about 7% fast; set rapier's `dt`
   from `Time<Fixed>`. Only the viewpoint's realm is stepped (a realm seen through a portal
   would freeze).
2. `actors`. Risk: in the three.js version actors fell through seams between parcel ground
   colliders. Ground here is a trimesh per parcel; parry's `FIX_INTERNAL_EDGES` flags likely
   cover only edges within one shape, not between colliders. Write a plan that walks an actor
   across parcel edges and corners. If it falls: make shared edge vertices bit-identical,
   overlap parcel colliders by a cell, enable CCD, and cast with a skin. Actors also need
   navigation, which Panoply lacks; faery's is in `engine/src/nav` (see [faery.md](faery.md)).
   Precincts carry actors (playground `p000-p000` has 13): restore the `actors` field in
   `PrecinctAsset` (now skipped, `TODO(bevy-0.20)`), and spawn them from the actors module with
   a home-precinct relationship whose despawn is linked (D24), not from scenery. Actor
   movement, AI and physics go in `FixedUpdate`; their rendered transforms (and the camera
   following them) must interpolate between fixed ticks, or they judder at 120 fps against
   64 Hz ticks. Check whether Bevy 0.20 has transform interpolation built in before adding any.
   Load actor exemplars through `ExemplarServer` (D27), not `AssetServer::load`. Models per
   D28: one skeleton per actor, features bound to its joints, colors by cached material clones.
   Hair (materials with extras `material: hair`) needs strand tangents and a hair shader (D29).
   The three.js version's actor assembly is `engine/src/world/actors/SkinnedModel.ts` (faery).
   The converters dropped actor fields and aspects (`scale`, `gender`, `enemy`, ...); see
   [faery.md](faery.md), Migration scripts, and decide per field: port, or replace.
3. `portals`: a rewrite, not a restoration (D21).
4. `editor` (see X3).

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
- **D26** No pipeline compiles while scrolling. Bevy 0.20 creates render pipelines
  synchronously on macOS (`bevy_render` `create_pipeline_task`), about 35 ms each, the first
  time a material and mesh-layout combination is drawn. `models::warmup` loads the model folders
  (`terrain/models`, `scenery/models`) at startup and draws every scene of each glTF once, tiny
  and below the ground at the view center, then despawns it; any glTF loaded later is warmed
  the same way. Instantiating scenes is budgeted too, 64 per frame for flora and for scenery
  (`FLORA_SCENES_PER_FRAME`, `SCENERY_SCENES_PER_FRAME`): parcels' flora and a precinct's
  scenery otherwise arrive as 300-500 scenes in one frame. `plans/scroll.py` fails if a
  pipeline is created during its cold walk.
- **D27** Exemplars are loaded by path through `panoply_exemplar::ExemplarServer`, never
  `AssetServer::load("file#Label")`. Bevy 0.20 starts a separate load of the whole file for each
  `file#Label` requested before the file has loaded (`bevy_asset` `load_internal`,
  `HandleLoadingMode::Force`; Bevy issue #10549): the playground's first precinct read
  `wall-tiles.exem.json` 128 times. `ExemplarServer::get` loads the catalog root, held in the
  `ExemplarCatalogs` resource, and returns `None` until it has loaded, then the labeled handle
  (labels are registered before the root counts as loaded). The catalog holds every label's
  handle, aliases included, so labels stay loaded. Precincts wait for their catalogs
  (`ReadPrecinctData`), and a parcel's ground waits for the precinct as before (D11). In the
  loader, `extends` within the same catalog uses `LoadContext::get_label_handle`, since a load
  by path would reload the file being loaded; a missing target is a warning. Checked with a
  temporary log in the loader: `plans/scenery.py` loads each of ten catalogs once.
- **D28** Actor models stay one glTF per species as authored (`humanoid-male.glb` holds every
  male body and feature; `humanoid-armature.glb` the shared clips), with no split transform.
  Bevy always loads a glTF whole, labels included, and uploads all its meshes; the files are
  small (humanoid male about 1.3 MB of mesh data, 40 clips about 0.5 MB, all characters about
  8.5 MB on disk, almost no textures), so keeping a species resident costs little, and one file
  warms in one pass (D26). Load the root `Gltf` and look up `named_scenes`: loading
  `file.glb#Scene` by label hits Bevy issue #10549 as exemplars did (D27). Clips bind by bone
  name path (`AnimationTargetId`), so shared clips drive any file's armature with matching
  bones. Assembly, as in the three.js `SkinnedModel.ts`: one skeleton per actor (the body
  scene's armature and one `AnimationPlayer`); each selected feature (hair, hat, beard) spawns
  only its mesh, with a `SkinnedMesh` whose joints are the body's joint entities matched by
  `Name`. `modelOpt` adds the armature to every skinned scene, so spawning features as whole
  scenes would duplicate the skeleton. Color slots are material names; an actor's `Colors`
  replace a slot material's base color with a clone cached by (material, color). Exemplar hex
  colors are sRGB (`Srgba::hex`); glTF `baseColorFactor` is linear. Colors for slots a mesh
  lacks are ignored. Expect little sharing: nearly every actor has its own colors (see X23).
  *Revisit* (with X6) if actors gain real textures, or a character file passes about 10 MB.
  `pipeline.ts` ran gltf-transform's `draco()`, but no output used Draco (the encoder was never
  registered); the call is removed, since Bevy's glTF loader cannot decode Draco. Output was
  unchanged.
- **D29** Hair shine uses Kajiya-Kay (specular from strand direction T: `sin(T, H)^shininess`,
  a band across the strands), likely with Scheuermann's two lobes (a bright primary, a
  hair-tinted secondary). The three.js `HairMaterial` approximated a band with
  `pow(sin(dot(R, V) * PI), shininess)`, which follows the surface, not the strands, and shows
  in close-ups. Hair UVs are auto-unwrapped, so T comes from a procedural comb in `pipeline.ts`,
  written as each hair mesh's `TANGENT` attribute (Bevy skins tangents): for scalp hair, the
  direction from a crown point (top of the mesh bounds, a little back) projected onto the
  surface; for facial hair, model-space down projected onto the surface; a fallback where the
  projection vanishes. Shading: first try `StandardMaterial` anisotropy (no shader); if it
  reads too realistic for the art style (D14, X12), an `ExtendedMaterial` with Kajiya-Kay in
  WESL.
- **D30** Lighting follows the three.js version, so painted textures read as painted. The camera
  uses `Tonemapping::Linear` (identity, like three.js's default; `None` would also turn off
  deband dithering), not TonyMcMapface, which desaturated the palette. Ambient outweighs the
  sun, as there (hemisphere 0.5, sun 0.4): sky `#c8d1d8`, ground `#b97a20`. Three.js 0.152
  used legacy light units (intensities scaled by π), so at Bevy's default exposure (about
  1/1000) the sun is 0.4 · π · 1000 ≈ 1250 lux and the hemisphere 0.5 · 1000 = 500; the
  defaults are 1.4 times both (1750, 700), which matched the old screenshots better. The sun's
  color and illuminance are `panoply_core::Sunlight`, reflected like `HemisphereLight`, applied
  to each realm's `RealmSun` light. Values are a starting point for tuning by eye (X12).
- **D31** Water and seabed follow the three.js version, adopted in small steps judged by eye.
  The three.js `LakeMaterial` was unlit (`lights: false`, its lighting result discarded). It
  converted its colors to linear but never encoded its output (no `encodings_fragment` in
  three.js 0.152), so linear values reached the sRGB canvas unconverted, darker and more
  saturated than its numbers. Panoply uses the colors as shown
  (`water_material::as_displayed_by_threejs`): water teal (0, 0.3, 0.3), sky (0.7, 0.8, 0.9)
  and (0.6, 0.7, 0.9). Water alpha is three.js's `mix(0.4, 1.0, opacity)` (about 0.5 straight
  down; was `opacity`, about 0.2). Underwater ground shows only its under layer (dirt), as in
  the three.js `TerrainMaterial` ("No top coats underwater"): `under_mix` rises to 1 by 0.33 m
  below sea level. The port had kept the grass top coat, whose high-frequency detail made the
  water read as clear. Departures, by eye: brown dirt under the water looked gray and dark, so
  the seabed tint is blue-green, `Srgba(0.0, 0.25, 0.3)` (three.js: dark blue (0, 0.1, 0.3)),
  up to 0.8 (three.js 0.7); and the water's sky weight is `opacity * 0.5` (was `- 0.1`).
- **D32** Shaders are embedded (`embedded_asset!`, `embedded://<crate>/...`), in the app and in
  every crate under `crates/`. Crates are split out to cut compile times, and a crate cannot
  have its own asset folder: Bevy has no mechanism for one, and a crate that found a path to
  its own folder could not be distributed. Embedding keeps each shader with the crate that
  defines its bindings. Terrain's shaders were asset files (`assets/terrain/shaders/`) until
  terrain moved into `panoply_terrain` (`3e29f50`). Textures are still loaded from the app's
  `assets/` by path, so `panoply_terrain` relies on the app to provide `terrain/textures/`.
  Embedded shaders hot-reload in editor builds: the `editor` feature turns on Bevy's
  `embedded_watcher`, which watches each embedded file's source path, so saving a `.wesl`
  recompiles it in the running game. Shipped builds (no `editor`) do not watch. One
  exception: `FloorNoisyMaterialExt` loads `assets/shaders/floor_noise.wesl` as an asset (the
  app, not a crate, so it hot-reloads through `file_watcher`).
- **D33** Terrain textures have mipmaps, made by `pipeline.ts` (D7). The PNGs exported from
  Krita live in `artwork/terrain/textures/` beside their `.kra` sources; the
  `terrain-textures` target writes `assets/terrain/textures/*.ktx2`: RGBA8, a full mip chain
  averaged with a 2x2 box filter in linear light (so mips do not darken; edges wrap, since
  the textures tile), zlib supercompression (Node's built-in `zlib`; Bevy's `zlib` feature).
  Converter: `pngjs` and `ktx-parse`, written into `pipeline.ts` itself, because overrun runs
  that one file in a VM and cannot import local modules. Every texture is stored as sRGB,
  since Bevy loaded the PNGs as sRGB and the shaders were tuned against that, including the
  water normal map's `- 0.25` (X11); Bevy takes sRGB-ness from `ImageLoaderSettings::is_srgb`,
  not the file. Loaders use `panoply_terrain::load_tiling_texture` (or `tiling_sampler` in an
  asset loader): repeat on every axis, linear mip filtering. The shaders no longer wrap UVs
  with `fract`, which with mips would draw seams where UVs jump; this replaced the disabled
  `config_textures_modes`, which set `Repeat` on U only and matched a folder no texture was
  in. Alternatives: Bevy's asset processor (`CompressedImageSaver`) would run a second
  processor beside overrun, and its block compression is lossy on painted textures; Bevy's
  `MipGenerationJobs` is for render targets and has no sRGB formats; `sharp` needs native
  libvips; zstd needs another npm package on Node 18. Size: 6.7 MB of KTX2 for 4.3 MB of PNG.
  Textures inside glTFs (flora leaves, scenery) still lack mips (X27).

## Deferred

- **X1** Migrate exemplars to BSN. *Gate:* BSN assets released, and BSN supports shared
  (non-instanced) templates. Includes updating the exemplar migration script, which lives in the
  `faery` repo (the earlier TypeScript/three.js engine), to emit BSN.
- **X2** Synthesized shared assets (successor to D6). *Gate:* a restored module needs them.
  Consider deterministic `AssetId`s from hashed params instead of a fake asset source.
- **X3** Editor restoration and its UI. Widgets: `bevy_feathers` (on `bevy_ui_widgets`).
  Update 2026-10-11: going with `bevy_immediate` on the fork (Next item 0), tried first in the
  `arbor_day` shell (X28) rather than on X26's sliders. Earlier notes: reactivity was undecided: Quill is dead; `bevy_reactor` (the author's; last on 0.17 and
  Cart's `next-gen-scenes` branch, so a fork means porting off a BSN prototype) is
  experimental; `bevy_immediate` (0.9 targets Bevy 0.20, integrates feathers and
  `bevy_scene`) gives widgets an immediate-mode API. Recommended: `bevy_immediate` for the
  editor (dev-only forms, inspectors, palettes), tried first as a spike on X26's water
  sliders; reactivity for game UI decided separately. Much of the editor's design can be
  taken from faery's editor (`packages/editor`; see [faery.md](faery.md), Editor, and
  `design/Editor.md`): read it before designing modes and tools. Design, from the author:
  - The editor is its own crate, enabled by the `editor` feature (today `src/editor`, D13).
  - Layout: a side panel on the left (300 px default) with a splitter; the game fills the
    rest. A key toggles the panel. Bevy main has a splitter (`bevy_ui_widgets`
    `split_pane.rs`, after 0.20): fork it or write one.
  - Modes: realm, terrain, scenery, catalogs, play (icons in the old repo).
  - The editor layout is a BSN scene with a slot for the game scene; without `editor`, the
    game scene is the root. The game view is the camera's viewport, following a UI node:
    `src/view/viewport.rs` already does this (`ViewportInsetElement` -> `ViewportInset` ->
    camera `Viewport`), but `update_viewport_inset` queries `GlobalTransform`, which 0.20 UI
    nodes lack (they have `UiGlobalTransform`), and mixes units (`ComputedNode::size` is
    physical pixels; `left`/`top` are not divided by the scale factor). Fix when the panel
    returns.
  - All editor state is saved in `bevy_settings` (mode, selections, panel width, view), so a
    restart returns to where work stopped; the edit-run loop restarts often. Under the
    remote driver, settings are already separate (D15); plans should also start from
    defaults (not load saved settings), so runs do not depend on the previous run.
  - Editor and game keys use separate `bevy_action_map` contexts; picking keeps panel clicks
    from the game. Driver plans run with `editor` on, so they should hide the panel (key or
    setting) to keep screenshots and frame times comparable.
  Also replace `bevy_mod_picking` with `bevy_picking`, `@PreferencesGroup` with
  `SettingsGroup`. *Gate:* editor restoration.
- **X4** Replace `PropagateRenderLayers` with Bevy's `Propagate<RenderLayers>`. *Gate:* scenery,
  actors and flora restored (they are the callers).
- **X5** `bevy-inspector-egui` (pinned bevy 0.15). *Gate:* a compatible release is needed.
- **X6** Revisit asset bundling. The GLTF layout carries three.js-era assumptions, made to cut
  HTTP requests: small models (e.g. flora) are combined into one file, and large Blender
  catalogs are never split. Under Bevy, per-model files and labeled sub-assets may suit better.
  *Gate:* restoring terrain/flora/scenery, or a change to the asset pipeline (see D7).
- **X7** Scripting and modding using the SAGA language from `bevy_reactor` (SAGA: "Scripts for
  Animated Graphic Adventures"). *Gate:* the `bevy_reactor` SAGA prototype is ready to integrate.
- **X8** Horizontal trackpad scroll rotates the viewpoint through a global `MouseWheel` reader
  (`input::scroll_rotate_viewpoint`), so once the editor panel returns, scrolling the panel
  would rotate the camera too. Route it through picking instead: observe `PointerScroll`,
  which goes to the entity under the pointer and bubbles up. Without `editor`, observe it on
  the window entity (the window backend hits the window when nothing else is under the
  pointer); with `editor`, on the viewport slot node, so the panel's widgets get their own
  scrolls. Once world entities are pickable, scrolls over them must bubble to the view or not
  block it. Scroll input in `bevy_action_map` is for handling a wheel globally, which this is
  not. *Gate:* the editor panel (X3).
- **X9** Finish splitting terrain and scenery into crates: move the parcel systems from
  `src/terrain` into `panoply_terrain`, and scenery into its own `panoply_scenery`. Flora's only
  tie to the app is `models::PropagateRenderLayers` (see X4). *Gate:* scenery restored.
- **X10** Reflections for shiny objects: `LightProbe` volumes with `EnvironmentMapLight` (using
  `skybox_diffuse_32.ktx2` and `skybox_specular.ktx2`) where shiny objects are, instead of on the
  camera. *Gate:* scenery or actors with shiny materials restored.
- **X11** Water, continued in small steps, each checked by eye in motion (`plans/lighting.py`,
  `bluff` and `shore`). D31 landed colors, alpha, the seabed and a little more sky. The sky
  weight is still well below three.js's (`mix(water, sky * 0.5, opacity * 0.5)`, about
  0.1-0.3; three.js had `mix(water, sky, opacity)`, fully sky at grazing angles); tune further
  by eye. Compared with the three.js version, the water lacks its large, soft, pale sky
  reflections over deep indigo (most of what made it read as water), is teal-gray rather than
  blue (its blue came from the sky term), and has narrower, dimmer foam without the ragged
  breakup at the seaward edge; our sun glint is new (three.js water was unlit) and large. Use
  `plans/lighting.py`'s `ocean` spot (189, 62, open water only). The sky term is the blocker:
  `water-clouds.png` is nearly binary, and the lookup follows the rippled reflection, which
  jumps between neighboring far pixels (no mips), so where the sky weight is high (grazing
  angles, far water) it reads as hard-edged blotches; near water it is weighted almost to
  zero, so it looks flat. Confirmed by turning `ENABLE_SKY` off (blotches gone). Neither a
  lower lookup frequency (`* 0.1`) nor clamping `reflect_vector /= y` toward the horizon
  helped. Next try a soft (blurred) cloud texture, then raise the sky weight; then foam; then
  the glint. Shaders now hot-reload in editor builds (D32). Optionally tie the sky colors to `HemisphereLight` so water dims at
  night and in interiors (X12). Blur the seabed with depth (`textureSampleBias`), now that
  textures have mips (D33). Water is always at sea level (the ground's underwater tint depends on it),
  so reflections of scenery could use one mirrored camera (planar reflection), if their cost
  fits the budget; Bevy's SSR needs deferred rendering and does not reach blended surfaces
  (see Notes). Texture noise in place of `snoise` was a deliberate performance change; keep
  it. Findings from a reverted attempt: restoring the old blend at full sky weight made the
  cloud texture read as fractal discoloration; replacing the clouds with a smooth elevation
  gradient lost the look of fluid. The ripple normal map loads as sRGB, offset by an
  empirical `- 0.25`; decoding it properly changes the ripples' strength and bias.
  *Gate:* author's direction.
- **X12** Art direction for lighting: painterly (pre-Raphaelite, art nouveau). D30 restored the
  three.js balance (no tone mapping, ambient over sun); tune `Sunlight` and `HemisphereLight`
  by eye with `plans/lighting.py` (`SUN=`, `AMBIENT=`, `SHOTS=`), then repaint textures in
  Krita where they still don't fit (grass and cobbles may be too high-frequency). The editor's
  terrain thumbnail (`terrain_thumbnail.rs`) still has its own 3000 lux light and default tone
  mapping. Underground realms a little more gothic, still
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
- **X22** A loading state: a game state (Bevy `States`) that holds until the scenery around the
  viewpoint has loaded, so a loading screen or other visual barrier covers the pop-in on entering
  the game. "Loaded" means the precincts in view have no `ReadPrecinctData`, their scenery scenes
  are instantiated (D26 budgets spread them over frames), and the shown parcels have ground and
  flora; later, actors too. A teleport (`GotoLocation`, portals) could reuse it. Low priority;
  the loading aesthetics come much later, but a placeholder (a plain loading indicator over a
  black screen) is fine before then. *Gate:* a game-entry flow (title screen or save loading)
  to put it in front of.
- **X23** Actor palette merge: join each actor mesh's primitives into one, with a per-vertex
  color-slot index, so an actor draws as one primitive (and one outline) with one material
  holding its palette. An actor is now about 10 primitives plus an outline child each, and
  per-entity work is the CPU floor (Notes); a crowd like the three.js test lineup (20-odd
  actors, nearly all colored differently) is about 600 skinned draws before shadows. Needs a
  gltf-transform step and a small shader; slot names must stay consistent per file (D28).
  *Gate:* actors restored, and a crowd scene measured.
- **X24** Realms without terrain. `underworld` (in `world.locations.json`) is floors only, with
  black everywhere else; the three.js version needed a placeholder terrain map for it (a 3x3
  stub, which `convert_realm_maps.mts` writes; deleted here). Check that a realm with no
  `.terrain` file spawns no parcels and no errors, and that parcels' wait for precinct effects
  (D11) does not hang; make a stub map only if it must. *Gate:* portals (the way in).
- **X25** Water currents, for rivers; partly built. Flow cannot move vertices, which would
  stretch or compress where a current meets still water. Instead the fragment shader mixes
  noise layers, each scrolling in its own direction (`FRAGMENT_WAVES` in `water.wesl`: four
  directions, +x, +z, -x, -z, three frequencies each), and the current vector varies each
  layer's amplitude: still water weights all directions equally (standing chop), and a current
  boosts the layers that agree with it, e.g. a base weight plus `max(0, dot(direction,
  current))`. Normalizing by the total weight, as the loop already does, keeps ripple strength
  even across the boundary. The current is meant to be the water mesh's `depth_motion.yz`
  (written as zero in `water_mesh.rs`; `.x` is depth); the unused `motion` variable and the
  commented strength gate in the loop are the start. With four axis directions, a diagonal
  current may read as two crossing patterns; six or eight directions would cover angles
  better, at more texture samples (now 12). The ripples are the only carrier of flow, so
  water shading changes (X11) must keep them readable as moving bumps. *Gate:* rivers (a
  source of current vectors in the terrain data).
- **X26** Live tuning of surface looks: a reflected resource (like `Sunlight` and
  `HemisphereLight`) holding the water's look (water and sky colors, sky scale and weight,
  alpha, chop, glint) and the seabed tint, copied into the materials on change, with sliders
  in the editor panel, in a sub-mode (e.g. of terrain or realm). Tuning water (X11) one
  rebuild and screenshot at a time was slow; shaders now hot-reload (D32), but uniforms set
  from Rust do not. *Gate:* the editor panel and the choice of UI reactivity (X3).
- **X27** Mipmaps for textures inside glTFs (flora leaves, scenery), which D33 does not
  cover: Bevy's glTF loader generates none. glTF carries KTX2 through `KHR_texture_basisu`,
  which requires Basis Universal compression (lossy), not D33's plain RGBA8; check first
  whether Bevy's glTF loader accepts other KTX2, or keep the images in separate `.ktx2` files
  bound by material. Alpha-masked leaves need care: averaged alpha thins cutouts at distance
  (preserve coverage per mip). *Gate:* flora shimmer or scenery texture aliasing worth fixing,
  judged once actors make whole scenes (X12).
- **X28** `arbor_day`: port the tree generator (`../seedling`, TypeScript) to Rust, in its own
  workspace, with a new growth model; no backward compatibility. All 48 flora models come from
  seedling. Requirements, the algorithm survey to do first, and open questions in
  [arbor-day.md](arbor-day.md). Its output should let X15 merge flora and X27 mip leaf
  textures without pipeline patching. Its own repo and workspace, on Bevy main like Panoply
  (Next item 0). Its authoring app shares X3's UI architecture. The shell comes first, with a
  stub generator (a tapered cylinder and a few cards): split pane and parameter panel in
  `bevy_immediate`, a 3D view in a viewport following a UI node, `bevy_settings`, action-map
  keys, a remote-driver plan. It serves as X3's `bevy_immediate` spike, replacing the water
  sliders. *Gate for the generator:* the growth-model survey and a recommendation.

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
- Hitch log (`diagnostics::hitch`, editor builds): logs each frame over 14 ms after startup,
  with what the three frames before it spawned (precincts, parcels, parcels shown, scenery,
  scenes, meshes, images, pipelines, physics steps, colliders), and each pipeline created after
  startup with its shader defs. Opt-in: `PANOPLY_HITCH_LOG=1 cargo run`, then grep the output
  for `Hitch:`; `plans/scroll.py` turns it on for its own run.
  Before D26, a cold walk (`plans/scroll.py`) showed 40-45 ms frames from pipeline creation and
  17-32 ms frames from scene bursts; after, neither. Frames of 15-24 ms with nothing counted
  remain, at the same rate in cold and warm passes, so not load-related; possibly the driver's
  polling. Screenshots stall a frame by 350-480 ms (the window comes to the front).
- Measure frame time by counting frames over wall time without polling: the driver's
  diagnostics return the latest single frame, and polling every 5 ms disturbs the app.
- Bevy 0.20 issues seen: (1) switching `DirectionalLight::shadow_maps_enabled` off at runtime
  panics the render thread ("Failed to get directional light visible entities for cascade",
  `bevy_pbr` `render/light.rs`); (2) an entity with two `MeshMaterial3d` components (e.g. an
  extended material alongside `StandardMaterial`) fails wgpu validation in the shadow pass;
  `models` now puts outlines on a child entity instead; (3) `RenderDiagnosticsPlugin` records
  only CPU pass times on Metal.

- The water shows a fine square grid (visible before and after D16). Almost certainly the debug
  `InfiniteGrid` (`main.rs`) seen through the transparent water: the same grid continues past
  the map edge in `lighting_bluff.png`. Not yet confirmed by hiding the grid.
- Bevy 0.20's SSR (`ScreenSpaceReflections`) works only with deferred rendering
  (`DeferredPrepass`), and reflects onto opaque G-buffer surfaces only, so it would not reach
  the water (`AlphaMode::Blend`, custom forward material) without switching the renderer to
  deferred and reworking the water. Its cost was not measured. From the game's high camera,
  most reflected rays leave the screen (sky), so it could only reflect the far banks.
- `ExemplarData`'s `Serialize` impl is `todo!()`; serializing an exemplar panics.
- Scenery content: playground `p000-p000` holds 1,048 scenery instances, 13 actors and 6 tiers,
  a usable test map for scenery performance (X15); the other playground precincts hold less.
  Overland has one precinct here; the old engine's overland rooms were not migrated.
- Floor regions have identity transforms: their outlines are in precinct coordinates.
- With `dynamic_linking`, run via `cargo run`; the bare binary can't find `libstd`.
