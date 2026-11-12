# Roadmap

Work not done, as chunks. What has landed is in [changelog.md](changelog.md), why things are as
they are in [decisions.md](decisions.md), work put off until an event in
[deferred.md](deferred.md), and findings not yet routed in [issues.md](issues.md).

A chunk's id, `PAN<n>`, is permanent: taken from the counter below and never reused, and named in
its commit subjects. Code waiting for a chunk is marked `TODO(PAN<n>)`, or `TODO(X<n>)` for a
deferral. Uncompiled code kept as reference until a chunk replaces it is marked
`REFERENCE(PAN<n>)`. When a chunk lands, its entry here is deleted and a line added to the
changelog.

**Next: PAN75.**

## Next

In order.

PAN3, Actors, in lettered parts. Requirements, from faery's actor module, are in
[actor-requirements.md](actor-requirements.md) (draft, for the author to edit). Layers by time scale
(existence, goals, planning, body, presentation) talk through components, with readiness as
components and every layer restartable from saved state (R104).

PAN38, Terrain mode's panel, in lettered parts: its layout, then its tools. PAN39 and PAN40 fill the
rest of it. Ported from the Quill editor (`REFERENCE(PAN5)`, `ui/mode_terrain.rs`,
`ui/tool_terrain_edit.rs`) and faery's `PlotEditPanel.tsx` (`terrain.png`). Each edit marks its
asset unsaved and goes on the undo stack.

PAN45, First formula, in lettered parts: SAGA's first customer, the chest's `when`
([saga.md](saga.md), Early customers). On demand only, with no cells or reactions; the action stays
native.

PAN28, Property editor, in lettered parts. A fork of arbor_day's `properties.rs` into
`panoply_editor`, so each develops for its own needs (sharing later is X48); its attributes replaced
by `bevy::annotations` where they overlap, as arbor_day's X25 plans for its own. faery's editor is
in [faery.md](faery.md), Editor, its screenshots included; bevy_reactor_3's `Inspectable`
(`crates/bevy_reactor_prop_inspect/src/inspectable.rs`) is a model for roots and field handles.
Widgets ask for edits rather than writing values (D72); each part adds its operations to the closed
set an edit is one of (`EditKind`).

- **PAN38d Terrain sculpting.** Raise, lower and flatten, each by brush or by rectangle, through the
  terrain cursor (`terrain_cursor.rs`). They edit the contour, so every parcel that uses it changes:
  the contours table becomes saveable here (`terrain/terrain_contour_undo.rs`).
- **PAN38e Flora drawing.** Draw trees, shrubs and herbs into the contour, and erase them, with a
  cursor around the cell under the pointer (the Quill editor's `DecalRectCursor`,
  `ui/overlays/terrain_cursor.rs`, and `terrain_pick_pos`), which the flora tools lack today.
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
- **PAN74 Floor type chooser.** The floor tool's list in the scenery panel (`exemplar_list.rs`,
  `TODO(PAN74)`): the floor types precincts name, the `scenery/floors/*.floor.ron` files. Not
  exemplars, so listed from the directory rather than loaded, which would load every floor's
  textures.
- **PAN28g Asset pickers.** A field that names an asset becomes a newtype for the kind it names
  (D45): `Skin(String)` holds a character model, so its row is a button opening a picker of
  character models only, as faery's was (the author has a screenshot), not a text input.
- **PAN28d Adding and removing aspects.** The exemplar pane's add menu: its absent optional fields,
  then Aspects…, which opens a chooser (faery's `scenery-add-aspect.png`,
  `scenery-choose-aspect.png`): a checkbox per aspect type that can attach to the exemplar's type
  (`ReflectAspect`, `can_attach`), checked where the exemplar has it, grouped by crate until mods
  have packages (X65); Save adds and removes them as one undo entry, new aspects from
  `ReflectDefault`. An × on each aspect pane's header removes that aspect, as Blender's modifier
  panels have. The exemplar's own fields become editable. `extends` is a button naming the
  prototype, not a text input; it opens the exemplar chooser scenery mode already has, listing
  exemplars of the same type, abstract ones included, and leaving out any that would make the chain
  a cycle.
