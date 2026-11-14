# Decisions

Why Panoply is the way it is, for choices expensive to reverse. A decision names the chunk that
made it, where there is one. A superseded decision stays, pointing at its successor; numbers are
never reused.

**Next: D86.**

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
- **D17** The camera distance in play is 11, as faery's was (PAN55). It trades reading the detail of
  characters and scenery against seeing enough of the surrounding terrain not to feel
  claustrophobic, and a typical combat must fit on screen, so no untargetable opponent hits from
  off-stage. Performance targets assume it: a rock-solid 120 fps, with terrain leaving room for
  scenery and actors.
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
- **D24** A precinct is the unit of authored content: one asset holds all of a 64 m area's scenery,
  floors, terrain effects and actors (everything but terrain). The goal is open worlds larger than
  fit in memory, built by a solo creator: walls, floors and fixtures are placed on a grid (an homage
  to isometric tile games, with perspective and looser quantization), and floors are extruded
  polygons that the editor stamps and merges like tiles, with free vertex editing when wanted. So
  actors are owned by their home precinct and despawn when it leaves the cache; most NPCs stay near
  their spawn point. Followers (global asset) and random monsters (transient, per encounter) are the
  exceptions. Ownership is a relationship whose despawn is linked, replacing `Precinct.actors`.
  Within a precinct, each tier (a floor level with its own cutaways) is an entity, a child of the
  precinct, with its floor regions as its children. Persisted play state follows the same unit
  (PAN82).
- **D25** An aspect type no plugin has registered is skipped with a warning
  (`panoply_exemplar::aspect_list`), not an error that fails the whole catalog or precinct, so
  modules can be restored one at a time (the wall-tile catalog uses portals' `Portal`). Saving such
  a precinct drops the skipped aspects; see X20.
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
  colors. *Revisit* (with X6) if actors gain real textures, or a character file passes about 10 MB.
  `pipeline.ts` ran gltf-transform's `draco()`, but no output used Draco (the encoder was never
  registered); the call is removed, since Bevy's glTF loader cannot decode Draco. Output was
  unchanged.
- **D29** Hair shine uses Kajiya-Kay (specular from strand direction T: `sin(T, H)^shininess`, a
  band across the strands) with Scheuermann's two lobes (a bright primary, a hair-tinted secondary),
  in an `ExtendedMaterial` (`hair.wesl`). The three.js `HairMaterial` approximated a band with
  `pow(sin(dot(R, V) * PI), shininess)`, which follows the surface, not the strands, and shows in
  close-ups. Hair UVs are auto-unwrapped, so T comes from a procedural comb in `pipeline.ts`,
  written as each hair mesh's `TANGENT` attribute (Bevy skins tangents): the direction from a crown
  point (top of the mesh bounds, a little back) projected onto the surface, with a fallback where
  the projection vanishes. Only materials with extras `material: hair` are combed, so facial hair is
  not. Both lobes' colors grow with the base color, so the shine fades with the hair's luminance.
  Not `StandardMaterial` anisotropy, which lights hair white on a camera with a normal prepass
  (I28), and could not tint the band. Light hair also has its terminator softened and its shaded
  side lifted by a wrapped diffuse term scaled by luminance (PAN65): without it, the lack of
  scattering showed on platinum hair, and dark hair absorbs the light anyway. It treats a cast
  shadow as self-shadow, so light hair glows a little in a wall's shade.
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
  assets), and main has what the editor needs (split pane, tabs, tree view, `bevy_inspector`). Every
  repo in the family names it exactly
  `git = "https://github.com/bevyengine/bevy.git", branch = "main"`: Cargo treats a `branch` and a
  `rev` as different sources even at the same commit, so a `rev` would build a second Bevy beside
  the dependencies'. `Cargo.lock` holds the commit, `a35f1b5`, the same as `arbor_day`,
  `bevy_action_map`, `bevy_immediate` and the driver's testbed. `bevy_action_map` is its git `main`
  too (0.2.0-dev). Bump all repos together, one chunk per bump, with
  `cargo update -p bevy_ecs --precise <commit>`; a plain `cargo update` takes main's head. API
  source: Cargo's checkout of that commit, per AGENTS.md.
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
- **D37** The editor is its own crate, `panoply_editor`, an optional dependency behind the app's
  `editor` feature (D13); feathers and `bevy_immediate` are its dependencies only, and `main.rs`
  adds `EditorPlugins`. It sees the engine crates, never the app. Engine crates own neutral knobs
  (resources, components, messages: `ViewportInset`, `ViewOverlay`, `Viewpoint`); the editor sets
  them and spawns entities of its own, overlays included, and engine crates do not test the `editor`
  feature. So an editor mode for a module waits until the module is in a crate (PAN7). The shell
  (`panoply_editor::shell`) is a feathers split pane: the panel at the left, hidden by default so
  plans, which start from empty settings, see the whole window; T toggles it; `EditorPanelSettings`
  saves its width and visibility. While it shows, the primary camera's viewport follows the view
  pane through `ViewportInset`; hidden, the inset is left alone, so plans can set it. HUD nodes
  marked `ViewOverlay` move into the view pane. The Show menu's popup holds checkboxes, not menu
  items: picking an item closes a menu, and a checkbox takes the focus without closing it, so
  several views toggle in one visit.
