# Roadmap

Work not done, as chunks. What has landed is in [changelog.md](changelog.md), why things are as
they are in [decisions.md](decisions.md), work put off until an event in
[deferred.md](deferred.md), and findings not yet routed in [issues.md](issues.md).

A chunk's id, `PAN<n>`, is permanent: taken from the counter below and never reused, and named in
its commit subjects. Code waiting for a chunk is marked `TODO(PAN<n>)`, or `TODO(X<n>)` for a
deferral. Uncompiled code kept as reference until a chunk replaces it is marked
`REFERENCE(PAN<n>)`. When a chunk lands, its entry here is deleted and a line added to the
changelog.

**Next: PAN104.**

Chunks are grouped by thread, a day's area of concern. Within a thread they are in order, foundation
first; a chunk that needs another thread's says so.

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
- **PAN84 Realms without terrain.** `underworld` (in `world.locations.json`) is floors only, with
  black everywhere else; the three.js version needed a placeholder terrain map for it (a 3x3 stub,
  which `convert_realm_maps.mts` writes; deleted here). Go To's `underworld` nav point is the way
  in. Check that a realm with no `.terrain` file spawns no parcels and no errors, and that parcels'
  wait for precinct effects (D11) does not hang; make a stub map only if it must.

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
- **PAN85b Currents mode.** An editor mode for drawing river currents, saved with the terrain data.
  Where they live is decided here: a contour is shared by every parcel that uses it, so a river
  crossing parcels likely needs data per parcel or per realm map, not per contour.
- **PAN85c Currents in the water.** `water_mesh.rs` reads PAN85b's currents in place of the test
  function, and a water mesh rebuilds when they are edited.

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
- **PAN93 Make global**, on PAN88's selection. A toggle that takes the selected actor out of its
  precinct, as faery's did. Where a global actor is kept, and how it loads and saves, is decided in
  the chunk; read faery's first.
- **PAN76b Cutaway editing**, on PAN77's selection. Adding, removing and resizing a tier's cutaway
  rects, each edit undoable and marking the precinct unsaved, and a Show menu item drawing them, as
  faery's Cutaway Rects did ([faery.md](faery.md), Editor). How faery's `SelectSceneryRectTool` made
  a rect into a cutaway is read in the chunk.

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
- **PAN79 Mechanisms**: the stock elements puzzles are built from, and how they are wired together,
  on SAGA's cells and reactions (X7). faery had doors, push and toggle buttons, pressure plates,
  remotely controlled openables, automatic doors, spear traps and fireball traps (whose fixed
  period, PAN87, becomes scripted: when a trap fires), wired by reactive formulas so that elaborate
  puzzles combine simple parts ([faery.md](faery.md), How far faery got; the aspects the converter
  dropped are under Migration scripts). The first sensor collider is the `PressurePlate` wall tile's
  (`wall-tiles.exem.json`). Beside them: triggers (a circle or rectangle the player enters),
  encounter zones, waymarks for scripted NPC events, and patrol paths, fixtures that goals name
  ([goal-format.md](goal-format.md), Open). One element signals another through cells, a formula
  over the other's state, wherever a formula can say it, and through an event only where none can:
  engines built on events, such as the Creation Kit, tangle when several signals must be correlated.
  Which parts are native aspects and which are SAGA is to decide. Proposed: logic that spans several
  entities lives on a relay, an exemplar placed in the precinct, not as wiring stored on its
  participants, which in faery left no list of what was wired and no owner when several took part. A
  relay holds two aspects. A native visualization aspect gives its gizmo in the editor (icon, shape,
  color). A logic aspect, declared in a SAGA script as faery's scripted aspects were, has fields and
  hooks. Its fields are config or state as D88 divides them (the syntax is open: perhaps `@` marks
  config, after BSN), each of a reflected Rust type the property editor already edits, so the layer
  above reflection (X64) is a list of rows, each edited through reflection; a field of a struct
  declared in SAGA waits for a use. Its hooks are not methods but a fixed set of reactive callbacks
  the engine knows. A field of the instance-reference type is a role: the editor draws a wire to its
  target and fills it by picking, never by typing. Logic on one entity is a scripted aspect on that
  entity. A placed relay overrides its logic aspect to bind its roles; keeping the exemplar's tuning
  inherited while it does is likely X99's gate. A bound instance gets an external id (D87). The
  editor lists a precinct's relays, draws a selected one's wires, and lists those that name a
  selected instance; selecting, moving and copying apply to relays, and copying a trap with its
  relay binds the copy's roles to the copied instances. One wiring gesture places a stock relay for
  a button opening a door, so the simplest case stays one step. This reverses
  [modding.md](modding.md), Types a mod declares, and meets X64's gate. Precedents: Planet's Edge's
  logic tiles beyond the map's edge, Source's `logic_relay`, and Unreal's placed Blueprint actors
  with picked references. The hard part is referencing and naming. A precinct is named by its realm
  and coordinates, and is like a module: its walls and fixtures that others can refer to are its
  exports, by their export id (`SceneryInstanceId::External`). That name is clumsy to type into a
  text box, so the field for a signal source browses the available signals with a filter, as PAN35's
  chooser does. A reference into a precinct that is not loaded suspends the formula reading it
  ([saga.md](saga.md), Values). faery's notes, of unknown currency, and likely to differ from what
  Panoply builds: in `design/Scripting.md`, a reference syntax, an `Instance` observable that is
  null while its target is not loaded, a `StructureRefNode` watching structures load and unload, and
  global variables or lazily loaded scope objects for long-range wiring; global references by realm
  and coordinates in `design/scripting/predicates.md`; animated instances (moving parts) kept apart
  from static instancing in `engine/src/models/InstancedModels.ts`. The first customer is PAN6's
  door opened by a button ([goals.md](goals.md), step 4).
