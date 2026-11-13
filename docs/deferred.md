# Deferred

Work decided against for now, each with its gate: the event that reopens it. When a gate fires,
the entry becomes a chunk in [roadmap.md](roadmap.md) and leaves here. An entry with no event is
dropped, not deferred. Numbers are never reused.

**Next: X111.**

- **X1** Migrate exemplars to BSN. *Gate:* BSN assets released, and BSN supports shared
  (non-instanced) templates. Includes updating the exemplar migration script, which lives in the
  `faery` repo (the earlier TypeScript/three.js engine), to emit BSN. As of 2026-11, BSN assets wait
  on an overhaul of Bevy's reflection. Consuming them, with `extends` as scene inheritance, looks
  straightforward; editing is the hard part. Bevy's scene editor will edit an AST of the scene file
  and serialize it to BSN, not the spawned entities, which lose imports, type names and loading
  directives. Open: use that AST in Panoply's editor, or keep exemplars as an editor-only AST of our
  own that the editor compiles to BSN.
- **X2** Synthesized shared assets (successor to D6). *Gate:* a restored module needs them. Consider
  deterministic `AssetId`s from hashed params instead of a fake asset source.
- **X5** `bevy-inspector-egui` (pinned bevy 0.15). Bevy main's `bevy_inspector` crate may
  replace it. *Gate:* a compatible release is needed.
- **X6** Revisit asset bundling. The GLTF layout carries three.js-era assumptions, made to cut
  HTTP requests: small models (e.g. flora) are combined into one file, and large Blender
  catalogs are never split. Under Bevy, per-model files and labeled sub-assets may suit better.
  *Gate:* restoring terrain/flora/scenery, or a change to the asset pipeline (see D7).
- **X7** Scripting and modding in SAGA ("Scripts for Animated Graphic Adventures"), designed in
  [saga.md](saga.md) and [modding.md](modding.md), beyond PAN45's first formula and PAN71's imports:
  value slots, then cells and reactions for PAN79's button and door. *Gate:* PAN71.
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
  grid cell (at most one herb per 1 m cell; a ray hit gives the cell), not by entity. Picked cells
  live in a per-realm resource keyed by world cell with a regrow time, outside the parcel so
  eviction does not regrow them (and in the save game if they must survive a reload). Flora
  placement skips picked cells. With merged flora (X15), hide an instance by collapsing its vertex
  range in the parcel's herb mesh. *Gate:* `bevy_picking` (PAN5) and gameplay input.
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
  ECS and BSN assets. Goals are not aspects, since an actor can hold several goals of one type with
  different parameters. Aspects own goals (R47), and owning goals is most of what a mod's aspects
  are for ([modding.md](modding.md)).
  - **Instanced trees.** Each actor spawns its own copy of each goal tree its aspects own: an entity
    per node, joined by a `Subgoal` relation (ordered, despawned with its parent, outside transform
    propagation) and rooted at the actor by `GoalOf`.
  - **Definition apart from state.** Each goal type is its own component of immutable parameters
    (`Wander { radius }`), so BSN writes a tree as components and `Subgoals [ ... ]`. It requires
    its state, a separate reflected component whose default is the reset state, and a
    `GoalEval::of::<Self>()`: a function pointer, monomorphized for the type, that fetches both and
    evaluates them. One system walks each tree from the top through those pointers, because a
    combinator calls its active child and acts on the result in the same pass. The Rust goal types
    are a closed set per build; mods compose them.
  - **Formulas.** A goal's condition is a reactive formula; an effect is usually a Rust type with
    parameters, but can be a formula. So SAGA (X7) needs a way to embed formulas in assets, as
    quests and dialogues do. A node's memo cells live on its entity. Landing re-evaluates an actor's
    goals (R37).
  - **Saves** (PAN82) hold only state that differs from the reset state: per aspect, a hash of its
    goal definition and each changed node's state, keyed by its path of child indices. When the hash
    no longer matches (a mod or content edit changed the tree), that aspect's state is dropped and
    its goals restart, which goals tolerate (R50). The same read-out, respawn and write-back is hot
    reload, so an edit reaches every actor, and it is what an actor recovers from (R104).

  The first goal: an NPC that waves when the player comes in range ([saga.md](saga.md), Early
  customers). *Gate:* BSN assets, and SAGA formulas in assets.
