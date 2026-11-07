# Issues

Triage: findings awaiting the author's decision, whether to make each a chunk in
[roadmap.md](roadmap.md), a deferral or a decision, or to drop it. Each says whether it was
confirmed by running something or only by reading. Numbers are never reused.

**Next: I18.**

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
- **I8** Two precincts (`playground/p002-p000`, `holm-overland/p008-p012`) name the exemplar
  `exemplars/regional/marheim#Carlyle.Tepper`, whose file does not exist, probably a faery archetype
  path the converter's `mapArch` had no mapping for. Those actors would load no aspects and have no
  body. Read, not run.
- **I9** The cottage's corner roof tiles (`CottageRoofShinglesConvex`, `...Concave`) each collide as
  one straight tile's ramp, shifted 0.5 m along the ridge, as in faery, while their models span 2.5
  m both ways. A valley (concave) is the union of two ramps, so two descriptors fit it; a hip
  (convex) is their intersection, which no descriptor shape makes. PAN3j fixed the same copied ramp
  on the 2- and 3-wide tiles. Seen by the author in the F3 view; the corners read, not run.
- **I11** `assets/terrain/shaders/ground.wgsl` looks unused: `GroundMaterial` loads the embedded
  `crates/panoply_terrain/src/shaders/ground.wesl`, and editing the `.wgsl` changed nothing in a
  run, which cost a performance test. Candidate: delete it, once `assets/terrain/shaders/` has been
  checked for other leftovers. Confirmed by running.
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
- **I13** Walls in shadow look flat: on the Marheim palace (nav point `town.marheim`), the sides
  facing away from the sun show none of their modeled relief (pilasters, panels, moldings), one tone
  across the face. Likely cause: in shadow only the hemisphere ambient lights them, and it varies
  only with how far a face points up, so every vertical face turned the same way gets the same
  light. Candidates: SSAO, a weak unshadowed fill light opposite the sun, or ambient that depends on
  more of the normal. Seen in the author's screenshot; the cause read, not run.
- **I15** In the Go To filter box, holding Up or Down steps the list once rather than repeating:
  `nav_point_list` binds them as `Button` actions, and the mapper passes OS key repeats only to
  class bindings. The fix is `bevy_action_map`'s chunk 208, a `.key_repeat()` condition; then bump
  the lock here and add it to the two bindings. Seen by the author; the cause read, not run.
- **I17** Every overrun run rewrites the terrain textures (`assets/terrain/textures/*.ktx2`, 14
  files) and `assets/terrain/models-new/temperate.glb` with different bytes, sizes shifting under
  1%, though their sources and `pipeline.ts` are unchanged; no difference shows. Likely a
  nondeterministic encoder; each run leaves unrelated binary changes to revert. Part of the
  discussion of replacing overrun. Confirmed by running (`git diff --stat` after a run).
