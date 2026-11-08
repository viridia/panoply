# Decisions

Why Panoply is the way it is, for choices expensive to reverse. A decision names the chunk that
made it, where there is one. A superseded decision stays, pointing at its successor; numbers are
never reused.

**Next: D58.**

- **D1** Superseded by D34. Build against crates.io Bevy releases, not git main.
- **D2** Settings use `bevy_settings` (`SettingsGroup` resources, `settings.toml`).
  `bevy_prefs_lite` and `bevy_basic_prefs` are abandoned. Window position falls back to winit's
  `outer_position()` via `bevy::winit::WINIT_WINDOWS` (a stopgap API, Bevy issue #17667).
- **D3** Shaders are WESL (`.wesl`); naga_oil is gone from Bevy. Material bindings use
  `@group(constants::MATERIAL_BIND_GROUP)`. Bevy modules are `bevy_pbr::render::*` etc.
- **D4** Exemplars stay as the composition mechanism for now. Keep the `panoply_exemplar` API the
  boundary, so that scenery/actors depend on "entity with aspects", not on exemplar internals.
- **D5** GLTF models spawn via `WorldAssetRoot`. A model instance (flora, scenery element,
  warm-up) carries `Propagate(realm layer)`, and Bevy's `HierarchyPropagatePlugin<RenderLayers>`
  (added by `PanoplyCorePlugin`, in `PostUpdate` before `CheckVisibility`, so a new scene never
  draws on layer 0) gives every descendant the layer, including children spawned later, such as
  outlines. It replaced an observer that copied layers once, on `WorldInstanceReady`.
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
- **D10** Automated runtime checks use the `bevy-remote-driver` package (`../bevy_remote_driver`,
  installed with `uv tool install git+…`), as AGENTS.md says. The game enables BRP itself in
  `src/remote_driver.rs`, only when `BEVY_REMOTE_DRIVER_PORT` is set, with Bevy's `bevy_remote`
  feature. Plans live in `plans/` and set `DRIVER = PanoplyDriver`, whose steps
  (`plans/_panoply_driver.py`) hold what plans repeat: `goto` writes `GotoLocation` and waits
  for `panoply_editor::diagnostics::ViewReady` (nothing loading or building in any loaded parcel
  or precinct, and warm-up done), then two frames for the render; `camera` finds the primary
  camera. `key(hold=…)` returns at the press, before the release, so a step
  that is not input (a screenshot, a timing) runs during the hold: a plan that needs the hold
  over presses, waits `frames`, and releases (`smoke.py`, `scroll.py`). Replaced the in-tree
  fork, `crates/bevy_remote_driver`, whose server methods all went upstream.
- **D11** Layering: `panoply_core` (realm, both grid sizes) <- `panoply_terrain` <-
  `panoply_scenery`, which also uses `panoply_render` (materials, models).
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
- **D15** Retired with D10's package, which runs each plan with `BEVY_SETTINGS_DIR` set to its own
  `settings/`, so plans never change the developer's saved settings without a separate identity
  (was `org.viridia.panoply.driven`; that folder in `~/Library/Preferences` is now unused). Plans
  that measure set the camera distance through `goto`.
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
  time a material and mesh-layout combination is drawn. `panoply_render`'s warm-up loads the model folders
  (`terrain/models`, `scenery/models`) at startup and draws every scene of each glTF once, tiny
  and below the ground at the view center, then despawns it; any glTF loaded later is warmed
  the same way. Instances follow the view center while they live: one left behind by a
  teleport is culled and warms nothing (outlines, added a frame after their scene, most often). Instantiating scenes is budgeted too, 64 per frame for flora and for scenery
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
- **D28** Actor models stay one glTF per species as authored (`humanoid-male.glb` holds every male
  body and feature; `humanoid-armature.glb` the shared clips), with no split transform. Bevy always
  loads a glTF whole, labels included, and uploads all its meshes; the files are small (humanoid
  male about 1.3 MB of mesh data, 40 clips about 0.5 MB, all characters about 8.5 MB on disk, almost
  no textures), so keeping a species resident costs little, and one file warms in one pass (D26).
  Load the root `Gltf` and look up `named_scenes`: loading `file.glb#Scene` by label hits Bevy issue
  #10549 as exemplars did (D27). Clips bind by bone name path (`AnimationTargetId`), so shared clips
  drive any file's armature with matching bones. Assembly, as in the three.js `SkinnedModel.ts`: one
  skeleton per actor (the body scene's armature and one `AnimationPlayer`); each selected feature
  (hair, hat, beard) spawns its named scene, which `modelOpt` gives its own copy of the armature:
  once it is ready, each `SkinnedMesh`'s joints are repointed to the body's bones matched by `Name`,
  and the copy is despawned. Building a feature's meshes from the `Gltf` asset instead would repeat
  the loader's per-primitive work (the `StandardMaterial` found by label through `bevy_pbr`'s hook,
  `GltfMaterialName` and `GltfMaterialExtras`, which coloring and outlines key on, bounds), and
  break as the loader changes. *Revisit* if Bevy can spawn a skinned mesh against another scene's
  skeleton (bevyengine/bevy#17114), or makes the loader's primitive spawning public. Color slots are
  material names; an actor's `Colors` replace a slot material's base color with a clone cached by
  (material, color). Exemplar hex colors are sRGB (`Srgba::hex`); glTF `baseColorFactor` is linear.
  Colors for slots a mesh lacks are ignored. Expect little sharing: nearly every actor has its own
  colors (see X23). *Revisit* (with X6) if actors gain real textures, or a character file passes
  about 10 MB. `pipeline.ts` ran gltf-transform's `draco()`, but no output used Draco (the encoder
  was never registered); the call is removed, since Bevy's glTF loader cannot decode Draco. Output
  was unchanged.
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
- **D34** Bevy is git main, not releases (supersedes D1): Panoply will not ship before 0.21 (BSN
  assets), and main has what the editor needs (split pane, tabs, tree view, `bevy_inspector`).
  Every repo in the family names it exactly `git = "https://github.com/bevyengine/bevy.git",
  branch = "main"`: Cargo treats a `branch` and a `rev` as different sources even at the same
  commit, so a `rev` would build a second Bevy beside the dependencies'. `Cargo.lock` holds the
  commit, `784b85d`, the same as `arbor_day`, `bevy_action_map`, `bevy_immediate` and the
  driver's testbed. `bevy_action_map` is its git `main` too (0.2.0-dev). Bump all repos together,
  one chunk per bump, with `cargo update -p bevy_ecs --precise <commit>`; a plain `cargo update`
  takes main's head. API source: Cargo's checkout of that commit, per AGENTS.md.
- **D35** `rapier3d` 0.36, used directly (D22), not through `bevy_rapier`, so not tied to a Bevy
  version. Its math is glam, the same release as Bevy's (0.33), so `Vec3` passes without conversion.
  `RealmPhysics` wraps rapier's `PhysicsWorld`, which bundles the sets; queries go through
  `RealmPhysics::world()`, whose query pipeline is built on demand from the broad phase (no stored
  `QueryPipeline`). Each step's `dt` is `Time<Fixed>`'s timestep: rapier's default 1/60 s against 64
  Hz ticks ran physics about 7% fast. Only the viewpoint's realm is stepped (a realm seen through a
  portal would freeze). Ground colliders stay the smoothed render mesh, holes and terrain-effect
  elevation included, not the coarser contour grid: some effects flatten the ground, and only the
  smoothed mesh has them. Collision groups are `panoply_core::realm_physics::collision_groups`,
  defined as each kind of collider arrives, after faery's ([faery.md](faery.md), Physics); Terrain,
  Floor, Wall and Fixture so far (D38). Editor builds (D13) have debug views
  (`panoply_editor::debug_views`, D37): `DebugViews`, saved with `bevy_settings`, holds physics on
  or off and a mask of groups to draw; F3 toggles physics. Rapier's debug render draws through a
  gizmo backend, only for colliders of shown parcels (D20), matched by the parcel entity in each
  collider's `user_data`: a trimesh is three lines per triangle, about 25k a parcel, so all 128
  cached parcels would be about 3M lines a frame. Even so, drawing halves the frame rate, which is
  accepted for a debug view. `GroundProbe` holds where a ray cast down at the view center hits,
  drawn as a sphere; `plans/smoke.py` checks it.