- **X20** Keep unknown aspects lossless (D25) in precincts: hold each as an opaque value that saves
  back unchanged, so the editor cannot drop data a disabled module owns. Catalogs already keep them,
  since a save merges edits into the file (D73). *Gate:* the editor saves a precinct (PAN5).
- **X21** Scenery follow-ups, each small: a show set for precincts like D20 (scenery taller than
  D20's 4 m allowance also needs its own); hot reload of `FloorSurface` assets (floors do not
  rebuild when one changes). *Gate:* scenery restored; take each when it is needed.
- **X22** A loading state: a game state (Bevy `States`) that holds until the scenery around the
  viewpoint has loaded, so a loading screen or other visual barrier covers the pop-in on entering
  the game. "Loaded" means the precincts in view have no `ReadPrecinctData`, their scenery scenes
  are instantiated (D26 budgets spread them over frames), and the shown parcels have ground and
  flora; later, actors too. A teleport (`GotoLocation`, portals) could reuse it. Startup's pipeline
  warm-up (D26) moves behind the title screen or lobby (new game, continue, load), which covers it,
  instead of running under the first view. Low priority; the loading aesthetics come much later, but
  a placeholder (a plain loading indicator over a black screen) is fine before then. *Gate:* a
  game-entry flow (title screen or save loading) to put it in front of.
- **X27** Mipmaps for textures inside glTFs (flora leaves, scenery), which D33 does not cover:
  Bevy's glTF loader generates none. Bevy 0.20's loader takes D33's plain KTX2 either way, without
  `KHR_texture_basisu`: an image URI loads through the asset server by extension, and an embedded
  image decodes by its `mimeType` (`image/ktx2`); `gltf-json` does not check MIME types.
  `arborDayTrees` embeds its leaf textures this way, through `pngToKtx2`, and they render. Embedded
  is the choice: arbor_day textures are unique per tree, so a separate file gains no sharing. Left:
  the other flora and scenery; alpha coverage per mip (averaged alpha thins cutouts at distance);
  and clamping, not wrapping, at the edges when downsampling non-tiling textures. *Gate:* flora
  shimmer or scenery texture aliasing worth fixing, judged once actors make whole scenes (X12).
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
  material has the `"outline"` extra, so every outline is drawn at `OutlineMaterial`'s default width
  (0.015) and the width arbor_day exports has no effect. One outline material per distinct width,
  cached by a key built from the material's parameters, as faery did: its `ModelCatalog.getMaterial`
  (`packages/engine/src/models/ModelCatalog.ts`, over `MaterialPool`) keyed each generated material
  by kind and parameters (`outline_<width>`, `color_<color>_<map>`, `billboard_<map>`, `flame_…`),
  the outline width read from the node, its parent or the material and rounded to 0.001 first, so
  near-equal widths share one material. The same cache would serve Panoply's other generated
  materials. *Gate:* the first arbor_day species placed in the game.
- **X34** "Path not found" errors for precincts that have no file (`p002-p002.msgpack` and others):
  the game loads every precinct position near the view, but only some exist. It needs to know which
  do. A list works in packaged and web builds, where folder loading may not: per realm, beside the
  precincts it lists (`scenery/precincts/<realm>/index.json`), a sorted JSON array of `[x, y]`
  coordinates, one per line so a diff shows each precinct added or removed (serde_json's pretty
  printer would split each pair over four lines, so the writer formats it). A finished realm may
  hold about 500 precincts; at that size JSON costs nothing over msgpack and reads in a diff and a
  text editor. The editor rewrites it whenever it creates or deletes a precinct, and `asset_xref.py`
  checks it against the folder, so a file added or removed by hand fails `verify.sh`. *Gate:* the
  editor creates precincts (PAN5), or the errors hide a real one.