- **PAN28e Instances, nav points, New and Clone.** Roots for an instance picked in selection mode
  and for the realm panel's selected nav point (faery's `NavPointEditor`). The scenery panel's New…
  and Clone… (`TODO(PAN28e)`), which add exemplars to a catalog: saving merges edits into the file
  (D73), so a new exemplar needs an insert beside it. Renaming an exemplar, whose old name joins its
  `alias` list, so that precincts naming it keep working until they are saved again.
- **PAN43 Inherited aspects.** Provisional. The aspects an exemplar inherits through `extends` show
  apart from its own, dimmed, as faery showed inherited fields (`scenery-actor-inherited.png`).
  Editing one overrides it, copying the whole aspect into the exemplar, and × on an override reverts
  to the inherited one. Inheritance in Panoply is by whole aspect, resolved when the chain is read
  (`UpdateAspects`, `find_aspect`), as faery's aspects were; faery's per-field inheritance was for
  its archetypes' own fields, which became aspects here (X99). X1's move to BSN would make `extends`
  scene inheritance.
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
- **PAN71 Imports**, after WESL's ([saga.md](saga.md), Modules). `ModuleResolver`, implemented on
  Bevy's `LoadContext`: check how Bevy main loads a dependency from within a loader. A module
  recompiles when one it imports reloads, and an import cycle is a compile error.
- **PAN72 SAGA language reference**, for mod authors, in `docs/saga-language.md`: syntax, types,
  literals and their suffixes, operators and casts, locals, `if`, functions, modules and imports,
  how a formula embeds in an asset (PAN45b), and the host functions and component reads a formula
  may use, each with a short example. It describes the language as it is, not [saga.md](saga.md)'s
  plans, and later chunks that change the language update it.
- **PAN30 Aspect hot reload.** An edited exemplar's aspects reach the actors already spawned from
  it; today only a changed precinct respawns its actors (`precinct_actors.rs`). Checked in
  `plans/actors.py`: edit an exemplar file, wait for the change on its actor, and restore the file.
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
  `panoply_portals`, over scenery and actors. Decide here, once crossing can be felt, how the view
  changes at a crossing (D64) and which way a hard cut faces: `follow_snapped` keeps the heading,
  while `follow_active_entity`'s far jump turns to the actor's facing.
- **PAN5 Editor**, beyond PAN1's shell (see X3). `src/editor`, written for Quill, is reference
  (`REFERENCE(PAN5)`): delete each part as `panoply_editor` replaces it. Scenery mode's overlays
  (faery's grid, floor cutaway, colliders, marks and portals) are `panoply_overlays` shapes in
  `mode_overlays.rs`. The author has screenshots of faery's editor. Its tools, modal (one per mode
  or submode button) or situational (the waypoint tool, opened from a field), and its property
  editor's field widgets are surveyed in [faery.md](faery.md), Editor; port each with the mode or
  field that needs it, the widgets chosen by field type (D45). faery's other debug views (listed in
  [faery.md](faery.md), Editor) join the Show menu with the features they show, not before. Among
  them, faery's parcel and precinct readiness grid, drawn from the survey's data
  (`TerrainPending::survey`, `SceneryPending::survey`, PAN10) when tuning load distances needs it on
  screen.
- **PAN6 Up in Arms.** The first playable: the "Up in Arms" quest, end to end. Its steps and
  what each needs are in [goals.md](goals.md).

## Sweeps

Passes over a whole document or module, a unit at a time, each unit small enough for one session.

- **PAN15 Design out of decisions.** Many decisions carry mechanism (how it works now) beside
  their reason, D10, D35 and D38 most. Move the mechanism into a new `docs/design.md`, in
  numbered sections (`TD<n>`), leaving each decision its choice, its reason and what reversing it
  would break. One area per unit (physics, rendering, loading, plans, ...), taken when a chunk
  touches that area.
