# Roadmap

Work not done, as chunks. What has landed is in [changelog.md](changelog.md), why things are as
they are in [decisions.md](decisions.md), work put off until an event in
[deferred.md](deferred.md), and findings not yet routed in [issues.md](issues.md).

A chunk's id, `PAN<n>`, is permanent: taken from the counter below and never reused, and named in
its commit subjects. Code waiting for a chunk is marked `TODO(PAN<n>)`, or `TODO(X<n>)` for a
deferral. Uncompiled code kept as reference until a chunk replaces it is marked
`REFERENCE(PAN<n>)`. When a chunk lands, its entry here is deleted and a line added to the
changelog.

**Next: PAN91.**

Chunks are grouped by thread, a day's area of concern. Within a thread they are in order, foundation
first; a chunk that needs another thread's says so.

## Terrain

PAN38, Terrain mode's panel, in lettered parts: its layout, then its tools. PAN39 and PAN40 fill the
rest of it. Ported from the Quill editor (`REFERENCE(PAN5)`, `ui/mode_terrain.rs`,
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

- **PAN74 Floors as exemplars** (D79). The floor types (`scenery/floors/*.floor.ron`,
  `FloorSurface`) move into a catalog as exemplars of a floor type, their `material` enum an aspect.
  Precincts' `floor_surfaces` name them as exemplars, rewritten, and `asset_xref.py` checks them as
  it checks other exemplars. The floor tool lists them through the exemplar list
  (`exemplar_list.rs`, `TODO(PAN74)`), and the property editor edits them. Check what a precinct
  naming one floor loads: the catalog's every texture.
- **PAN76a Cutaways.** When the player is under a ceiling, scenery above them is hidden, including
  actors on those floors (R15). Rects are authored per tier (`TierSer.cutaways`); choosing them and
  the 3D test against element and actor bounds follow the three.js version ([faery.md](faery.md),
  Scenery). Tier entities (D24) carry their rects; elements need bounds. Hidden elements use
  `Visibility::Hidden` so descendants inherit it (D19). Once interiors show, check outlines on
  glass: through a transparent mesh, its inverted hull's far side shows, not just the rim. faery
  drew a `glass` material's back faces first, depth only (`ModelCatalog.ts`, `glass`), so the hull
  behind the glass failed the depth test; Panoply ignores the `glass` extra (the `Alembic` in
  `decorations.glb` has it; `GlassTest`'s `black-glass` is another variant).
- **PAN77 Scenery rect selection.** The RectSelect tool, whose button does nothing yet: drag out a
  rect on the current tier of the precinct under the pointer, snapped to the wall snap size, drawn
  as an overlay and kept as the scenery selection with its precinct. As faery's
  `editor/src/tools/SelectSceneryRectTool.ts`, where a drag that began on the same precinct grew the
  rect to include it.
- **PAN88 Instance picking.** A click in scenery mode selects the wall, fixture or actor under the
  pointer, as faery's `SelectSceneryInstanceMode` did, kept as the scenery selection; PAN28e's
  instance properties are its first customer. Small or invisible things are picked through their
  `hint` and `marker` colliders (about 15 of each in `furnishings.exem.json` and
  `wall-tiles.exem.json`), which must stay out of the game's own collisions and queries.
- **PAN76b Cutaway editing**, on PAN77's selection. Adding, removing and resizing a tier's cutaway
  rects, each edit undoable and marking the precinct unsaved, and a Show menu item drawing them, as
  faery's Cutaway Rects did ([faery.md](faery.md), Editor). How faery's `SelectSceneryRectTool` made
  a rect into a cutaway is read in the chunk.

## Interaction

- **PAN59 Door.** A `Door` aspect on the cottage door (`CottageWallDoorway`), played by its
  `OpenDoor` clip through `SceneryAnimations` and offered through `Interactions`. Its collider is
  built and follows the animated glTF node its descriptor's `animation` names, as faery's
  `PhysicsMovable` did (a kinematic body moved by the model's own animation, with no separate
  animation data), so the leaf is solid and within the probe's reach through the whole swing. Closes
  I26.
- **PAN78 Signs and books**: things to read. A sign is a scenery aspect whose interaction shows its
  text; a document is an inventory item that links to text content and a page style (faery's book
  aspect opened its content in a frame, `codex` by default). The second aspect after PAN21's first,
  since its action needs only a reading view.
- **PAN79 Mechanisms**: the stock elements puzzles are built from, and how they are wired together,
  on SAGA's cells and reactions (X7). faery had doors, push and toggle buttons, pressure plates,
  remotely controlled openables, automatic doors, spear traps and fireball traps (whose fixed
  period, PAN87, becomes scripted: when a trap fires), wired by reactive formulas so that elaborate
  puzzles combine simple parts ([faery.md](faery.md), How far faery got; the aspects the converter
  dropped are under Migration scripts). The first sensor collider is the `PressurePlate` wall tile's
  (`wall-tiles.exem.json`). Beside them: triggers (a circle or rectangle the player enters),
  encounter zones, and waymarks for scripted NPC events. To decide: how one element signals another
  (an entity event, as PAN21's actions are, or a formula over the other's state), and which parts
  are native aspects and which are SAGA. The hard part is referencing and naming. A precinct is
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

## Editor

PAN28, Property editor, in lettered parts. A fork of arbor_day's `properties.rs` into
`panoply_editor`, so each develops for its own needs (sharing later is X48); its attributes replaced
by `bevy::annotations` where they overlap, as arbor_day's X25 plans for its own. faery's editor is
in [faery.md](faery.md), Editor, its screenshots included; bevy_reactor_3's `Inspectable`
(`crates/bevy_reactor_prop_inspect/src/inspectable.rs`) is a model for roots and field handles.
Widgets ask for edits rather than writing values (D72); each part adds its operations to the closed
set an edit is one of (`EditKind`).

- **PAN28i Maps.** A map is edited as a list of key–value pairs, as faery edited it, using the list
  rows from PAN28f. Each entry is a row with a text input for its key and its value's widget beside
  it, plus a × that removes the entry. The heading's + adds an entry with an empty key and the value
  type's default. Changing a key is a new `EditKind`: it moves the entry and undoes in one step. A
  key that is empty or already in the map isn't committed. `Colors` and `Features` get custom
  editors later, as faery's `ColorMap` and `FeatureSelection` were (from `ColorSlots` and
  `FeatureSlots`).
- **PAN28j Editor polish.** A `Precision` annotation, as arbor_day's (`properties.rs`): the decimal
  places a number input rounds to while dragging, given to feathers as `NumberInputPrecision`, in a
  crate the aspect crates see; `ActorScale` gets one, since dragging it shows nine places. List
  items captioned "#1", "#2" rather than "Item 1" (`item_caption`, and the plans' selectors). The
  aspects chooser opens from a button of its own on the exemplar pane's header, as Blender's Add
  Modifier is, rather than from the end of the fields menu; its icon still to choose (a puzzle
  piece, say; `layers.png` is the Layers tool's).
- **PAN28k Features and colors.** `Features` and `Colors` edited as faery's `FeatureSelection` and
  `ColorMap` were: a checkbox per slot of the actor's `FeatureSlots`, a color per slot of its
  `ColorSlots`, each resolved through `extends`, never an arbitrary key. A key outside the slots,
  left by a change of prototype, stays in the file and is skipped: the editor does not show it, and
  spawning an actor leaves it out without an error, so switching back restores it. A feature borrows
  its colors from the body, so none are lost. The mismatch a change of `Skin` alone leaves, slots
  naming scenes the skin's file lacks (SergeantStone's male facial hair on a female skin), is the
  validator's to report (`asset_xref.py` today).
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
- **PAN5 Editor**, beyond PAN1's shell. `src/editor`, written for Quill, is reference
  (`REFERENCE(PAN5)`): delete each part as `panoply_editor` replaces it. The design is faery's
  editor (`packages/editor`, `design/Editor.md`; [faery.md](faery.md), Editor): read it before
  designing a mode or tool. Two things the shell lacks: a resource in `panoply_core` saying whether
  the game or the editor handles world clicks, so the game's (PAN17) run only while the game has the
  pointer, where the mode decides, not the panel (play hands the pointer back, hiding the panel
  keeps the tool); and the editor's selections saved in `bevy_settings` with its other state, so a
  restart returns to where work stopped. Scenery mode's overlays (faery's grid, floor cutaway,
  colliders, marks and portals) are `panoply_overlays` shapes in `mode_overlays.rs`. The author has
  screenshots of faery's editor. Its tools, modal (one per mode or submode button) or situational
  (the waypoint tool, opened from a field), and its property editor's field widgets are surveyed in
  [faery.md](faery.md), Editor; port each with the mode or field that needs it, the widgets chosen
  by field type (D45). faery's other debug views (listed in [faery.md](faery.md), Editor) join the
  Show menu with the features they show, not before. Among them, faery's parcel and precinct
  readiness grid, drawn from the survey's data (`TerrainPending::survey`, `SceneryPending::survey`,
  PAN10) when tuning load distances needs it on screen.

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
  to them. Its radius leaves melee in reach. An actor inside another's space wakes (PAN3p).
  Neighbours by testing every pair in the realm until a grid is needed. The playground's townsfolk
  are the test. X50 builds NPCs' steering on it.
