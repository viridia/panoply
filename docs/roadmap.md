# Roadmap

Work not done, as chunks. What has landed is in [changelog.md](changelog.md), why things are as
they are in [decisions.md](decisions.md), work put off until an event in
[deferred.md](deferred.md), and findings not yet routed in [issues.md](issues.md).

A chunk's id, `PAN<n>`, is permanent: taken from the counter below and never reused, and named in
its commit subjects. Code waiting for a chunk is marked `TODO(PAN<n>)`, or `TODO(X<n>)` for a
deferral. Uncompiled code kept as reference until a chunk replaces it is marked
`REFERENCE(PAN<n>)`. When a chunk lands, its entry here is deleted and a line added to the
changelog.

**Next: PAN113.**

Chunks are grouped by thread, a day's area of concern. Within a thread they are in order, foundation
first, but the order is preference: what a chunk cannot start without is its `*Needs:*`, after its
title, naming each open chunk, deferral or issue it waits for directly (what those wait for is
theirs to name). `xref.py` checks that each is still open.

## Terrain

PAN38, Terrain mode's panel, in lettered parts: its layout, then its tools. PAN39 and PAN40 fill the
rest of it. Ported from the Quill editor (`src/editor`, `ui/mode_terrain.rs`,
`ui/tool_terrain_edit.rs`) and faery's `PlotEditPanel.tsx` (`terrain.png`). Each edit marks its
asset unsaved and goes on the undo stack.

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
- **PAN80 Biome blend sliders.** Biome blend properties as data, with sliders: move
  `BiomeSurfaceAttrs` out of the constants in `ground_material.rs` into `terrain.biomes.json`,
  reflectable and live-editable, so a surface's transition can be tuned by eye. The file lists
  biomes, several sharing a surface (the meadows), and the properties are per surface, so where they
  sit in it is to be decided. `blend_var` sets how much of a parcel the transition spans and the gap
  between `blend_t0` and `blend_t1` the sharpness of a clump's border; separate fields for the two
  would suit sliders better.

## Water

PAN85, Water currents, for rivers, in lettered parts; partly built. Flow cannot move vertices, which
would stretch or compress where a current meets still water. Instead the fragment shader mixes noise
layers, each scrolling in its own direction (`FRAGMENT_WAVES` in `water.wesl`: four directions, +x,
+z, -x, -z, three frequencies each), and the current vector varies each layer's amplitude: still
water weights all directions equally (standing chop), and a current boosts the layers that agree
with it, e.g. a base weight plus `max(0, dot(direction, current))`. Normalizing by the total weight,
as the loop already does, keeps ripple strength even across the boundary. The current is the water
mesh's `depth_motion.yz` (written as zero in `water_mesh.rs`; `.x` is depth); the unused `motion`
variable and the commented strength gate in the loop are the start. The ripples are the only carrier
of flow, so water shading changes (X11) must keep them readable as moving bumps.

- **PAN81 Surface tuning.** Live tuning of surface looks: a reflected resource (like `Sunlight` and
  `HemisphereLight`) holding the water's look (water and sky colors, sky scale and weight, alpha,
  chop, glint) and the seabed tint, copied into the materials on change, with sliders in the editor
  panel, in a sub-mode (e.g. of terrain or realm). Tuning water (X11) one rebuild and screenshot at
  a time was slow; shaders now hot-reload (D32), but uniforms set from Rust do not.
- **PAN85a Current shading.** The effect, on test data: `water_mesh.rs` writes currents from a test
  function of position into `depth_motion.yz` (a straight channel, a bend, and a still edge), judged
  by eye in motion at a plan spot. With four axis directions, a diagonal current may read as two
  crossing patterns; six or eight directions would cover angles better, at more texture samples (now
  12); how many is settled here.
- **PAN85b Currents mode.** *Needs:* PAN85a. An editor mode for drawing river currents, saved with
  the terrain data. Where they live is decided here: a contour is shared by every parcel that uses
  it, so a river crossing parcels likely needs data per parcel or per realm map, not per contour.
- **PAN85c Currents in the water.** *Needs:* PAN85b. `water_mesh.rs` reads PAN85b's currents in
  place of the test function, and a water mesh rebuilds when they are edited.

## Scenery

A chunk that adds a mode or a tool starts from faery's design for it ([faery.md](faery.md), Editor,
its tool survey and screenshots), each chunk naming its faery source; then from what Blender does,
scoped down. Scenery mode's overlays (faery's grid, floor cutaway, colliders, marks and portals) are
`panoply_overlays` shapes in `mode_overlays.rs`. `src/editor`, written for the Quill editor, is
reference (`REFERENCE(PAN95)`): each chunk that replaces a part of it deletes that part.