- **PAN78 Signs and books**, among the last chunks of all: things to read. A sign is a scenery
  aspect whose interaction shows its text; a document is an inventory item that links to text
  content and a page style (faery's book aspect opened its content in a frame, `codex` by default).
  The text is a book: a catalog entry (D90), not an exemplar ([faery.md](faery.md), Catalog
  entries).

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

- **PAN28e Instances, nav points, New and Clone.** After PAN88, PAN92 and PAN93. Roots for an
  instance picked in selection mode and for the realm panel's selected nav point (faery's
  `NavPointEditor`). An instance's aspects show as overrides of its exemplar's. Whether an instance
  may also add aspects its exemplar lacks is settled with how behavior composes
  ([Exemplars.md](Exemplars.md), Composing behavior), helped by a read-only scan of the precincts
  for instances that do (Poul Ryba's carries `tambry:poul`, his goals at the forge). The scenery
  panel's New… and Clone… (`TODO(PAN28e)`), which add exemplars to a catalog: saving merges edits
  into the file (D73), so a new exemplar needs an insert beside it. Renaming an exemplar, whose old
  name joins its `alias` list, so that precincts naming it keep working until they are saved again.
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

- **PAN99 Composing behavior, decided.** Research, then a branch: settles
  [Exemplars.md](Exemplars.md), Composing behavior, and with it whether an instance may add aspects
  (PAN28e); dialogue and remarks are settled (D86). A scenario walkthrough of the leading
  alternatives (aspects as definitions; native and external aspects), each step tagged asset,
  formula or native, over a sample: faery's 15 per-character aspects (`behaviors/demo.tsx`,
  `tambry.tsx`, `heroncreek.tsx`) and its generic behavior (`standard.ts`, `work.ts`); its aspects
  that touch behavior (`conversant`, `follower`, `shop`, `combat`) and logic doors
  (`mechanisms.ts`); the portal and container aspects; and quests at Marheim's scale, which the
  author supplies. Its gaps answer the open questions: an aspect an exemplar declares and an
  instance must supply (`PortalTarget`), a package on disk before BSN (X1), formulas embedded in
  assets, and a missing or unknown piece (D47, X20). Then a branch, in its own worktree, implements
  the chosen alternative's plumbing, with no goal runtime: an aspect key in place of the `TypeId` in
  the union, the short type path in the editor and the type name in the files; an external aspect,
  attached but inert; the add menu listing package assets; saving and undo with keyed entries;
  `find_aspect` still reading native aspects along the chain. It measures the cost and finds what
  the audit missed, and is kept if it stands on its own. Lands as a decision, with X17, PAN28e and
  X74 brought in line.

