# Notes

Measurements, Bevy's observed behaviour, and recipes, as of when each was written. A note that
calls for work becomes an issue in [issues.md](issues.md).

- Test machines, by the short hostname that `docs/perf.csv` records:
  - `Falryn`: MacBook Pro (MacBookPro18,3), Apple M1 Pro (8 performance and 2 efficiency cores,
    16-core GPU, Metal 3), 32 GB, built-in 3024x1964 Liquid Retina XDR display (up to 120 Hz).
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
- Hitch log (`panoply_editor::diagnostics::hitch`): logs each frame over 14 ms after startup,
  with what the three frames before it spawned (precincts, parcels, parcels shown, scenery,
  scenes, meshes, images, pipelines, physics steps, colliders), and each pipeline created after
  startup with its shader defs. Opt-in: `PANOPLY_HITCH_LOG=1 cargo run`, then grep the output
  for `Hitch:`; `plans/scroll.py` turns it on for its own run.
  Before D26, a cold walk (`plans/scroll.py`) showed 40-45 ms frames from pipeline creation and
  17-32 ms frames from scene bursts; after, neither. Frames of 15-24 ms with nothing counted
  remain, at the same rate in cold and warm passes, so not load-related. Not the driver's
  polling: a run without the driver, idle at the start location with the window in front
  (2026-11-01), logged four single missed vsync slots (15-19 ms) in the first seconds after
  loading, nothing spawned, then none; 119.4 fps between them. Moving the window to the back
  causes a run of 16 ms frames and a stall of about 60 ms, so read hitch logs from a run kept in
  front (`> file`). With pipelined rendering, `Time` comes from the render thread, so a stall
  shows in `delta`, and in the log's frame number, a frame or more after it happens.
  Screenshots stall a frame by 350-480 ms (the window comes to the front).
- The FPS counter flickers even in an empty view, as Bevy's `fps_overlay` example does: it shows
  `smoothed()`, and `FrameTimeDiagnosticsPlugin` sets the smoothing to 2/121, which
  `Diagnostic` reads as seconds (16.5 ms, about two frames at 120 fps), so it is nearly the raw
  per-frame rate. Not a frame-pacing problem; a Bevy issue is drafted but not filed. A display
  counting frames over a second, with the slowest frame as a "low" FPS, would read steadily.
- Actors at the playground's townsfolk (145, 6), distance 20, `AT=townsfolk plans/ablate.py`
  (2026-11-02): 84 actors loaded, all standing; 12.7 ms a frame, 6.1 ms with the actors despawned.
  Hiding them saves under 0.5 ms and stopping their animation about 1 ms. The rest is `move_actors`,
  timed at 8.1 ms per fixed tick (about 100 µs an actor, some 6.5 ms a frame at 80 fps): the
  controller's move about half, each of the two walkable-ground shape casts about a quarter (about
  15 µs a cast), the falling ray nothing; no actor or collider stands out. Setting actors'
  `Transform` only when it changes made no measurable difference. With resting actors (PAN3p) the
  same plan measures 7.8 ms, 6.1 ms despawned: of the actors' remaining 1.7 ms, drawing them is
  about 0.9 and animating them 0.4. `docs/perf.csv` has `plans/perf.py`'s before and after; the
  stops at (80, 80) and (112, 112) gained too, from actors within their loaded precincts.
- Measure frame time by counting frames over wall time without polling: the driver's
  diagnostics return the latest single frame, and polling every 5 ms disturbs the app.
- Bevy 0.20 issues seen: (1) switching `DirectionalLight::shadow_maps_enabled` off at runtime
  panics the render thread ("Failed to get directional light visible entities for cascade",
  `bevy_pbr` `render/light.rs`); (2) an entity with two `MeshMaterial3d` components (e.g. an
  extended material alongside `StandardMaterial`) fails wgpu validation in the shadow pass;
  `models` now puts outlines on a child entity instead; (3) `RenderDiagnosticsPlugin` records
  only CPU pass times on Metal.
- Gizmos draw in every camera whose render layers overlap their group's `GizmoConfig`, and the
  default is layer 0, which here only the HUD camera has (realms take layers from 1,
  `ReservedLayers`). A gizmo group for the world needs realm layers, as `PhysicsGizmos` has;
  on layer 0 its lines never reach the 3D view and appear as stray lines through the HUD's 2D
  projection instead.
- Bevy 0.20's SSR (`ScreenSpaceReflections`) works only with deferred rendering
  (`DeferredPrepass`), and reflects onto opaque G-buffer surfaces only, so it would not reach
  the water (`AlphaMode::Blend`, custom forward material) without switching the renderer to
  deferred and reworking the water. Its cost was not measured. From the game's high camera,
  most reflected rays leave the screen (sky), so it could only reflect the far banks.
- Scenery content: playground `p000-p000` holds 1,048 scenery instances, 13 actors and 6 tiers,
  a usable test map for scenery performance (X15); the other playground precincts hold less.
  Overland has one precinct here; the old engine's overland rooms were not migrated.
- Floor regions have identity transforms: their outlines are in precinct coordinates.
- With `dynamic_linking`, run via `cargo run`; the bare binary can't find `libstd`.