- **D38** Static scenery colliders come from the exemplars' `SceneryColliders` descriptors, not the
  glTFs (X36). Each precinct has one fixed body at its origin (`Precinct::physics`, removed with the
  precinct); not one per tier, since elements are the precinct's children and cutaways (PAN76a) test
  each element anyway. Each element and floor region owns its colliders through
  `panoply_core::OwnedColliders`, an immutable component whose discard hook removes them, so a moved
  or replaced element rebuilds alone (`SceneryElementRebuildColliders`). `ColliderDesc::shape`
  places shapes exactly as faery's `createCollider.ts` did, since the content was authored against
  it: `size` is half-extents, `offset` the bottom center, shapes other than ellipsoids are raised by
  their half-height, and only ramps turn by `facing`. Ellipsoids and ramps are convex hulls. Only
  static solids are built (walkable or not); shapes are built per instance, not cached per exemplar,
  since only hulls cost anything and 3 of 580 descriptors are ellipsoids. A floor region is one
  closed trimesh with `FIX_INTERNAL_EDGES`, built in its mesh task: `PHYSICS_FLOOR_THICKNESS` thick
  under the drawn top (`level + raise + TIER_OFFSET`), holes included, faces oriented in code
  whatever the rings' winding (unit test in `floor_mesh.rs`). One shape per region leaves no seams
  on its top for the controller to catch on; faery's convex pieces did, and dropped holes. Floor
  depressions (planned, for sliding-block puzzles: a property of the floor surface) move the top, so
  the collider follows them. Groups: Floor, Wall, Fixture (by the exemplar's `meta_type`).
  Colliders' `user_data` is the owning entity, for picking later; the F3 view draws every scenery
  collider, and ground only for shown parcels.
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
  rays arrive (PAN88, PAN3), their query filters exclude `CANOPY`.
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
  "Actors" entry in the Show menu. An actor gets physics when its footprint is built (D67) and a ray
  down from it hits a collider. (PAN3f)
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
  modes (realm, terrain, scenery, catalogs) will be sub-states of edit, not siblings of play with a
  computed state for what the editor modes share. A build without the editor has only play, and what
  every editor mode shares (the free camera, Go To moving the view) is then the parent state itself.
  Each mode's keys are a context active only in it (`active_in_state`), so WASD and the arrows drive
  the free camera in edit and the player's actor in play, as in faery. Reversing it means the
  computed state and an editor-modes list kept in step with it. (PAN22)
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
  MSAA pays for most of it: at `perf.py`'s one stop at play distance the pair costs about 0.7 ms
  more than MSAA did, on Aerdra and the Mac alike (`docs/perf.csv`). High costs up to 3 ms more than
  Low, and Low shows no noise. SMAA has been judged only on the two high-density displays, Mac and
  Aerdra, where aliasing hardly shows. Reversing it means giving up the occlusion to have MSAA back,
  and leaves the prepass unused. (PAN52)
- **D58** The editor has one undo history, not a context per asset as bevy#23373 has per entity. One
  mode edits several assets and two modes edit one (the terrain map, from PAN37 and PAN38), and
  separate histories would lose the order of those edits. Each entry records the mode it was made
  in, and undoing or redoing returns there, so the change lands where its panel shows it. Switching
  modes is not an entry: browsing would add steps to undo and clear redo. Reversing it means
  deciding which history Ctrl+Z means in each mode. (PAN42)
- **D59** What can be done to a thing is one universal aspect, `Interactions`, a list of entries in
  the asset, on anything: a mod can add assets and formulas but no Rust aspects, and actions like
  Unlock cross kinds of thing, where faery wrote them per aspect. Aspects for a kind (`Container`)
  hold its data and native code and no entries, and an exemplar's aspects are the interface `self`
  has. An entry's `when` is a formula, `None` meaning always offered; its `action` names a native
  event registered with the aspect it needs, so an exemplar without that aspect cannot perform it.
  An override replaces the whole list (X74). The probe sweeps a box from feet to head rather than
  casting a ray, to find a chest on the ground and a sign at eye level alike. Reversing it means
  per-kind entries, which only Rust could extend. (PAN21)
- **D60** UI that shows or edits dynamic data is written as if Bevy had a reactive framework, as
  faery had solid.js, so that adopting one when it arrives changes little. A view is a function of
  its state: it reads the model where the model lives and keeps no copy, each read standing in for a
  future tracked dependency. Children that depend on data are built by `bevy_immediate`, keyed and
  owned by their parent, never by a queued spawn into a remembered entity, which can be despawned
  before the spawn lands. What is computed from the data is a plain function of its inputs
  (`filtered_items`), a future memo. A widget's own state (a filter, a selection) sits in a
  component on its root, a future signal. A widget that edits changes the model through a named
  operation (`NavPointEdits`) and does not update its own display, which follows from the model. A
  system that derives a property outside a view (`enable_buttons`) recomputes it from the model and
  writes only on a difference, a future effect. Reversing it means settling on push updates per
  widget, as `build_rows` was, each with its own staleness to manage.
- **D61** The game's world picking casts the pointer's ray at rapier colliders, not meshes, as faery
  did: colliders are what the probe and physics already agree on, and an animated or many-meshed
  model needs no special handling. Behind every hit stands `WorldBackstop`, an entity that exists to
  be hovered during play and carries the game's cursor; the editor's panels keep the system cursor,
  since `DefaultCursor` is global and a window's `EntityCursor` is not read. The editor's view pane
  ignores the pointer, so hits reach the world through it. Reversing it means mesh picking, whose
  hits on terrain and instanced foliage would have to be filtered. (PAN60)
- **D62** How scenery moves is one aspect, `SceneryAnimations`, a list in the exemplar of clips
  played together, each following a named boolean input or looping, as faery's `animations` did for
  all its doors, containers, switches and traps. An aspect for a kind (`Container`) owns the state
  an input reads and knows nothing of clips. Inputs are an enum, each read natively, until SAGA can
  read them. Reversing it means each kind binding and playing its own clips. (PAN63)
- **D63** The listener is not the character's head or the camera but the part of the screen the
  player is attending to, so a sound's loudness is faery's curve from the viewpoint, set when it
  starts, and left and right are the screen's: Bevy's spatial audio only pans it, from a listener on
  the camera. A tiny `SpatialScale` holds rodio's inverse-square distance term at 1, which would
  otherwise measure from the camera and change with zoom. Reversing it means tuning the scale
  against the camera distance and losing the authored cut-off at 16 m. (PAN64)
- **D64** The camera works as film and theatre do: a jump in space or time (another realm, a
  teleport, a passage of time) is a hard cut, and movement within a scene a smooth change of view. A
  gliding camera across a jump reads as travel that did not happen. `Go To` and
  `Viewpoint::follow_snapped` cut; D42's trailing is the smooth case. In play, restoring a saved
  game and respawning after death always cut, however near; a cut scene's change of focus to another
  character is smooth when the two are near, for continuity, which setting `follow` already gives,
  and cuts when they are far. (PAN56)
- **D65** Creating a realm writes its files at once, rather than waiting for Save: a realm exists
  only as files, and an asset with no path cannot go through `UnsavedAssets`. So creation is not on
  the undo stack. Reversing it means giving in-memory assets a path and letting realms load without
  a file. (PAN35)
- **D66** Editor overlays are retained meshes from `panoply_overlays`, forked from
  `bevy_reactor_gizmoids` without its reactive scope, which Panoply does not use; its `ShapeBuilder`
  was rewritten to fix its stroke bugs. An overlay is a component holding just what its shape
  depends on, written only on a difference, and the mesh is rebuilt when it changes: faery's many
  tool overlays without a reactive framework, each a shape and a short update system. The crate
  holds no Panoply types, so it can leave the workspace. Reversing it means immediate gizmos,
  redrawn every frame and without the faint underlay where the scene hides them. (PAN36) Terrain
  overlays are lines instead, Bevy's retained gizmos (`Gizmo`, `GizmoAsset`), rebuilt the same way:
  a contour seen edge-on loses a flat stroke but not a line, and the underlay, which shows where an
  overlay passes through scenery, matters little on the ground. (PAN38b)
- **D67** An actor moves, and gets physics, only where its footprint is built: every parcel and
  precinct under its capsule plus half a metre is loaded and has no colliders still to build
  (`GroundReady`, the `area_waiting` of `TerrainPending` and `SceneryPending`). Elsewhere it holds
  as it is, mid-fall included. Physics ground comes from the parcel and precinct caches, which
  follow the view and evict what they drop with its colliders, so an actor away from the view can
  lose its ground; moved then, it fell through where it stood (notes, the 3 fps spiral). The
  alternative, keeping what lies under actors out of eviction, ties physics further to the drawing
  caches. No step count is kept for new colliders: `move_actors` runs after the realm steps, and
  only the stepped realm's actors move. Reversing it means actors simulate on ground that may be
  gone. faery's `isPhysicsReady` was the same check (faery.md, Actors and navigation). (PAN66b)
- **D68** A frame catches up at most four fixed steps (`MAX_CATCH_UP_STEPS`: `Time<Virtual>`'s
  `max_delta` at 62.5 ms, in place of Bevy's 250 ms, 16 steps at 64 Hz); time past that is dropped.
  A step that costs more than its 15.6 ms otherwise freezes the game: each frame owes more steps
  than the last until the cap, which at 16 steps held the editor at 3 fps (notes, the 3 fps spiral).
  Capped at four, overload slows the game instead, at R + 4c a frame for a step cost c and the rest
  of the frame R. Play is not timed finely, and below about 12 fps the game stops being watchable
  ([goals.md](goals.md), Art), so slowing is acceptable where freezing is not; four still absorbs an
  ordinary hitch without losing time, where two would drop time on any frame over 31 ms. The clamp
  is on virtual time, so animation, camera easing and the fixed steps slow together. The hitch log
  names frames over the cap and the time each dropped. Reversing it means overload freezes the game.
  (PAN66c)
- **D69** A formula in an asset is a string in the field it drives, typed `Formula<T>`, compiled as
  the asset loads against one host frozen at startup ([saga.md](saga.md), Formulas in assets). A
  string keeps assets plain JSON, and compiling at load reports an error when the content changes,
  not when a player first meets the formula. One that fails compiles alone and keeps its error,
  since a typo would otherwise drop every exemplar in its catalog, and the editor can list them
  (X94). Reversing it means a tagged form in the asset, or compiling on first use. (PAN45b)
- **D70** A save chunk's entries are written by aspects, keyed by the aspect's name, with serde
  payloads as MessagePack maps of named fields ([save-games.md](save-games.md), Chunk format). A
  name is short and survives code moving between modules, where a type path does neither; named
  fields give schema evolution by presence and absence, with no version number; and serde ignores a
  field it does not know, where Bevy's reflect deserializer rejects it. Reversing it means a format
  version with migrations, or saves that break on refactors. (PAN45c)
- **D71** Whether a formula's `self` has the components it reads is checked once its exemplar's
  `extends` chain has loaded, not as it compiles ([saga.md](saga.md), Values). A formula compiles as
  its catalog loads (D69), before an `extends` naming another file may have, so the compiler checks
  types and lists the components, and each exemplar's `LoadedWithDependencies` checks them against
  the aspects along its chain. Checking when an instance spawns would find the error only once one
  was placed; checking only at runtime would give up the check. Reversing it means compiling only
  after every chain has loaded, or no check before play. (PAN45c)
- **D72** The property editor's widgets read values and never write them. Each changed widget asks
  for an edit, a root (where the value lives, named by a stable key, D49), a path within it and a
  new value of the field's real type; one system makes the frame's edits after the panel has run,
  records each in the undo history, then copies the shown values for the next frame. Only the walk
  that draws the rows and the roots touch reflection, so a new place to edit (an instance, a nav
  point) is a new root, a type a mod declares (X64) is another walk and root beside reflection's,
  and moving to `bevy_reflect2` (X98) touches those alone. arbor_day's panel, which it forked, wrote
  through `&mut dyn PartialReflect` as it drew, which undo could not record. Reversing it means
  widgets bound to the data again, with undo snapshotting whole values. (PAN28a)
- **D73** Saving a catalog writes its edits into the file's own JSON, read again at save time,
  rather than writing the exemplars out. Each edited exemplar is serialized as loaded and as it is
  now, and only where the two differ does the file change. The exemplars are not the whole file: the
  loader skips aspect types no plugin registers, serde ignores fields no type has, and the file
  omits defaults and writes `extends` relative to itself. What they never held is in neither
  serialization, so it survives, and a save's diff shows only the edit. Reversing it means saving
  the model and losing all of that, or a model that keeps every unknown. Creating an exemplar
  (PAN28e) needs an insert path beside the merge. (PAN28b)
- **D74** A stateful aspect is its authored state, serialized in the exemplar or the authored
  instance; the state play changes is a separate component the aspect inserts, saved in the save
  game, and left out when it equals the authored state ([save-games.md](save-games.md), Chunk
  format). The asset stays the author's, editable while the game runs, and a save holds only what
  play changed. `Container` and `ContainerState` already work so; it is the rule for every stateful
  aspect. Reversing it means aspects whose fields play overwrites, so saves must hold whole aspects
  and an edited asset cannot reach a saved game. (PAN28b)
- **D75** A text input's edit is committed on Enter or on losing focus, and Escape puts its text
  back, through one observer pair on every Bevy `TextInput` (`text_commit.rs`), which triggers
  Bevy's own `ValueChange<String>` with `is_final` set, the event whose doc already covers "input
  lost focus". Bevy's text input reports only keystrokes, and an edit per keystroke would rebuild
  instances for every half-typed path. The module holds no Panoply types, so when Bevy commits text
  itself ([bevy#26500](https://github.com/bevyengine/bevy/issues/26500)), it goes. Reversing it
  means each widget handling Enter and focus on its own. (PAN28c)
- **D76** A property root lends its value to the edit (`PropertyRoot::modify`) rather than writing
  values itself, and the editor makes every kind of edit in one place (`EditOp`): a write, an item
  put into a list, an item taken out, each returning the edit that undoes it. Bevy's `apply` never
  shortens a list, so a list needs edits of its own, and with the operations in one place each new
  root (an instance, a nav point) gets them all by lending its value. A variant switch is a write of
  the whole enum, built from the fields the variants share and the rest's defaults. Reversing it
  means every root implementing every operation. (PAN28f)
- **D77** A model picker lists every named scene of its kind's folders under a row per file, as
  faery's grouped them: `characters/` for a `Skin`, `scenery/models/` and `terrain/models/` for a
  `SceneryModel`. A `Skin`'s list includes the features (hair, hats). Bodies and features are
  sibling scenes, and nothing in the glTF tells them apart. Leaving out the scenes that archetypes
  list as features, or matching name prefixes, would guess. Reversing it means marking body scenes
  in Blender, as scene extras, and re-exporting every character file. (PAN28g)
- **D78** What a property editor root can do beyond `PropertyRoot` is a trait of its own, found on
  the root by a cast through reflection (`#[reflect_trait]`, as the loader finds `Aspect`): roots
  derive `Reflect` and are registered, and an exemplar's root registers `AspectHolder`, which adds
  and removes aspects. A root that lacks the trait refuses the edit. Methods on `PropertyRoot` with
  failing defaults would put each kind of root's operations on every root: nav points, quests and
  dialogues hold no aspects. Reversing it means those methods. (PAN28d)
- **D79** An asset type is an exemplar when its instances may differ in which parts they have, and
  gain from composing aspects: walls, fixtures, actors, items, terrain effects, and floors (PAN74).
  A sequence or a state machine, as a quest or a dialogue, is an asset type of its own, since
  aspects would not compose it. Floors were made assets in 2024, when a floor's only aspect was its
  material; as exemplars they get the list, the property editor and saving that exemplars have, and
  room for footsteps, slipperiness and depressions. Reversing it means an editor of its own for each
  asset type that is not an exemplar. (PAN28d)
- **D80** An actor's place is not saved with its home precinct, on eviction or on save: when the
  precinct loads again, or a save is loaded, its actors start at their authored places and their
  goals take them to wherever the hour puts them. Time passes while a precinct is away, and its
  people go about their day rather than wait, frozen, where the player left them; arriving at night
  may find someone already in bed. A load is a hard cut (D64), so actors moving back across it does
  not read as vanishing in view, and a saved place can never come back stale, inside scenery a later
  phase added (X109). What play changed that goals cannot redo (a death, an inventory, an actor's
  home) is still saved ([save-games.md](save-games.md)). Followers travel with the player and keep
  their places in the player's chunk. Reversing it means saving every moved actor's place, and a
  world that waits for the player.
- **D81** Code and content are licensed apart, by path, in one repo: code stays MIT or Apache 2.0,
  so others can build on the engine, and content (`assets/`, `artwork/`: models, textures, sounds,
  music, maps, exemplars, writing) is CC BY-NC-SA 4.0, so it can be remixed but not sold. Content
  made by others keeps its own licence, listed in [CREDITS.md](../CREDITS.md), as Bevy lists its
  assets. The game's name is kept, so a fork renames itself and the name always means the official,
  free release. The threat is a stranger selling the game, or a copy carrying malware, abroad where
  a lawsuit cannot reach; the stores they sell through act on a takedown notice, which a
  non-commercial licence makes clean and MIT does not. One repo, since a content repo of its own
  would slow every chunk that touches both. Reversing it means the content is anyone's to sell.
- **D82** A mod is data and SAGA, never native code or shaders: no plugins, no dynamic libraries, no
  WESL ([modding.md](modding.md), What a mod can do). SAGA has no file or network access, so a mod
  cannot reach the player's machine; a shader can still hang the GPU, so mods ship none. A mod's
  files are untrusted input: each loader rejects a malformed one without panicking, as SAGA's
  compiler does. Reversing it means vetting every mod as a program.
- **D83** The property editor's layout, mocked up as boards B, C and D of
  https://claude.ai/artifact/YPCJCQ8fXaev38pqT3Dmm9. A row is a caption in a column of fixed width
  at its left and a widget filling the rest; a vector's three inputs share its row. Every × sits in
  one column at the right, in headers and rows alike: headers keep the rows' right padding, and the
  × is no wider than the column. Captions are in sentence case ("Half extents"), each given by a
  `DisplayName` annotation on its field rather than derived from the field's name; list items are
  "#1". A struct's or item's fields are set in behind one stripe, and a variant's fields sit beside
  their siblings, not deeper. Panes, structs and items collapse behind a chevron, and a collapsed
  one shows a summary on its header (a list's length, a collider's shape and size). An aspect's pane
  header has an icon beside its title. The rows still to come: colors a swatch per slot, an unset
  one dashed; features a checkbox per slot in two columns, with "3 of 14" on the header; a map's
  entries a key input and a value; an inherited aspect dimmed, "inherited" on its header. Chosen
  over a Blender-style split with stacked vectors (A), too tall for a 328 px panel, and over a
  restyle after feathers' Figma (inset sub-panes, one ⋯ menu per header, captions inside widgets),
  which read less clearly. Reversing it means restyling every kind of row. (PAN28l)
- **D84** An actor's feature and color slots are read from its skin's character file, marked in
  Blender, rather than listed by hand in `FeatureSlots` and `ColorSlots` aspects. A feature is a
  top-level object with the custom property `feature`, which `pipeline.ts` makes a scene of its name
  (the file's own scene, holding every object, is not a feature); a color slot is a material with
  `color_slot`, which the body scene or a feature scene uses. The hand lists had drifted from the
  meshes: 16 exemplars held 25 colors for materials their skins lacked (deleted here), one prototype
  listed a slot its mesh lacked and missed the material it had, and one listed a feature its file
  lacked. Color slots are opt-in, so a material such as an eye, which varies from model to model, is
  never a slot by accident. The editor reads the slots from the loaded `Gltf`'s scene worlds
  (`GltfMaterialName`, extras), not from the file, through `PropertyMapKeys`, a reflected trait by
  which a map aspect gives the keys it may hold. A key outside the slots stays in the file, unused
  and not shown; `asset_xref.py` reports it. Reversing it means hand lists again, which a validator
  would have to check against the meshes. (PAN28k)
- **D85** An aspect's map with string keys is a `BTreeMap`, and the catalogs keep each such map
  sorted by key, so saving changes only the lines of the keys edited. Saving puts a key the file
  lacks after the nearest key before it in the type's serialization (D73); a `HashMap` serialized in
  hash order, so a renamed key, or one whose removal was undone, landed on an arbitrary line. Order
  means nothing to these maps: each is looked up by key, and the panel shows keys sorted anyway. The
  catalogs' maps were sorted once, here. Reversing it means an ordered map keeping each file's
  order, with undo putting a removed key back at its index. (PAN28k)
