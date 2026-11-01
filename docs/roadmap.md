# Roadmap

Work not done, as chunks. What has landed is in [changelog.md](changelog.md), why things are as
they are in [decisions.md](decisions.md), work put off until an event in
[deferred.md](deferred.md), and findings not yet routed in [issues.md](issues.md).

A chunk's id, `PAN<n>`, is permanent: taken from the counter below and never reused, and named in
its commit subjects. Code waiting for a chunk is marked `TODO(PAN<n>)`, or `TODO(X<n>)` for a
deferral. Uncompiled code kept as reference until a chunk replaces it is marked
`REFERENCE(PAN<n>)`. When a chunk lands, its entry here is deleted and a line added to the
changelog.

**Next: PAN16.**

## Next

In order.

- **PAN3 Actors.** First slice: the player's actor loads from a precinct, gets its model,
  stands on the ground with the controller, walks by WASD (R29), and plays clips chosen from
  its velocity; no goals, pathfinder or click to move yet. The slice may read the keys
  directly; a later slice moves player movement onto `bevy_action_map`. Later slices layer on
  by time scale (existence, goals, planning, body, presentation), each talking through
  components, with readiness as components and every layer restartable from saved state
  (R104). Risk: in the three.js version actors fell through seams between parcel ground
  colliders. Ground here is a trimesh per parcel; parry's `FIX_INTERNAL_EDGES` flags likely
  cover only edges within one shape, not between colliders. Write a plan that walks an actor
  across parcel edges and corners. If it falls: make shared edge vertices bit-identical,
  overlap parcel colliders by a cell, enable CCD, and cast with a skin. Actors also need
  navigation, which Panoply lacks; faery's is in `engine/src/nav` (see [faery.md](faery.md)).
  Character controller: first candidate (D36) is `rapier3d`'s `KinematicCharacterController`,
  which has slope limits, ground snapping and depenetration (stairs are ramps, so its autostep
  is not needed); test it against [actor-requirements.md](actor-requirements.md) (R24 to R28).
  Plans for the player's controls: keyboard play (WASD or arrows to move, E to interact with
  what the player faces) works with the driver's keys today; clicking an actor or fixture needs
  clicks in the world (`bevy_remote_driver`'s X9). Clicking ground to walk to is Panoply's own:
  a server method of its own (the driver's X2), or a waypoint entity spawned there and clicked
  through X9. The game's click handling runs only while the game has the pointer (X3, editor
  tools). An actor gets physics only where the colliders under it are built (X33). Stuck
  rescue, the backstop for traps the canopy (D39) does not cover (trees against walls, canopy
  bowls too steep to climb, clearings enclosed beyond its margin): an actor that is not grounded, or makes no progress against its
  intent, for some seconds moves to the nearest free spot; before navigation, the nearest cell
  outside the canopy cover. Tune the canopy and trunk height (`CANOPY_HEIGHT`, D39) once the
  controller is in hand.
  Precincts carry actors (playground `p000-p000` has 13): restore the `actors` field in
  `PrecinctAsset` (now skipped, `TODO(PAN3)`), and spawn them from the actors module with
  a home-precinct relationship whose despawn is linked (D24), not from scenery. Actor
  movement, AI and physics go in `FixedUpdate`; their rendered transforms (and the camera
  following them) must interpolate between fixed ticks, or they judder at 120 fps against
  64 Hz ticks. Check whether Bevy 0.20 has transform interpolation built in before adding any.
  Load actor exemplars through `ExemplarServer` (D27), not `AssetServer::load`. Models per
  D28: one skeleton per actor, features bound to its joints, colors by cached material clones.
  Hair (materials with extras `material: hair`) needs strand tangents and a hair shader (D29).
  The three.js version's actor assembly is `engine/src/world/actors/SkinnedModel.ts` (faery).
  The converters dropped actor fields and aspects (`scale`, `gender`, `enemy`, ...); see
  [faery.md](faery.md), Migration scripts, and decide per field: port, or replace.
  Requirements, from faery's actor module: [actor-requirements.md](actor-requirements.md)
  (draft, for the author to edit). Crowd and follower behavior (R86, R87) needs test scenarios:
  driver plans that stage, for example, eight skeletons rushing the player, or two followers
  through a doorway. Once actors are back, remove the `TEMP(PAN3)` items from `main.rs`
  (`spawn_material_test`, `ExemplarCheck`) and `plans/exemplars.py`.
- **PAN13 Ledger tooling.** Adapt `../ldd_template`'s `xref.py`, `show.py` and `devfmt.py` to
  Panoply: the `PAN` prefix, these ledgers, and the requirements in
  [actor-requirements.md](actor-requirements.md). Add a check that each changelog entry is one
  line under the cap its header states, and a `verify.sh` that runs the checks and `cargo
  clippy`.
- **PAN14 Guidance out of memory.** The agent's memory holds project guidance that belongs in
  git: plan-writing advice (state set through the driver, waits by latency class) goes to
  `../bevy_remote_driver/docs/writing-plans.md`; workflow (one directory run of every plan at a
  chunk's end, absolute plan paths, one driver run at a time, fewer launches) to `AGENTS.md`.
- **PAN4 Portals**: a rewrite, not a restoration (D21). `src/portals` is reference
  (`REFERENCE(PAN4)`), deleted once portals work.
- **PAN5 Editor**, beyond PAN1's shell (see X3). `src/editor`, written for Quill, is reference
  (`REFERENCE(PAN5)`): delete each part as `panoply_editor` replaces it. The author has
  screenshots of faery's editor. faery's other debug views (listed in
  [faery.md](faery.md), Editor) join the Show menu with the features they show, not before.
  Among them, faery's parcel and precinct readiness grid, drawn from the survey's data
  (`TerrainPending::survey`, `SceneryPending::survey`, PAN10) when tuning load distances needs
  it on screen.
- **PAN8 Prune the plans.** From a full run's times (the driver reports each plan's), ask of
  each plan what it checks that no other does, and whether that is worth its time and the focus
  it takes. Candidates: `tonemapping.py`, whose question D30 settled; `shadows.py` and
  `outlines.py`, stills that could be shots in `lighting.py`. Measurements stay, tagged `manual`.
- **PAN6 Up in Arms.** The first playable: the "Up in Arms" quest, end to end. Its steps and
  what each needs are in [goals.md](goals.md).

## Sweeps

Passes over a whole document or module, a unit at a time, each unit small enough for one session.

- **PAN15 Design out of decisions.** Many decisions carry mechanism (how it works now) beside
  their reason, D10, D35 and D38 most. Move the mechanism into a new `docs/design.md`, in
  numbered sections (`TD<n>`), leaving each decision its choice, its reason and what reversing it
  would break. One area per unit (physics, rendering, loading, plans, ...), taken when a chunk
  touches that area.