- **D36** As few dependencies as possible. A crate that depends on Bevy has to be forked to
  follow Bevy main (D34), so each one (character controller, particle engine, navigation mesh
  generator) is a fork to maintain. A crate with no Bevy dependency, such as `rapier3d` (D35),
  needs no fork. Prefer, in order: Bevy's own, a crate without Bevy, our own code, a Bevy crate.
- **D37** The editor is its own crate, `panoply_editor`, an optional dependency behind the
  app's `editor` feature (D13); feathers and `bevy_immediate` are its dependencies only, and
  `main.rs` adds `EditorPlugin`. It sees the engine crates, never the app. Engine crates own
  neutral knobs (resources, components, messages: `ViewportInset`, `ViewOverlay`, `Viewpoint`);
  the editor sets them and spawns entities of its own, overlays included, and engine crates do
  not test the `editor` feature. So an editor mode for a module waits until the module is in a
  crate (PAN7). The shell (`panoply_editor::shell`) is a feathers split pane: the panel at the
  left, hidden by default so plans, which start from empty settings, see the whole window; T
  toggles it; `EditorPanelSettings` saves its width and visibility. While it shows, the
  primary camera's viewport follows the view pane through `ViewportInset`; hidden, the inset is
  left alone, so plans can set it. HUD nodes marked `ViewOverlay` move into the view pane. The
  Show menu's popup holds checkboxes, not menu items: picking an item closes a menu, and a
  checkbox takes the focus without closing it, so several views toggle in one visit.
