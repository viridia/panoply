# Decisions

Why Panoply is the way it is, for choices expensive to reverse. A decision names the chunk that
made it, where there is one. A superseded decision stays, pointing at its successor; numbers are
never reused.

**Next: D126.**

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
  a precinct drops the skipped aspects; see X20. faery's, so dropped, come back with the precinct
  converter's merge (X69).
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
- **D27** Exemplars and catalog entries (D91) are loaded by path through
  `panoply_exemplar::CatalogServer`, never `AssetServer::load("file#Label")`. Bevy 0.20 starts a
  separate load of the whole file for each `file#Label` requested before the file has loaded
  (`bevy_asset` `load_internal`, `HandleLoadingMode::Force`; Bevy issue #10549): the playground's
  first precinct read `wall-tiles.exem.json` 128 times. `CatalogServer::get` loads the catalog root,
  held in the `CatalogRoots` resource, and returns `None` until it has loaded, then the labeled
  handle (labels are registered before the root counts as loaded). The catalog holds every label's
  handle, aliases included, so labels stay loaded. Precincts wait for their catalogs
  (`ReadPrecinctData`), and a parcel's ground waits for the precinct as before (D11). In the loader,
  `extends` within the same catalog uses `LoadContext::get_label_handle`, since a load by path would
  reload the file being loaded; a missing target is a warning. Checked with a temporary log in the
  loader: `plans/scenery.py` loads each of ten catalogs once.
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
  drawn as a sphere.
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
  the collider follows them. Groups: Floor, Wall, Fixture (by the exemplar's `kind`). Colliders'
  `user_data` is the owning entity, for picking later; the F3 view draws every scenery collider, and
  ground only for shown parcels.
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
  The capsule (`CollisionSize`, R4) is swept as a query shape; that it was not also a collider is
  superseded by D95. An actor gets physics when its footprint is built (D67) and a ray down from it
  hits a collider. (PAN3f)
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
- **D59** Superseded by D114. What can be done to a thing is one universal aspect, `Interactions`, a
  list of entries in the asset, on anything: a mod can add assets and formulas but no Rust aspects,
  and actions like Unlock cross kinds of thing, where faery wrote them per aspect. Aspects for a
  kind (`Container`) hold its data and native code and no entries, and an exemplar's aspects are the
  interface `self` has. An entry's `when` is a formula, `None` meaning always offered; its `action`
  names a native event registered with the aspect it needs, so an exemplar without that aspect
  cannot perform it. An override replaces the whole list. The probe sweeps a box from feet to head
  rather than casting a ray, to find a chest on the ground and a sign at eye level alike. Reversing
  it means per-kind entries, which only Rust could extend. (PAN21)
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
  point) is a new root, a type a mod declares (PAN104b) is another walk and root beside
  reflection's, and moving to `bevy_reflect2` (X98) touches those alone. arbor_day's panel, which it
  forked, wrote through `&mut dyn PartialReflect` as it drew, which undo could not record. Reversing
  it means widgets bound to the data again, with undo snapshotting whole values. (PAN28a)
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
  play changed. `Lock` and `LockState` already work so; it is the rule for every stateful aspect.
  Reversing it means aspects whose fields play overwrites, so saves must hold whole aspects and an
  edited asset cannot reach a saved game. (PAN28b)
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
- **D79** Superseded by D90. An asset type is an exemplar when its instances may differ in which
  parts they have, and gain from composing aspects: walls, fixtures, actors, items, terrain effects,
  and floors (PAN74). A sequence or a state machine, as a quest or a dialogue, is an asset type of
  its own, since aspects would not compose it. Floors were made assets in 2024, when a floor's only
  aspect was its material; as exemplars they get the list, the property editor and saving that
  exemplars have, and room for footsteps, slipperiness and depressions. Reversing it means an editor
  of its own for each asset type that is not an exemplar. (PAN28d)
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
- **D86** Dialogue and remarks compose apart from goals, each through one native aspect listing set
  assets, with a flag to inherit the parent's list ([Exemplars.md](Exemplars.md), Dialogue and
  remarks). An aspect per package needs keyed or external aspects, still open for goals; one
  ordinary aspect needs neither, can be built now, and keeps a guard's quest lines an override, not
  an added aspect. Given up: a quest's goals and lines travelling as one bundle, and two mods adding
  lines to one character both surviving (X123). Reversing it means dialogue in behavior packages.
  (PAN99)
- **D87** A precinct instance's `SceneryInstanceId` is `Keyed`, a key saved with it, or `Internal`,
  a number assigned as the precinct loads, neither saved nor stable across edits. Its label, the
  name text uses, is kept beside the id, so a rename changes no identity. Scripting and saved state
  ([save-games.md](save-games.md)) refer to instances by key only, so an instance has one while it
  has a reason, and saving gives and takes keys ([instances.md](instances.md), Ids). Other walls and
  fixtures, most of a precinct, carry none, which keeps them featherweight. Whether a door or chest
  is open does not save, so it needs no key for that. Reversing it means internal ids saved and kept
  stable through every edit. (PAN99, PAN110)
