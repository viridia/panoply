# Deferred

Work decided against for now, each with its gate: the event that reopens it. When a gate fires,
the entry becomes a chunk in [roadmap.md](roadmap.md) and leaves here. An entry with no event is
dropped, not deferred. Numbers are never reused.

**Next: X68.**

- **X1** Migrate exemplars to BSN. *Gate:* BSN assets released, and BSN supports shared
  (non-instanced) templates. Includes updating the exemplar migration script, which lives in the
  `faery` repo (the earlier TypeScript/three.js engine), to emit BSN. As of 2026-11, BSN assets wait
  on an overhaul of Bevy's reflection. Consuming them, with `extends` as scene inheritance, looks
  straightforward; editing is the hard part. Bevy's scene editor will edit an AST of the scene file
  and serialize it to BSN, not the spawned entities, which lose imports, type names and loading
  directives. Open: use that AST in Panoply's editor, or keep exemplars as an editor-only AST of our
  own that the editor compiles to BSN.
- **X2** Synthesized shared assets (successor to D6). *Gate:* a restored module needs them.
  Consider deterministic `AssetId`s from hashed params instead of a fake asset source.
- **X3** Editor restoration and its UI. Widgets: `bevy_feathers` (on `bevy_ui_widgets`).
  Update 2026-10-11: going with `bevy_immediate` on the fork (`viridia/bevy_immediate`, branch
  `bevy-main`: only its Bevy-main fixes, rebased on upstream, offered upstream, dropped at
  0.21), tried first in the
  `arbor_day` shell (X28) rather than on X26's sliders. Earlier notes: reactivity was undecided: Quill is dead; `bevy_reactor` (the author's; last on 0.17 and
  Cart's `next-gen-scenes` branch, so a fork means porting off a BSN prototype) is
  experimental; `bevy_immediate` (0.9 targets Bevy 0.20, integrates feathers and
  `bevy_scene`) gives widgets an immediate-mode API. Recommended: `bevy_immediate` for the
  editor (dev-only forms, inspectors, palettes), tried first as a spike on X26's water
  sliders; reactivity for game UI decided separately. Much of the editor's design can be
  taken from faery's editor (`packages/editor`; see [faery.md](faery.md), Editor, and
  `design/Editor.md`): read it before designing modes and tools. Design, from the author:
  - The editor is its own crate, `panoply_editor` (D37). Its shell landed in PAN1: the panel,
    its splitter and key, and the viewport following the view pane. The old editor,
    `src/editor` (disabled), moves into the crate mode by mode.
  - Modes: realm, terrain, scenery, catalogs, play (icons in the old repo).
  - All editor state is saved in `bevy_settings` (mode, selections, panel width, view), so a
    restart returns to where work stopped; the edit-run loop restarts often. Plans start
    from empty settings already (D15), so runs do not depend on the previous run.
  - Editor and game keys use separate `bevy_action_map` contexts; picking keeps panel clicks
    from the game.
  - The editor takes over the pointer in the world. A resource in `panoply_core` says whether
    the game or the editor handles world clicks; the game's click handling (PAN3) runs only
    while the game has it. Each editor tool (terrain sculpting, floor editing, scenery
    placement) is a mode that observes the pointer events it needs, on world meshes or on
    virtual planes aligned with the world axes, through a picking backend of the editor's own
    (the old `PlanePickBackend`, `src/editor/lib`), beside mesh picking. Each tool has its own
    action context, and the game's are off while a tool has the pointer. The mode decides, not
    the panel: Play mode hands the pointer back to the game, and hiding the panel keeps the
    tool.
  Also replace `bevy_mod_picking` with `bevy_picking`, `@PreferencesGroup` with
  `SettingsGroup`. *Gate:* editor restoration.
- **X5** `bevy-inspector-egui` (pinned bevy 0.15). Bevy main's `bevy_inspector` crate may
  replace it. *Gate:* a compatible release is needed.
- **X6** Revisit asset bundling. The GLTF layout carries three.js-era assumptions, made to cut
  HTTP requests: small models (e.g. flora) are combined into one file, and large Blender
  catalogs are never split. Under Bevy, per-model files and labeled sub-assets may suit better.
  *Gate:* restoring terrain/flora/scenery, or a change to the asset pipeline (see D7).
- **X7** Scripting and modding in SAGA ("Scripts for Animated Graphic Adventures"), designed in
  [saga.md](saga.md) and [modding.md](modding.md), beyond PAN45's first formula: value slots, then
  cells and reactions for X63's button and door. *Gate:* PAN45.
- **X10** Reflections for shiny objects: `LightProbe` volumes with `EnvironmentMapLight` (using
  `skybox_diffuse_32.ktx2` and `skybox_specular.ktx2`) where shiny objects are, instead of on the
  camera. The sky itself is never in view; the skybox is only there to light. Without an environment
  map, mirrored surfaces look flat, and environment maps have proved expensive, so this is an
  experiment in cost: measure each probe layout with `plans/perf.py`. *Gate:* scenery or actors with
  shiny materials restored.