- **X36** Colliders authored in the glTF, beside the exemplar descriptors (D38): a model that
  carries its own colliders uses them, and any other keeps its `SceneryColliders`, so old content
  needs no rework, new models can be authored in Blender, and an old one moves when next touched.
  Moving all of them would mean re-authoring every scenery model. Carrier undecided: Skein
  components (untried here) or specially named collision meshes read by `pipeline.ts`. Descriptors
  and Skein both become components once exemplars are BSN (X1). Placing descriptors by hand was
  workable in faery; how well it goes here depends on the editor's property grid (translation and
  scale of standard shapes, PAN28) with the F3 view live. *Gate:* the author tries Skein, or the
  first model whose colliders are authored in Blender.
- **X40** `docs/guidelines.md`, after `../ldd_template`: judgement a maintainer would otherwise
  learn by correction, for guidance that outgrows `AGENTS.md` or is project-specific enough to
  keep in git rather than the agent's memory. *Gate:* `AGENTS.md` grows long enough that its
  rules crowd out orienting.
- **X41** Archery. `RecurveBow` in `artwork/props/weapons.blend` is a static mesh: it needs a string
  bone and a draw clip of its own, played in step with the archer's, which the shared humanoid clips
  also lack (draw, aim, release). An arrow model too, held on the string while drawn, then a missile
  (PAN87). Until then the bow is wielded as a static prop (PAN3o). *Gate:* ranged combat.
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
  travel (`TravelTo`) goes through the character controller, so furniture would stop it. With it, an
  actor lies down in a bed (`BedSingle`, `FancyDoubleBed`) and plays R33's sleep state, held on its
  last frame (R35), with PAN86's snoring; a pose action finishes when its animation does (R37).
  *Gate:* the pose action (R43), or the first actor to sit or lie down (R82).
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
- **X48** One property editor for Panoply and arbor_day, in a crate both use. PAN28 forks
  arbor_day's instead, so each grows without satisfying the other. *Gate:* both have settled, or the
  same fix has to be made in both.
- **X50** NPCs steering apart while they travel (R23, R86), on PAN90's repulsion: a small sideways
  bias, so two actors meeting head-on do not stall, and allies' wider spacing (0.5 m). *Gate:* NPCs
  travel among each other (PAN16, X17).