- **PAN100 Precinct index.** The game asks for every precinct near the view, and only some have
  files, so each missing one logs "Path not found" (`p002-p002.msgpack` and others), burying the
  errors that matter. It needs to know which exist. A list works in packaged and web builds, where
  folder loading may not: per realm, beside the precincts it lists
  (`scenery/precincts/<realm>/index.json`), a sorted JSON array of `[x, y]` coordinates, one per
  line so a diff shows each precinct added or removed (serde_json's pretty printer would split each
  pair over four lines, so the writer formats it). A finished realm may hold about 500 precincts; at
  that size JSON costs nothing over msgpack and reads in a diff and a text editor. The editor
  rewrites it whenever it creates or deletes a precinct (since PAN94, its first edit of a precinct
  with no file), and `asset_xref.py` checks it against the folder, so a file added or removed by
  hand fails `verify.sh`.
- **PAN95 Place floors**, on PAN94's saving and PAN74a's floor entries: floors drawn on the current
  tier, with the floor tool's shapes (`TODO(PAN95)` in `scenery_panel.rs`). With the floor tool's
  options, each tool's panel becomes its own: its `construct` emits its options row and its list, as
  faery's modes were each a component, and `show_tool_options`, which shows the wall toolbar and
  moves the list's grid row from outside, goes.
- **PAN98 Paint terrain effects**, on PAN94's saving and PAN74b's entries: the terrain effect tool
  (`SceneryTool::DrawTerrainFx`) painting effects into the precinct.
- **PAN77 Scenery rect selection.** The RectSelect tool, whose button does nothing yet: drag out a
  rect on the current tier of the precinct under the pointer, snapped to the wall snap size, drawn
  as an overlay and kept as the scenery selection with its precinct. As faery's
  `editor/src/tools/SelectSceneryRectTool.ts`, where a drag that began on the same precinct grew the
  rect to include it.
- **PAN92 Instance gizmos and delete**, on PAN88's selection. Move and rotate the selected instance
  with gizmos on it, replacing faery's move and rotate modes, and delete it; each edit undoable and
  marking the precinct unsaved. Nothing draws a gizmo on a selection yet: what Bevy offers for one
  is read in the chunk.
- **PAN102 Marker fixtures**, on PAN88's picking. Fixtures with no model, standing for abstract
  things (a waymark, a region, a patrol path, a music cue), drawn by the editor alone. faery's
  marker aspects (`engine/src/world/scenery/markers.ts`: `waymark`, `circle`, `rect`, `waypoints`),
  dropped by the converter ([faery.md](faery.md), Migration scripts), become aspects whose overlays
  the editor draws while their precinct is selected, with faery's map pin as a billboard at each,
  picked through their `marker` colliders. Regions are read by wander's tether (R52) and music on
  entry (X61), and the converter keeps the instances Tambry and Heron Creek place. A patrol path is
  one fixture with an ordered list of points, not a fixture per point: a guard's field names one id,
  and the order is the list's. Its points are stored relative to the fixture, so PAN92's gizmo moves
  and turns the whole route; faery's were in world space.
- **PAN76b Cutaway editing.** *Needs:* PAN77. On its selection, adding, removing and resizing a
  tier's cutaway rects, each edit undoable and marking the precinct unsaved, and a Show menu item
  drawing them, as faery's Cutaway Rects did ([faery.md](faery.md), Editor). How faery's
  `SelectSceneryRectTool` made a rect into a cutaway is read in the chunk.

## Review

The engine's design, seen whole, and what follows from it.

- **PAN109 Engine review.** A de novo review of the engine's design: about 50,000 lines of Rust,
  which fit no one context, so map, then reduce. The map is distillation by a cheap model, file by
  file, as `local_sweep.py` does now: a fixed set of factual questions (what a file defines, the
  systems it runs and where they are scheduled, the components, resources and events it reads and
  writes, what it cites from the ledgers), answered with line citations and never judgments, since a
  weaker model drifts and invents sooner. It gains a DeepSeek backend beside Ollama's, the author's
  choice over a week of Claude's tokens; sending the source to a hosted API is the author's to
  approve, and a local DeepSeek through Ollama is the other way. A checker, `sweep_check.py`, tests
  each citation mechanically: the lines exist and hold the identifier the claim names, so drift is
  caught before anything builds on it. Then Claude reduces: per crate, a brief from its files'
  answers (responsibilities, data flow, public surface, dependencies, where it disagrees with the
  decisions it cites, and what looks accidental: duplication, faery-shaped structure, D23, dead
  ends); then one pass over the briefs with the ledgers and design docs, for what no crate shows
  alone: layering, which code owns which fact, identity and lifetime (PAN110), systems' order across
  crates. Findings go to [issues.md](issues.md) for the author to route; any acted on is read at the
  source first.
- **PAN110 Instance identity.** *Needs:* PAN109. Builds [instances.md](instances.md): keys and
  labels with a checked-in index, bindings pulled as an instance spawns, global actors as kernels in
  `ActorRoster`, and the despawn latch. Its Open section is settled first. Proved by a walkthrough
  over [Exemplars.md](Exemplars.md)'s three quests and PAN79's relay. Revises D87, D88, R7, R10,
  R72, R83 and [save-games.md](save-games.md)'s followers and moved homes; X69's matching and
  PAN79's naming follow it.

