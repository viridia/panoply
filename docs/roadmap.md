# Roadmap

Work not done, as chunks. What has landed is in [changelog.md](changelog.md), why things are as
they are in [decisions.md](decisions.md), work put off until an event in
[deferred.md](deferred.md), and findings not yet routed in [issues.md](issues.md).

A chunk's id, `PAN<n>`, is permanent: taken from the counter below and never reused, and named in
its commit subjects. Code waiting for a chunk is marked `TODO(PAN<n>)`, or `TODO(X<n>)` for a
deferral. Uncompiled code kept as reference until a chunk replaces it is marked
`REFERENCE(PAN<n>)`. When a chunk lands, its entry here is deleted and a line added to the
changelog.

**Next: PAN65.**

## Next

In order.

PAN3, Actors, in lettered parts. Requirements, from faery's actor module, are in
[actor-requirements.md](actor-requirements.md) (draft, for the author to edit). Layers by time scale
(existence, goals, planning, body, presentation) talk through components, with readiness as
components and every layer restartable from saved state (R104).

- **PAN57 Wait while saving.** Save writes the file, and the file watcher's reload of it arrives 0.3
  s or more later, replacing the asset with the file as saved: an edit made in between is silently
  lost, its undo entry and unsaved mark left behind. Save shows a wait cursor (`OverrideCursor` set
  to `SystemCursorIcon::Wait`) and an invisible modal barrier that intercepts clicks and keys, until
  each written asset has reported `AssetEvent::Modified` or `AssetLoadFailedEvent`; with edits
  blocked, a `Modified` can only be the reload. An asset whose write failed is not waited for. With
  watching off there is no reload to wait for. A timeout, logged, is the backstop. Before PAN38,
  where the next brush stroke would be lost. `plans/smoke.py` waits for the barrier to go after
  Save.
- **PAN58 Toast.** A brief message in the editor that fades by itself and takes no input; Bevy's
  widgets have none. First customers: a failed save (PAN57), which only the log reports today, and
  the label of what Ctrl+Z just undid or redid (`UndoAction::label`, PAN42), which tells the author
  what changed when it is out of sight (X72).
- **PAN44 Quit confirmation.** Escape (`Quit`, `src/input.rs`) and the window's close button exit at
  once, losing edits that Save has not written. While `UnsavedAssets` is not empty, both ask first,
  in a modal dialog like PAN29's text entry: Save, Discard or Cancel. The close button needs
  `WindowPlugin::close_when_requested` off and `WindowCloseRequested` handled.
- **PAN56 Saved editor view.** The editor's realm, position and azimuth saved between runs, as faery
  did, so an iteration need not navigate back: a settings group beside `ZoomSettings`, written with
  its debounce and only when the view changes. The saved realm comes before `nav_to_center`'s
  default, which remains the fallback when that realm is gone.
- **PAN35 Create and Change Realm.** Two buttons in the realm panel, each opening a modal dialog:
  Create Realm asks for a name in PAN29's text-entry dialog and makes a new, empty realm; Change
  Realm picks one of the realms in a list-selection dialog and moves the view there. The dialog
  holds a generic chooser, generalized from PAN26's `nav_point_list.rs`: a scrolling list sorted by
  name and a filter box using `search_filter`. PAN41's exemplar choosers reuse it.
- **PAN36 Mode overlays.** Each editor mode draws its own overlays while active. Realm mode's first:
  a grid on the ground at parcel and precinct boundaries, and the realm's border (faery's
  `FlatGridOverlay` and `ParcelBoundsOverlay`, mounted by `MapEditPanel.tsx`); Panoply's Quill
  editor drew it already (`src/editor/ui/overlays/map_bounds.rs`). Terrain mode's cursor and
  selected parcel come with PAN38; the scenery mode's (faery's grid, floor cutaway, colliders, marks
  and portals) with PAN5.
- **PAN37 Map size control.** In the realm panel, eight buttons that grow or shrink the realm by a
  parcel on each of its four sides, two per side (faery's `MapSizeControl.tsx`; the pattern recurs
  elsewhere in faery). The control rotates with the camera's azimuth, so its sides match the map's
  on screen; faery also squashed it vertically to match the view's tilt. A press changes the terrain
  map's bounds (`realm.parcel_bounds`, `terrain_map.rs`) and marks it unsaved for PAN29's save
  button. New ground: rendering and picking of transformed UI nodes (`UiTransform`) follow the
  transform in principle but are little exercised, so expect Bevy bugs, each found in the locked
  checkout and reported or fixed upstream. Panoply's Quill editor never had one.
