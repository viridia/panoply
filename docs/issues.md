# Issues

Triage: findings awaiting the author's decision, whether to make each a chunk in
[roadmap.md](roadmap.md), a deferral or a decision, or to drop it. Each says whether it was
confirmed by running something or only by reading. Numbers are never reused.

**Next: I74.**

- **I4** A model and clip viewer, as faery's `catwalk` was (`../faery-catwalk`, Electron and
  three.js): load a glTF, pick a character in a multi-model file, play its clips from a menu.
  Undecided whether a Bevy version is worth it. If so, a candidate home is the editor's catalogs
  mode, previewing an archetype as the game assembles it, colors and features included, rather than
  a separate tool. PAN24's repairs and X45's outside clips make it more pressing. A smaller step: a
  component that plays a named clip on an actor in the game, set through the remote protocol (as
  `TravelTo` is) or a debug key, which shows a clip with the game's outlines, crossfades and layers,
  which catwalk cannot.
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
  highlight to a screenshot. Likely an oversight upstream, worth a one-line PR
  ([bevy#26499](https://github.com/bevyengine/bevy/issues/26499)). Confirmed by running.
- **I35** Feathers' list view scrolls its rows with `Overflow::scroll_y()` (`listview.rs`), which
  leaves x visible, so a row's text wider than the row draws past the list's right edge. Measured:
  rows 386 px wide, a long name's text 452. The chooser clips each row itself (`TODO(I35)` in
  `chooser.rs`); upstream, the list's scrolling node would clip x as well. Confirmed by running.
- **I36** Feathers' number input has no Up and Down keys: faery's nudged a focused number by its
  step, a hundred steps with Cmd (`obsidian/src/components/NumberInput.tsx`), which aligning
  colliders leaned on. Feathers has drag and chevrons, snapped to the step since PAN28c. Candidates:
  an upstream PR ([bevy#26535](https://github.com/bevyengine/bevy/issues/26535)), or an observer in
  Panoply until then. Read, not run.
- **I37** Feathers' number input draws its label (the X, Y and Z beside an axis's sigil) in the same
  gray as its input area in the property editor, where the label should be a different gray: the
  surface level (`ThemeContext`) adjusts the input's background but not the label's
  (`TEXT_INPUT_LABEL_BG`). Likely a feathers bug, worth an upstream fix
  ([bevy#26534](https://github.com/bevyengine/bevy/issues/26534)). Seen by the author. The cause,
  read in `dark_theme.rs`: at `SurfaceLevel::Higher` the field fill is raised to `GRAY_3`, which is
  also the label's solid fill. Panoply gives the label a fill of its own, a step lighter there
  (`raise_input_labels`, `TODO(I37)`).
- **I38** Feathers' number input drags a step's worth only every 100 logical pixels: with a
  `NumberInputStep`, the drag speed is the step times `BASE_DRAG_SPEED` (0.01) per pixel
  (`scrubber_on_drag_start`, `number_input.rs`), and an integer drags as step 1 at the same rate.
  With `NumberInputSnapToStep` the value stays put for about an inch, then jumps one step, so
  `LightSource.offset` at `@Step(0.05)` is hard to nudge. faery dragged one step per pixel. Shift
  already slows a drag tenfold. Candidates upstream
  ([bevy#26533](https://github.com/bevyengine/bevy/issues/26533)): a factor near 0.1 to 0.25 for a
  stepped input, or a drag speed the input can be given. Panoply cannot set it meanwhile
  (`DragState` is set as the drag starts). Seen by the author; the rate read in the code.
- **I39** Feathers' number input chevrons stick on some values of an `f32` input snapped to its
  step: `step_snapped_f64` (`number_input.rs`) allows 1e-9 of a step for rounding, and an `f32`
  widened to `f64` is off by about 1e-7 relative. 0.35f32 is 0.3499999940, 6.9999999 steps of 0.05,
  so the next multiple up is 7 steps, 0.35 again; 0.3f32 going down stays at 0.3 the same way. The
  fix upstream: a tolerance for the value's format (near 1e-6 relative for `F32`), with a test
  stepping `0.35f32`. Meanwhile the property editor gives the input an `f32` field as the `f64` of
  its shortest decimal (`shown_f32`, `TODO(I39)`). Seen by the author on collider dimensions; the
  arithmetic reproduced in Python. ([bevy#26536](https://github.com/bevyengine/bevy/issues/26536)).
- **I41** Feathers' tree view (`FeathersTreeView`, `tree_view.rs`) puts its rows under an inner
  scrolling node, but the headless tree (`bevy_ui_widgets`' `tree.rs`) looks for rows only among the
  `TreeView` entity's own children, both to derive each row's indent level and to list the visible
  rows that Up and Down step through. So nested rows would not indent, and the arrow keys would do
  nothing. The model picker puts `TreeView` on its scrolling node itself (`model_picker.rs`), and
  there rows indent. Likely worth an upstream fix. Read, not run.
- **I42** Removing an aspect in the editor does not check the exemplar's formulas again:
  `missing_aspects` runs only as an exemplar's chain loads (`script_components.rs`), on the
  assumption that an exemplar extending another never loses an aspect. A formula reading a removed
  aspect's component, in the exemplar or one extending it, errors only after the catalog reloads.
  Read, not run.
- **I44** The property panel scrolls well past its last row on `MossyBrickWallTee`, leaving a large
  gap after the aspects. Feathers hides a closed select's or menu's popup with `Visibility::Hidden`
  (`select.rs`, `menu.rs`), so it stays in layout, and taffy adds every absolutely positioned box to
  its containing block's scrollable overflow (`perform_oof_layout`, `compute/oof.rs`), as CSS does.
  In Bevy every node is a containing block, so a popup hanging below the last select (each
  collider's `Type`, seven options) stretches the scroll area, open or closed. `Display::None` or
  `FeathersMenuLazy` fixes only the closed case. The real fix is upstream: popups as portals, root
  nodes that `Popover` places by an anchor entity rather than by their parent, as a browser's top
  layer does. Bevy's `FixedNode` is that: a node laid out as a root of its own, keeping its parent.
  The editor makes it a required component of `Popover` (`FixedPopoversPlugin`, `TODO(I44)`);
  upstream, `Popover` would require it
  ([bevy#26541](https://github.com/bevyengine/bevy/issues/26541)). Seen by the author; the mechanism
  read in taffy.
- **I45** A map's + adds its entry last, which on a long pane is below the panel's scroll, and
  nothing scrolls to it or gives its key input the keys; while an entry with an empty key is there,
  + does nothing. Candidate: focus the empty key's input on +, scrolling it into view, which covers
    both. Seen in `plans/smoke.py`'s `map_property` screenshot.
- **I47** The toolbar's rotate left and right tools mean something only in terrain and scenery
  modes, yet take a toolbar row in every mode. Candidate: move each into its mode's panel, freeing
  the row. Seen by the author.
- **I48** Feathers' color input never shows its color: the swatch stays white while the picker edits
  the value (map entries in `Colors`; a `LightSource` color too, unnoticed since it is near white).
  Its scene names the swatch `#swatch` inside the nested `@caption: bsn!` it passes to
  `FeathersMenuToolButton`, and refers to it from the outer macro as `ButtonEntityRefs(#swatch)`;
  since bevy#26314 a nested macro keeps its own call site, so the two are separate entities and the
  color goes to one never drawn. It had worked because both names were index 0 of their macro, the
  collision bevy#25407 reported. Whether a name should reach into a nested `bsn!` is awaiting cart's
  ruling: if it should, the fix is in BSN, keying names across nested macros; if not, in feathers,
  finding the swatch without one. The editor copies the color to the drawn swatch meanwhile
  (`ColorSwatchesPlugin`, `TODO(I48)`). The color landing elsewhere confirmed by running (a probe
  read each entity's `ColorSwatchValue`); the cause read in the commit.
- **I49** A `Skin`'s model picker still lists the features (hair, hats) beside the bodies (D77).
  Since PAN28k a feature's node carries the extra `feature`, so the picker could leave those scenes
  out, which is D77's reversal without the scene extras it planned. Read, not run.
- **I51** `plans/smoke.py` once failed at the characters catalog's save (`expect_disabled` after
  `click("Save")`): Save stayed enabled, and no save ran or failed in the log, so the click reached
  nothing. The plan alone passed right after. The previous step was the redo of an `ActorScale`
  edit, which a field's keys and the history settle over a few frames. Candidate: wait on something
  the redo changes before clicking. Seen once, in a directory run (PAN4a); the cause not looked
  into.
- **I52** faery chose scenery tools by key, `w` for walls and `f` for floors
  (`SceneryEditPanel.tsx`, `createShortcuts`), which Panoply leaves out: W is the editor camera's
  forward (`ViewpointControl` in `src/input.rs`), so tool keys need other letters, or a modifier.
  Read, not run.
- **I53** Bevy draws nothing for a mesh whose `Mesh3d` was removed and inserted again:
  `late_sweep_material_instances` drops the entity's material instance when its `Mesh3d` goes, and
  `extract_mesh_materials` takes it back only on `Changed<ViewVisibility>` or
  `Changed<MeshMaterial3d<M>>`, neither of which a returning `Mesh3d` sets (`bevy_pbr`'s
  `material.rs`). Worth reporting upstream. `panoply_overlays` hid an empty shape by removing its
  mesh, so the fixture tool's dial showed on the first drag only; it now hides the entity instead.
  Confirmed by running (PAN96): a probe read the dial's bounds and visibility, and screenshots
  showed it missing, then drawn after the change.
- **I54** `plans/smoke.py` once failed at `collider_layout` on code unchanged since PAN97: after a
  collider item's toggle was clicked, its `offset` row was still there (`absent` timed out). The
  next run passed the step. Seen once (PAN74a's baseline run); the cause not looked into.
- **I55** Two floors differ from faery's `floor-tiles.json`: `RoughStone` uses `rough-stone.png`
  with no raise where faery used `rough-stone-warm.png` raised 0.02 (the texture `SlideyStone`
  uses), and `IceCavefloor` has `sides: false` where faery drew its sides. Which is meant is the
  author's call; the catalog keeps Panoply's. Read, not run.
- **I56** Raising a floor type in the editor (PAN74a's live edit of `raise`) rebuilds its floors'
  colliders in place, pushing the slab up through actors standing on it, and they stay embedded:
  Rapier's controller depenetrates only when asked for no movement ([notes.md](notes.md), the
  editor's fall to 3 fps). Lowering it again drops them. Candidate: when `insert_floor_meshes`
  replaces a floor's collider, re-ground the actors over it, as R27's put-back does for returning
  ground. Seen by the author; the mechanism read, not run.
- **I57** `plans/smoke.py` failed in PAN74a's directory run at `fixture_tool`: a drag to the right
  read 160° where 270° is expected (`FacingReadout`). It had passed alone with the same code. Two
  candidates, neither checked: actors embedded by the floor test's raise (I56) slowing frames past
  the drag's three-frame wait, or the hitch log scroll.py leaves on for later plans (I30).
- **I58** `plans/smoke.py` (1,690 lines, one launch) chains every editor feature, each section
  relying on the state earlier ones leave, so reaching a new feature runs all before it, and each
  chunk lengthens every run. Proposed: split by area (game, scenery tools, property editor, terrain,
  realm), each plan setting up its own state, so one plan, or one section named in the environment,
  runs alone; launches take about 3 s. And narrow a chunk's end to the game plans, `perf.py` and the
  plans of the areas it touched, all of them only when shared editor code changes: the game's
  integrity matters more than the editor's. The second changes AGENTS.md's Verify. Discussed with
  the author, not yet decided.
- **I59** Whether inventory items are exemplars or catalog entries, to be discussed. An item is
  always in a container, a pile if nothing else, which is the entity in the world; its icon, its
  wielded model and its pile's look show it, and none is the item. Picking up a coin merges it into
  the stack held: a slot is the item's handle, a count and any state of its own, stacking only
  without state. For exemplars: aspects shared with scenery (a book read as a sign is, the reading
  interface needing only the aspect's data) and `extends` for families. Leaning: an exemplar spawned
  once per item type as an invisible, stateless entity kept in memory, its aspects resolved there as
  on any instance; a slot refers to it and is copied by value, and an aspect's action runs on that
  entity as scenery's do. D90's test would then read "composes aspects, or shares them with things
  that do". Discussed with the author.
- **I60** A terrain effect's `Effect` shows in the property editor as its bits, read-only (`2` for
  soil): `TerrainTypes` is a bitflags newtype over `u8`. Candidate: a checkbox per flag, as a map
  with keys from its type has a row per key. Seen in `plans/smoke.py`'s `terrain_effect_entry`
  screenshot (PAN74b); the aspect showed it the same.
- **I61** `CliffHintFullWide` in `wall-tiles.exem.json` is a wall with nothing but a `hint`
  collider, so it cannot be seen or picked. The author takes it for a removed feature, perhaps of
  terrain flattening. Candidate: delete it, in the converter, or the next conversion brings it back.
  Read, not run.
- **I62** Widgets that show a value (list rows, a select's chosen value, an id in a pane) are set in
  the same Fira Sans Regular as buttons, which act. The author would set values a weight lighter, as
  widget collections usually set actions heavier than values. Feathers bundles only Regular and Bold
  (`bevy_feathers/src/assets/fonts`), and its buttons and labels are Regular. Candidate: ship Firava
  ([hellogreg/firava](https://github.com/hellogreg/firava), OFL 1.1, not on Google Fonts), Fira Sans
  4.2 made variable, weights 100 to 900 on one axis, regular and italic, as `Firava.ttf` and
  `FiravaItalic.ttf`. Set `FontWeight::LIGHT` on the value text Panoply builds (`chooser.rs`'s rows,
  the summaries and values in `properties.rs`, `instance_list.rs`'s id), or move all of Panoply's
  text to it. One font then serves every weight, and Panoply proves the move feathers has discussed
  but not made, which would bring its own value widgets (select, text and number inputs) along. Its
  family is presumably Firava rather than Fira Sans, so Panoply's text names the families in order,
  Firava first, Fira Sans for what it lacks, then Key Symbols (Bevy's text takes a list of families
  since the last cycle). It holds about 450 characters to Fira Sans's 1,700, which matters for
  feathers more than for an English editor. Unchecked: the family name in its name table, and how
  Bevy's text picks a weight from a variable font. Seen by the author; the font read on its repo.
- **I63** A click inside a modal dialog that its target does not consume escapes the dialog: the
  text input (`bevy_ui_widgets`' `text_input`) handles no `PointerClick`, and the modal stops only
  presses at its body (`modal_dialog_barrier_on_click` observes `PointerPress`), so a click in the
  rename dialog's box bubbles through the dialog and the UI root to the window, past picking hygiene
  (D94). Selection mode once took it for a click on the ground and cleared the selection. The
  modal's observer is named for clicks but observes presses (`modal.rs`), and a press and the click
  picking sends on its release bubble each on their own, so stopping one leaves the other. Upstream
  candidates: the barrier and the dialog body consuming `PointerClick` too, as the name intends, the
  cleanest; and the text input consuming the clicks it uses for focus and the caret. Confirmed by
  running (PAN88's probe: original target `TextEntryDialog/…/Entry`, arriving at the window).
- **I64** The actor tool places an actor among generated trees on the ground, and its physics then
  lifts it onto the canopy 2 m up (D39): the placement cast passes through the canopy (`cast_filter`
  in `placement.rs`), while the actor's capsule overlaps it and is pushed out upward. So the actor
  stands where the author did not put it. Candidates: land the placement on the canopy, as physics
  will, or refuse forest cells for actors. Confirmed by running (PAN89's probe: Marcus placed at the
  playground's tree line stood at y 2.01, over ground near 0).
- **I65** The driver's plans fail when the author uses the machine during a run: the real pointer
  moving over the game's window sends its own `CursorMoved` beside the plan's, and a blanked or
  covered screen makes a screenshot come back empty. On battery the author has to move the mouse now
  and then to keep the screen from blanking. Both happened in PAN16b's run of `smoke.py`: an empty
  `cut.png`, then 2,009 scenery elements where 2,008 were expected, after the pointer moved during a
  placement. Candidates, in `bevy_remote_driver`: drop the OS's pointer events while a plan drives,
  keeping only those it injects; and hold the display awake for the run (as `caffeinate -d` does on
  macOS), or read the frame from the render target rather than the screen. Seen by the author; the
  causes read, not run. Again in PAN4b's directory run, the author's mouse moving: a click on a wall
  row after `text_property`'s saves left the choice on `SpearTrap`; `smoke.py` alone, untouched,
  passed.
- **I67** Navigation takes a footpath from a terrain effect's look: a cell is a path when its effect
  is cobbled (`footpaths` in `panoply_nav`'s `input.rs`), as faery took its road and stone. The
  author expects two dirt effects that look the same but navigate differently, a trail through a
  forest (a path) and a patch with no grass (not one), which the look cannot tell apart. Candidate:
  a field on the terrain effect entry (`Surface`) saying whether it is a footpath, set on the roads
  and trails, which the capture reads through the precinct's effect table rather than the vertex
  attributes. Discussed with the author.
- **I68** Roofs have navigation polygons: shingles under about 45° pass the walkable slope
  (`WALKABLE_NORMAL_Y`), though no route reaches them. A click on a roof makes it the goal, and the
  route ends at the reachable point nearest it, which may be the floor under it indoors rather than
  the ground outside, as a click on a wall did before D109. Candidates: leave roofs out of the mesh
  (an element's collider kind, or polygons no route can reach), or treat a click on an unreachable
  surface as a wall click. Seen by the author in the navigation view; the click read, not run.
- **I69** Feathers' checkbox squeezes its box when its caption is wider than the space it is given:
  the outline node has a fixed width and height but the default `flex_shrink` (`checkbox.rs`), so in
  the actor features' two-column grid `FacialHairMuttonChops` narrows its box to a sliver. The fix
  upstream: `flex_shrink: 0` on the outline
  ([bevy#26630](https://github.com/bevyengine/bevy/issues/26630)). Panoply cannot set it meanwhile
  without matching the node by position, since its marker (`CheckboxOutline`) is private. The
  caption then overflows its column rather than squeezing the box; how the grid should take long
  keys is a separate question. Seen by the author; the cause read in `checkbox.rs`.
- **I70** Whether an actor can climb onto furniture at sitting height (benches, hay bales, about
  0.45 m), above R105's 0.25 m step. The author leans to yes, with resistance: slower than walking,
  perhaps a clip of its own, rather than the stride that takes a curb. A new feature, not part of
  stepping. Discussed with the author.
- **I71** Every launch warns that `editor.history.undo` and `.redo` read `KeyZ` twice: `undo.rs`
  binds each once with Ctrl and once with Super, as it should. A false positive in
  `bevy_action_map`: its duplicate check (`plan.rs`, `DuplicateBinding`) compares a binding's
  `input` and not its `chord`, where `.with()` puts the modifiers, so two chords of one key count as
  one binding. The fix is in the mapper, comparing the chord too; then bump the lock here. Panoply
  has nothing to change. Seen in every run's log; the cause read in `plan.rs` and `builder.rs`.
- **I72** Five portals have no target, in faery's precincts as in Panoply's: in the playground,
  `sp1`, `mbep1` and `mbep2` (`p000-p000`) and `kwbp0` (`p002-p000`) name no realm; in
  `holm-underground/p016-p013`, a portal with no instance id targets `holm-overland` at `[0, 0, 0]`.
  Candidates: set them by hand once PAN4d's portal chooser lands, or delete the test portals no one
  needs. Read in the precinct files by `repair_portal_targets.mts` (faery's `scripts/`), which
  restored the targets the converter had zeroed.
- **I73** `plans/ablate.py`'s `camera inactive` panics the app when it switches the primary camera
  back on: `prepare_lights` unwraps the camera's shadow cascades (`light.rs:1918`), which
  `build_directional_light_cascades` builds only for cameras already active, and a BRP write lands
  after it in the frame. Any camera's `is_active` set from a plan does the same; the game, setting
  none, is safe. Candidates: measure without rendering some other way, or report it upstream, the
  cascades lookup skipping a view it has none for. Confirmed by running (PAN4b's portal ablation).
