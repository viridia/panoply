# Roadmap

Work not done, as chunks. What has landed is in [changelog.md](changelog.md), why things are as
they are in [decisions.md](decisions.md), work put off until an event in
[deferred.md](deferred.md), and findings not yet routed in [issues.md](issues.md).

A chunk's id, `PAN<n>`, is permanent: taken from the counter below and never reused, and named in
its commit subjects. Code waiting for a chunk is marked `TODO(PAN<n>)`, or `TODO(X<n>)` for a
deferral. Uncompiled code kept as reference until a chunk replaces it is marked
`REFERENCE(PAN<n>)`. When a chunk lands, its entry here is deleted and a line added to the
changelog.

**Next: PAN19.**

## Next

In order.

PAN3, Actors, in lettered parts. Requirements, from faery's actor module, are in
[actor-requirements.md](actor-requirements.md) (draft, for the author to edit). Layers by time scale
(existence, goals, planning, body, presentation) talk through components, with readiness as
components and every layer restartable from saved state (R104).

- **PAN3a Load and place.** Restore the `actors` module and register its aspects, so
  `characters.exem.json` loads. One archetype (humanoid male), loaded through `ExemplarServer`
  (D27), not `AssetServer::load`; its root `Gltf` loaded, and only the body scene spawned from
  `named_scenes` (D28): one skeleton, no features or colors, in bind pose, at a fixed spot in the
  playground, without physics. The three.js version's assembly is
  `engine/src/world/actors/SkinnedModel.ts` (faery). Remove `TestHumanoid` from
  `spawn_material_test`. Plan: the actor standing there.
- **PAN3b Play a clip.** One idle clip from `humanoid-armature.glb` drives the body's armature,
  bound by `AnimationTargetId`, through one `AnimationPlayer` per actor. First, since binding across
  files is D28's riskiest assumption.
- **PAN3c Outlines.** The skinned outline looks right on the actor (D14); outline widths from the
  model (X32) if the actor needs them. `plans/outlines.py` looks at the actor instead of the test
  objects; remove the rest of `spawn_material_test`.
- **PAN3d Colors.** The `Colors` aspect replaces slot materials with clones cached by (material,
  color), per D28.
- **PAN3e Features.** Hair, hats and beards spawn as meshes only, each with a `SkinnedMesh` bound to
  the body's joints by `Name` (D28). Hair keeps the standard material until PAN3n.
- **PAN3f Collider and controller.** A capsule from the archetype's size (R4). First candidate (D36)
  is `rapier3d`'s `KinematicCharacterController`, which has slope limits, ground snapping and
  depenetration (stairs are ramps, so its autostep is not needed); test it against R24 to R26.
  Movement and physics go in `FixedUpdate`; rendered transforms (and the camera following them)
  interpolate between fixed ticks, or they judder at 120 fps against 64 Hz ticks. Check whether Bevy
  0.20 has transform interpolation built in before adding any. An actor gets physics only where the
  colliders under it are built: a minimal wait for now, X33's footprint query later. Plan: the actor
  dropped onto the ground lands and stays.
- **PAN3g WASD.** Movement relative to the view, turning gradually (R29); the camera follows the
  actor (R102). Reads the keys directly, which the driver's keys can press.
- **PAN3h Move to point.** A body action, not a goal: travel in a straight line to a point at a pace
  (R32, R43), reporting when it finishes (R45). Started from a plan or a debug hook.
- **PAN3i Blending.** Crossfades between clips (R34); gait from speed (R17, R18), keeping the
  stride's phase across a change (R19).
- **PAN3j Animation states.** The state follows the current action and the actor's state (idle,
  locomotion, falling, landing): R25, R33, R35, R38, R41.
- **PAN3k Interaction probe.** E casts from the actor along its facing and logs what it hits; the
  query filter excludes `CANOPY` (D39).
- **PAN3l Precinct actors.** Precincts carry actors (playground `p000-p000` has 13): restore the
  `actors` field in `PrecinctAsset` (now skipped), and spawn them from the actors module with a
  home-precinct relationship whose despawn is linked (D24), not from scenery. The player is one of
  them. The converters dropped actor fields and aspects (`scale`, `gender`, `enemy`, ...); see
  [faery.md](faery.md), Migration scripts, and decide per field: port, or replace. Remove
  `ExemplarCheck` from `main.rs` and `plans/exemplars.py`.
- **PAN3m Robustness.** In the three.js version actors fell through seams between parcel ground
  colliders. Ground here is a trimesh per parcel; parry's `FIX_INTERNAL_EDGES` flags likely cover
  only edges within one shape, not between colliders. Write a plan that walks an actor across parcel
  edges and corners. If it falls: make shared edge vertices bit-identical, overlap parcel colliders
  by a cell, enable CCD, and cast with a skin. Falling out of the world (R27). Stuck rescue, the
  backstop for traps the canopy (D39) does not cover (trees against walls, canopy bowls too steep to
  climb, clearings enclosed beyond its margin): an actor that is not grounded, or makes no progress
  against its intent, for some seconds moves to the nearest free spot; before navigation, the
  nearest cell outside the canopy cover. Tune the canopy and trunk height (`CANOPY_HEIGHT`, D39).
- **PAN3n Hair shading.** Materials with extras `material: hair` get strand tangents and a hair
  shader (D29).
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
- **PAN18 Player input on `bevy_action_map`.** Moves PAN3g's and PAN3k's direct key reads onto
  actions.
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