- **PAN89 Actor colliders**, so a ray can hit an actor: PAN88's picking, the probe (PAN21) and
  PAN17's clicks. Today an actor's capsule is only a query shape that `move_actors` sweeps. A
  collider in the realm's rapier world, in a new `ACTOR` collision group that the character
  controller's filter leaves out, so actors never collide through rapier (R86). Only an actor that
  moved updates its collider, so resting actors (PAN3p) cost nothing; check it in `plans/perf.py`.
  Check whether a collider without a body must be a sensor to stay out of contact generation. faery
  also had an inflated capsule for picking ([faery.md](faery.md), Physics); one capsule serves both
  until picking feels fiddly.
- **PAN30 Aspect hot reload.** An edited exemplar's aspects reach the actors already spawned from
  it; today only a changed precinct respawns its actors (`precinct_actors.rs`). Checked in
  `plans/actors.py`: edit an exemplar file, wait for the change on its actor, and restore the file.
- **PAN16 Navigation.** Panoply has none; faery's is in `engine/src/nav` (see [faery.md](faery.md)).
  First evaluate [rerecast](https://crates.io/crates/rerecast): navigation mesh generation is hard
  to maintain alone, which outweighs the cost of a fork (D36); check whether its generator works
  without Bevy, which would need none. Test it against Panoply's idiosyncratic needs: a mesh only
  around loaded precincts, built as they stream in (R83); door regions (R98); exclusion zones cut
  where actors stand (R86); and whatever else faery's `nav` does that rerecast lacks. A missing
  feature may be upstreamed rather than forked: the author has talked with rerecast's author. Crowd
  and follower behavior (R86, R87) needs test scenarios: driver plans that stage, for example, eight
  skeletons rushing the player, or two followers through a doorway. An actor that cannot advance
  plays R33's blocked state and replans (R22).
- **PAN17 Click to move.** Clicking an actor or fixture needs clicks in the world
  (`bevy_remote_driver`'s X9). Clicking ground to walk to is Panoply's own: a server method of its
  own (the driver's X2), or a waypoint entity spawned there and clicked through the driver's X9. The
  game's click handling runs only while the game has the pointer (PAN5). A click that misses every
  interactive thing lands on `WorldBackstop` (D61), and a ground click needs the ground point, which
  it does not report.

## Portals

PAN4, Portals, in lettered parts: a rewrite, not a restoration (D21). `src/portals` is reference
(`REFERENCE(PAN4)`), deleted when PAN4c lands.

- **PAN4a Portal aspects.** The old code's `Portal` (the aperture, on a wall or fixture exemplar)
  and `PortalTarget` (per instance: the far end's realm and place), loaded from precincts, and
  gizmos showing each aperture. Portals go in a crate, since the editor places them (D37, D40):
  first candidate, `panoply_portals`, over scenery and actors, which crossing needs (PAN4c).
- **PAN4b Portal view.** What the aperture shows: the far end, rendered by a camera of its own (D21:
  off-axis projection fitted to the aperture, its own load region). Bevy's `examples/3d/mirror.rs`
  (a second camera on a render target) and `examples/camera/custom_projection.rs` (an oblique clip
  plane) are the starting point, replacing the old `active_portal.rs`.
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
  Modifiers on (D55), by a means this chunk settles. In the same pass, each character file marks its
  body scenes apart from its features with a scene extra, so the model picker can leave features out
  of a `Skin`'s list (D77's reversal).
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

## Sweeps

Passes over a whole document or module, a unit at a time, each unit small enough for one session.

- **PAN15 Design out of decisions.** Many decisions carry mechanism (how it works now) beside
  their reason, D10, D35 and D38 most. Move the mechanism into a new `docs/design.md`, in
  numbered sections (`TD<n>`), leaving each decision its choice, its reason and what reversing it
  would break. One area per unit (physics, rendering, loading, plans, ...), taken when a chunk
  touches that area.