## Aspects

What an aspect is, and how an instance overrides its exemplar's, for behavior and for the scripted
aspects that mechanisms, containers and signs are built on.

- **PAN108 Aspect keys.** D107's plumbing, native only, with no goal runtime: an aspect key in place
  of the `TypeId` in `UpdateAspects` and `OwnedAspects`, the short type path in the editor (adding,
  removing, undo, each aspect's property root) and the type name in the files; each native aspect's
  config a `Bundle` behind a dyn-compatible `Aspect`, detached by a generic `remove` in place of the
  hand-written `RemoveComponent` statics; `OwnedAspects` keeping each aspect's resolved definition
  beside its detach, with a per-entity iteration over them for gathering across kinds; a test aspect
  registered by a key that is not a Rust type, attached and detached but inert, standing in for a
  scripted one until PAN104; `find_aspect` still reading native aspects along the chain. Nothing a
  player or author sees changes.
- **PAN104 Scripted aspects.** *Needs:* PAN105, PAN108. An aspect declared in a SAGA script, as
  faery's scripted aspects were, with fields and hooks. A `.saga` module declares any number of
  named aspects, keyed by package path and name, each naming the kinds of instance it attaches to
  (`aspect Lock for Fixture, Wall`), as `Aspect::can_attach` does. A field is config, unmarked;
  `state`, which play changes and a save keeps, starting from its authored value (D74); or
  `derived`, a formula recomputed as what it reads changes, never stored. A keyword changes what a
  field means; an attribute (`#[range(0.0, 4.0)]`) only how the editor shows it. Fields an instance
  sets go in one unnamed `instance { ... }` block, an aspect of its own keyed by the aspect's with a
  fixed suffix (D103); one there without a default is one the instance must supply, checked as it
  spawns, as QML's `required property` is. Each field is of a reflected Rust type the property
  editor already edits, so the layer above reflection (X64) is a list of rows, each edited through
  reflection; a field of a struct declared in SAGA waits for a use. Its hooks are not methods but a
  fixed set of reactive callbacks the engine knows, written `on <name>(...)` and checked by name, so
  a misspelled hook is an error; `present` is one (D106). So are `attach`, an effect run once as the
  aspect attaches, whose results a save keeps (faery's `enemy` rolled its loot in `init`), and one
  that fires when a deadline the aspect set passes ([saga.md](saga.md), When reactions run), for a
  door's auto-close or a button's release, where faery's `init` subscribed to every frame.
  Precedents: Unreal's `EditDefaultsOnly` and `EditInstanceOnly`, Verse's `@editable`, Svelte's
  `$props`, `$state` and `$derived`, Verilog's `parameter`, `reg` and `assign`. An aspect's state is
  a component (D107). Leaning, at runtime: as `bevy_mod_scripting` does, each scripted aspect's
  state registers its own `ComponentId` (`ComponentDescriptor::new_with_layout`), all with one Rust
  type's layout and drop, holding the aspect's key and its slots, so change ticks are per aspect and
  the value reflects and serializes as that one type; what reads them goes by id through Panoply's
  registry, never `ReflectComponent`, which would insert the shared type's own id. To check: entity
  cloning of such a component, and saved state keyed by field name, so a changed schema still loads.
  Its config registers a component of its own the same way, inserted as it attaches (D107). A hook
  or an interaction's action is an effect (D105): it writes its own aspect's state, the native
  components declared for scripts to write, and host commands, all through SAGA's buffer, so D59's
  actions also take an effect in place of a native event. Proved by one aspect on one entity, whose
  state a formula elsewhere reads; wiring across entities is PAN79's. This reverses
  [modding.md](modding.md), Types a mod declares, and meets X64's gate.

## Interaction

- **PAN59 Door.** A `Door` aspect on the cottage door (`CottageWallDoorway`), played by its
  `OpenDoor` clip through `SceneryAnimations` and offered through `Interactions`. Its collider is
  built and follows the animated glTF node its descriptor's `animation` names, as faery's
  `PhysicsMovable` did (a kinematic body moved by the model's own animation, with no separate
  animation data), so the leaf is solid and within the probe's reach through the whole swing. An
  actor in the swing: in faery, unsticking turned the overlap into velocity and threw the actor
  across the map. Panoply's push-out (R26) moves position only, a capped amount per tick, but skips
  resting actors (`ActorResting`), so the leaf must wake what it sweeps; checked with an actor
  standing in the swing. Closes I26.
- **PAN79 Mechanisms.** *Needs:* PAN59, PAN71, PAN102, PAN104. The stock elements puzzles are built
  from, and how they are wired together, on SAGA's cells and reactions (PAN105) and scripted
  aspects. faery had doors, push and toggle buttons, pressure plates, remotely controlled openables,
  automatic doors, spear traps and fireball traps (whose fixed period, PAN87, becomes scripted: when
  a trap fires), wired by reactive formulas so that elaborate puzzles combine simple parts
  ([faery.md](faery.md), How far faery got; the aspects the converter dropped are under Migration
  scripts). The first sensor collider is the `PressurePlate` wall tile's (`wall-tiles.exem.json`).
  What a sensor senses is native, by collision group, as faery's `canSense` sensed actors only, with
  a formula filter only once a puzzle needs one (a heavy object on a plate). Beside them: triggers
  (a circle or rectangle the player enters), encounter zones, waymarks for scripted NPC events, and
  patrol paths, fixtures that goals name ([goal-format.md](goal-format.md), Open). One element
  signals another through cells, a formula over the other's state, wherever a formula can say it,
  and through an event only where none can: engines built on events, such as the Creation Kit,
  tangle when several signals must be correlated. Which parts are native aspects and which are SAGA
  is to decide. Proposed: logic that spans several entities lives on a relay, an exemplar placed in
  the precinct, not as wiring stored on its participants, which in faery left no list of what was
  wired and no owner when several took part. A relay holds two aspects. A native visualization
  aspect gives its gizmo in the editor (icon, shape, color). A logic aspect is a scripted aspect
  (PAN104). A field of the instance-reference type is a role: the editor draws a wire to its target
  and fills it by picking, never by typing. Logic on one entity is a scripted aspect on that entity.
  A placed relay's roles are its logic aspect's `instance` block, set on the instance, while its
  tuning stays inherited from the exemplar (D103). A bound instance gets an external id (D87). The
  editor lists a precinct's relays, draws a selected one's wires, and lists those that name a
  selected instance; selecting, moving and copying apply to relays, and copying a trap with its
  relay binds the copy's roles to the copied instances. One wiring gesture places a stock relay for
  a button opening a door, so the simplest case stays one step. Precedents: Planet's Edge's logic
  tiles beyond the map's edge, Source's `logic_relay`, and Unreal's placed Blueprint actors with
  picked references. The hard part is referencing and naming, which PAN110 may change. A precinct is
  named by its realm and coordinates, and is like a module: its walls and fixtures that others can
  refer to are its exports, by their export id (`SceneryInstanceId::External`). That name is clumsy
  to type into a text box, so the field for a signal source browses the available signals with a
  filter, as PAN35's chooser does. A reference into a precinct that is not loaded suspends the
  formula reading it ([saga.md](saga.md), Values). faery's notes, of unknown currency, and likely to
  differ from what Panoply builds: in `design/Scripting.md`, a reference syntax, an `Instance`
  observable that is null while its target is not loaded, a `StructureRefNode` watching structures
  load and unload, and global variables or lazily loaded scope objects for long-range wiring; global
  references by realm and coordinates in `design/scripting/predicates.md`; animated instances
  (moving parts) kept apart from static instancing in `engine/src/models/InstancedModels.ts`. The
  first customer is PAN6's door opened by a button ([goals.md](goals.md), step 4).
- **PAN106 Containers.** *Needs:* PAN71, PAN104. What a container holds. An open chest shows its
  contents on an inventory screen whose background its container type chooses, and items move
  between it and the player; a locked one opens for the key its instance names, and a corpse, once
  combat kills (PAN6), is looted the same way ([goals.md](goals.md), step 7). The first feature that
  moves items, so it brings in X42. Built as D105 divides it: `Container` and `Lock` become scripted
  aspects over native components that routing (PAN16j), colliders and `SceneryAnimations` read,
  whether open and whether locked, which the native `Lock` and `ContainerState` hold now; the item
  list is a native `Inventory`, which scripts reach through host functions (`has_item(actor, key)`).
  The screen is native, opened by a host command naming its style. Closing it is an event to the
  entity that opened it, which a hook handles (`on screen_closed`), so the screen never writes a
  script's field, as faery's wrote the container's `open` ([faery.md](faery.md), Aspects); a corpse
  handles it with no lid. It opens once `SceneryAnimations` reports the lid's clip done, not after
  faery's fixed `openDelay`, which needs that event.
- **PAN78 Signs and books.** *Needs:* PAN104. Among the last chunks of all: things to read. A sign
  is a scenery aspect whose interaction shows its text; a document is an inventory item that links
  to text content and a page style (faery's book aspect opened its content in a frame, `codex` by
  default). The text is a book: a catalog entry (D90), not an exemplar ([faery.md](faery.md),
  Catalog entries).

## Editor

A chunk that adds a mode, a tool or a field widget starts from faery's design for it
([faery.md](faery.md), Editor), as the Scenery thread's do; a field's widget is chosen by its type
(D45), and a debug view of faery's joins the Show menu with the feature it shows.

PAN28, Property editor, in lettered parts. A fork of arbor_day's `properties.rs` into
`panoply_editor`, so each develops for its own needs (sharing later is X48); its attributes replaced
by `bevy::annotations` where they overlap, as arbor_day's X25 plans for its own. faery's editor is
in [faery.md](faery.md), Editor, its screenshots included; bevy_reactor_3's `Inspectable`
(`crates/bevy_reactor_prop_inspect/src/inspectable.rs`) is a model for roots and field handles.
Widgets ask for edits rather than writing values (D72); each part adds its operations to the closed
set an edit is one of (`EditKind`).

- **PAN28e Instances, nav points, New and Clone.** *Needs:* PAN92, PAN108. Roots for an instance
  picked in selection mode and for the realm panel's selected nav point (faery's `NavPointEditor`).
  An instance's aspects show as overrides of its exemplar's, and those it adds (D104) as added. The
  scenery panel's New… and Clone… (`TODO(PAN28e)`), which add exemplars to a catalog: saving merges
  edits into the file (D73), so a new exemplar needs an insert beside it. Renaming an exemplar,
  whose old name joins its `alias` list, so that precincts naming it keep working until they are
  saved again.
- **PAN43 Inherited aspects.** Provisional. The aspects an exemplar inherits through `extends` show
  apart from its own, dimmed, as faery showed inherited fields (`scenery-actor-inherited.png`).
  Editing one overrides it, copying the whole aspect into the exemplar, and × on an override reverts
  to the inherited one. Inheritance in Panoply is by whole aspect, resolved when the chain is read
  (`UpdateAspects`, `find_aspect`), as faery's aspects were; faery's per-field inheritance was for
  its archetypes' own fields, which became aspects here (X99). X1's move to BSN would make `extends`
  scene inheritance.

## Actors

PAN3, Actors, in lettered parts. Requirements, from faery's actor module, are in
[actor-requirements.md](actor-requirements.md) (draft, for the author to edit). Layers by time scale
(existence, goals, planning, body, presentation) talk through components, with readiness as
components and every layer restartable from saved state (R104).

- **PAN34 Incident log.** Unusual events, recorded for later analysis: what happened, to which
  actor, where and when. First PAN3m's put-backs, which log nothing today, and PAN3q's rescues; a
  rescue's record says what trapped the actor, so traps can be found and fixed in the content. Where
  the log is kept, and whether a plan reads it, decided when it is written; what else counts as
  unusual is added as it turns up.
- **PAN3q Stuck rescue.** *Needs:* PAN34. The backstop for traps the canopy (D39) does not cover
  (trees against walls, canopy bowls too steep to climb, clearings enclosed beyond its margin): an
  actor that is not grounded, or makes no progress against its intent, for some seconds moves to the
  nearest free spot; before navigation, the nearest cell outside the canopy cover. An actor that
  spawns inside solid scenery is rescued the same way at once: an authored place that a phased layer
  (X109) or a mod covered. Each rescue goes in PAN34's log, so the content can be fixed. Tune the
  canopy and trunk height (`CANOPY_HEIGHT`, D39).
- **PAN30 Aspect hot reload.** An edited exemplar's aspects reach the actors already spawned from
  it; today only a changed precinct respawns its actors (`precinct_actors.rs`). Checked in
  `plans/actors.py`: edit an exemplar file, wait for the change on its actor, and restore the file.
- **PAN3t Reach to open.** An actor opening or shutting a container plays a reach, `StandPoint` (an
  arm raised, 1.5 s), as faery played it for an actor opening a door, a one-second gesture faded in
  over 0.1 s (`createRoutePlanner.ts`). A gesture state played once over the idle, then back to it;
  no clip is named for it, so the state names `StandPoint` and falls back to `Stand`. Interaction
  actions are given only the element (`register_interaction_action`), so the actor using one has to
  reach the action, and `ContainerToggle` names it. The actor faces the container as it reaches.
  PAN59's door uses the same gesture. Checked in `plans/actors.py`, at the chest check: E plays the
  gesture, then the idle.
- **PAN17b Click to interact.** *Needs:* PAN3t. A click on a fixture that offers an interaction,
  which the cursor over it already shows (`pick_world`), walks the player into position to use it,
  then performs it with its pose, as E does at the probe (R92). The position is its nearest use mark
  where it has them (R79), which this chunk introduces, or else within the interaction distance (1.5
  m, the probe's reach), facing it. A door is clicked the same way once PAN59 lands. W or S on the
  way cancels it, as for a ground click. Actors, which offer no interaction yet, join with the first
  that does.
- **PAN16h Routes in behaviors.** Its first customer is PAN111's henchmen, the first NPCs whose
  behavior walks somewhere, asking for a route as PAN17a's click does. A route is searched on the
  main thread, under a millisecond across the lair and village fixtures (`tests/route.rs`), an
  unreachable goal included. NPCs' requests wait in a queue, run each frame until a budget is spent,
  the player's click ahead of them; a search paused across frames, which restarts when a precinct it
  touched gets a new mesh, only if one search alone exceeds the budget; tasks off the main thread
  only if both fall short.
- **PAN16j Doors in routes.** *Needs:* PAN59. faery read each door's `open` signal inside its
  search, but a door does not change during one query, so its cost is set before the query from
  PAN16f's table (passable at a cost, or blocked if automatic, or locked) and the door-use waypoints
  added after it, along the corridor (`findDoorUseLocation`). A door is its element's entity
  (`NavMesh::doors`); one missing is a door despawned while its precinct's mesh is rebuilt (D100),
  skipped. Following the route, an actor stops at the door's use mark, plays the opening, then goes
  through (R52).
- **PAN16k Portals in routes.** *Needs:* PAN4c. The portal area is set per query, open to the
  player's click-to-move routes and closed to NPCs' (R31), as faery's portal areas were.
- **PAN16i Routing around actors.** Exclusion zones where actors stand (R86): the generator does not
  cut them in at run time, having left out faery's patching (D99). Likely relaxed to two parts. A
  cost per query, as faery's `actor` surface already was in its search, but only on a polygon the
  actor leaves no way past: one too narrow at the actor for the agent to go around it, judged by
  width, not area, since a corridor is large but narrow. The cost follows the two actors' weights
  (D102): one the agent outweighs costs the time to shove it, and one far heavier blocks the polygon
  for this query. An actor in a field costs nothing, so polygon size need not be capped; faery
  capped its regions at 4 m cells (`REGION_QUANTA`) to bound the cost of cutting actors in, which
  Panoply does not do. Then the smoothed path bends around the actors near it: where a segment
  passes within an actor's radius plus the agent's plus a margin, waypoints on the tangents of that
  circle, on the side whose segments stay on the mesh (a raycast along it). The margin keeps a path
  out of personal space where there is room, though the push lets capsules overlap (D102): an actor
  enters another's space only when forced to. Bending suits standing actors, who are where the plan
  saw them; moving ones are the aura's (R23). Where neither side stays on the mesh, the route
  replans. Before bending is built, an artifact for the author: its test cases, illustrated, to find
  the edge cases. Crowd and follower behavior (R86, R87) needs test scenarios: driver plans that
  stage, for example, eight skeletons rushing the player, or two followers through a doorway.

## Portals

PAN4, Portals, in lettered parts: a rewrite, not a restoration (D21). What an aperture shows is
D111.

- **PAN4c Crossing.** The player crosses a portal into the target realm, keeping offset and velocity
  (R31), detected by a volume at the aperture, which brings actors into `panoply_portals`. Decide
  here, once crossing can be felt, how the view changes at a crossing (D64) and which way a hard cut
  faces: `follow_snapped` keeps the heading, while `follow_active_entity`'s far jump turns to the
  actor's facing.
- **PAN4d Portal editing.** Helpers that keep a portal's two ends in step: a portal chooser
  (`chooser_dialog`) to pick the far end, which sets both ends' `PortalTarget`, and whatever else
  syncing them needs, decided in the chunk.

## Effects

- **PAN75 Point lights.** A `LightSource` (walls and fixtures; three lit fixtures so far, each 2 m)
  spawns a Bevy `PointLight` as a child of its instance, so it streams in and out with the precinct:
  `range` from `radius`, `color`, `intensity` (none set yet, so a default chosen by eye), at
  `offset`, shadows off. faery capped a scene at about six point lights, since three.js lit every
  fragment with every light; Bevy's clustered forward renderer bins lights into cells of the view
  and lights a fragment only with its cell's, so the cost follows the lights reaching a fragment,
  here 0 or 1, and `range` is what keeps a light's cells few. Measure it in `plans/perf.py`: a spot
  with many lit fixtures, against the same spot without them. Shadows are left for later: each
  shadowed point light renders the depth around it six times a frame, a cube map, which is the cost
  that stays. Later: `enabled`, which needs instance properties, and missiles, which `can_attach`
  does not yet allow.
- **PAN83 Reflections** for shiny objects: `LightProbe` volumes with `EnvironmentMapLight` (using
  `skybox_diffuse_32.ktx2` and `skybox_specular.ktx2`) where shiny objects are, instead of on the
  camera. The sky itself is never in view; the skybox is only there to light. Without an environment
  map, mirrored surfaces look flat, and environment maps have proved expensive, so this is an
  experiment in cost: measure each probe layout with `plans/perf.py`.
- **PAN86 Particles**, as designed in [particles.md](particles.md): its first slice, the campfire's
  smoke. faery's `Campfire` had a `particles` aspect, `particles:WhiteSmoke` at `[0, 0.15, 0]`
  (`los-data/meta/particles.json`), which the converter dropped; Panoply's `Campfire`
  (`camping.exem.json`) gets it back, beside the `Test.Particles` fixtures faery kept for trying
  effects. Snoring waits for a sleep state (X44).
- **PAN87 Missiles.** *Needs:* PAN86. Entities without automatic instancing that collide, light,
  sound and emit particles, as faery's particles did and GPU particles cannot
  ([particles.md](particles.md), Two layers). The first are the fireball traps: `FireballEmitter`
  and `FireballEmitterRapid` (`furnishings.exem.json`), placed in the playground (`p002-p001`) and
  the necropolis (`holm-underground/p017-p013`). faery's `particles` aspect on them fired
  `particles:Fireball` and `particles:FireballTrapRapid` (`los-data/meta/particles.json`): a
  fireball that collides (`collisionMask`), lights within 4 m, sounds on spawn and on hitting, and
  bursts into PAN86's particles. faery timed a trap's shots by the effect's emission rate
  (`emissionRate` 0.5 with `maxParticles` 1, a shot every 2 s); here launching is the trap's own,
  not a particle setting: a fixed period on its aspect for now, scripted in PAN79. Its fire damage
  waits for actors to have health, in PAN6's combat.
- **PAN20 Outline width by distance.** An experiment: flatten the curve by which outlines thin with
  distance, without making them constant. `outline.wesl` extrudes in mesh-local units, so on screen
  the width falls as 1 / depth. Scale the extrusion by `(depth / reference)^k`: `k = 0` is today,
  `k = 1` a constant screen width; try values between. Depth to the mesh's origin rather than each
  vertex, so a long model's outline does not taper along it. `k` and the reference as one setting
  for every outline material, tuned by eye in the editor at several zooms; in play the camera
  distance is fixed (11), so there it only evens out the near and far rows of the view. Divide the
  extrusion by the model's scale too, so an actor's outline keeps its width under `ActorScale`,
  which today thickens it.

## Outlines and clips

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

## SAGA

- **PAN71 Imports**, modeled on WESL's ([saga.md](saga.md), Modules), of a `.saga` module's `pub`
  functions. Its constants wait on X91, its structs on X87, and imported exemplars on X118.
  `ModuleResolver`, implemented on Bevy's `LoadContext`: check how Bevy main loads a dependency from
  within a loader. A module recompiles when one it imports reloads, and an import cycle is a compile
  error. The parser also takes a bare `import a::b;`, which becomes a namespace (`b::f`) or an
  error.
- **PAN107 Value slots.** Step 2 of [saga.md](saga.md), Order of work, as its Values section lays
  them out: host-declared structs and fixed-size arrays flattened into slots, and a stack with no
  owned data, replacing what The crate lists under what value slots change. The `TODO(PAN107)`s in
  `panoply_saga` include tuples, tuple structs and enums, which Values does not cover and whose
  enums it wraps rather than flattens: each is built here or routed elsewhere.
- **PAN105 Cells and reactions.** *Needs:* PAN107. Step 3 of [saga.md](saga.md), Order of work, its
  first customer an animation's `when` (Early customers).
- **PAN72 SAGA language reference.** *Needs:* PAN71, PAN104. For mod authors, in
  `docs/saga-language.md`: syntax, types, literals and their suffixes, operators and casts, locals,
  `if`, functions, modules and imports, how a formula embeds in an asset (PAN45b), and the host
  functions and component reads a formula may use, each with a short example. It describes the
  language as it is, not [saga.md](saga.md)'s plans, and later chunks that change the language
  update it.

## Saves

- **PAN19 New game.** Resets every precinct, and the actors in it, to its authored state: despawn
  them and spawn them again from their assets and exemplars. Saves are sparse, holding only what
  play changed (PAN82), so loading a save is a new game with the save's changes applied. R12 is the
  same reset for one actor.
- **PAN82 Save games.** *Needs:* PAN19. Save games with structural sharing: one chunk of play
  changes per precinct, written on eviction; a save is only a root index of chunk versions. Design
  and what it asks of the engine in [save-games.md](save-games.md). X14's picked cells would live in
  the precinct chunk. The camera angle is saved too; a load follows the player with
  `Viewpoint::follow_snapped`, which keeps it. The first state to save: a chest left open.

## First playable

- **PAN111 Combat arena.** *Needs:* PAN16h, X53. The first test of combat's fun, the biggest fun
  risk ([goals.md](goals.md)), before PAN6 wraps it in a quest: the hero against a few henchmen in
  the playground (`p002-p001`), with health, the action bar's plain actions, and Justice's signature
  skills as opportunity actions ([combat.md](combat.md), The action bar; [virtues.md](virtues.md),
  Combat skills). Skills and timings are assets that reload while the game runs. Placeholder clips
  are enough to judge the decisions; whether players accept enemies waiting as a convention depends
  on presentation, and is judged here only roughly.
- **PAN6 Up in Arms.** *Needs:* PAN16h, PAN79, PAN106, PAN111. The first playable: the "Up in Arms"
  quest, end to end. Its steps and what each needs are in [goals.md](goals.md). Its quest phases
  Sergeant Stone and the Crimson Watch, so it builds presence (D106).

## Performance

- **PAN112 Entity budget.** The playground's mine entrance holds about 37,500 entities, 15,600 of
  them meshes, and every camera checks each mesh's visibility, so a portal pays for them again
  ([notes.md](notes.md), a portal's cost). Flora is most of them: a tree is about seven entities
  (its scene root, a node and a mesh each for bark and leaves, and outline children), and about
  3,800 trees and shrubs were loaded; outline children are 5,900 more. First the rest of the census:
  parcels shown and hidden, and flora per parcel, since D101 loads the simulated precincts' parcels
  with their flora's models though actors need only their colliders. Then, from what it shows: flora
  models only for parcels near the view; trees spawned as their meshes, without the glTF scene's
  root and nodes; flora instanced, an entity per parcel, model and part; or outlines drawn by a pass
  of their own rather than as child entities. The last two are structural, proposed before code.

## Technical debt

Code that works but stands in the way: a pattern the rest of the code has moved past, or a
workaround for what should be fixed at its cause. Each is taken when its area is next touched, or
sooner when it bites.

- **PAN103 Tools on picking events.** The editor's world tools read the mouse by polling
  `PointerState`, which passes by picking's routing: what a widget or dialog in front of the view
  does with a press does not reach them. So closing a dialog by clicking outside it, which despawns
  the barrier the press landed on, reads as a press in the view, and starts a wall stroke or a
  placement (selection mode had the same until PAN88 moved it to events). Move the wall tool
  (`wall_tool.rs`), the placement gesture of the fixture and actor tools (`placement.rs`) and the
  terrain brushes (`terrain_cursor.rs`) to picking backends and `Pointer<Press>`, `Pointer<Drag>`
  and `Pointer<DragEnd>` observers, as terrain mode's parcel selection and selection mode take
  theirs (`parcel_selection.rs`, `instance_selection.rs`). Each gesture spans frames, press to
  release, and Escape cancels one mid-drag. And the click observers that handle a click without
  consuming it, which D94 asks of them, do: `select_parcel` (`parcel_selection.rs`), and the
  chooser's and the model picker's rows (`chooser.rs`, `model_picker.rs`).
- **PAN91 One reload per save.** An edit made just after saving can be lost: the save barrier
  (`update_save_barrier`, `unsaved.rs`) lifts on the saved file's first reload, and one save can
  bring a second, which replaces the exemplars with the file as saved. Seen in `plans/smoke.py`'s
  `slots_property` (an undo after the save, then a reload, then a save with nothing to write); its
  log showed a lone "Reloaded exemplars/characters.exem.json" after the save, then "Reloading folder
  exemplars" with another. Likely cause, unconfirmed: the catalogs load as a folder and as files, so
  a save fires a file reload and a folder reload. Read in Bevy's source how a folder reload treats a
  file already loaded; then make a save cause one reload, or have the barrier wait for every reload
  it causes.
- **PAN101 Editor-owned assets unwatched.** The file watcher is for changes another app makes:
  models exported from Blender, shaders, textures, which stay watched. The assets only the editor
  changes (precincts, catalogs, terrain maps, nav points) it already propagates itself, so watching
  them only echoes its own saves: the save barrier and its timeout exist to wait out that echo. A
  checkout or converter run is followed by a restart, or later a reload command. Bevy watches per
  asset source, not per type (`reload_path` in `bevy_asset`'s server), so the likely shape is a
  second source over `assets/` with no watcher, for those assets. Propose before code, settling:
  paths that cross sources (precincts name exemplars as plain paths, catalogs extend relative to
  themselves), external ids written into memory at save rather than arriving with the reload (D88),
  plans restoring files in memory rather than by writing them, and removing the barrier. Two
  symptoms go with the echo. In `plans/smoke.py` a catalog written back by the plan reloaded 0.11 s
  after the next save, the barrier took it for that save's reload, and the real one then overwrote
  the undo that followed, so the second save kept the edit (`saved_diff`, PAN94's runs). And each
  catalog save respawns the models of every element of that catalog (`rebuild_changed_se_aspects`,
  `spawn_se_models`), so the walls in view vanish for a few frames. An upstream alternative: the
  asset server skipping the reload of a file it wrote through its own `AssetWriter`
  ([bevy#26572](https://github.com/bevyengine/bevy/issues/26572)); check it first.

## Sweeps

Passes over a whole document or module, a unit at a time, each unit small enough for one session.

- **PAN15 Design out of decisions.** Many decisions carry mechanism (how it works now) beside
  their reason, D10, D35 and D38 most. Move the mechanism into a new `docs/design.md`, in
  numbered sections (`TD<n>`), leaving each decision its choice, its reason and what reversing it
  would break. One area per unit (physics, rendering, loading, plans, ...), taken when a chunk
  touches that area.