- **PAN38 Terrain sculpting.** Terrain mode's panel and its tool palette: raise, lower and flatten,
  each by brush or by rectangle; draw trees, shrubs and herbs, and erase flora; rotate the selected
  parcel's contour. Ported from the Quill editor (`REFERENCE(PAN5)`): `ui/mode_terrain.rs`,
  `ui/tool_terrain_edit.rs`, `ui/overlays/terrain_cursor.rs` and `selected_parcel.rs`,
  `terrain/terrain_map_undo.rs`. Edits mark the terrain map unsaved for PAN29's save button and go
  on PAN42's undo stack.
- **PAN39 Contour chooser.** The terrain contours as a list of 3D thumbnails, choosing the selected
  parcel's contour, with controls to group and order the list (faery's + − ▲ ▼, `terrain.png`). From
  the Quill editor's `ui/controls/contour_chooser.rs`, `renderers/terrain_thumbnail.rs` and
  `terrain/terrain_contour_undo.rs`. Its thumbnails are offscreen render targets of preview parcels
  in a hidden realm; rendering many at once stalled the engine, so one camera was retargeted to one
  thumbnail a frame, and an edited contour's thumbnail updated a few frames later. Whether to keep
  that or render only the rows in view is decided in the chunk.
- **PAN40 Water and biome.** In terrain mode, buttons that toggle whether the selected plot is water
  or terrain, and a selector for the current parcel's biome (faery's `PlotEditPanel.tsx`,
  `terrain.png`).
- **PAN41 Exemplar chooser.** Catalogs mode's first panel: the exemplar kinds (actor, wall, floor,
  fixture), then the chosen kind's exemplars in PAN35's chooser (the Quill editor's
  `ui/controls/exemplar_chooser.rs`). The same chooser serves later in scenery mode, for the
  exemplar to place and an existing instance's exemplar (PAN5). PAN28 edits the chosen one.