- **D38** Static scenery colliders come from the exemplars' `SceneryColliders` descriptors, not
  the glTFs (X36). Each precinct has one fixed body at its origin (`Precinct::physics`, removed
  with the precinct); not one per tier, since elements are the precinct's children and
  cutaways (X19) test each element anyway. Each element and floor region owns its colliders
  through `panoply_core::OwnedColliders`, an immutable component whose discard hook removes
  them, so a moved or replaced element rebuilds alone (`SceneryElementRebuildColliders`).
  `ColliderDesc::shape` places shapes exactly as faery's `createCollider.ts` did, since the
  content was authored against it: `size` is half-extents, `offset` the bottom center, shapes
  other than ellipsoids are raised by their half-height, and only ramps turn by `facing`.
  Ellipsoids and ramps are convex hulls. Only static solids are built (walkable or not);
  shapes are built per instance, not cached per exemplar, since only hulls cost anything and
  3 of 580 descriptors are ellipsoids. A floor region is one closed trimesh with
  `FIX_INTERNAL_EDGES`, built in its mesh task: `PHYSICS_FLOOR_THICKNESS` thick under the drawn
  top (`level + raise + TIER_OFFSET`), holes included, faces oriented in code whatever the
  rings' winding (unit test in `floor_mesh.rs`). One shape per region leaves no seams on its
  top for the controller to catch on; faery's convex pieces did, and dropped holes. Floor
  depressions (planned, for sliding-block puzzles: a property of the floor surface) move the
  top, so the collider follows them. Groups: Floor, Wall, Fixture (by the exemplar's
  `meta_type`). Colliders' `user_data` is the owning entity, for picking later; the F3 view
  draws every scenery collider, and ground only for shown parcels.
- **D39** Generated trees get a trunk capsule each (blocks walking) and a canopy over the forest
  (catches what falls onto it, so an actor dropped there cannot land in a pocket between trunks 0.4
  m apart). Design in `panoply_terrain::flora_physics`'s module doc: the canopy is quantized to the
  terrain grid, covering tree cells and enclosed clearings found by a flood fill that looks 4 cells
  into neighbouring parcels, one continuous trimesh 2 m over the ground, never over cells with
  terrain effects. Both are computed in the flora task and owned by the parcel's flora entity
  (`OwnedColliders`, D38); `user_data` is the parcel, so the F3 view draws them for shown parcels
  only. Trunk radius is `FloraTableEntry::trunk_radius` (default 0.3, twice the bark's, for the
  branches packed around it; scaled with the instance); shrubs and herbs have none. Trees placed as
  fixtures are authored: they get their descriptors' colliders and no canopy, and a pocket among
  them is an authoring error. The canopy collides with everything for now: when picking and sight
  rays arrive (X3, PAN3), their query filters exclude `CANOPY`.
