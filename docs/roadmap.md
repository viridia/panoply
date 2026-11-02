# Roadmap

Work not done, as chunks. What has landed is in [changelog.md](changelog.md), why things are as
they are in [decisions.md](decisions.md), work put off until an event in
[deferred.md](deferred.md), and findings not yet routed in [issues.md](issues.md).

A chunk's id, `PAN<n>`, is permanent: taken from the counter below and never reused, and named in
its commit subjects. Code waiting for a chunk is marked `TODO(PAN<n>)`, or `TODO(X<n>)` for a
deferral. Uncompiled code kept as reference until a chunk replaces it is marked
`REFERENCE(PAN<n>)`. When a chunk lands, its entry here is deleted and a line added to the
changelog.

**Next: PAN30.**

## Next

In order.

- **PAN8 Prune the plans.** From a full run's times (the driver reports each plan's), ask of each
  plan what it checks that no other does, and whether that is worth its time and the focus it takes.
  Candidates: `tonemapping.py`, whose question D30 settled; `shadows.py`, a still that could be a
  shot in `lighting.py`. Measurements stay, tagged `manual`. The author, who sees the window but not
  the log, cannot tell what a plan is doing: a `caption` step in `_panoply_driver.py` sets the
  window's title, and each section of a plan opens with one saying what it checks ("Ray stops on the
  table's collider"). A section hard to caption is a candidate to cut.
- **PAN14 Guidance out of memory.** The agent's memory holds project guidance that belongs in git:
  plan-writing advice (state set through the driver, waits by latency class) goes to
  `../bevy_remote_driver/docs/writing-plans.md`; workflow (one directory run of every plan at a
  chunk's end, absolute plan paths, one driver run at a time, fewer launches) to `AGENTS.md`.

PAN3, Actors, in lettered parts. Requirements, from faery's actor module, are in
[actor-requirements.md](actor-requirements.md) (draft, for the author to edit). Layers by time scale
(existence, goals, planning, body, presentation) talk through components, with readiness as
components and every layer restartable from saved state (R104).

- **PAN3k Interaction probe.** E casts from the actor along its facing and logs what it hits; the
  query filter excludes `CANOPY` (D39).
- **PAN3m Robustness.** In the three.js version actors fell through seams between parcel ground
  colliders. Ground here is a trimesh per parcel; parry's `FIX_INTERNAL_EDGES` flags likely cover
  only edges within one shape, not between colliders. Write a plan that walks an actor across parcel
  edges and corners. If it falls: make shared edge vertices bit-identical, overlap parcel colliders
  by a cell, enable CCD, and cast with a skin. Falling out of the world (R27): a deliberate hole in
  the playground's terrain drops a walking actor through today. Stuck rescue, the backstop for traps
  the canopy (D39) does not cover (trees against walls, canopy bowls too steep to climb, clearings
  enclosed beyond its margin): an actor that is not grounded, or makes no progress against its
  intent, for some seconds moves to the nearest free spot; before navigation, the nearest cell
  outside the canopy cover. Tune the canopy and trunk height (`CANOPY_HEIGHT`, D39).
- **PAN25 Nav points.** Named places to travel to: `WorldLocation` and its loader, from
  `src/world/locations.rs`, moved as `NavPoint` to `panoply_core` beside `GotoLocation`, loading
  `assets/world.locations.json` (51 points). Two roles: the editor travels to any of them, and those
  with an icon mark the in-game map (later). faery's `INavPoint` also had `viewAngle` and `center`
  (the icon's map position); add them when travel or the map needs them.
- **PAN26 Go To dialog.** G opens a feathers dialog: a text input filtering the nav points sorted by
  name, Cancel and Go. Click selects, double-click or Go travels by `GotoLocation`, Esc cancels. The
  filter, used across the editor later, ports faery's `createSearchFilter` (`editor/src/controls/`,
  with its tests): spaces AND terms, commas OR them; a term matches as a case-insensitive substring,
  or letter by letter against word starts, each letter starting a word or continuing the word just
  matched (`bb` matches "BasicBlock", `stvi` "stick-village"). Words start at a capital, `-`, `_` or
  space. A matcher of its own, not faery's unanchored regex, which let a typed `.` or `+` change the
  pattern. The list widget is written for PAN29 to reuse.
- **PAN22 Game modes.** Which mode the app is in (play, an editor mode; later menus, dialogue, cut
  scenes) as Bevy states, with each mode's input in its own `bevy_action_map` context, active only
  in that mode. Today the free camera's context and PAN3g's direct key reads are both always on, so
  the arrows and WASD each drive one thing in every mode. First see X3, which settles how editor
  modes take the pointer and keys. Go To (PAN26) in play moves the player, and the camera follows as
  usual; in an editor mode it moves the camera, as before.
- **PAN18 Player input on `bevy_action_map`.** Moves PAN3g's and PAN3k's direct key reads onto
  actions, in PAN22's play context.
- **PAN27 Mode selector.** A control at the top of the editor panel choosing the editor mode (realm,
  terrain, scenery, catalogs, play; X3), saved in settings. Before designing: the author's
  screenshots of faery's mode buttons, and the Bevy editor mocks' toolbar. After PAN22, whose states
  it switches.
- **PAN28 Property editor.** A fork of arbor_day's `properties.rs` into `panoply_editor`, so each
  develops for its own needs (sharing later is X48); its attributes replaced by `bevy::annotations`
  where they overlap, as arbor_day's X25 plans for its own. Beyond arbor_day: nested structs and
  arrays of structs, and optional fields as faery's aspects had them: a struct is a map whose keys
  may be absent; a `None` field has no row, a `Some` row has an × button that removes it, and a
  boxed-plus menu in the panel header lists the absent fields to add. A field's annotations apply
  through `Option` and `Vec` to their contents, since the `f32` in `Option<f32>` cannot be
  annotated; Bevy's inspector does this, in a commit after Panoply's locked Bevy, so this chunk may
  bump Bevy. Every edit is undoable (D49): setting a value at an asset key and property path, adding
  or removing an optional field, inserting or removing an array item; a drag's steps coalesce into
  one entry. The undo stack is a module of `panoply_editor`, from the author's unmerged bevy#23373,
  its contexts keyed by asset rather than `Entity`.
- **PAN29 Realm mode panel.** PAN26's list in the panel with Go, + (names the current view as a new
  nav point) and − (removes the selected one), and the selected point's fields below in PAN28's
  editor; faery's `MapEditPanel`, `NavPointSelector` and `NavPointEditor`. Saving is explicit, as in
  faery: a save button beside PAN27's mode selector, disabled while nothing is unsaved, writes
  `world.locations.json`.