- **D88** An aspect's config is what is authored: its fields, written on an exemplar, or on an
  instance to override the exemplar's aspect of that type; the two are layers of one kind, not two
  kinds as faery's props and config were. Its state is what play changes and a saved game keeps (a
  container's contents, a lock), declared by `Aspect::has_state`; app preferences are settings
  (`SettingsGroup`). So an instance whose exemplar chain has an aspect with state gets a key (D87)
  when its precinct saves. Reversing it means a save game finding an instance some other way.
  (PAN94, PAN110)
- **D89** A placing tool only places: a press puts the instance where its own shapes, cast along the
  pointer's ray, first touch, dropped onto what is under them, and a drag turns it toward the
  pointer on a dial, before the release commits it (`placement.rs`). So a mug goes on a table
  without selecting the table, and the turn that most often follows a placement needs no mode
  switch. Picking what is there is selection mode's (PAN88), and moving or turning it afterwards the
  gizmos' (PAN92). A drag's turn is that placement's alone; the keys set the tool's facing, since a
  vase set at an angle is more often followed by a bed square to a wall than by another vase.
  Wall-hung fixtures need no case of their own: they are raised in their exemplars, and land against
  a wall. Reversing it means place mode also selecting, as a sequencer's notes do, and an instance
  on another losing to the instance under it. (PAN96)
- **D90** An asset type is an exemplar when its instances may differ in which parts they have, and
  gain from composing aspects into entities: walls, fixtures, actors, items. Every other asset type
  loaded and saved is a catalog entry, a typed record with no aspects and no prototype: floors,
  terrain effects, books, quests, dialogues, spawn tables. Catalog entries share one generic loader,
  list, property editor root (D78) and save path, each monomorphized per kind (D91), so a new kind
  needs its type, not an editor of its own, as faery's `IMetadata` was the base of everything its
  editor loaded and saved. Floors and terrain effects compose nothing: a floor has no parts, every
  terrain effect holds the one aspect `TerrainEffect` but the hole, which holds `TerrainHole`, none
  extends another, and none spawns. D79 made them exemplars for the editor exemplars had. Reversing
  it means aspects on records that have no parts.
- **D91** A file of catalog entries holds one kind: a JSON object from label to entry, its extension
  naming the kind (`floors.floor.json`), any number of files per kind. An entry is `file#Label`, as
  an exemplar is, loaded through `CatalogServer` (D27); it has no aliases, so a rename rewrites its
  references. A kind is a Rust type (`CatalogEntry`), read through reflection onto its default, so a
  file leaves out what is at its default, and saved as D73 saves a catalog, a field edited back to
  its default leaving the file. Shared code is generic over the kind, not type-erased: the loader
  (`EntryCatalog<T>`), the editor's root (`EntryRoot<T>`) and list, a line per kind. Files mixing
  kinds, as regional bundles of spawn tables and dialogues might want, are X117. Reversing it means
  type data per kind and untyped handles throughout. (PAN74a)
- **D92** A loader of a catalog entry does no GPU work: an entry is data, and what is built from it,
  as a floor's material, is built by a system and rebuilt on `AssetEvent::Modified`. So an edit in
  the property editor, or a reload, shows at once; the floor loader built its material once, and no
  change reached it. Reversing it means a reload per edit. (PAN74a)
- **D93** The editor picks a small or invisible thing by its `marker` colliders: chalk marks on the
  stage, such as patrol paths, music cues, the sensors of traps and quests, and wandering's tethers.
  They are colliders in the `MARKER` group, which the game's own casts and movement leave out by
  filtering on `BLOCKING`, so a marker is the editor's handle alone and the game reads the mark
  itself. Hints and markers are collider types rather than aspects of their own so that one editor
  places every collider-like volume. A hint is for navigation alone (PAN16f), never built as a
  collider nor picked, as faery built none: beside a door or a stair, it would take the click meant
  for it. Selection mode's ray is the game's (D61) with other groups, stopping at walls, floors and
  ground, so nothing is picked through a wall. Reversing it means a pick shape of another kind, and
  queries that need no filter to miss it. (PAN88)
- **D94** The editor's world tools take the pointer through picking: a backend reports what the
  pointer is over in the world, and the tool observes picking's events on it. So whatever is in
  front, the panel, a widget over the view or a dialog's barrier, takes the press first, as picking
  routes it; a tool polling `PointerState` sees every press, and has to guess which were meant for
  it (PAN103). Picking hygiene: an observer that handles a picking event consumes it
  (`propagate(false)`), so it reaches nothing behind or above its target, unless the observer only
  adds to what another does. Reversing it means each tool filtering presses itself. (PAN88)