- **D40** Actors are a crate, `panoply_actors`, since the editor sees only engine crates (D37) and
  will edit actors. The precinct file format is a crate of its own, `panoply_precinct`
  (`PrecinctAsset`, its loader and saver, and the instance data it holds), which `panoply_scenery`
  and `panoply_actors` both depend on: a precinct holds both scenery and actors (D24), and with the
  format in scenery, scenery would have to name actor types, a dependency cycle. Scenery still
  registers the asset and spawns precincts. (PAN3a)
- **D41** Actors move with `rapier3d`'s `KinematicCharacterController` (D36: no fork), in
  `FixedUpdate` after the realm steps, so colliders inserted since the last tick are in the broad
  phase. Bevy main has no transform interpolation, so `panoply_actors::actor_physics` draws each
  actor between its last two ticks' positions, as Bevy's `physics_in_fixed_timestep` example does.
  The capsule (`CollisionSize`, R4) is a query shape, not a collider in the world: nothing yet needs
  to hit an actor, and leaving it out spares filtering the actor out of its own casts. Reversing
  that, for actors blocking each other or picking, adds a collider and a filter, and lets rapier's
  debug render draw it; until then the F3 view draws capsules itself, under the `ACTOR` group's
  "Actors" entry in the Show menu. An actor gets physics when nothing terrain or scenery is pending
  and a ray down from it hits a collider, until X33's footprint query. (PAN3f)
- **D42** The camera follows an entity named in `Viewpoint::follow` (R102's active entity), set to
  the player's actor at spawn. Moving the viewpoint any other way (`GotoLocation`, the free camera's
  arrow keys) clears it, so plans and the editor look anywhere without fighting the actor; W or S
  sets it again. It trails the entity as faery's camera did, rather than sticking to it, so falls
  and starts show on screen; locked to the actor, a fall read as the ground rising, at the right
  speed but mushy to the eye. Keys follow faery's player controller: W walks away from the camera, S
  backs toward it, A and D turn the camera, and the actor travels along its facing while turning
  toward the key's heading, so it curves rather than strafes (R29). W and S left the free camera,
  which flies on the arrows alone. An actor counts as grounded only on walkable ground (a cast down
  finds a normal with y at least 0.7): rapier's own `grounded` includes steep slopes, where an actor
  that stopped falling would stick instead of sliding (R24). (PAN3g)
- **D43** Character models face -Z in their files, and an actor faces +Z at heading 0, so Bevy's
  loader turns each character scene's root around (`GltfConvertCoordinates::rotate_scene_entity`).
  It is set by a `.glb.meta` beside each file, written by `pipeline.ts` (D7) for every model in
  `artwork/characters`: a meta file applies to every load, including the warm-up's folder load,
  where a setting passed in code would depend on which load came first. Not `GltfPlugin`'s default,
  which would turn scenery too, and not props, which hang from the hand's bone. (PAN3i)