- **PAN34 Incident log.** Unusual events, recorded for later analysis: what happened, to which
  actor, where and when. First PAN3m's put-backs, which log nothing today, and PAN3q's rescues; a
  rescue's record says what trapped the actor, so traps can be found and fixed in the content. Where
  the log is kept, and whether a plan reads it, decided when it is written; what else counts as
  unusual is added as it turns up.
- **PAN3q Stuck rescue**, the backstop for traps the canopy (D39) does not cover (trees against
  walls, canopy bowls too steep to climb, clearings enclosed beyond its margin): an actor that is
  not grounded, or makes no progress against its intent, for some seconds moves to the nearest free
  spot; before navigation, the nearest cell outside the canopy cover. An actor that spawns inside
  solid scenery is rescued the same way at once: an authored place that a phased layer (X109) or a
  mod covered. Each rescue goes in PAN34's log, so the content can be fixed. Tune the canopy and
  trunk height (`CANOPY_HEIGHT`, D39).
- **PAN90 Bumping.** The player walking into a standing actor pushes it aside, and it plays R33's
  stumble state, at most 4 m/s (R28). A repulsion between actors (R23, R86), not a collision: the
  aura is added to an actor's travel before the controller moves it, so a pushed actor is still
  swept against the world and is never pushed through a wall. A smoothed velocity rather than a
  force, so it does not jitter; a weight per actor, so NPCs yield to the player more than the player
  to them. The weight follows size and stance: a dwarf does not shove a giant, a guard on duty
  stands planted, and a scripted goal may make an actor immovable, to block a path for a scene.
  Whether it is computed or an aspect's value that goals change is settled here; PAN16i's route
  costs read it. Its radius leaves melee in reach. An actor inside another's space wakes (PAN3p).
  Neighbours by testing every pair in the realm until a grid is needed. The playground's townsfolk
  are the test. X50 builds NPCs' steering on it.
- **PAN30 Aspect hot reload.** An edited exemplar's aspects reach the actors already spawned from
  it; today only a changed precinct respawns its actors (`precinct_actors.rs`). Checked in
  `plans/actors.py`: edit an exemplar file, wait for the change on its actor, and restore the file.
- **PAN16a Navigation mesh and input.** Panoply has no navigation; faery's is in `engine/src/nav`
  (see [faery.md](faery.md)). Two generators are prototyped (PAN16c, PAN16d) and compared (PAN16e),
  all on main: each a module of a crate of its own (`panoply_nav`), both compiled in and chosen at
  run time, since the comparison needs both in one build, which separate branches or Cargo features
  would not give (features would also leave one unchecked by `verify.sh`'s clippy). Their
  dependencies (`rerecast`, `i_overlay`, `i_triangle`) need no Bevy and are small; the loser's go
  with it. This chunk lands what both share.
  - The mesh type, Panoply's own, which both generators produce: shared vertices, convex polygons, a
    neighbour per edge, an area per polygon, and the side of each edge on the precinct boundary,
    which PAN16g joins across precincts.
  - A precinct's navigation input as a snapshot: its static colliders' shapes and poses, its ground,
    floor and flora trimeshes, and its door and hint colliders, whose volumes `ColliderDesc::shape`
    does not give today (it returns `None` for all but solids). PAN16f builds it in the game; here
    it is saved to a fixture file from a loaded precinct (an editor command or a driver step), so
    both prototypes' tests read the same input, the densest real precinct (the playground's
    cottage), without loading assets and catalogs.
- **PAN16b Navmesh visualization.** A debug view of PAN16a's mesh type, ahead of the prototypes so
  each is seen while it is built, and PAN16e sees both over the same precinct, a setting in the view
  choosing which generator builds the mesh. Checked on a mesh made by hand. Copied from
  `bevy_rerecast`'s `debug.rs` (MIT/Apache, credited in a comment): polygon outlines into a retained
  `GizmoAsset`, a translucent unlit fill, rebuilt only when a mesh changes. Its asset-event plumbing
  is not copied, since Panoply's meshes are per-precinct data. Added: fill coloured by area, so door
  and blocked polygons stand out; the boundary edges in their own colour, to see that neighbours
  meet; for Recast, the fill from its detail mesh, since its polygon heights can be off by the
  climb. Interiors show once PAN76a cuts roofs away.