- **D95** An actor's capsule is also a collider in its realm's world, in the `ACTOR` group, so that
  picking, the probe and selection mode can hit an actor; the controller's casts filter on
  `BLOCKING`, which leaves `ACTOR` out, so actors never block each other (R86). The collider has no
  body: rapier treats it as fixed, and its broad phase drops pairs of fixed colliders before the
  narrow phase sees them, so no contacts are computed and it need not be a sensor. Its filter is
  every group, since a query meets only colliders whose filter holds the query's groups. It follows
  the actor's tick pose, set only when that moved, because any mutable access marks a collider for
  the broad phase's update; and its insertions and moves do not count as changes to the world, which
  wake resting actors (PAN3p), since nothing stands on an actor. The F3 view still draws actors'
  capsules itself, at their drawn pose, rather than rapier's at the tick's. One capsule serves
  picking and movement, where faery inflated a second for picking ([faery.md](faery.md), Physics),
  until picking an actor feels fiddly. Reversing it means filtering every pick by hand against a
  list of actors. (PAN89)
- **D96** Cutaways (R15) are chosen as the three.js version chose them ([faery.md](faery.md),
  Scenery), but by the triangle between the ray up from the player and the line to the camera, where
  faery tried a point on each: on each tier the triangle is a segment from under the player toward
  the camera, and every rect it crosses is chosen, so a ceiling anywhere between the player and the
  camera is caught. The camera always looks down, so what lies above it is never in view and the
  segment may stop there. Elements and actors are hidden by their origin, as in faery, not their
  bounds, which would also hide the ground floor's walls beside a rect, reaching up into its volume;
  floor regions by their outline, each on its own, where faery hid a tier's whole floor. The
  player's cutaways and the editor's (Cut, the wall tool's) are lists in one `Cutaways` resource,
  and only what it changes, or what was added or moved, is tested again. Picking and placement pass
  through what is hidden by asking whether the collider's owner is drawn (`owner_shown`, its
  `InheritedVisibility`), not by a group of their own as faery cleared its Visible bit: whatever
  hides an entity, cutaways or anything later, then hides it from picking too, and no collider
  changes when a cutaway does. Reversing it means rewriting colliders' groups as cutaways change.
  (PAN76a)
- **D97** Navigation's input and mesh are Panoply's own types in `panoply_nav`, in the precinct's
  coordinates (its corner at the origin), so a boundary edge lies exactly on x or z = 0 or 64 and a
  saved input does not depend on where its precinct is. The input is plain data: scenery as its
  `ColliderDesc`s where each element stands, so a collider's type, margin and walkability reach the
  generator, and tree trunks as solid cylinders beside them, and each portal's aperture as a
  `Portal` collider, which the generator marks as a portal area, as faery's builder did, so a route
  crosses it or not by who asks: the player's click-to-move may, NPCs may not (R31, PAN16d); floors
  as triangles; and the ground at one vertex per meter, taken from the ground collider's corners,
  not its sixteen triangles to the cell, so its heights carry the smoothing and terrain effects but
  not the 0.25 m detail between; and each 1 m cell's footpath flag (cobbled effects), with a ring of
  the neighbours' cells, as faery's builder took them for its path regions (PAN16c). The canopy
  (D39) is left out, since no route goes over the trees. Rapier's own shapes were the alternative,
  but saving them needs its `serde-serialize` feature and writes each trimesh's BVH, tying the
  fixtures to parry's version. The mesh holds 3D vertices, not faery's plane and 2D vertices per
  polygon, which stay inside the generator (PAN16d); the detail triangles that carried Recast's
  non-planar polygons went with it (D99). Reversing it means changing the generator's input and
  output. (PAN16a)
- **D98** The port of faery's builder (`panoply_nav::clipping`) favours a lean mesh over an exact
  one: navigation approximates how a creature moves, an actor's capsule is round, and its motion
  smooths the corners of a route, so a turn's exact curvature goes unseen, while each vertex costs
  in every later step, the route search included. So footprints are faery's per shape, a box
  square-cornered and a round shape an octagon, grown by the agent's radius and margin; footprints
  grown exactly, as rounded hulls, tripled the lair's polygons (2,878 to 997). Outlines are cut in
  one fixed-point space, about a millimeter, so the same point from two operations is one point and
  neighbours are found by their shared edges, each split first where another piece's vertex lies on
  it; faery's R-trees went. A surface removes whatever lies under it within an agent's height; at
  one height only one keeps the part, a floor over the ground and otherwise the first built, since
  two that both win leave a gap between them. Surfaces whose edges meet within half a climb join,
  where faery rounded a floor's height to the meter to join it. A walkable collider blocks the
  ground and floors but not other walkable colliders' tops, so a piece's ramps and platform join, as
  faery's ramps did not cut ramps; its top past its own edge is held to its highest point, so a
  stair does not cut the floor it climbs to. Floors are not obstacles, as in faery, so the ground
  beside a raised floor stays. (PAN16d)