- **D44** An actor's clip follows its measured speed along its facing over the last fixed tick, not
  its intent, so an actor stopped by a wall stands rather than walking in place, and backing up
  plays the walk backward. Gaits and their thresholds follow faery (R18): above 3.5 m/s, not at it,
  is a run, so the player's jog stays a fast walk. Crossfades are Bevy's `AnimationTransitions` over
  one `AnimationGraph` per skeleton, every clip a node under the root, so a layer (R39's gestures)
  adds nodes and a mask. Species name their clips their own way through the `ClipNames` aspect,
  faery's `actionMap`. (PAN3i)
- **D45** What a field's value refers to is its type, not an annotation. faery's reflection marked
  plain types with a `subtype` for the property editor (a string marked `dialogueState` showed a
  drop-down of the dialogue's states; see [faery.md](faery.md), Editor); in Panoply such a field is
  a newtype (`struct DialogueStateId(String)`, serialized as its inner value), and the editor picks
  a widget by the field's reflected type. JavaScript had no runtime types, so faery needed
  annotations; Bevy's reflection has them, and the newtype also stops code passing a realm name
  where a model path belongs. Annotations remain for how a primitive (`f32`, `String`) is shown, not
  what it means: a range, a step, a multi-line text box, as reflect custom attributes.
- **D46** Facings are stored in degrees (0 faces +z, 90 faces +x), for scenery and actors alike, and
  turned to radians once, at spawn. Ease of editing no longer decides it (feathers' number input
  shows units), but degrees hold authored angles exactly: quarter turns and snapped increments are
  whole numbers, where radians are never exact, so comparing facings (axis-aligned walls, whether an
  edit changed anything, stable saves) needs no tolerance. Radians would match the runtime values
  (`ActorMotion::facing`) and save the conversion. (PAN3l)
- **D47** A precinct's actors name their exemplars by path, as scenery's type tables do, rather than
  as handles loaded with the precinct, so a missing exemplar cannot keep the precinct (and its
  scenery) from loading. The player is not a precinct actor: a new game places the player (X46).
  (PAN3l)
- **D48** Animation states are a fixed enum in code, each with faery's clip fallback chain, fade and
  playback, rather than data in the species' assets: species differ by which clips they have, which
  the chains absorb, and by names, which `ClipNames` maps (D44). The state is chosen each frame from
  the body's measured motion, as gaits are: falling is physics' flag (more than 0.7 m above the
  ground since leaving it, so steps down and slides are not falls), turning is a measured turn rate
  while standing. A landing is an envelope: the crossfade in, a hold at the clip's first frame (its
  deepest crouch, which the crossfade would otherwise blend away), the clip's rise, the crossfade
  out. Moving cuts it short after the hold, so input waits on a clip no longer than that. (PAN3j)
- **D49** Undo history addresses what it changes by a stable key: the asset's key and a property
  path within it, never an `Entity`. Values may be component values edited through reflection, as
  aspects are (assets realized as components); what history must not hold is entities or the
  relationships between them (hierarchies), since undoing a spawn would then rewrite `Entity`
  references across the history, the problem that stalled the author's undo PR (bevy#23373). A
  changed asset is realized again.
- **D50** A frame-rate regression is fixed before more features land: once measured, it becomes the
  first chunk in the roadmap. Slowdowns that overlap are hard to tell apart, so none is left to
  accumulate behind another. The target is D17's. (PAN3p is the first.)
- **D51** Parcel ground collides as a trimesh, not a height field: terrain has holes
  (`TerrainOptions::Hole`), which a height field cannot have. Actors fell through the seams between
  parcels only in an early faery that used height fields; they have never fallen through the
  trimeshes, here or in faery's later versions. So off the map's edge and through a hole are the
  only ways out of the world. Off the map, and where no tile is placed, a realm has its default
  contour, usually open water or the empty tile, which has no ground; the playground's is the empty
  tile, and it has a large hole at (23, 43).
- **D52** Widgets get their keys from the action mapper, not from Bevy's `InputDispatchPlugin`,
  which the app disables; how is in `src/widget_keys.rs`. One arbiter then decides every key: with
  Bevy's dispatch beside the mapper, a key typed in a text field also reached game actions, and a
  consuming context could not keep a key from a focused widget. Feathers stays unmodified, since the
  mapper delivers the `FocusedInput` its widgets already read; a context per widget kind,
  `bevy_action_map`'s D23 pattern, would have meant reimplementing keyboard handling for every
  feathers widget. Where a key goes follows focus, read from marker components the plugin makes
  required by each widget: a zero-sized type per kind rather than an enum, so a game links only the
  widgets it uses. Meant for any game pairing a mapper with feathers, so the module names no Panoply
  type and can move to `bevy_action_map` (its X14). Reversing it lets typing reach game actions
  again. (PAN33)
- **D53** The app's mode is a Bevy state, `GameMode`, of two values, play and edit; the editor's
  modes (realm, terrain, scenery, catalogs; X3) will be sub-states of edit, not siblings of play
  with a computed state for what the editor modes share. A build without the editor has only play,
  and what every editor mode shares (the free camera, Go To moving the view) is then the parent
  state itself. Each mode's keys are a context active only in it (`active_in_state`), so WASD and
  the arrows drive the free camera in edit and the player's actor in play, as in faery. Reversing it
  means the computed state and an editor-modes list kept in step with it. (PAN22)
- **D54** The editor edits assets in memory and saves them only when asked. An edit changes the
  loaded asset (`Assets::get_mut`), so the world shows it at once, and marks it in `UnsavedAssets`;
  the save button writes every marked asset, each type registered with `register_saveable`, through
  a temporary file renamed over the original. faery showed many edits only after a save, through the
  file watcher's reload; Panoply's never wait for it. Asset events never mark an asset, so the
  reload of a file just saved leaves it saved; an asset whose reload is expensive (a precinct, which
  respawns its actors) must also ignore the reload of its own save, which its first editing chunk
  handles. Writes are synchronous until an asset is large enough to need a task. JSON is written in
  the authored files' form (faery's `JSON.stringify`): arrays of numbers on one line, whole floats
  as integers, so a save changes only what was edited. Reversing it means edits that show only after
  saving. (PAN29)