- **PAN16c Recast prototype.** From PAN16a's fixture to its mesh type:
  [rerecast](https://crates.io/crates/rerecast)'s core crate (1.0), a Bevy-free port of Recast that
  generates meshes only, driven by our own code (D36): `bevy_rerecast` follows Bevy releases and
  assumes Avian, but its `generator/mod.rs` is the template. A test that prints build time and peak
  memory at 0.1 m and 0.15 m cells; if the time is too long, 16 m tiles, which bound a rebuild to
  the tiles an edit touches.
  - Input as triangles: ground, floors and flora already are, as colliders, and scenery solids
    become them through parry's `to_trimesh` (as `avian_rerecast`'s `collider_to_trimesh.rs` does).
  - The precinct as a Recast tile: `ConfigBuilder::tiling` pads the build area by the agent radius
    plus 3 cells, and `into_polygon_mesh` marks the edges on the tile's boundary with their side.
    Built from the precinct's own geometry alone, as faery built its tracts; left empty, the padding
    would erode the mesh away from the boundary and leave no boundary edges, so the precinct's edge
    columns are copied outward across it after rasterizing (`Heightfield`'s spans are public): open
    ground stays open to the line, and a wall meeting the line continues past it.
  - Cell size: erosion takes the agent radius rounded up to whole cells, from each side of an
    opening, and rasterizing takes any cell a jamb touches. The cottage doorway is 0.8 m and a
    human's radius 0.3 m: at 0.2 m cells, 2 cells are eroded from each side of about 3 or 4 free
    ones, which closes it; at 0.1 m, 3 from each side of 8 leaves 2. Recast's rule of a third to
    half the radius also gives 0.1 m.
  - Doors: one `DOOR` area type, marked by each door collider's footprint as a `ConvexVolume`.
    Faery's door region was a sliver as thick as the leaf (`addDoorObstacles` in
    `NavigationMeshBuilder.ts`), which its exact clipping kept; on Recast's grid that strip is a
    cell or two, under `min_region_area`, which would delete it and close the doorway, so the
    footprint is grown across the wall's depth. Area types never share a region, so door polygons
    are their own.
  - Hints and floors that block: area 0, a hint's footprint grown by the agent radius, since marking
    comes after erosion and would otherwise let a body overlap it.

  A missing feature may be upstreamed rather than forked: the author has talked with rerecast's
  author.
- **PAN16d Port prototype.** PAN16a's fixture and mesh type, from a port of faery's builder: 2D
  polygons cut by exact boolean operations and joined as a graph, about 2,000 lines of TypeScript
  (`NavigationMeshBuilder.ts`, `NavigationMesh.ts`, `NavRegion.ts`, the tract files), whose cost
  follows the obstacles, where Recast's follows the area over the cell size squared, empty ground
  included, and whose edges are exact, where Recast's fall on its grid. Ported lean: without faery's
  4 m cells, which bounded the cost of cutting actors in (PAN16i routes around them instead), and
  without its broad phase (`rbush`) or half-edge editing, both for patching a mesh in place.
  Instead, a layer's obstacle outlines, inflated by the agent radius, merged and subtracted in batch
  by `i_overlay` (Bevy-free, integer-robust; it replaces `polygon-clipping` and `inflatePolygon`),
  then split into convex polygons (Hertel–Mehlhorn over a Delaunay triangulation, `i_triangle`),
  with adjacency from shared edges. Read faery's code for bugs as it is ported.
  - Input as footprints: the collider shapes' outlines, the floor regions, and the terrain's
    walkability ported from faery's (`navHeightMap`).
  - Doors are the leaf's footprint, as faery's were, and hints are subtracted as solids are, as
    faery did.
  - Joining layers at stairs and ramps (`isSamePlane`, `projectVertexToPlane` in `NavRegion.ts`) is
    left out and judged by reading in PAN16e, since it is where faery's bugs are likeliest.
- **PAN16e Generator decision.** PAN16c's and PAN16d's meshes compared on build time, peak memory,
  whether the cottage doorway stays open, polygon count, and how closely the edges follow the
  obstacles, in PAN16b's view; the port's layer joining judged by reading. Lands as a decision. The
  loser's module and dependencies are removed, and with them the setting that chose between them;
  the winner's grows into PAN16f.