- **X11** Water, continued in small steps, each checked by eye in motion (`plans/lighting.py`,
  `bluff` and `shore`). D31 landed colors, alpha, the seabed and a little more sky. The sky
  weight is still well below three.js's (`mix(water, sky * 0.5, opacity * 0.5)`, about
  0.1-0.3; three.js had `mix(water, sky, opacity)`, fully sky at grazing angles); tune further
  by eye. Compared with the three.js version, the water lacks its large, soft, pale sky
  reflections over deep indigo (most of what made it read as water), is teal-gray rather than
  blue (its blue came from the sky term), and has narrower, dimmer foam without the ragged
  breakup at the seaward edge; our sun glint is new (three.js water was unlit) and large. Use
  `plans/lighting.py`'s `ocean` spot (189, 62, open water only). The sky term is the blocker:
  `water-clouds.png` is nearly binary, and the lookup follows the rippled reflection, which
  jumps between neighboring far pixels (no mips), so where the sky weight is high (grazing
  angles, far water) it reads as hard-edged blotches; near water it is weighted almost to
  zero, so it looks flat. Confirmed by turning `ENABLE_SKY` off (blotches gone). Neither a
  lower lookup frequency (`* 0.1`) nor clamping `reflect_vector /= y` toward the horizon
  helped. Next try a soft (blurred) cloud texture, then raise the sky weight; then foam; then
  the glint. Shaders now hot-reload in editor builds (D32). Optionally tie the sky colors to `HemisphereLight` so water dims at
  night and in interiors (X12). Blur the seabed with depth (`textureSampleBias`), now that
  textures have mips (D33). Water is always at sea level (the ground's underwater tint depends on it),
  so reflections of scenery could use one mirrored camera (planar reflection), if their cost
  fits the budget; Bevy's SSR needs deferred rendering and does not reach blended surfaces
  (see [notes.md](notes.md)). Texture noise in place of `snoise` was a deliberate performance
  change; keep it. Findings from a reverted attempt: restoring the old blend at full sky weight made the
  cloud texture read as fractal discoloration; replacing the clouds with a smooth elevation
  gradient lost the look of fluid. The ripple normal map loads as sRGB, offset by an
  empirical `- 0.25`; decoding it properly changes the ripples' strength and bias.
  *Gate:* author's direction.
- **X12** Art direction for lighting: painterly (pre-Raphaelite, art nouveau). D30 restored the
  three.js balance (no tone mapping, ambient over sun); tune `Sunlight` and `HemisphereLight`
  by eye with `plans/lighting.py` (`SUN=`, `AMBIENT=`, `SHOTS=`), then repaint textures in
  Krita where they still don't fit (grass and cobbles may be too high-frequency). The editor's
  terrain thumbnail (`terrain_thumbnail.rs`) still has its own 3000 lux light and default tone
  mapping. Underground realms a little more gothic, still
  painterly (`RealmLighting::Interior`). Day/night cycle: sun shadows fade out in the evening
  before they grow long; the moon casts shadows only at night. Night uses the cinematic blue
  filter: red and green illumination reduced, blue brighter (artistic, not realistic).
  Outlines read a little thinner than in the old engine, and the dark shadows make them hard
  to see; compare the old engine's extrusion (units, any distance scaling) with
  `outline.wesl` when tuning. Some models still put `outline` on objects instead of materials,
  which the game ignores; the author is moving it in Blender.
  *Gate:* scenery restored, for whole scenes to judge.
- **X13** Sweep the old JavaScript/three.js engine (`../faery`) for design elements worth
  carrying over. It was far more complete than this version; it was abandoned for jankiness,
  not design. About 300k lines (TypeScript, GLSL) in 14 packages (`game`, `los-game`,
  `los-editor`, `genmap-cli`, `server`, `game-ui`, ...) plus `design/` docs. Needs its own
  session or several; produce a written catalog, not code. *Gate:* PAN3 landed, the last
  restoration. A first-pass map, with pointers per roadmap chunk, is [faery.md](faery.md); read the relevant
  section before restoring a module, and add to it rather than rescanning.
- **X14** Herb picking: picking a herb hides that instance for a while, then it regrows. Pick by
  grid cell (at most one herb per 1 m cell; a ray hit gives the cell), not by entity. Picked
  cells live in a per-realm resource keyed by world cell with a regrow time, outside the parcel
  so eviction does not regrow them (and in the save game if they must survive a reload). Flora
  placement skips picked cells. With merged flora (X15), hide an instance by collapsing its
  vertex range in the parcel's herb mesh. *Gate:* `bevy_picking` (X3) and gameplay input.
- **X15** Merge flora into per-parcel meshes. Flora costs about 1.5 ms at distance 20 and about
  5 ms at 40 ([notes.md](notes.md)); hiding and the show set (D19, D20) cut little, so the cost
  is the shown flora entities. Plan: cache each model once as a template (part meshes, transforms, material
  key) in an `Arc`; `gen_flora`'s async task bakes placements (`Mesh::transformed_by`) and
  merges (`Mesh::merge`), like ground meshes. Trunks (solid color, no UVs) merge into one
  vertex-colored mesh per parcel with one outline child; leaves group by texture; herbs get
  their own mesh with per-cell vertex ranges (X14). Flora outlines need their own
  `OutlineMaterial`, width times the entry's mean scale, since the shader extrudes in
  mesh-local space (until then, trunk outlines are thicker on arbor_day models, placed near
  scale 1, than on seedling ones, near 0.65). Drop a hidden parcel's merged meshes and rebuild on show (about 2-3 MB per
  parcel). Removes about 5,000 transform-only scene entities, and with them their
  `Propagate<RenderLayers>` bookkeeping (D5). *Gate:* scenery restored and a full scene measured, to see
  whether the headroom is needed.