- **D99** Navigation meshes are built by the port of faery's builder (D98), not Recast. On the two
  fixtures it built a precinct in 16 to 28 ms at under 1 MB, where Recast at 0.1 m cells took 103 to
  123 ms and 29 MB, for about as many polygons (1,014 to Recast's 1,044 in the lair); and its edges
  follow the obstacles exactly, where Recast's fall on its grid, which squeezed stairways and needed
  doorways and footpaths widened to survive. Its costs: about 1,100 lines Panoply maintains, against
  a crate, and polygons that are planar, the ground's heights rounded to a quarter meter as faery
  did, so a mesh lies up to 0.125 m off the ground; routes need only where to go, and actors stand
  on physics. Faery's joining of surfaces (`isSamePlane`, `projectVertexToPlane`) adds nothing the
  port lacks, read rather than ported: it merged one plane's regions, which the port does by plane,
  and joined edges collinear in 3D, which shared fixed-point vertices and the join tolerance cover.
  So `rerecast`, the editor's choice of generator and the polygons' detail triangles are gone.
  Reversing it means a generator filling the same `NavMesh` from the same input. (PAN16e)
- **D100** A precinct's navigation mesh is rebuilt when one of the markers that rebuild what it is
  built from is inserted (its scenery, an element's colliders, a floor, its terrain effects, a
  parcel's ground or flora), not when a tool's edit ends: one rule covers every tool, the catalog's
  live edits and a reload, and a drag's rebuilds mid-way cost about 20 ms each off the main thread.
  Only a precinct that has a mesh or a build is marked, so loading builds none unasked. A stale
  build is dropped, which cancels its task, in place of a generation count; the old mesh stays until
  the new one lands. What changes navigation without those markers queues `MarkNavMeshStale`. A door
  polygon names its door by element, so a gate's two leaves are one door, and the element's entity
  is held because a door's state lives there; the mesh's lifetime fits within the entity's, except
  while the old mesh outlives a despawned door, which a reader takes as stale. Reversing it means a
  counter on the precinct and an end-of-edit signal from each tool. (PAN16f)
- **D101** The simulated precincts (R8, `Viewpoint::simulated_precincts`) stay loaded with every
  parcel under them, whatever the camera sees: the precinct and parcel caches load them beside the
  camera's own area, and a parcel out of every camera's view stays hidden, colliders built but not
  drawn. A navigation mesh needs all sixteen of a precinct's parcels, and the camera's area at game
  zoom (about 28 m about the viewpoint) is smaller than one precinct, so before this a mesh waited
  until wandering had cached them all; actors there need the same ground. The box's rule has one
  owner, read by both caches and `SimulatedPrecinct`. No ring of parcels beyond is needed: a
  parcel's edges blend its neighbours' contours, but from the realm map, copied into the parcel. The
  cost is at most 64 parcels building ground and flora out of view. Reversing it means meshes that
  wait on the camera, or built from the parcels loaded. (PAN16f)
- **D102** Actors push each other apart as a velocity added to their travel, eased and capped at 4
  m/s, rather than as contacts between bodies, so the controller still sweeps a pushed actor against
  the world (`actor_bump.rs`). The push starts once two capsules overlap by 0.1 m: a capsule is
  personal space, wider than the body, so overlapping capsules still read as one actor stepping
  aside, and a player walking up to talk does not shove. Who yields is decided by weight, computed
  from the capsule's volume, not set in an aspect, since size is already authored; an `ActorFooting`
  of immovable makes it infinite, a state a goal sets, as for a guard blocking a door. The player
  weighs what its size does; a heavier player made NPCs give way unnaturally. PAN16i's route costs
  read the same `actor_weight`. Reversing it means rapier bodies for actors, or an authored weight.
  (PAN90)
- **D103** An override replaces a whole aspect, and fields set at different stages go in different
  aspects, as `Portal` and `PortalTarget` do: a chest's container type in `Container`, its key in
  `Lock` ([Exemplars.md](Exemplars.md), Config and state). No field inherits alone; X99 keeps its
  gate. In faery no field was set at both stages, and instances changed an aspect's config only
  twice, by mistake ([faery.md](faery.md), Aspects), so the split follows lines its aspects already
  drew. A scripted aspect splits the same way, into aspects of its own that `#[require]` joins
  (D121). Given up: an instance that wants a field of its exemplar's stage restates that whole
  aspect. Reversing it means `Inherit<T>` fields and a property editor that shows each as inherited
  or set. (PAN99)
- **D104** An instance may add aspects its exemplar chain lacks, but never remove an inherited one,
  as in faery, which added them 23 times, for per-character behavior and for logic doors
  ([faery.md](faery.md), Aspects). So `Lock` is a mixin any chest takes, rather than a chest and a
  lockable chest being two exemplars. A formula on the exemplar that reads an aspect only instances
  add reads it as optional (`?.`, `??`), which D71's check allows ([Exemplars.md](Exemplars.md),
  Config and state), and an instance that adds an aspect with state gets an external id, as D88
  gives one whose chain has it. Dropping inherited content is the aspect's own, as `Dialogues`'
  `inherit` flag does (D86). Reversing it means behavior and wiring on per-instance exemplars.
  (PAN99)
- **D105** State that native systems read is a native component, and behavior over it may be
  scripted. A scripted aspect writes its own state and the native components declared for scripts to
  write, never another scripted aspect's fields, so native code never knows a script's field names,
  and one scripted aspect drives another only through native state or formulas reading it. faery's
  aspects composed through one namespace of fields per instance, so `wall:directDoor` drove `door`'s
  `open` by name, and its container screen closed the lid by writing it ([faery.md](faery.md),
  Aspects). A door's open and locked state stays native, since routing reads it (PAN16j), while a
  key check or a trap's timing can be a script (PAN106). Reversing it means native systems reading
  scripted fields by name. (PAN99)
- **D106** Superseded by D123. Whether an actor is present (R10) is a native component, since the
  actor controller, the editor and paused goals read it (D105), and the conditions it combines come
  from aspects: a behavior package's `present` hook, a reactive formula in PAN104's set, and a
  native `Presence { when }` aspect, which an instance placed for one quest stage adds (R85, D104).
  The actor is present when it has no condition or any condition holds, as in faery, whose three
  uses phase *Up in Arms*' Sergeant Stone and Crimson Watch (`heroncreek.tsx`); the latch that keeps
  an actor on screen until it leaves, and the editor showing every actor, stay native. faery's other
  actor hook, `idle`, has no successor: its one use, `combatant`'s stance while in combat, is an
  animation state (D48) that comes with combat, falling back to `Stand`; a character's own idle clip
  is `ClipNames`', and idle activities are `Pose` goals. Reversing it means presence decided by one
  aspect, or an idle hook choosing clips. (PAN99)
- **D107** An aspect is native, a Rust type, or scripted, declared in a `.saga` module (PAN104);
  behavior packages are scripted aspects ([Exemplars.md](Exemplars.md), Packages written in SAGA).
  Both are aspects in every sense: the union, `OwnedAspects`, the editor, undo and the files key
  them by an aspect key, not a `TypeId`: a native aspect's short type name, as the files write it
  now, or a scripted one's package path and name, whose `::` keeps the two apart. An aspect's state
  is a component on its entity, as D74 has it for native aspects, so reactions and saves treat both
  kinds alike. So is its config by default: attach inserts the resolved config, as every native
  aspect does now, so systems query it, formulas track it, and a reapplied aspect reaches them as a
  change; an aspect whose config nothing queries may skip it, where memory or archetype counts ask.
  A native aspect's config is a `Bundle`: several components where systems want them apart, `()`
  where nothing queries it, so skipping is a type, and detaching is a generic `remove` of the bundle
  rather than a hand-written detach per aspect. `Aspect` itself stays dyn-compatible, the bundle
  typed behind a per-type erased layer, as `GoalEval::of::<Self>()` is for goals (X17). A scripted
  aspect's config is one registered component, or none when it declares no config. The entity's
  union also keeps each aspect's resolved definition, beside its detach, so that gathering like data
  from aspects of different kinds (an actor's goals from several packages), which Bevy's queries
  cannot, iterates one entity's aspect list and calls trait methods; where there is no entity, as
  for an item in an inventory, the definition is read from the asset (`find_aspect`). Gathering
  across kinds for every entity every frame stays dear, and nothing needs it. Native and scripted
  aspects look and behave the same in the editor, as far as that can be made true: one aspect list,
  one add menu, the same inherited and overridden display, rows edited through reflection, and an
  instance part grouped with its aspect, the aspects it requires (D121), scripted or native. So an
  author never needs to know which kind an aspect is. Given up: a package as an asset type of its
  own. Reversing it means behavior kept out of the aspect list. (PAN99)
- **D108** A missing piece costs only itself. A quest role whose actor is missing stays unbound and
  is logged, as a missing fixture is, and the quest goes on (D47). A scripted aspect whose module is
  missing or fails to compile is not attached, and its instance's data for it is kept as written, so
  a save writes it back unchanged (X20). A goal `tree` that fails to compile disables the aspects
  that use it, not its module, as a formula in an asset fails alone (D69). Reversing it means a
  module's error taking its precinct or its character with it. (PAN99)
- **D109** Routes come from a port of faery's `NavRouteTask`, not a crate, since Panoply builds its
  own meshes; it keeps faery's costs per area and its getting as near as it can, and fixes its
  search, which took the first route found rather than the cheapest (`route.rs`), and which entered
  each polygon at its crossing's middle: a long edge, as along a precinct's seam, cost metres no
  route walks, and turned routes aside, so a polygon is entered where the walk toward the goal is
  shortest (PAN17a). A goal off the mesh is reached as near as can be, or within the query's reach,
  as a chest is opened from any side, so a route fails only for a start off the mesh or a precinct
  whose mesh is not built. A click on a wall routes to the ground at its foot on the side clicked,
  not to the floor just behind it, which is nearer. Reversing it means a crate's mesh format, or
  routes that fail outright where faery got close. (PAN16g)
- **D110** Following a route is a body action, `TravelTo` through the route's turns, below a
  request, `RouteTo`, that plans it, waits while a mesh is not built, and replans when blocked; the
  player's click and NPCs' behaviors make the same request, as faery gave its player a temporary
  action of the kind it gave NPCs (R92). Blocked (R22) counts from where the stall began, not tick
  by tick as faery's did: pressed against a slope, an actor climbs a little and slides back, and a
  tick's climb reset faery's count, so it never replanned. A waypoint overshot slightly is passed if
  the actor is near the next leg, rather than turned back to; the slack is after the corner, not
  before it, since a route's corners lie the agent's radius from what they bend around, and that is
  the humanoids' capsule radius with no margin. Reversing it means a planner per kind of actor,
  actors that stay pressed against a slope, or actors that turn back at a corner. (PAN17a)
- **D111** A portal's view is a camera of its own (D21): the primary camera moved by the offset from
  the near aperture to the far one, keeping its rotation and lens, as faery's was, so a portal
  translates and never turns. Its projection, `PortalProjection`, is the primary camera's
  perspective cut to the aperture's rectangle on screen, with Lengyel's oblique near plane at the
  far aperture, built into one matrix. Bevy's `SubCameraView` makes the same cut for drawing only: a
  camera's frustum comes from `get_clip_from_view`, which ignores the sub view, and the camera
  system resets the aspect ratio to the camera's own viewport, so culling, Bevy's and D20's, and the
  shadow cascade would fit a frustum centered on the view. The camera renders into the corner of an
  image the size of the primary camera's viewport, which the aperture shows texel for pixel. The
  aperture is on a render layer of its own (`PORTAL_APERTURE_LAYER`), which the primary camera
  renders and portal cameras do not: a portal may look into its own realm, and a camera may not read
  the image it writes. The far end loads 8 m about, as the viewpoint's surroundings do, for every
  portal within 24 m of the viewpoint, so that it is there before the aperture comes into view;
  nothing there is simulated (X119). Reversing it means culling that drops what an aperture shows,
  or a camera that can read its own image. (PAN4b)
- **D112** Plans are run by what a failure costs. A broken game reaches players; a broken editor
  reaches only its author, who fixes it on noticing. So a chunk's end runs the game's plans and the
  plans of the editor areas it touched, and the first chunk to end each day runs every plan, so that
  an editor break is found with at most a day's chunks behind it. The logic under a rarely used
  editor feature (a floor's height raised) is checked by `cargo test`, with no window, and its plan
  clicks it at most once, to show it is wired. Plans do not check overlays or layout: overlays are
  debugging aids, used to find bugs themselves, and layout is visual, seen when the pane is next
  used. Reversing it means every chunk's end paying for the whole editor again. (PAN118)
- **D113** A scripted aspect's config and state are each a component with an id of its own,
  registered at runtime (`ComponentDescriptor::new_with_layout`), all laid out as one Rust type,
  `ScriptedFields`, which holds the aspect's declaration and its values ([saga.md](saga.md),
  Aspects). So change ticks, queries and formula tracking tell scripted aspects apart as they do
  native ones, and a clone handler of its own copies them when an entity is cloned. They are read by
  id through `ScriptedComponents`, never `ReflectComponent`, which would go by `ScriptedFields`' own
  id. A scripted aspect's key is its module's path from the asset root and its name
  (`scripts::toggle_switch::ToggleSwitch`), absolute, so no `package::`. A formula reads it by the
  accessor its `#[accessor(...)]` names; one without it cannot be read. Until D117's index (X139), a
  formula sees the scripted aspects its own catalog file names, the modules its loader loads anyway.
  Given up: a scripted aspect is opaque to Bevy's reflection, so the inspector, BRP and saves reach
  its fields only through Panoply's code. Reversing it means one component holding every scripted
  aspect of an entity, whose change ticks cannot tell them apart. (PAN104a)
- **D114** Interactions are declared in scripted aspects, beside the state they act on, each with a
  caption, a `when` and an effect run as it is performed (D105); the JSON `Interactions` aspect and
  `register_interaction_action` go. D59 made one universal aspect because a mod could add no Rust
  aspect; a scripted aspect is how a mod adds behavior, so the reason holds, and an aspect's
  behavior stays in one place. An exemplar offers the interactions of every aspect it has, and its
  aspects are the interface `self` has, as in D59. The probe sweeps a box from feet to head, as D59
  had it. Reversing it means interactions written in exemplars again, apart from the state they
  change.
- **D115** Behavior lives in assets, not Rust. Rust holds the state that native systems read
  (physics, routing, animation), host functions and host commands; what is done with them is SAGA by
  default: a sensor's filter, a lock's key check, a trap's timing, an interaction's action. Native
  code needs a reason, a system that reads the result or a cost a script cannot meet. So a mod
  reaches as far as the game's own content does, and game design is tuned without a rebuild. With
  D105, which keeps the state native. Reversing it means behavior that only Rust can change.
- **D116** A native aspect follows scripted state through a read-only formula in its config: a
  scenery animation's `when`, replacing the `AnimationInput` enum, plays its clips forward while it
  is true. Native code holds the author's formula and never a script's field name, so D105 holds.
  Not marker components that a script sets: those need a marker per state (open, active, ...), and
  give a save two sources of truth to keep in step; with a formula the state is the one source, and
  a save loaded mid-transition snaps to the pose its `when` gives. The line between assets and
  modules: a read-only formula that binds a native aspect's config stays embedded in the asset (an
  animation's `when`, `Presence { when }`, a dialogue option's condition); anything with an effect
  lives in a module. Reversing it means native state mirrored for every scripted state an animation
  shows.
- **D117** Scripted accessors share one namespace with native components, so a formula embedded in
  an asset reads `self.toggle_switch.on`, typed, without an import: the import's only job was the
  type, which a dynamically typed language would not need. An accessor is unique across loaded
  modules. The modules that declare one are listed in an index and load at startup, so behavior
  packages, large and regional, need not ([saga.md](saga.md), Aspects). Given up: a pass reading
  only declarations, which reads every regional file at startup, and finding a module from an
  accessor's name, which a renamed file breaks. Reversing it means imports in assets that embed
  formulas. (X139)
- **D118** An instance's kind is a closed enum, `InstanceKind` (PAN119): native systems branch on it
  (colliders, cutaways, the placing tools), so a kind a mod added would have nothing that builds or
  places it. A floor is no instance and has no kind. The kind is on the exemplar, not on each placed
  instance, so a precinct's file never decodes one. Reversing it means a registry of kinds, each
  with what builds and places it.
- **D119** SAGA's cells live in one `Reactions` resource, not in components on their owners: a
  cell's formula reads the world while the reaction pass writes the cell, and the pass takes the
  resource out of the world to do both. That makes the pass an exclusive system, so it runs in a
  schedule of its own after `Update`, where it holds up no parallel systems, and what `Update` reads
  of a cell is a frame old. Ownership stays by entity, as [saga.md](saga.md), Memos has it: whatever
  adds a cell removes it when the component holding its id goes. Every cell and memo compares its
  new value with its last, and only a difference counts as a change, so there is no memoized and
  unmemoized computation for an author to choose between; a value type with no cheap equality, as
  lists will be, counts as always changed. A memo outlives its cell's evaluations, its state must,
  so a call compares its arguments with the last call's. (PAN105) Reversing it means cells as
  components, read and written by a system that cannot also hand the world to the VM.
- **D120** What an aspect declares besides fields is a keyword form, `interaction use { ... }` and
  later `on attach`, whose contents the host checks: an interaction is a struct literal of the
  interaction type the host registers, its fields each a constant, a formula or an effect, and hook
  names will be a table the same way. So the parser knows two keywords, the SAGA crate knows nothing
  of captions, and a misspelled field is a compile error. The alternative, one general declaration
  whose slots may hold functions, checked against a schema per kind, gives the parser no keywords
  but makes every effect a closure and gives fields and hooks one shape. An interaction's fields are
  constants but for `when` and `perform`, so `when` is the one thing that changes what is offered,
  and the prompt and the cursor need not follow their own formulas; a changing caption is two
  interactions. Its fields are written `name: value`, with no shorthand ([saga.md](saga.md),
  Aspects). (PAN104c) Reversing it means hosts registering whole declaration kinds, or prompts that
  react to formula captions.
- **D121** A scripted aspect has no `instance` block: fields set at different stages are different
  aspects, scripted as native ones are (D103), and an aspect that needs another names it,
  `#[require(SignalInput)]`. The requirement is checked as an instance spawns, against the aspects
  it resolves to, its own and then its chain's, and never against an exemplar, so a base exemplar
  holds no empty default for it, and an instance and a derived exemplar override the required aspect
  alike, whole. A required aspect the resolved aspects lack attaches at its defaults; if a field of
  it has no default, it cannot, and the aspect requiring it is not attached, logged with the
  instance and the field (D108). So a field without a default is required, as QML's
  `required property` is, rather than taking its type's default. The editor shows an aspect with the
  aspects it requires (D107). Given up: an instance's fields named bare in its aspect's interactions
  and read through that aspect's accessor; they are read through the required aspect's. Reversing it
  means instance overrides that differ from a derived exemplar's. (PAN104b)
- **D122** Mechanisms are wired by signal bindings, pulled by the sink: a binding names an instance,
  one of its aspects and a field of it, is held in an aspect of the sink, and is never known to the
  source. Only a field marked as an output can be bound, `#[signal]` on a scripted field and
  `@Signal` on a native one, so the picker offers outputs alone, usually leaving one aspect and one
  field and so choosing them for the author, and renaming an output visibly breaks what binds it.
  Each source's output is its own field (a lever's `on`, a plate's `SensorState.active`), so there
  is still no shared `active` such as faery's `controlledOpenable` read. Loaded, a binding becomes
  the read a formula would make, so the reaction pass tracks it and D71 checks it: data for the
  author, a formula's read for the engine. One whose instance, aspect or field is gone reads `false`
  and is logged (D108). A mechanism taking an input requires a general input aspect, one source
  (`SignalInput`) or several combined, any or all (`SignalInputs`). The editor draws a sink's wires
  from its bindings and a source's from a reverse index it builds, of the loaded precincts as they
  load and are edited, and across the game in the instance index, which gains a line per binding
  that `asset_xref.py` checks. Relays are only gates, logic over several signals giving a new one
  (not, and, or, a latch, a timer): placed instances whose inputs are bindings and whose output is a
  signal. PAN79 proposed relays holding all wiring because faery's wiring, stored on the parts it
  joined, left no list of what was wired; the reverse index is that list. Signals are booleans until
  a counter or a level needs more, when the picker filters outputs by the input's type. Reversing it
  means wiring held on relays, or formulas typed as wiring. (PAN79)
- **D123** Whether an actor is present (R10) and what it plays while standing each come from one
  native aspect embedding a formula, as `SceneryAnimations` does: `Presence { when }`, a `bool`, and
  `Idle { clip }`, a clip name from the actor's `ClipNames`. Keyed by type, an entity holds at most
  one of each, so nothing combines conditions or picks between clips, which faery's `present` and
  `idle` hooks on scripted aspects never specified; an instance placed for one quest stage replaces
  its exemplar's `Presence` whole (D103). Behavior packages have no `present` hook: a package that
  has a say exposes a field, and the formula reads it. A formula embedded in an asset imports no
  module (D117), so logic longer than a line goes in a scripted aspect's `derived` field, read
  through its accessor (`self.wounds.staggering`). The actor is present without a `Presence`; the
  latch that keeps it on screen until it leaves, and the editor showing every actor, stay native.
  Without an `Idle`, or with a clip the model lacks (warned of once), standing falls back to D48's
  chain, and a `Pose` goal plays over either; faery's combatant stance is an `Idle` reading combat.
  How a quest reaches an actor's presence is X123's; if quest-side, the quest supplies the bound
  actor's `Presence`, overriding its own. Reversing it means conditions combined from several
  aspects, or an idle hook choosing clips. (PAN125)
- **D124** A SAGA slot holds only a primitive, an entity or a string. A value scripts cannot see
  into (an enum, a list, a handle) keeps its slot in its struct's layout, so the fields after it
  keep their offsets, but a formula that reads it, or a value holding it, does not compile; enums
  are X144. Such slots once referred to the reflected value, but nothing read them, and a native
  taking one would have taken `&dyn PartialReflect` with no type checked. The VM still borrows the
  world for `'w`, for a component's strings, which are borrowed rather than copied. Reversing it
  means a slot kind for reflected values again, with the type of what it refers to checked where it
  is read. (PAN124)
- **D125** The game loads only the precincts a realm's index lists
  (`scenery/precincts/<realm>/index.json`, `PrecinctIndex`), rather than reading the folder, which
  packaged and web builds may not allow, or asking for every precinct in view. A precinct with no
  file is spawned with no asset (`Precinct::asset` is `None`), and counts as built. The index is
  loaded by its type and its loader claims no extension, since `.json` is the exemplar catalogs'.
  The editor gives such a precinct a handle at its first edit by loading its path, which fails and
  logs once, as Bevy has no public way to make a path's handle without loading it; saving writes the
  file and lists it in the index. A new realm starts with an empty index. The game reads an index
  only when it spawns a precinct, and nothing reacts to one changing: a save writes several files
  with no transaction, so nothing should hang on the order the watcher reports them in. A precinct
  whose file appears while the game runs shows once the cache spawns it again. Reversing it means
  folder loads, or a request per precinct in view again. (PAN100)