- **PAN16f Navmesh generation.** The winning generator in the game, one mesh per precinct.
  - Built from the precinct's own geometry alone, when it loads: the neighbours' obstacles are
    PAN16g's, which intersects the two sides' boundary edges when a route crosses. An obstacle just
    across the line does not narrow this side's mesh, so a route running beside the seam may pass
    within the agent radius of it.
  - Off the main thread, as ground, floor and flora meshes are (`AsyncComputeTaskPool`): the main
    thread only takes PAN16a's snapshot, which clones shapes rather than triangulating them, and the
    task does the rest. A route asked of a precinct whose mesh is not ready gets "not ready". An
    edit in the editor regenerates when the edit ends, a generation count discarding stale results.
    A cache or baked meshes wait on PAN16e's build times.
  - Doors (R98): the door collider is the closed leaf, kinematic once PAN59 builds it, so it stays
    out of the input, which takes static colliders only. The task maps each `DOOR` polygon to its
    door by testing the polygon's centroid against the precinct's door list (a `Vec`; a precinct has
    a few dozen doors at most) into a table indexed by polygon. Areas are kept for cost classes. The
    editor flags door volumes that touch, which would merge into one polygon.
  - Hints: a hint collider is where routes do not go but actors may stand: beside the stone
    sarcophagus, where its lid lands, and on the hinge side of a door, where the open leaf rests
    (`CottageWallDoorway`). A usage mark placed on the mesh then lands outside it.
  - Floors that block: faery's `BlackoutFloor` was `blocked`, the unlit black fill between cave
    walls in the underground, where no one walks. Panoply's floor entries have no such field yet.
  - Agent radius is part of a mesh. Two size classes: human (0.3 m), used by smaller actors too, and
    large, for actors larger than human, built only for precincts where one is simulated (R83). A
    large mesh closes the doorways a large actor does not fit, which is the behavior wanted.
- **PAN16g Pathfinding.** Routes across the joined precinct meshes, with the route drawn in PAN16b's
  view. A crate, if its Bevy-free core takes costs or filters per query and joins meshes
  (`landmass`, `polyanya` under `vleue_navigator`); else a port of `NavRouteTask.ts` (A*, a funnel
  pass, tract seams), which runs on any mesh of convex polygons. Read with `local_sweep.py`
  inventories first, faery's `nav` and the candidate crates, then the files they point to: an
  inventory finds where to read, but misses what one file guarantees and another relies on.
  - Seams: a route crosses a precinct boundary only where the boundary edges of the two sides
    overlap, a 1D intersection along the line, as faery's `perimeter` edges were
    (`neighborTransition` in `NavRouteTask.ts`); each side's edges are already cut short by its own
    obstacles.
  - Doors: faery read each door's `open` signal inside its search, but a door does not change during
    one query, so its cost is set before the query from PAN16f's table (passable at a cost, or
    blocked if automatic, or locked) and the door-use waypoints added after it, along the corridor
    (`findDoorUseLocation`).
  - Ladders: rerecast has no off-mesh links, so with either generator they are the query side's.

  Settle the choice with one scenario walkthrough: a guard routes through a door into the next
  precinct.
- **PAN16h Routes in behaviors.** Actors follow routes: a behavior asks for one, waits while the
  mesh is not ready, opens doors on the way (R52), and an actor that cannot advance plays R33's
  blocked state and replans (R22).