- **X16** Revisit `avian3d` (D22). *Gate:* avian merges multiple physics worlds (issue #935)
  with game entities kept in one ECS world, per-world spatial queries, and moving a body
  between worlds (an actor crossing a portal); and a release supports the Bevy in use.
- **X17** Actor goals (the three.js version's goal-tree AI, `engine/src/world/actors/goals`) move to
  ECS and BSN assets: each goal's hierarchy of elements becomes a hierarchy of entities and
  components. Most reactions pair a condition with an effect. Conditions are reactive formulas; an
  effect is usually a Rust type with parameters, but can be a formula. So SAGA (X7) needs a way to
  embed formulas in assets. The same holds for the other hierarchical assets (quests, dialogues).
  Definitions are shared, not instanced per actor, and an actor's goal state is kept apart from
  them: a small cursor (step, timers, bindings) that names definition nodes by stable id, not
  `Entity`. A cursor is what a sparse save holds (X18) and what a layer restarts from (R104); one
  shared definition serves every actor running it, and a content edit reaches them all. Shared
  scenes need not be Bevy's: each definition asset can be spawned once into a hidden registry
  outside any realm, its state pointing in by id, and respawned on hot reload. Bevy's shared scene
  assets (proposed by the author, received warmly, likely not first-tier) would only tidy that.
  *Gate:* BSN assets, with shared scenes or a definition tree spawned once per asset, and SAGA
  formulas in assets.
- **X18** Save games with structural sharing: one chunk of play changes per precinct, written on
  eviction; a save is only a root index of chunk versions. Design and what it asks of the
  engine in [save-games.md](save-games.md). X14's picked cells would live in the precinct chunk.
  *Gate:* actors restored (the first content with real mutable state).
- **X19** Cutaways: when the player is under a ceiling, scenery above them is hidden, including
  actors on those floors. Rects are authored per tier (`TierSer.cutaways`); choosing them and the 3D
  test against element and actor bounds follow the three.js version ([faery.md](faery.md), Scenery).
  Tier entities (D24) carry their rects; elements need bounds. Hidden elements use
  `Visibility::Hidden` so descendants inherit it (D19). Once interiors show, check outlines on
  glass: through a transparent mesh, its inverted hull's far side shows, not just the rim. faery
  drew a `glass` material's back faces first, depth only (`ModelCatalog.ts`, `glass`), so the hull
  behind the glass failed the depth test; Panoply ignores the `glass` extra (the `Alembic` in
  `decorations.glb` has it; `GlassTest`'s `black-glass` is another variant). *Gate:* scenery and
  actors restored.
- **X20** Keep unknown aspects lossless (D25): hold each as an opaque value that saves back
  unchanged, so the editor cannot drop data a disabled module owns. *Gate:* editor restored.
- **X21** Scenery follow-ups, each small: a show set for precincts like D20 (scenery taller
  than D20's 4 m allowance also needs its own); `LightSource` point lights per precinct
  (registered, not yet acted on; candles in the old engine); hot reload of `FloorSurface`
  assets (floors do not rebuild when one changes). *Gate:* scenery restored; take each when it is needed.
- **X22** A loading state: a game state (Bevy `States`) that holds until the scenery around the
  viewpoint has loaded, so a loading screen or other visual barrier covers the pop-in on entering
  the game. "Loaded" means the precincts in view have no `ReadPrecinctData`, their scenery scenes
  are instantiated (D26 budgets spread them over frames), and the shown parcels have ground and
  flora; later, actors too. A teleport (`GotoLocation`, portals) could reuse it. Startup's
  pipeline warm-up (D26) moves behind the title screen or lobby (new game, continue, load),
  which covers it, instead of running under the first view. Low priority;
  the loading aesthetics come much later, but a placeholder (a plain loading indicator over a
  black screen) is fine before then. *Gate:* a game-entry flow (title screen or save loading)
  to put it in front of.
- **X23** Actor palette merge: join each actor mesh's primitives into one, with a per-vertex
  color-slot index, so an actor draws as one primitive (and one outline) with one material
  holding its palette. An actor is now about 10 primitives plus an outline child each, and
  per-entity work is the CPU floor ([notes.md](notes.md)); a crowd like the three.js test lineup
  (20-odd actors, nearly all colored differently) is about 600 skinned draws before shadows. Needs a
  gltf-transform step and a small shader; slot names must stay consistent per file (D28).
  *Gate:* actors restored, and a crowd scene measured.
- **X24** Realms without terrain. `underworld` (in `world.locations.json`) is floors only, with
  black everywhere else; the three.js version needed a placeholder terrain map for it (a 3x3
  stub, which `convert_realm_maps.mts` writes; deleted here). Check that a realm with no
  `.terrain` file spawns no parcels and no errors, and that parcels' wait for precinct effects
  (D11) does not hang; make a stub map only if it must. *Gate:* portals (the way in).
- **X25** Water currents, for rivers; partly built. Flow cannot move vertices, which would
  stretch or compress where a current meets still water. Instead the fragment shader mixes
  noise layers, each scrolling in its own direction (`FRAGMENT_WAVES` in `water.wesl`: four
  directions, +x, +z, -x, -z, three frequencies each), and the current vector varies each
  layer's amplitude: still water weights all directions equally (standing chop), and a current
  boosts the layers that agree with it, e.g. a base weight plus `max(0, dot(direction,
  current))`. Normalizing by the total weight, as the loop already does, keeps ripple strength
  even across the boundary. The current is meant to be the water mesh's `depth_motion.yz`
  (written as zero in `water_mesh.rs`; `.x` is depth); the unused `motion` variable and the
  commented strength gate in the loop are the start. With four axis directions, a diagonal
  current may read as two crossing patterns; six or eight directions would cover angles
  better, at more texture samples (now 12). The ripples are the only carrier of flow, so
  water shading changes (X11) must keep them readable as moving bumps. *Gate:* rivers (a
  source of current vectors in the terrain data).
- **X26** Live tuning of surface looks: a reflected resource (like `Sunlight` and
  `HemisphereLight`) holding the water's look (water and sky colors, sky scale and weight,
  alpha, chop, glint) and the seabed tint, copied into the materials on change, with sliders
  in the editor panel, in a sub-mode (e.g. of terrain or realm). Tuning water (X11) one
  rebuild and screenshot at a time was slow; shaders now hot-reload (D32), but uniforms set
  from Rust do not. *Gate:* the editor panel and the choice of UI reactivity (X3).
- **X27** Mipmaps for textures inside glTFs (flora leaves, scenery), which D33 does not
  cover: Bevy's glTF loader generates none. Bevy 0.20's loader takes D33's plain KTX2 either
  way, without `KHR_texture_basisu`: an image URI loads through the asset server by extension,
  and an embedded image decodes by its `mimeType` (`image/ktx2`); `gltf-json` does not check
  MIME types. `arborDayTrees` embeds its leaf textures this way, through `pngToKtx2`, and they
  render. Embedded is the choice: arbor_day textures are unique per tree, so a separate file
  gains no sharing. Left: the other flora and scenery; alpha coverage per mip (averaged alpha
  thins cutouts at distance); and clamping, not wrapping, at the edges when downsampling
  non-tiling textures. *Gate:* flora shimmer or scenery texture aliasing worth fixing, judged
  once actors make whole scenes (X12).
- **X28** `arbor_day` (`../arbor_day`): port the tree generator (`../seedling`, TypeScript) to
  Rust, in its own workspace, with a new growth model; no backward compatibility. All 48 flora
  models come from seedling. Requirements, the algorithm survey to do first, and open questions
  are in its `docs/requirements.md` (moved from here 2026-10-11); its progress in its
  `docs/ledger.md`. Its output should let X15 merge flora and X27 mip leaf textures without
  pipeline patching. On Bevy main like Panoply (D34). Its authoring app shares X3's UI
  architecture, and its shell, built first, serves as X3's `bevy_immediate` spike, replacing
  the water sliders. *Gate for the generator:* the growth-model survey and a recommendation.
- **X29** Restore a maximized window as maximized. `window_settings.rs` saves the size and
  position a maximized window has, so a restart gives a window of that size that the OS does not
  treat as maximized (the macOS green button does not restore it). Save the state too (winit's
  `is_maximized`) and restore it with `Window::set_maximized`. *Gate:* the author finds it worth
  doing; `arbor_day`'s `src/settings.rs` has the same code.
- **X31** A hot-reloaded texture keeps its old mips (bevy#26225): when the size, format and mip
  count are unchanged, Bevy rewrites only mip 0, so a re-exported arbor_day leaf color shows
  only up close. Restart the game to see it. The fix merged on Bevy main as bevy#26320
  (`4e2eb9911`; arbor_day's X48), which D34's commit includes. *Gate:* met; delete once a
  re-exported leaf color shows at every distance without a restart.
- **X32** Outline widths from the model: `models::process_material_extras` reads only whether a
  material has the `"outline"` extra, so every outline is drawn at `OutlineMaterial`'s default
  width (0.015) and the width arbor_day exports has no effect. One outline material per distinct
  width, cached by a key built from the material's parameters, as faery did: its
  `ModelCatalog.getMaterial` (`packages/engine/src/models/ModelCatalog.ts`, over `MaterialPool`)
  keyed each generated material by kind and parameters (`outline_<width>`,
  `color_<color>_<map>`, `billboard_<map>`, `flame_…`), the outline width read from the node, its
  parent or the material and rounded to 0.001 first, so near-equal widths share one material.
  The same cache would serve Panoply's other generated materials. *Gate:* the first arbor_day species placed in the game.
- **X33** Region readiness: one query, given an area, that is true when no part of it overlaps a
  parcel or precinct still being built. Actors pass their footprint: an actor gets a physics
  body only where the colliders under it are built. Built means inserted and the realm stepped
  once since: rapier 0.36 adds an inserted collider to the broad phase, which queries and the
  character controller use, only in `step`, and only the viewpoint's realm steps (D35). A
  per-realm step count against each parcel's insert step would do; the collider itself is made
  in the ground-mesh task, so construction needs no state of its own. Plans pass a point (the viewpoint) or the
  camera's estimated view (the parcel show set, D20, already computes one). A loading screen
  after a portal would use it too. Render readiness may need its own check, beside physics.
  faery's version is in [faery.md](faery.md), Actors and navigation. The plans' half landed in
  PAN9 as `ViewReady`, which covers every loaded parcel and precinct rather than an area; the
  `*Pending` system params it combines (`TerrainPending`, `SceneryPending`, `ModelsPending`)
  are where an area filter would go. *Gate:* actors (PAN3), for the footprint query.
- **X34** "Path not found" errors for precincts that have no file (`p002-p002.msgpack` and
  others): the game loads every precinct position near the view, but only some exist. It needs
  to know which do, by a directory scan or a catalog file. A catalog works in packaged and web
  builds, where folder loading may not. It could be a `pipeline.ts` target that lists
  `assets/scenery/precincts/<realm>/` into a catalog: unusual in reading from `assets/` rather
  than `artwork/`, but rerun by overrun whenever the editor saves a precinct. Two floor types are missing too. `slate.floor.ron` was never written; faery's `Slate`
  is textured (`slate-grid.png`, already in `assets/textures/`). `terrain-fx.earth.floor.ron`
  is not a floor: faery's `TerrainFx.Earth` aliases the `Road` terrain effect (`cobbles` here),
  which faery's `Structure.ts` filled as a terrain effect when it appeared among a precinct's
  floor types; `convert_precincts.mts` made it a floor surface path. *Gate:* the editor creates
  precincts (X3), or the errors hide a real one.
- **X35** Tunnels and cave entrances, which must be navigable: the ground can have holes
  (`TerrainHole`, per 1 m cell, in the mesh and the collider), but `TerrainOptions::Flatten`
  is declared and never set or read, here or in faery (`TerrainEffect.ts`, commented out), and
  nothing builds flat passageways with near-vertical side walls; that is outside the height
  field. Decide whether they are terrain or scenery (floor regions and wall pieces in a hole).
  *Gate:* the first cave or mine entrance in a map.
- **X36** Colliders authored in the glTF, beside the exemplar descriptors (D38): a
  model that carries its own colliders uses them, and any other keeps its `SceneryColliders`,
  so old content needs no rework, new models can be authored in Blender, and an old one moves
  when next touched. Moving all of them would mean re-authoring every scenery model. Carrier
  undecided: Skein components (untried here) or specially named collision meshes read by
  `pipeline.ts`. Descriptors and Skein both become components once exemplars are BSN (X1).
  Placing descriptors by hand was workable in faery; how well it goes here depends on the
  editor's property grid (translation and scale of standard shapes, X3) with the F3 view
  live. *Gate:* the author tries Skein, or the first model whose colliders are authored in
  Blender.
- **X38** Particles and missiles. faery's particles interacted with the world: they despawned on
  collision, could be light and sound sources, and spawned other particles (a fireball or cannonball
  trailing smoke). Third-party Bevy particle engines lack these, and a port of faery's could not do
  them on the GPU either. So effects split in two: particles, instanced and GPU-driven, purely
  visual; and missiles, entities without automatic instancing that collide, light, sound and emit
  particles. Requirements, the engines considered and the design Panoply leans toward are in
  [particles.md](particles.md). *Gate:* the first effect: snoring (R36 in
  [actor-requirements.md](actor-requirements.md)) or a missile.
- **X39** Scenery colliders beyond D38's static solids: doors (a kinematic body following the
  glTF node the descriptor's `animation` names, moved by the model's own animation, with no
  separate animation data), sensors, ladders, `hint` and `marker` colliders (the editor's
  picking proxies), portal volumes, and faery's slab under the world. Trees are D39.
  *Gate:* the actor feature or editor tool that uses each (PAN3, X3).
- **X40** `docs/guidelines.md`, after `../ldd_template`: judgement a maintainer would otherwise
  learn by correction, for guidance that outgrows `AGENTS.md` or is project-specific enough to
  keep in git rather than the agent's memory. *Gate:* `AGENTS.md` grows long enough that its
  rules crowd out orienting.
- **X41** Archery. `RecurveBow` in `artwork/props/weapons.blend` is a static mesh: it needs a string
  bone and a draw clip of its own, played in step with the archer's, which the shared humanoid clips
  also lack (draw, aim, release). An arrow model too, held on the string while drawn, then a missile
  (X38). Until then the bow is wielded as a static prop (PAN3o). *Gate:* ranged combat.
- **X42** Inventory and equipment: an actor's items (R5), and the ones it has equipped, shown on
  their slots' bones (R14) in place of the `WieldedItems` aspect. Item exemplars keep only their
  model and slot; the rest of faery's item fields (stats, price, weight, icon) are dropped
  ([faery.md](faery.md), Migration scripts). *Gate:* the first feature that moves items: loot,
  trade, or a quest item.
- **X43** Choosing among interactions (PAN21): a radial menu on click, or a key hint per enabled
  interaction for E's probe. In faery no object ever had two enabled at once (an unlocked door could
  offer both close and lock, but nothing in the game calls for locking one), so E performs the one
  enabled. *Gate:* content with an object that has two interactions enabled at once.
- **X44** The precise move (R32): to an exact point, height included, with collisions off. PAN3h's
  travel (`TravelTo`) goes through the character controller, so furniture would stop it. *Gate:* the
  pose action (R43), or the first actor to sit or lie down (R82).
- **X45** Clips from outside sources, so the author's animation time stops limiting combat moves and
  poses, as it has so far. Candidates: Mixamo (royalty-free in projects, but no redistribution of
  the files, which a public repo or extractable assets may break) and CC0 sets such as Quaternius's;
  check each licence's current terms. Each clip needs retargeting onto the humanoid skeleton (D28)
  and a style check against the hand-keyed clips; layering in the animation graph (D44, R39) cuts
  how many are needed. PAN24's trial measures the retargeting cost. *Gate:* combat design (PAN6 or
  before) needs more moves or poses than are worth keying by hand.
- **X46** The player's start: a new game offers four avatars (a limited customization), then an
  opening cut scene places the chosen one at the start. Until then `spawn_player` drops a fixed
  avatar into the playground. Not a precinct actor (D47). *Gate:* cut scenes.
- **X47** R33's other animation states (stumble, blocked, take damage, dying, dead, sleep), R35's
  play-once-and-hold, and R37's behavior on a one-shot's completion. PAN3j plays only the states
  something in the game can cause (D48). *Gate:* each with what causes it: pushing (R28), blocked
  travel (R22), combat, death, sleeping fixtures.
- **X48** One property editor for Panoply and arbor_day, in a crate both use. PAN28 forks
  arbor_day's instead, so each grows without satisfying the other. *Gate:* both have settled, or the
  same fix has to be made in both.
- **X49** Actor capsules as colliders, so a ray can hit an actor. Today an actor's capsule is only a
  query shape that `move_actors` sweeps. A collider in the realm's rapier world, in a new `ACTOR`
  collision group that the character controller's filter leaves out, so actors never collide through
  rapier (R86); the probe and picking include it. Only an actor that moved updates its collider, so
  resting actors (PAN3p) cost nothing. Check whether a collider without a body must be a sensor to
  stay out of contact generation. faery also had an inflated capsule for picking
  ([faery.md](faery.md), Physics); one capsule serves both until picking feels fiddly. *Gate:* the
  first thing that must hit an actor: picking (X3) or the probe (PAN21).
- **X50** Repulsion between actors (R23, R86): the aura is added to an actor's travel before the
  controller moves it, so a pushed actor is still swept against the world and is never pushed
  through a wall. A smoothed velocity rather than a force, so it does not jitter; a weight per
  actor, so NPCs yield to the player more than the player to them; and a small sideways bias, so two
  actors meeting head-on do not stall. Its radius leaves melee in reach. An actor inside another's
  space wakes (PAN3p). Neighbours by testing every pair in the realm until a grid is needed. *Gate:*
  NPCs travel among each other (PAN16, X17).
- **X51** Wake only the resting actors near a collider change. Today any insertion or removal wakes
  every resting actor in the realm for a tick (PAN3p), so while precincts and parcels stream in, the
  count changes on most ticks and a crowd stays awake. Instead, `RealmPhysics`, through which every
  insertion and removal goes, keeps each changed collider's bounding box (read before removal) in a
  `SmallVec`, so the common case allocates nothing; `move_actors` wakes the resting actors whose
  capsules overlap one, and clears it. The search starts as a test of each resting actor against
  each box; a spatial hash grid or a BVH when actor counts need one, or rapier's own once actors are
  colliders (X49), an intersection query in the `ACTOR` group. Or, before the exact test, a filter:
  a bitmask of x and z cells taken modulo its width, whose aliasing only wakes extra actors, though
  a parcel's ground fills much of it. *Gate:* a frame-time hitch measured while walking into a crowd
  as its precincts load.
- **X52** An actor that falls out of the world goes back to the last save point, where a respawn
  after death puts it, rather than to the last place it stood (R27, PAN3m). *Gate:* save points.
- **X53** Skill trees as assets, so skills and spells are content, not engine code
  ([goals.md](goals.md)). Nodes are data: id, name, icon, description, branch, layout position, and
  prerequisites (other nodes, points spent, level; a formula only for the unusual one). A node's
  effects are of three kinds: passive modifiers (a stat, an operation, an amount, and an optional
  condition formula such as `isWielding(Sword)`); granted abilities, each an asset of its own
  (targeting, cost, cooldown, animation, an effect formula), as spells are; and named stats or flags
  that engine systems read at extension points (barter reads a price stat). Stat names are content
  too, referred to by id. A character sheet is cells (saga.md): derived stats are reactive formulas
  over other stats, and combat resolution is formulas evaluated per attack, from a ruleset asset.
  faery's `design/talents.md` sketches branches and their talents. The proving slice: one stat, one
  modifier with a condition, one granted ability. *Gate:* the first three steps of saga.md's order
  of work.
- **X54** Graphics settings for weaker GPUs, such as `Aerdra`'s integrated one (notes.md). A
  `GraphicsSettings` resource deriving `SettingsGroup`, as `WindowSettings` does, is all that saving
  them takes. The controls go in the editor pane for now, and move to a dialog once the game has
  in-game dialogs. Plans start from empty settings, so they measure the defaults and `docs/perf.csv`
  stays comparable. The options should be the ones that matter to this engine, not a general list: a
  game whose camera cannot see the horizon has no use for a draw distance. By the measurements so
  far: MSAA (3.9 ms, and `plans/ablate.py` already switches it live); a render scale, if
  `MainPassResolutionOverride` proves to save the pixels' cost without an upscaler (about 4 ms at
  1080p's pixel count, untested); and outlines (0.5 ms). Thinning trees would save the most after
  those, but trees have colliders (`flora_physics.rs`), so a tree thinned from view alone would
  still block the player: not a setting unless collision is settled first. *Gate:* a build goes to
  players, or the author wants lower settings for their own play on `Aerdra`, whichever comes first.
- **X55** Tracy, for frame-to-frame problems the averages hide. An opt-in build
  (`--features bevy/trace_tracy`, so `Cargo.toml` need not change). `tracy-capture` records a run
  headless, and `tracy-csvexport` should turn it into a table an agent can read, so nobody need read
  the timeline; its versions must match the client's, as Bevy's `docs/profiling.md` explains. It
  would not help a GPU bottleneck: `plans/ablate.py`'s GPU figures and vendor tools cover that, and
  samply already covers CPU cost. *Gate:* a hitch that the hitch log (`PANOPLY_HITCH_LOG=1`) cannot
  explain.
- **X56** The in-game map: nav points with an `icon` mark it. faery's `INavPoint` also had `center`,
  the icon's position on the map where it differs from the point's, and `viewAngle`, the heading on
  arrival; add each to `NavPoint` when something reads it. *Gate:* the map is planned, or travel
  needs a heading.
- **X57** Bevy widget bugs the author is fixing upstream (bevy#26444, bevy#26445), all seen in the
  Go To dialog, which Panoply leaves alone meanwhile:
  - A list's focus outline (`highlight_active` in feathers' `listview.rs`) is pickable and covers
    its row, so the second click of a double-click lands on it and `PointerClick::count` restarts:
    double-clicking a nav point does nothing.
  - Feathers colors a text input's cursor only when the theme changes, so an input spawned later
    keeps the default dark gray cursor.
  - A modal makes focus visible as it opens (`modal_dialog_barrier_on_spawn` in `bevy_ui_widgets`'
    `modal.rs`), so the filter box shows a focus ring with no keyboard navigation, which closes up
    the gap between it and the list.
  - bevy#26447: the list view's scrollbar positions itself against its container's edge
    (`right: px(4)`), where in feathers the container owns the padding. Go To nests the list as
    `group() > group_body()`, whose padding is lopsided by that inset until it is gone.

  *Gate:* a Bevy bump that includes the fixes; then check each in the Go To dialog, and add a
  double-click to its check in `plans/smoke.py`.
- **X58** The game UI's font in assets, not code: `panoply_core::fonts` loads Macondo Swash Caps and
  names its family. *Gate:* BSN assets (X1); then a game UI scene names the font, and the module
  goes.
- **X59** Wide editors: data too unwieldy for the panel's narrow column gets an editor that replaces
  the 3D view while its entry is selected, as faery's dialogue state machine editor did
  ([faery.md](faery.md), Editor, `catalogs-dialogue.png`). *Gate:* the first catalog kind whose
  entries do not fit the panel is taken up; dialogues are the likely first.
- **X60** Placeholder text in the editor's text boxes ("Filter…" in the nav point list's filter, a
  hint in the text-entry dialog). `EditableText` lists placeholders as planned but not implemented
  (`bevy_text`'s `editing.rs`), and feathers' text input has none; an overlaid label was judged not
  worth it. *Gate:* a Bevy bump whose `EditableText` or feathers text input has placeholders.
- **X61** Audio. Panoply has no sound. The old engine had ambient sound emitters placed as a scenery
  aspect (`SoundSource`), music, water sounds and an audio-effect system (`AudioFx`,
  `AudioFxTemplate`). The proving slice: one ambient emitter aspect and music per realm, on Bevy's
  built-in audio. *Gate:* PAN6, or the author takes up sound.
- **X62** Signs and books: things to read. A sign is a scenery aspect whose interaction shows its
  text; a document is an inventory item that links to text content and a page style (faery's book
  aspect opened its content in a frame, `codex` by default). The second aspect after PAN21's first,
  since its action needs only a reading view. *Gate:* PAN21.
- **X63** Mechanisms: the stock elements puzzles are built from, and how they are wired together.
  faery had doors, push and toggle buttons, pressure plates, remotely controlled openables,
  automatic doors and spear traps, wired by reactive formulas so that elaborate puzzles combine
  simple parts ([faery.md](faery.md), How far faery got; the aspects the converter dropped are under
  Migration scripts). Beside them: triggers (a circle or rectangle the player enters), encounter
  zones, and waymarks for scripted NPC events. To decide: how one element signals another (an entity
  event, as PAN21's actions are, or a formula over the other's state), and which parts are native
  aspects and which are SAGA (X7). The hard part is referencing and naming. A precinct is named by
  its realm and coordinates, and is like a module: its walls and fixtures that others can refer to
  are its exports, by their export id (`SceneryInstanceId::External`). That name is clumsy to type
  into a text box, so the field for a signal source browses the available signals with a filter, as
  PAN35's chooser does. A reference into a precinct that is not loaded suspends the formula reading
  it ([saga.md](saga.md), Values). faery's notes, of unknown currency, and likely to differ from
  what Panoply builds: in `design/Scripting.md`, a reference syntax, an `Instance` observable that
  is null while its target is not loaded, a `StructureRefNode` watching structures load and unload,
  and global variables or lazily loaded scope objects for long-range wiring; global references by
  realm and coordinates in `design/scripting/predicates.md`; animated instances (moving parts) kept
  apart from static instancing in `engine/src/models/InstancedModels.ts`. The first customer is
  PAN6's door opened by a button ([goals.md](goals.md), step 4). *Gate:* PAN21; before PAN6.
- **X64** Aspect types declared in SAGA, which the editor inspects through a layer above reflection
  ([modding.md](modding.md), Types a mod declares). Bevy plans that layer, for dynamic data, on
  `bevy_reflect2` (bevy#26356); use Bevy's if it has landed. *Gate:* a mod needs an aspect that
  native aspects, formulas and the generic variables aspect cannot express.
- **X65** Package identity, for more than one package: ids namespaced by package, so two mods'
  `Sword` cannot collide; an order for packages that override the same exemplar; what a save keeps
  of a removed mod's aspects (X20 covers only the editor); and a mod built against an older version
  of its game's primitives. The model for ids is WESL's imports
  ([spec](https://github.com/wgsl-tooling-wg/wesl-spec/blob/main/Imports.md)), which Panoply's
  shaders already use: `package::` names the package's own items, any other first segment names a
  dependency declared in its manifest (`wesl.toml`), and visibility (`public`, package, `private`)
  decides what a package exports, as X63's precinct exports do. SAGA's imports and asset ids would
  share one scheme. *Gate:* a second package loads beside the game.
- **X67** The `.blend` files in `artwork/scenery/` and `artwork/props/` (25) move to Blender 5.2 as
  the characters did (D55): `blend_inventory.py` and a `blend_cleanup.py --preview` against today's
  `.glb`s first, since their materials may hit the same exporter changes. *Gate:* one of them is
  next opened to edit.