- **PAN28 Property editor.** For the exemplar chosen in PAN41 and the realm panel's selected nav
  point (faery's `NavPointEditor`). A fork of arbor_day's `properties.rs` into `panoply_editor`, so
  each develops for its own needs (sharing later is X48); its attributes replaced by
  `bevy::annotations` where they overlap, as arbor_day's X25 plans for its own. faery's editor is in
  [faery.md](faery.md), Editor, its screenshots included. Beyond arbor_day: nested structs and
  arrays of structs (faery's weakest part, so its layout is a starting point only), and optional
  fields as faery's aspects had them: a struct is a map whose keys may be absent; a `None` field has
  no row, a `Some` row has an × button that removes it, and a boxed-plus menu in the panel header
  lists the absent fields to add. A field's annotations apply through `Option` and `Vec` to their
  contents, since the `f32` in `Option<f32>` cannot be annotated; Bevy's inspector does this, in a
  commit after Panoply's locked Bevy, so this chunk may bump Bevy. Every edit is undoable in PAN42's
  stack (D49): setting a value at an asset key and property path, adding or removing an optional
  field, inserting or removing an array item; a drag's steps coalesce into one entry, with
  `coalesce` added to `UndoAction` from bevy#23373, which PAN42 left out. Nesting must not squeeze
  the values: in faery, labels and values in separate columns, under nesting, cramped the labels and
  cut values off.
- **PAN43 Inherited fields.** Provisional. Fields an exemplar inherits through `extends` show apart
  from its own, as faery's did in dim italics (`scenery-actor-inherited.png`); editing one overrides
  it, and × on an override reverts it to the inherited value rather than removing the field. faery
  had this from JavaScript prototypes; in Panoply `extends` is a handle to the parent exemplar
  (D27), so how the editor tells own fields from inherited ones is decided in the chunk. X1's move
  to BSN would make `extends` scene inheritance.
- **PAN34 Incident log.** Unusual events, recorded for later analysis: what happened, to which
  actor, where and when. First PAN3m's put-backs, which log nothing today, and PAN3q's rescues; a
  rescue's record says what trapped the actor, so traps can be found and fixed in the content. Where
  the log is kept, and whether a plan reads it, decided when it is written; what else counts as
  unusual is added as it turns up.
- **PAN3q Stuck rescue**, the backstop for traps the canopy (D39) does not cover (trees against
  walls, canopy bowls too steep to climb, clearings enclosed beyond its margin): an actor that is
  not grounded, or makes no progress against its intent, for some seconds moves to the nearest free
  spot; before navigation, the nearest cell outside the canopy cover. Each rescue goes in PAN34's
  log. Tune the canopy and trunk height (`CANOPY_HEIGHT`, D39).
- **PAN59 Door.** A `Door` aspect on the cottage door (`CottageWallDoorway`), played by its
  `OpenDoor` clip through `SceneryAnimations` and offered through `Interactions`. Its collider is
  built and follows the animated glTF node its descriptor's `animation` names, as faery's
  `PhysicsMovable` did (X39's door part), so the leaf is solid and within the probe's reach through
  the whole swing. Closes I26.
- **PAN45 First formula.** SAGA's first customer: the chest's `when`, as formulas in
  `Proto.Container` ([saga.md](saga.md), Early customers), which splits its one toggling entry into
  Open and Close. Extracts `panoply_saga` from `../bevy_reactor_3/crates/bevy_reactor_formulae`,
  tests passing, with `.saga` files and the syntax grammar; settles how an asset embeds a formula,
  which dialogue conditions and goals (X17) reuse; and reads `ContainerState`'s `open` through a
  compiled component accessor, type-checking `self` against the exemplar's resolved aspects. On
  demand only: no cells or reactions. The action stays native. Checked in `plans/actors.py`, which
  tests the chest.
- **PAN30 Aspect hot reload.** An edited exemplar's aspects reach the actors already spawned from
  it; today only a changed precinct respawns its actors (`precinct_actors.rs`). Checked in
  `plans/actors.py`: edit an exemplar file, wait for the change on its actor, and restore the file.
- **PAN3n Hair shading.** Materials with extras `material: hair` get strand tangents and a hair
  shader (D29).
- **PAN23 Outlines from node extras.** Panoply outlines a mesh only when its material's extras set
  `outline`; faery read the node's extras first, then the parent node's, then the material's, and
  outlined every skinned mesh by default (`SkinnedModel.ts`, `ModelCatalog.ts`). Male hair, much
  clothing and every prop set it on the node, so they have no outline. Read `GltfExtras` on the
  mesh's entity and its parent before `GltfMaterialExtras`; decide whether skinned meshes keep
  faery's default. `scripts/outlines.py` lists what changes. The playground's creature group (124,
  22; `creatures.png` in `plans/actors.py`) shows the kreekin, outlined only by node. Weapons also
  carry a hand-built `Outline` hull, which a node outline would extrude again. To see outlines while
  editing, a Blender Python script, run once per `.blend`: after the Armature modifier on every
  mesh, an Edge Split on sharp edges, then a Solidify (flipped normals, Even Thickness off, an
  outline material with backface culling), its thickness driven by the object's `outline` property.
  It shows two things: meshes with no outline, and sharp convex corners that need a bevel. The Edge
  Split is for the second: `outline.wesl` extrudes along the exported normals, which split at sharp
  edges and open cracks at hard corners, while a Solidify alone offsets along unsplit vertex normals
  and would hide them. Check the preview against the game on one model with a hard corner, including
  flat-shaded faces, which the Edge Split may not split. Exports must leave the hull out, with Apply
  Modifiers on (D55), by a means this chunk settles.
- **PAN24 Clip repairs.** With PAN23, as one Blender session. `Punch` keys `upper_arm.R` rolled
  about 140° from its roll in `Stand` (and 70-90° from the mirrored left arm), wringing the
  shoulder; the keys hold no quaternion sign flips, so the poses themselves are wrong. A Blender
  Python script rolls the bone back about its own axis on every key, by an angle judged in catwalk,
  and counter-rotates `forearm.R`'s keys so the forearm and hand stay where they are. A clip audit
  in `scripts/`, as `outlines.py`, lists clips whose bones' roll strays far from their mirror or
  from `Stand`. The broken female clips are I6. A trial for X45: retarget one CC0 combat clip onto
  `HumanoidArmature` and play it beside the hand-keyed ones, to measure the cost per clip and
  whether the styles mix.
- **PAN46 Blender procedures.** In the same Blender session as PAN24, so that new clips and
  characters are looked up rather than relearned. A section in `artwork/characters/Blender.md`, for
  Blender 5.2, click by click with the traps at their steps: a new character on the shared armature
  (parent with automatic weights, fix the weights, play the existing clips). The author follows the
  draft once in Blender and corrects it.
- **PAN47 Clip panel.** A Panoply add-on for Blender 5.2, replacing the NLA editor for clip work, so
  clips are never renamed or edited through it. A panel lists the clips; selecting one makes it the
  action being edited (no tweak mode to forget), with play. New and Delete change a clip's NLA track
  and action together, a new clip's action named as its track; Rename renames the track only, since
  other files link actions by name (D55). Export uses the file's remembered settings (D55), and
  `blend_inventory.py`'s report shows after each. Keying stays in the Dope Sheet and Graph Editor;
  the NLA editor's blending and timing go unused, since the animation graph blends clips in the
  game. `Blender.md` gains a section on installing the add-on and using it, and the author tries it
  in the same session as PAN46.
- **PAN20 Outline width by distance.** An experiment: flatten the curve by which outlines thin with
  distance, without making them constant. `outline.wesl` extrudes in mesh-local units, so on screen
  the width falls as 1 / depth. Scale the extrusion by `(depth / reference)^k`: `k = 0` is today,
  `k = 1` a constant screen width; try values between. Depth to the mesh's origin rather than each
  vertex, so a long model's outline does not taper along it. `k` and the reference as one setting
  for every outline material, tuned by eye in the editor at several zooms; in play the camera
  distance is fixed (11), so there it only evens out the near and far rows of the view.
- **PAN19 New game.** Resets every precinct, and the actors in it, to its authored state: despawn
  them and spawn them again from their assets and exemplars. Saves are sparse, holding only what
  play changed (X18), so loading a save is a new game with the save's changes applied. After PAN3l,
  which puts actors in precincts. R12 is the same reset for one actor.
- **PAN16 Navigation.** Panoply has none; faery's is in `engine/src/nav` (see [faery.md](faery.md)).
  First evaluate [rerecast](https://crates.io/crates/rerecast): navigation mesh generation is hard
  to maintain alone, which outweighs the cost of a fork (D36); check whether its generator works
  without Bevy, which would need none. Test it against Panoply's idiosyncratic needs: a mesh only
  around loaded precincts, built as they stream in (R83); door regions (R98); exclusion zones cut
  where actors stand (R86); and whatever else faery's `nav` does that rerecast lacks. A missing
  feature may be upstreamed rather than forked: the author has talked with rerecast's author. Crowd
  and follower behavior (R86, R87) needs test scenarios: driver plans that stage, for example, eight
  skeletons rushing the player, or two followers through a doorway.
- **PAN17 Click to move.** Clicking an actor or fixture needs clicks in the world
  (`bevy_remote_driver`'s X9). Clicking ground to walk to is Panoply's own: a server method of its
  own (the driver's X2), or a waypoint entity spawned there and clicked through the driver's X9. The
  game's click handling runs only while the game has the pointer (X3, editor tools). A click that
  misses every interactive thing lands on `WorldBackstop` (D61), and a ground click needs the ground
  point, which it does not report.
- **PAN4 Portals**: a rewrite, not a restoration (D21). `src/portals` is reference
  (`REFERENCE(PAN4)`), deleted once portals work. Portals go in a crate, since the editor places
  them (D37, D40); which one is open. The old code's portal aspect attaches to walls and fixtures
  (`panoply_scenery`'s instance types), its views need `panoply_core`'s realms and cameras, and
  crossing moves the player (R31, `panoply_actors`). First candidate: a crate of their own,
  `panoply_portals`, over scenery and actors.
- **PAN5 Editor**, beyond PAN1's shell (see X3). `src/editor`, written for Quill, is reference
  (`REFERENCE(PAN5)`): delete each part as `panoply_editor` replaces it. The author has screenshots
  of faery's editor. Its tools, modal (one per mode or submode button) or situational (the waypoint
  tool, opened from a field), and its property editor's field widgets are surveyed in
  [faery.md](faery.md), Editor; port each with the mode or field that needs it, the widgets chosen
  by field type (D45). faery's other debug views (listed in [faery.md](faery.md), Editor) join the
  Show menu with the features they show, not before. Among them, faery's parcel and precinct
  readiness grid, drawn from the survey's data (`TerrainPending::survey`, `SceneryPending::survey`,
  PAN10) when tuning load distances needs it on screen.
- **PAN6 Up in Arms.** The first playable: the "Up in Arms" quest, end to end. Its steps and
  what each needs are in [goals.md](goals.md).

## Sweeps

Passes over a whole document or module, a unit at a time, each unit small enough for one session.

- **PAN15 Design out of decisions.** Many decisions carry mechanism (how it works now) beside
  their reason, D10, D35 and D38 most. Move the mechanism into a new `docs/design.md`, in
  numbered sections (`TD<n>`), leaving each decision its choice, its reason and what reversing it
  would break. One area per unit (physics, rendering, loading, plans, ...), taken when a chunk
  touches that area.
