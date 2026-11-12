# Issues

Triage: findings awaiting the author's decision, whether to make each a chunk in
[roadmap.md](roadmap.md), a deferral or a decision, or to drop it. Each says whether it was
confirmed by running something or only by reading. Numbers are never reused.

**Next: I36.**

- **I2** `ExemplarData`'s `Serialize` impl is `todo!()`; serializing an exemplar panics. Read, not
  run.
- **I4** A model and clip viewer, as faery's `catwalk` was (`../faery-catwalk`, Electron and
  three.js): load a glTF, pick a character in a multi-model file, play its clips from a menu.
  Undecided whether a Bevy version is worth it. If so, a candidate home is the editor's catalogs
  mode (X3), previewing an archetype as the game assembles it, colors and features included, rather
  than a separate tool. PAN24's repairs and X45's outside clips make it more pressing. A smaller
  step: a component that plays a named clip on an actor in the game, set through the remote protocol
  (as `TravelTo` is) or a debug key, which shows a clip with the game's outlines, crossfades and
  layers, which catwalk cannot.
- **I5** Landing from a fall feels like stepping onto a shock absorber. The fall itself is right: a
  probe of a 4 m drop showed speed growing at the set gravity until touchdown. The likely cause is
  the camera's chase (D42), which also trails in height and eases out over its last half metre, so
  the view keeps sinking for about a second after the actor lands. Candidates: a faster vertical
  catch-up, or none until PAN3j's landing clip, whose leg flex is part of the effect. Confirmed in
  the author's video of a fall from the playground's wooden stairs (at 2.45 m/s² gravity): the
  actor's fall accelerates steadily, but the camera did about 70% of its descent after touchdown,
  sliding for 1.2 s and creeping for 1.5 s more.
- **I6** Two female gait clips in `humanoid-armature.glb` look broken in catwalk: `WalkFastFemale`
  has stiff arms (perhaps a bad bone export), and `RunFemale` is broken. Female archetypes' `Run`
  plays `RunMale` meanwhile; `WalkFast` still plays `WalkFastFemale`. Seen by the author in catwalk.
  After PAN48's re-export from Blender 5.2, `RunFemale` looks odd but no longer broken, seen by the
  author in catwalk and the game, though its poses compare equal to the 3.5 export's.
- **I9** The cottage's corner roof tiles (`CottageRoofShinglesConvex`, `...Concave`) each collide as
  one straight tile's ramp, shifted 0.5 m along the ridge, as in faery, while their models span 2.5
  m both ways. A valley (concave) is the union of two ramps, so two descriptors fit it; a hip
  (convex) is their intersection, which no descriptor shape makes. PAN3j fixed the same copied ramp
  on the 2- and 3-wide tiles. Seen by the author in the F3 view; the corners read, not run.
- **I12** Physics drawing (F3) costs grow with everything the session has loaded: each frame,
  rapier's `debug_render` turns every collider in the realm into immediate-mode gizmo lines, and
  scenery colliders are drawn for every loaded precinct, shown or not (`debug_views.rs`). In
  `plans/smoke.py` on battery, F3 frames ran 21-28 ms after a visit to (60.5, 41.5), and fell to
  about 3 FPS in runs that had visited the stick village (35.5, 114.5) first. Not a fixed-step
  spiral: a frame catching up 16 physics steps took 23 ms. Candidate fix: retained gizmos
  (`GizmoAsset`, `Gizmo`), built once per parcel or precinct from rapier's lines grouped by collider
  owner, as a child of the owner, so hidden owners hide their lines and F3 and the group checkboxes
  only switch visibility; actors' capsules stay immediate. Unchecked: whether retained gizmos are
  frustum-culled or only follow `Visibility`. Confirmed by running.
- **I15** In the Go To filter box, holding Up or Down steps the list once rather than repeating:
  `chooser` binds them as `Button` actions, and the mapper passes OS key repeats only to class
  bindings. The fix is `bevy_action_map`'s chunk 208, a `.key_repeat()` condition; then bump the
  lock here and add it to the two bindings. Seen by the author; the cause read, not run.
- **I17** Every overrun run rewrites the terrain textures (`assets/terrain/textures/*.ktx2`, 14
  files) and `assets/terrain/models-new/temperate.glb` with different bytes, sizes shifting under
  1%, though their sources and `pipeline.ts` are unchanged; no difference shows. Likely a
  nondeterministic encoder; each run leaves unrelated binary changes to revert. Part of the
  discussion of replacing overrun. Confirmed by running (`git diff --stat` after a run).