- **PAN21 Interactions.** What things offer to be done to them, as faery's aspects did
  (`packages/game/src/aspects/`, after The Sims): each entry has an id, caption, cursor, icon,
  distance, a `when` and an action. Entries belong to aspect types, registered in code; assets only
  attach and configure aspects, as faery's did. Actions are entity events triggered on the target,
  so goals can trigger them too (R71, X17). `when` is evaluated on demand, when the probe or a goal
  asks, not reactively. Variants share an id (a container's "Open" and "Empty"). `when` and the
  action are each native code or, after X7, a formula from the asset; only native for now. E
  performs the one enabled interaction on what PAN3k's probe hits, with its caption as an on-screen
  hint; a choice among several is X43. One aspect end to end, the one whose action needs least else
  (a door changes its collider and model; a container needs an inventory dialog). After PAN18, so E
  is an action.
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
  editing, a Blender Python script, run once per `.blend`: it makes a geometry-node inverted hull
  (offset along normals, flip faces, an outline material with backface culling, joined to the mesh)
  and adds it after the Armature modifier on every mesh, its width driven by the object's `outline`
  property. Exports must leave the hull out: check whether the glTF exporter skips a modifier that
  is off for render, else export with modifiers unapplied.
- **PAN24 Clip repairs.** With PAN23, as one Blender session. `Punch` keys `upper_arm.R` rolled
  about 140° from its roll in `Stand` (and 70-90° from the mirrored left arm), wringing the
  shoulder; the keys hold no quaternion sign flips, so the poses themselves are wrong. A Blender
  Python script rolls the bone back about its own axis on every key, by an angle judged in catwalk,
  and counter-rotates `forearm.R`'s keys so the forearm and hand stay where they are. A clip audit
  in `scripts/`, as `outlines.py`, lists clips whose bones' roll strays far from their mirror or
  from `Stand`. The broken female clips are I6. A trial for X45: retarget one CC0 combat clip onto
  `HumanoidArmature` and play it beside the hand-keyed ones, to measure the cost per clip and
  whether the styles mix.
- **PAN20 Outline width by distance.** An experiment: flatten the curve by which outlines thin with
  distance, without making them constant. `outline.wesl` extrudes in mesh-local units, so on screen
  the width falls as 1 / depth. Scale the extrusion by `(depth / reference)^k`: `k = 0` is today,
  `k = 1` a constant screen width; try values between. Depth to the mesh's origin rather than each
  vertex, so a long model's outline does not taper along it. `k` and the reference as one setting
  for every outline material, tuned by eye in the editor at several zooms; in play the camera
  distance is fixed (20), so there it only evens out the near and far rows of the view.
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
  game's click handling runs only while the game has the pointer (X3, editor tools).
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