- **D55** Character, scenery and prop files are Blender 5.2 files, exported with the settings each
  file remembers (`EXPORT_SETTINGS` in `scripts/blendtools.py`), which File > Export > glTF 2.0
  opens with and `blend_export.py` uses. A clip is an NLA track, named by the track (NLA Tracks
  mode): Actions mode also exports hidden objects' actions and doubles the channels of an action two
  strips use. Remembered settings, not 4.2's collection exporters: an exporter cannot sit on a
  scene's root collection, and one on a collection leaves out any object later added outside it.
  Actions keep their names, since other files link them from `humanoid-armature.blend` by name.
  Blender 5.2 exports 3.5's characters differently in ways `blend_cleanup.py` repairs in the file,
  each matching what 3.5 exported; the one that was never well defined is IK/FK switches, constraint
  influences some clips leave unkeyed, which take whatever value Blender last evaluated, so every
  clip now keys them. Specular is the exception, left as 5.2 exports it: 3.5 wrote tints above 1,
  derived from the base color, and exporters before it wrote none. `blend_inventory.py` reports a
  file's departures from all this, and `glb_compare.py` what a re-export changed. Reversing it means
  going back to 3.5, whose exporter read the stash tracks and shortened strips correctly only by
  accident. (PAN48, PAN50)
- **D56** faery's archetypes reach Panoply by merge, through `convert_archetypes.mts`, not by hand
  as each feature lands. faery's world was built unevenly, some kinds of content far more complete
  than others, so each feature that lands has data waiting in faery; with the merge, adding the
  aspect's case to the converter and rerunning brings it to every exemplar at once, while hand edits
  in Panoply stay. It relies on Panoply's catalogs and exemplar names matching faery's one for one,
  as they did when it was written. How it merges is in [faery.md](faery.md), Migration scripts.
  Reversing it means freezing the converter and porting each aspect by hand. (PAN51)
- **D57** The primary camera uses SSAO at Low quality with SMAA, in place of 4x MSAA. Faces in shade
  get only ambient light, which alone leaves their relief flat. SSAO requires MSAA off, and dropping
  MSAA pays for nearly all of it: on Aerdra the pair costs about what MSAA did (`docs/perf.csv`), on
  the Mac 1-2 ms more, while High costs Aerdra about 3 ms more and Low shows no noise. SMAA has been
  judged only on the two high-density displays, Mac and Aerdra, where aliasing hardly shows.
  Reversing it means giving up the occlusion to have MSAA back, and leaves the prepass unused.
  (PAN52)