- **I18** `humanoid-armature.blend`'s `Face` and `Eyes` materials read `face-texture.png` from the
  old `fta-tiles` repo, which is nowhere on this machine, so `HumanMaleForm` exports without that
  texture. Candidates: find the image in faery's assets under another name, or drop the texture.
  Reported by `blend_inventory.py`; the effect read, not run.
- **I21** The chaparral biome has no ground texture: `GroundMaterialCache::get_material` leaves its
  surface out of the layers, so the ground fades to bare dirt toward a chaparral corner. faery
  painted it as moss is painted, with its own texture, between taiga and sand. Read, not run.
- **I23** The marble palace's walls are smooth-shaded in many places that should be flat, so flat
  surfaces look rounded; a fix for the author in Blender. Possibly from PAN50's move to 5.2, which
  replaced Auto Smooth: comparing a wall's vertex count in the 3.5 and 5.2 exports would tell, and
  if so other scenery may share it. Any batch repair leaves out the stick walls, whose smooth
  shading looks good. Hard edges split the normals `outline.wesl` extrudes along, so check the
  corners for cracks (PAN23). Seen by the author; the cause unconfirmed.
- **I24** Scenery has no hemisphere light: only `ground.wesl` applies it, and other materials get
  `GlobalAmbientLight` at the average color (D16), so a shaded wall's ambient is uniform. faery lit
  everything with it, amber on undersides and lower faces. Candidate: patch `StandardMaterial`'s
  ambient term. The darkening at the bottom of the stick walls, which the author likes, cannot be
  this light; an `SSAO=off` shot at the stick village (35.5, 114.5) would show whether it is SSAO.
  Read, not run.
- **I25** A gap shows between scenery bases and the terrain on dirt areas. Seen by the author; the
  cause not looked into.
- **I26** The character can tunnel through doors by pressing against them. Seen by the author.
  Likely cause: door colliders are not built at all (`ColliderDesc::shape` returns `None` for them),
  read, not run. PAN59 builds them.
- **I28** Bevy's `StandardMaterial` anisotropy lights a surface white where its directions are not
  computed, since the lighting is gated only on `STANDARD_MATERIAL_ANISOTROPY`. In
  `pbr_fragment.wesl` they need the `pbr_anisotropy_texture` feature even for an untextured
  material, and are skipped under `LOAD_PREPASS_NORMALS` (a normal prepass with MSAA off, as SSAO
  has). Worth reporting upstream. Hair rendered white with the feature off and on; that the prepass
  is the remaining cause is read, not run.
- **I29** `pipeline.ts` sets `KHR_materials_unlit` on `black` and `unlit` materials, but the
  extension is not registered on the `NodeIO`, so gltf-transform drops it on writing, with a warning
  the error-only logger hides. Those materials work only through their extras at runtime. Confirmed
  in the same writer path for anisotropy; the unlit case read, not run.
- **I30** `plans/scroll.py` turns on the hitch log by setting `PANOPLY_HITCH_LOG` in `os.environ`
  when it is imported. A directory run imports every plan into one process, so each plan after it in
  the run logs hitches too, and the same plan run alone does not: `smoke.py` logged hitches in the
  directory run and none by itself. Candidates: set it only for `scroll.py`'s own launch, if the
  driver takes a per-plan environment, or turn it on for every plan. Confirmed by running.
- **I32** SAGA's literals take no type from their context: `5` is always `i32` and `1.0` always
  `f32`, so `let x: i64 = 5;` and `let y: f64 = 1.0;` are type errors without a suffix (`5i64`,
  `1.0f64`). The fix: unsized integer and float literal types that unify with any integer or float,
  defaulted to `i32` and `f32` after solving. Confirmed by running (`panoply_saga`'s tests).
- **I33** Formulas in exemplar JSON are not highlighted: the `.saga` grammar (`tools/vscode-saga/`)
  needs an injection into JSON, which must know the keys that hold a formula (only `when` so far).
  Read, not run.
- **I34** Bevy's `Selected` (`bevy_ui::interaction_states`) does not derive `Reflect`, so a plan
  cannot read which list row is highlighted; `plans/smoke.py` checks the saved choice and leaves the
  highlight to a screenshot. Likely an oversight upstream, worth a one-line PR. Confirmed by
  running.
- **I35** Feathers' list view scrolls its rows with `Overflow::scroll_y()` (`listview.rs`), which
  leaves x visible, so a row's text wider than the row draws past the list's right edge. Measured:
  rows 386 px wide, a long name's text 452. The chooser clips each row itself (`TODO(I35)` in
  `chooser.rs`); upstream, the list's scrolling node would clip x as well. Confirmed by running.