- **PAN16i Routing around actors.** Exclusion zones where actors stand (R86): neither generator cuts
  them in at run time (rerecast has no tile cache, and the port drops faery's patching). Likely
  relaxed to two parts. A cost per query, as faery's `actor` surface already was in its search, but
  only on a polygon the actor leaves no way past: one too narrow at the actor for the agent to go
  around it, judged by width, not area, since a corridor is large but narrow. The cost follows the
  two actors' weights (PAN90): one the agent outweighs costs the time to shove it, and one far
  heavier blocks the polygon for this query. An actor in a field costs nothing, so polygon size need
  not be capped; faery capped its regions at 4 m cells (`REGION_QUANTA`) to bound the cost of
  cutting actors in, which Panoply does not do. Then the smoothed path bends around the actors near
  it: where a segment passes within an actor's radius plus the agent's, waypoints on the tangents of
  that circle, on the side whose segments stay on the mesh (a raycast along it). Bending suits
  standing actors, who are where the plan saw them; moving ones are the aura's (R23). Where neither
  side stays on the mesh, the route replans. Before bending is built, an artifact for the author:
  its test cases, illustrated, to find the edge cases. Crowd and follower behavior (R86, R87) needs
  test scenarios: driver plans that stage, for example, eight skeletons rushing the player, or two
  followers through a doorway.
- **PAN17 Click to move.** Clicking an actor or fixture needs clicks in the world
  (`bevy_remote_driver`'s X9). Clicking ground to walk to is Panoply's own: a server method of its
  own (the driver's X2), or a waypoint entity spawned there and clicked through the driver's X9. The
  game's click handling runs only in `GameMode::Play`. A click that misses every interactive thing
  lands on `WorldBackstop` (D61), and a ground click needs the ground point, which it does not
  report.

## Portals

PAN4, Portals, in lettered parts: a rewrite, not a restoration (D21). `src/portals` is reference
(`REFERENCE(PAN4)`), deleted when PAN4c lands.

- **PAN4b Portal view.** What the aperture shows: the far end, rendered by a camera of its own (D21:
  off-axis projection fitted to the aperture, its own load region), with gizmos showing each
  aperture to judge it against. Bevy's `examples/3d/mirror.rs` (a second camera on a render target)
  and `examples/camera/custom_projection.rs` (an oblique clip plane) are the starting point,
  replacing the old `active_portal.rs`. `panoply_portals` depends on scenery only; crossing (PAN4c)
  adds actors.
- **PAN4c Crossing.** The player crosses a portal into the target realm, keeping offset and velocity
  (R31), detected by a volume at the aperture. Decide here, once crossing can be felt, how the view
  changes at a crossing (D64) and which way a hard cut faces: `follow_snapped` keeps the heading,
  while `follow_active_entity`'s far jump turns to the actor's facing.
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
- **PAN87 Missiles**: entities without automatic instancing that collide, light, sound and emit
  particles, as faery's particles did and GPU particles cannot ([particles.md](particles.md), Two
  layers). The first are the fireball traps: `FireballEmitter` and `FireballEmitterRapid`
  (`furnishings.exem.json`), placed in the playground (`p002-p001`) and the necropolis
  (`holm-underground/p017-p013`). faery's `particles` aspect on them fired `particles:Fireball` and
  `particles:FireballTrapRapid` (`los-data/meta/particles.json`): a fireball that collides
  (`collisionMask`), lights within 4 m, sounds on spawn and on hitting, and bursts into PAN86's
  particles. faery timed a trap's shots by the effect's emission rate (`emissionRate` 0.5 with
  `maxParticles` 1, a shot every 2 s); here launching is the trap's own, not a particle setting: a
  fixed period on its aspect for now, scripted in PAN79. Its fire damage waits for actors to have
  health, in PAN6's combat.
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

- **PAN71 Imports**, after WESL's ([saga.md](saga.md), Modules). `ModuleResolver`, implemented on
  Bevy's `LoadContext`: check how Bevy main loads a dependency from within a loader. A module
  recompiles when one it imports reloads, and an import cycle is a compile error.
- **PAN72 SAGA language reference**, for mod authors, in `docs/saga-language.md`: syntax, types,
  literals and their suffixes, operators and casts, locals, `if`, functions, modules and imports,
  how a formula embeds in an asset (PAN45b), and the host functions and component reads a formula
  may use, each with a short example. It describes the language as it is, not [saga.md](saga.md)'s
  plans, and later chunks that change the language update it.

## Saves

- **PAN19 New game.** Resets every precinct, and the actors in it, to its authored state: despawn
  them and spawn them again from their assets and exemplars. Saves are sparse, holding only what
  play changed (PAN82), so loading a save is a new game with the save's changes applied. After
  PAN3l, which puts actors in precincts. R12 is the same reset for one actor.
- **PAN82 Save games** with structural sharing: one chunk of play changes per precinct, written on
  eviction; a save is only a root index of chunk versions. Design and what it asks of the engine in
  [save-games.md](save-games.md). X14's picked cells would live in the precinct chunk. The camera
  angle is saved too; a load follows the player with `Viewpoint::follow_snapped`, which keeps it.
  The first state to save: a chest left open.

## First playable

- **PAN6 Up in Arms.** The first playable: the "Up in Arms" quest, end to end. Its steps and
  what each needs are in [goals.md](goals.md).

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