- **X51** Wake only the resting actors near a collider change. Today any insertion or removal wakes
  every resting actor in the realm for a tick (PAN3p), so while precincts and parcels stream in, the
  count changes on most ticks and a crowd stays awake. Instead, `RealmPhysics`, through which every
  insertion and removal goes, keeps each changed collider's bounding box (read before removal) in a
  `SmallVec`, so the common case allocates nothing; `move_actors` wakes the resting actors whose
  capsules overlap one, and clears it. The search starts as a test of each resting actor against
  each box; a spatial hash grid or a BVH when actor counts need one, or rapier's own once actors are
  colliders (PAN89), an intersection query in the `ACTOR` group. Or, before the exact test, a
  filter: a bitmask of x and z cells taken modulo its width, whose aliasing only wakes extra actors,
  though a parcel's ground fills much of it. *Gate:* a frame-time hitch measured while walking into
  a crowd as its precincts load.
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
- **X57** A modal makes focus visible as it opens (`modal_dialog_barrier_on_spawn` in
  `bevy_ui_widgets`' `modal.rs`), so the Go To dialog's filter box shows a focus ring with no
  keyboard navigation, which closes up the gap between it and the list. The author is fixing it
  upstream (bevy#26445). *Gate:* a Bevy bump that includes the fix; then check it in the dialog.
- **X58** The game UI's font in assets, not code: `panoply_core::fonts` loads Macondo Swash Caps and
  names its family. *Gate:* BSN assets (X1); then a game UI scene names the font, and the module
  goes.
- **X59** Wide editors: data too unwieldy for the panel's narrow column gets an editor that replaces
  the 3D view while its entry is selected, as faery's dialogue state machine editor did
  ([faery.md](faery.md), Editor, `catalogs-dialogue.png`). *Gate:* the first catalog kind whose
  entries do not fit the panel is taken up; dialogues are the likely first.
- **X61** Audio beyond transition sounds (D63). The old engine had ambient sound emitters placed as
  a scenery aspect (`SoundSource`), music, water sounds and an audio-effect system (`AudioFx`,
  `AudioFxTemplate`). The proving slice: one ambient emitter aspect and music per realm, on Bevy's
  built-in audio. The falloff (D63) is tuned then, against Poul Ryba at his forge in Tambry: his
  hammer faint from just off-screen and louder on walking up to him. *Gate:* PAN6, or the author
  takes up sound.
- **X64** Aspect types declared in SAGA, which the editor inspects through a layer above reflection
  ([modding.md](modding.md), Types a mod declares). Bevy plans that layer, for dynamic data, on
  `bevy_reflect2`, whose MVP has landed (bevy#26356); use Bevy's if the layer has. In the editor
  such a type needs its own walk of the rows and its own applying of edits, beside reflection's
  (PAN28). *Gate:* a mod needs an aspect that native aspects, formulas and the generic variables
  aspect cannot express.
- **X65** Package identity, for more than one package: ids namespaced by package, so two mods'
  `Sword` cannot collide; an order for packages that override the same exemplar; what a save keeps
  of a removed mod's aspects (X20 covers only the editor); and a mod built against an older version
  of its game's primitives. The model for ids is WESL's imports
  ([spec](https://github.com/wgsl-tooling-wg/wesl-spec/blob/main/Imports.md)), which Panoply's
  shaders already use: `package::` names the package's own items, any other first segment names a
  dependency declared in its manifest (`wesl.toml`), and visibility (`public`, package, `private`)
  decides what a package exports, as PAN79's precinct exports do. SAGA's imports and asset ids would
  share one scheme. *Gate:* a second package loads beside the game.
- **X69** Stop faery's precinct converter overwriting ([faery.md](faery.md), Migration scripts):
  once Panoply edits a precinct, a rerun would undo it. Freeze it as the realm map converter is, or
  make it merge as the archetype converter does, if faery's precincts still hold content Panoply has
  not taken (instance inventories and aspects, layers). *Gate:* the editor saves a precinct (PAN5),
  or one is edited by hand.
- **X70** Rebuild demorun's cottages. Demorun was made to film a character running through many
  biomes and kinds of scenery; its cottages (`p002-p002`) are mangled and mostly roofless, since
  they were built from wall tiles that faery's later rethink of the cottage set removed. *Gate:* the
  editor places scenery (PAN5).
- **X72** Undo of an edit that is off-screen: the camera does not move on undo, so the change can
  happen out of sight. Candidate: an action optionally reports where it happened, and the camera
  moves only if that is off-screen or in another realm. *Gate:* PAN38 or PAN5, the first edits with
  a place on the map, if it annoys in use.
- **X73** Undoing back to the saved state leaves the asset marked unsaved, so Save stays enabled.
  Harmless, since Save rewrites the same content, but PAN44's prompt would ask needlessly. The fix
  is a saved-at position per asset in the history. *Gate:* PAN44.
- **X74** Merging `Interactions` entries by `id` along `extends`, so a variant such as a trapped
  chest restates only the entry it changes; an override now replaces the whole list (D59). Needs a
  way to remove an inherited entry. *Gate:* that repetition hurting in real content.
- **X75** The look of the interaction prompt (PAN21): the decorative box around the key, and the
  key's font, for now `KEY_PROMPT_FONT` (Rubik). The key should be somewhat hefty in weight, and in
  a different-looking font from the caption, though non-swash Macondo has a far more readable
  capital E; both fonts should be light-hearted, not serious or mechanical. *Gate:* the HUD's visual
  pass.
- **X76** A layout for the HUD, so its on-screen controls have a place each and do not stomp on one
  another; the interaction prompt is the first, centred at the bottom of the view. *Gate:* the
  second on-screen control in play.
- **X77** Localization. Player-facing text stays English in the assets, where it is authored: a
  catalog of ids would make dialogue trees unwritable. A script, as `asset_xref.py` is, extracts it
  into gettext `.po` catalogs, one per language, from a list of the fields that hold such text (an
  exemplar's `display_name`, `InteractionDesc::caption`, later a dialogue line and a book's
  content), which grows in the chunk that adds a field. Each string is keyed by where it lives
  (asset, object, field path; a dialogue's node and line), so nothing is written back into the
  assets, and renaming an exemplar follows its aliases (PAN28e). Each entry keeps the English it was
  translated from, so a changed line is marked stale, not lost; the extractor attaches what its
  place knows (speaker, quest, asset) for the translator. Lines are fixed, with no variables,
  plurals or gender, so every line can be voiced: a voice file is named by its line's key and goes
  stale with it, and a line that would need a variable is written as variants, a branch per form, as
  voiced RPGs do. Interface text avoids sentences ("Coins: 3"). The English game reads its assets
  directly; another language looks each key up and falls back to English. *Gate:* before captions
  and other player-facing text are authored in quantity.
- **X78** The rest of faery's hover feedback (PAN60, `SceneView.tsx`): the disabled cursor over an
  instance offering nothing, and an actor's nameplate on hover. Its other cursors are in `../faery`,
  to copy as entries name them. *Gate:* content or play that misses each.
- **X79** Deleting a realm from the editor. Today its files (`realms/<name>.realm.json`,
  `terrain/maps/<name>.terrain`) are removed by hand, which also undoes a realm made by mistake
  (D65). *Gate:* realms deleted often enough to miss it.
- **X80** Nav points drawn as markers in the editor's view, those of other realms dimmed, so an
  underground level can be laid out against its overland counterpart without showing two realms at
  once. *Gate:* building a level that needs it; at the earliest, PAN36's overlays.
- **X81** A switch for the selected parcel's overlay between the contour as edited, which it shows
  now, and the smoothed ground the game draws, as faery had. *Gate:* sculpting (PAN38c) that needs
  to see where the smoothed ground lies.
- **X83** Bevy's math types handed to rapier directly. Bevy is on glam 0.34 and rapier on 0.33
  (through `glamx`), so each crossing converts with `panoply_core::rapier_math`'s `to_rapier` and
  `to_bevy`. *Gate:* a rapier release on Bevy's glam; then delete the module and its calls.
- **X84** The townsfolk stop's 1 ms lost to bevy#26371 (PAN68's bump): 11.61 and 11.82 ms in two
  runs, against 10.40 to 10.94 before it (`docs/perf.csv`). It adds `Changed<GlobalTransform>` to
  `check_entities_needing_specialization`, so every moving mesh is re-specialized each frame, though
  only transparent items' cached sort centers needed refreshing; townsfolk has the most moving
  meshes. Not confirmed by reverting it. Reported on Discord (2026-11-10); Bevy is tracking an
  alternate fix, with no PR yet. *Gate:* that fix in the Bevy we build against; then compare
  townsfolk in `perf.py`.
- **X85** Placeholders through feathers (PAN69). Bevy's `Placeholder` has no `Default`
  ([bevy#26532](https://github.com/bevyengine/bevy/issues/26532)), so `bsn!` cannot spawn it and the
  chooser's filter box gets its hint from an observer (`hint_filter`), and feathers has no
  placeholder style, so the hint sets its own: dim, by `Placeholder`'s default alpha, and italic.
  The text-entry dialog's box has no hint yet; its openers would pass one. Follow-up PRs to the
  placeholder are under discussion. *Gate:* a `Default` or a feathers style in the Bevy we build
  against; then spawn the hint in `bsn!` with the theme's style.
- **X86** Dynamic lists in SAGA, beyond fixed-size arrays ([saga.md](saga.md), Values). Open: how
  one lives on a stack that holds no owned data, and whether lists weaken the rule that sets of
  entities are native aggregates. *Gate:* a helper module that needs a list whose length is not
  fixed.
- **X87** Struct types declared in SAGA, laid out in slots as host-declared ones are
  ([saga.md](saga.md), Values). Not X64's aspect types, which the editor and saves see: these live
  only within scripts. *Gate:* a helper module that needs one.
- **X88** A SAGA language server, for semantic tokens and diagnostics ([saga.md](saga.md), Editor
  support). The compiler stops at the first error; the server needs recovery, to report more than
  one. *Gate:* modules long enough that fixing one error per compile slows writing them.
- **X89** Kinds of host function: which a formula may call and which only an effect may
  ([saga.md](saga.md), Formulas and effects), checked at compile time by a tag on each registration.
  *Gate:* the first effect.
- **X90** Scans in place of polling, for SAGA's reactions ([saga.md](saga.md), When reactions run):
  a reverse index from `(Entity, ComponentId)` to cells, a `Changed<C>` system per watched type,
  `summary_tick` on rarely changing components, and observers for removals. *Gate:* the reaction
  pass's polling shows in `plans/perf.py`, or reactive cells number in the hundreds.
- **X91** Module-level `let` and `const` in SAGA: constants a helper module shares among its
  functions. They parse, and compile to an unsupported-feature error. *Gate:* a helper module needs
  one.
- **X92** Closures in SAGA, downward only ([saga.md](saga.md), Values): lambda syntax, function
  types as parameter types, structural unification of function types, and host functions that call
  back into the VM. After value slots, whose single stack makes a capture an absolute index. *Gate:*
  the first native aggregate that wants a script predicate.
- **X93** A corpus of realistic SAGA formulas, for benchmarks and as a regression suite: a few dozen
  of faery's predicates (`design/scripting/predicates.md` in faery) and its aspects' conditions,
  translated to `.saga` under `crates/panoply_saga/tests/corpus/`, run over a world of a few hundred
  actors. It measures the reaction pass, tracking's dedup among its costs. *Gate:* the read counts
  of the reactions step ([saga.md](saga.md), Order of work) show tracking as a cost, or X90's gate
  fires.
- **X94** An editor list of the formulas that failed to compile, each naming its asset and the
  object it belongs to, with a way to open it to fix. Each `Formula` keeps its error
  (`Formula::error`). *Gate:* PAN28's property editor, which edits a formula field.
- **X95** JSON schemas for exemplar files, for VS Code's completion and validation when editing them
  by hand. Generate them from the type registry, as `bevy_remote`'s `registry.schema` does for each
  reflected type, so only the envelope (names to entries with `type` and `aspects`) is written by
  hand. faery's schemas (`packages/schemas`) describe a format Panoply no longer has and are not
  ported. *Gate:* hand-editing exemplar JSON becomes routine again.
- **X96** Check the formulas in a precinct instance's own aspects, as exemplars' are (D71), against
  the instance's aspects and its exemplar's chain, once the precinct's exemplars have loaded. Until
  then such a formula is checked only as it runs: a missing component fails its evaluation, logged
  once. *Gate:* the first precinct instance with a formula of its own.
- **X97** Catalogs mode's panel: faery's catalog kinds (quests, dialogues, remarks, books,
  particles, items, item tables, spawn tables, cut scenes), the chosen kind's entries, New, Clone
  and a filter (`catalogs-quests.png`). Some kinds may be exemplar-based and reuse PAN41's chooser;
  which is decided once their data is in Panoply. *Gate:* the first catalog's data migrated from
  faery.
- **X98** Port the property editor's walk and apply (PAN28) to `bevy_reflect2`, as an early customer
  whose findings go upstream. Worth it only while Bevy is still shaping the crate; once its own
  crates have moved, Panoply follows with them. *Gate:* `bevy_reflect2` can reach a field by path,
  read its attributes and make a default value, soon enough that a report can still change it.
- **X99** Inheritance of single fields within an aspect, which faery had for its archetypes' own
  fields through JavaScript prototypes. An aspect that wants it bears the cost: a field of type
  `Inherit<T>`, whose `Inherited` is a key the file omits, `Absent` a `null`, and `Present(v)` a
  value, resolved where the chain is, at evaluation: an `Aspect::inherit` hook that `UpdateAspects`
  calls with each farther aspect of the type. The editor shows an `Inherited` field dimmed with the
  value it resolves to. An exemplar's `display_name` is the plain case, an `Option` read along the
  chain. *Gate:* an aspect that needs one field overridden and the rest inherited, likely colors.
- **X100** A report of the fields catalogs hold that no type reads, to adopt (as PAN28b adopted
  colliders' `margin`) or retire: the keys in a file's JSON missing from its exemplars serialized as
  loaded, which is half of what saving does (D73). *Gate:* the first pass to retire fields.
- **X101** Gizmos in the view for the field being edited, such as a collider's box or an offset's
  point, dragged to fit the visible mesh, where today the numbers are nudged in the panel. *Gate:*
  the property editor has a selected field, which a gizmo would follow.
- **X102** A collider switched to another shape starts the dimensions it does not share at their
  types' default, 0, so a box made a sphere has no size, and the physics drawing shows nothing of
  it. Candidates: dimension newtypes with sizable defaults (`Radius`, `HalfExtents`, D45), judged
  clutter for now; or marking a collider of no size where colliders are drawn. *Gate:* the editor
  draws colliders itself (X101, or I12's retained gizmos).
- **X103** Style sheets for the game UI's widgets, as designed in
  [widget-styles.md](widget-styles.md). *Gate:* the first game-UI widget beyond the interaction
  prompt.
- **X104** Combat strategy in goals, faery's weakest area. Priorities stay fixed, so a band still
  wins over the bands below it; goals of equal priority are ordered by a utility score, a formula
  computed from the situation (threat, range, health, what is ready). The score is a memo, so it
  wakes goals only when it changes (R49), and a rival tactic takes over only when it leads by a
  margin, so close scores do not flap. *Gate:* X17's goals run, and an NPC fights back.
- **X105** Sweep the game's per-frame systems for workarounds that satisfy the borrow checker at a
  cost: a query collected into a `Vec` before mutating, a cloned handle or asset, `mem::take` or
  `resource_scope` around a resource. Each is fine alone; many in a hot path stand in for a split of
  the data. The editor (`panoply_editor`) is exempt, as its performance matters little. Read only: a
  list for the author, ranked by how often each runs. *Gate:* game-side performance work begins, or
  a frame-time regression in `docs/perf.csv` that a profile does not explain.
- **X108** Combat's animation states: R33's take damage, dying and dead, the last two held on their
  last frame (R35), and taking damage re-evaluating goals (R37). PAN3j plays only the states
  something in the game can cause (D48). *Gate:* actors have health: PAN87's fireballs or PAN6's
  fights, whichever deals damage first.
- **X109** Scenery phasing, by layers: scenery that appears with a quest stage, such as the harvest
  festival's booths. A realm has named layers, each with a reactive `when` formula (SAGA's cells,
  X7). A layer's content is a patch of walls, fixtures and floors, kept with the precinct or apart
  from it, and only ever adds: a layer never removes what the base or another layer placed. Actors
  phase by their own aspect (R10). A layer that switches applies only while its precinct is off
  screen or behind a cut, as R10 has actors do, so nothing appears or vanishes in view. In the
  editor, layers are shown or hidden for authoring, and a control picks the current layer, to which
  every wall, fixture and floor placed then belongs; the scenery palette's Layers button
  (`SceneryTool::Layers`) is its place. faery had layers too (`engine/src/world/structures/`), not
  ported. No phased terrain or terrain effects: possible, with no customer. A mod's scenery is the
  same additive patch: it adds walls, fixtures and actors to the game's precincts, and precincts
  where the game has none, so a mod ships its own precinct index (X34), joined with the game's.
  *Gate:* the first quest stage that changes scenery, or the first mod that adds scenery.
- **X110** The game UI's typeface (`GAME_UI_FONT`, Macondo Swash Caps). Plain Macondo reads better,
  its capital E most of all (X75); the choice is to switch to it, or to make a font of the author's
  own that combines the best of both. Neither has bold or italics, so emphasis in a book's text or a
  line of dialogue needs another face, or another means: a colour, small caps. *Gate:* the first
  player-facing text that needs emphasis (PAN78's books are likely), or the HUD's visual pass (X75),
  whichever comes first.
- **X107** Ladders. The `Ladder` wall tile (`wall-tiles.exem.json`) has a `ladder` collider and
  `top` and `bottom` marks, the climb's ends; nothing reads either. *Gate:* actors climb.
