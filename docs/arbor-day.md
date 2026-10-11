# arbor_day: requirements

A Rust port of the tree generator in `../seedling` (TypeScript, Electron, three.js), renamed
because `seedling` is taken on crates.io. `arbor_day` (and `arbor-day`, which crates.io treats
as the same name) is free as of 2026-10-11.

It will live in its own workspace and repository. This document starts here because the tool
shares context with Panoply: the art style, the asset pipeline it feeds, and the editor UI
architecture (X3 in the [migration ledger](migration.md)). It moves with the tool when that
repository exists.

This document says what the tool must do. It does not pick a growth algorithm; that needs a
survey first (Research, below).

Backward compatibility is not a goal. The existing plants stay as they are, and seedling stays
available to regenerate them. New models may replace them species by species if they look
better.

## What it is for

Low-poly plants for large forests where each plant is small on screen. Two things set it apart
from general tree generators (SpeedTree, Blender's Sapling, The Grove, Arbaro):

1. **Fine structure goes into the texture, not the mesh.** Twigs and leaves are painted onto
   a few alpha-masked cards; only the trunk and major limbs are geometry. The budget is
   hundreds of triangles per plant, not tens of thousands.
2. **The art style is art nouveau, after Mucha.** Plants have dark outlines like the rest of
   the game. Bark meshes carry outline metadata, which the game draws as an inverted hull
   ([outlines.md](outlines.md)); leaf textures have their outlines painted in.

Botanical accuracy matters only as far as it reads at a distance. Silhouette, crown shape, and
art direction come first.

## What exists today

### seedling

- `MeshGenerator.ts` holds the parameters and grows the tree. Three growth patterns:
  monopodial (a leader with lateral branches at intervals), and two dichotomous variants
  (repeated forks). Parameters are a trunk group plus a repeating group per branch level
  (length, angle, axis rotation, symmetry, deflect, flex). Several apply to only one pattern.
- Bark is a tube of 6-sided rings, a new ring every `segmentLength`, with a point cap at each
  tip. Child branches are separate tubes that start inside the parent; nothing is welded.
- Foliage is one leaf card per branch tip: 2x2 quads with a hard crease along the middle,
  drooped laterally and axially (8 triangles).
- The leaf texture (`genLeafTexture.ts`) is a 128x128 canvas: one leaf shape (a mirrored cubic
  Bezier outline with serration, rake, and jitter, in `genLeafShape.ts`), stamped as a fan and
  in pairs along an axis ("phalanx"). It is stroked black for the outline, then filled with a
  lateral or axial two-color gradient and random hue and lightness shifts. Twig stems are coded
  but disabled (an empty list).
- One RNG stream (`prando`) serves growth, leaf shape, and leaf color in turn, so changing
  a branch parameter also reshuffles the leaf texture.
- Save files are the exported `.glb`: the parameters live in the scene's extras under the key
  `sapling`. There is no separate source file.
- `collision.ts` (pick branch angles to minimize self-shading) exists but is not used.
- Trees often came out contorted, with branches and leaf cards facing every which way. The
  code shows why (see Order and variation, below).
- The editor is a React control panel of property groups with a three.js preview that orbits
  on horizontal scroll.

### What Panoply does with the output

- All 48 flora models are seedling output: 22 trees in five biome groups, 11 shrubs, 7 crops,
  3 bushes (`artwork/flora/*/*.glb`). The asset pipeline (`packages/build-assets/pipeline.ts`,
  `flora()`) merges each folder into one file (`assets/terrain/models/<group>.glb`), one
  named scene per plant.
- The pipeline patches what seedling writes: leaf materials from `BLEND` to `MASK` with cutoff
  0.1, `outline: 0.015` on every other material (seedling puts `outline: 0.01` on the node), and
  cameras removed. seedling also writes `billboard: true` on the leaf node; nothing reads it.
- Placement (`src/terrain/flora.rs`): at most one plant per 1 m cell, chosen by weighted biome
  table, random Y rotation, uniform scale 0.5 to 0.8. Parcels are 16 m square.
- Measured sizes: bark 144 to 2,364 triangles, leaves 48 to 648. In the temperate trees bark is
  four to six times the leaf count, the reverse of what tiny trees need.
- Flora is the largest per-frame cost in the scene (migration ledger, Notes: about 1.5 ms at
  view distance 20, 5 ms at 40). X15 plans to merge flora per parcel: trunks into one
  vertex-colored mesh with one outline, leaves grouped by texture. X27 notes that leaf textures
  inside glTFs have no mipmaps.

## Requirements

Priority: **M** must, **S** should, **C** could.

### Output

- **M** One glTF binary per plant: a scene named after the plant, a bark mesh and a leaf mesh.
- **M** Bark: `POSITION`, `NORMAL`, opaque material with the outline width as a material extra,
  `"outline": <width>`, as the game reads it. No pipeline patching needed.
- **M** Leaves: `POSITION`, `NORMAL`, `TEXCOORD_0`, double-sided, alpha mode `MASK` with an
  authored cutoff, texture embedded.
- **M** Units are meters, +Y up, origin at the base of the trunk at ground level.
- **S** Bark color as a vertex color (or a material the merge in X15 can turn into one), so
  trunks of different species merge into one mesh.
- **S** Leaf textures from several plants can share one atlas, so merged leaves draw with
  fewer materials (X15 groups leaves by texture).
- **S** Leaf textures with mipmaps whose alpha coverage is preserved per level, so cutouts do
  not thin out at distance (X27). Either written into the glTF in a form Bevy loads, or as a
  separate `.ktx2` next to it.
- **C** Several variants of one species (different seeds) in one file, as sibling scenes.

### Bark geometry

- **M** A triangle budget per plant, set by the author and reported live. The generator spends
  it where it shows: on the silhouette and the lower trunk, not on fine branches.
- **M** Normals continuous around and along each tube. The outline shader extrudes along
  normals, so a split normal opens a gap in the outline.
- **M** Ring sides vary by branch radius (e.g. 5 or 6 on the trunk, 3 or 4 on small limbs),
  and rings are spaced by curvature, not by fixed length.
- **S** Branches end where foliage cards take over. Twigs too small to read as geometry are
  dropped from the mesh and drawn in the leaf texture instead.
- **S** Root flare at the base, optional.
- **C** Welded branch joints. Not needed for the outline, since a child that starts inside its
  parent hides its own outline there; only worth it if joints show.

### Foliage

- **M** Foliage cards placed by the growth model, not only one per branch tip: clusters along
  the outer branches sized to fill the crown envelope.
- **M** Card shapes beyond the current creased quad: at least crossed cards and cupped cards,
  chosen so the crown reads as a mass from any angle (plants are rotated randomly).
- **M** Leaf textures painted with outlines: a dark stroke around each leaf or leaf cluster,
  its width set in texture pixels and checked at the size the card shows on screen.
- **M** Twigs and small stems painted into the texture, with the bark color and outline.
- **S** More than one leaf texture region per plant (e.g. a dense cluster for the crown
  interior, a sparse one for edges).
- **S** Painted styles beyond a single leaf shape: needle clusters for conifers, blossoms,
  bare twigs for dead trees.
- **S** Texture resolution set per plant (128 today; 256 may be needed for clusters).
- **C** A painted shading gradient across the card (darker toward the crown center), since
  the game's lighting reaches the cards only as flat surfaces.

### Growth model

- **M** One model, or a small number with shared parameters, that covers the shapes in the
  game now: broad crowns (temperate), columns (poplar, juniper), cones (pine, hemlock), bare
  and dead trees, shrubs, and crops.
- **M** Parameters an artist can predict: each one changes one visible thing, with no growth
  mode that turns half of them off.
- **M** Control of the overall silhouette: a crown envelope (cone, ellipsoid, column, or a
  curve) that branches and foliage fill, not only a result of per-level rules.
- **S** Gravity and phototropism (branches bend down under weight, or up toward light), the
  role `flex` plays today.
- **S** Asymmetry and irregularity controls, so a species does not look machine-made.

### Order and variation

The card illusion depends on a balance. Cards need enough variety of facing that the crown
reads as a natural mass, not a grid, but most of them face the sky. In seedling, the facing of
both branches and cards falls out of the chain of branch transforms, so variation compounds
instead of staying bounded:

- **Roll accumulates.** Each child's frame is the parent's frame times a random rotation about
  the branch axis (`axis`, summed into `branchRotation`) times the fork angle. Nothing refers
  back to world up except `flex`, so by the fourth or fifth level a branch's sideways
  direction is arbitrary, and fork angles point limbs down or inward as often as out.
- **`flex` depends on segment count.** It bends each segment toward or away from vertical, by
  an amount scaled by the segment's tilt, so the total bend depends on `segmentLength` and
  compounds along long limbs.
- **Card facing is tied to the tip's direction.** A card lies along the tip direction, with
  its face normal in the vertical plane through that direction. A tip 30 degrees off vertical
  gives a card facing 60 degrees off the sky, close to vertical; only level tips give skyward
  cards. A vertical tip has no defined plane and keeps whatever roll it inherited.

Requirements:

- **M** Branch frames keep a reference to world up (rotation-minimizing frames along each
  limb, with roll set against up), so roll never accumulates through the levels.
- **M** Where branches leave a parent is set by phyllotaxis rules (alternate, opposite,
  whorled, spiral at the golden angle of about 137.5 degrees), with bounded jitter, not by a
  random rotation added at each fork.
- **M** Card orientation does not depend on the branch tangent. A card's normal is drawn from
  a distribution around a target direction (a blend of world up and outward from the crown
  center), with an authored spread (e.g. most normals within 30 to 45 degrees of the target).
  Rotation of the card around its normal is free, since that is where the variety reads
  from. Cards are placed by the branches; only their position follows them.
- **M** Every random variation is a bounded distribution around a designed value, never a
  rotation composed onto an already random frame.
- **M** A debug view that colors each card by the angle of its normal from up, and shows that
  spread as a histogram, so the balance can be tuned by eye and by number.
- **S** Branch direction as a target that blends parent direction, the phyllotaxis direction,
  and a per-level tropism (up for poplars, level for spreading oaks, down for weeping
  willows), applied per unit length so it does not depend on ring spacing.
- **S** Limits on branch direction relative to up per level, so limbs do not point inward or
  down unless the species says so.

### Parameters, determinism, variants

- **M** Same parameters and seed, same output, on every platform, in a given tool version.
- **M** Independent random streams per subsystem (trunk, branching, foliage placement, leaf
  shape, leaf color), so changing one leaves the others unchanged.
- **M** Parameters saved as a readable source file (RON or JSON), separate from the export.
- **S** A species file plus a seed list for variants, so a biome gets several trees per
  species without hand work.
- **M** A varied parameter is stored as a center and a spread, not as low and high ends
  (seedling's `[lo, hi]` ranges). Moving the center then changes the tree without touching
  how irregular it is, and the spread can be relative (a fraction of the center) so it stays
  sensible as the center moves. In seedling's saved plants most ranges are zero-width or the
  default 0.1 wide; only a few (the pines' branch axis, e.g. `[0.43, 1.85]`) use a real
  spread.
- **S** The shape of the distribution is selectable: uniform, or weighted toward the center,
  which gives "mostly ordered, now and then irregular" (Order and variation). Per parameter
  or one global choice, to be decided.

### Authoring app

The app uses the UI architecture planned for Panoply's editor (X3), so the two share widgets,
conventions, and lessons:

- **M** A Bevy app. Widgets from `bevy_feathers`; reactivity as X3 decides (recommended there:
  `bevy_immediate`, pending a spike).
- **M** Layout as in the editor: a parameter panel on the left with a splitter, the 3D view
  filling the rest through a camera viewport that follows a UI node.
- **M** App state (open file, panel width, view, selected group) saved with `bevy_settings`,
  so a restart returns to where work stopped.
- **S** Keys through `bevy_action_map`, and a remote-driver plan (as in Panoply) that loads a
  species, screenshots it, and quits, for checking changes without clicking.

- **M** A varied parameter is one row: a feathers slider for the center, with the spread drawn
  as a band behind the thumb (a decorative child, which the headless slider allows), and a
  feathers number input for the spread. The number input scrubs (click-drag, with a
  `SoftLimit` for the drag range and modifier keys for speed), so the spread can be dragged
  too. No custom widget needed.
- **C** A true dual-thumb range slider (drag either end or the band, as seedling's
  `ComboRangeSlider`), as a headless widget in `bevy_ui_widgets` with feathers styling,
  contributed upstream like the split pane and tabs. Only if the row above proves clumsy.

Features:

- **M** Live preview while editing parameters: orbit, zoom, wireframe, triangle count.
- **M** Preview with the game's look: outline shell, alpha mask, the game's sun and hemisphere
  light, and a ground plane.
- **M** A view of the leaf texture beside the 3D preview.
- **S** A preview at game distance and scale: a small patch of forest of the current plant
  (and its variants), so the author judges it as it will be seen.
- **S** Undo and redo.
- **C** Side-by-side comparison with another plant.

### Pipeline

- **M** A command-line mode that generates the `.glb` from a source file without a window.
- **S** The asset pipeline runs that command, so the source files, not the `.glb` files, are
  what live in `artwork/flora`, and changing the generator regenerates every plant.

## Non-goals

- Leaf-level geometry, or close-up hero trees.
- Botanical simulation for its own sake (detailed L-system grammars, growth over seasons).
- Wind animation, for now. Keep it possible: a per-vertex weight (distance from the trunk) is
  cheap to add later.
- Impostors or billboards for far distances; that is the game's LOD problem, not the
  generator's, though a generator could render them later.

## Research before choosing the growth model

Survey what games and tools use, judged by fit to a few hundred triangles, an art-directed
silhouette, and bounded control of branch and card orientation (Order and variation), not by
realism:

- Parametric per-level models: Weber and Penn, "Creation and Rendering of Realistic Trees"
  (1995), the basis of Blender's Sapling and Arbaro. Close to seedling's approach.
- Space colonization: Runions, Lane, Prusinkiewicz, "Modeling Trees with a Space Colonization
  Algorithm" (2007). Branches grow toward attraction points scattered in a crown envelope,
  which gives silhouette control directly.
- Self-organizing trees: Palubicki et al. (2009), light and space competition between buds.
- Game practice: SpeedTree's generator hierarchy and leaf-card workflow; the branch-and-card
  methods in GDC talks and postmortems for games with large forests (e.g. *Horizon Zero Dawn*,
  *The Witcher 3*, *Ghost of Tsushima*); stylized low-poly tree tutorials, where crowns are
  often a few deformed blobs or card clusters rather than grown branches.
- Leaf-card and crown normals: how stylized games shade foliage cards to read as a soft mass
  (normals bent outward from the crown center).

The output of this step is a short write-up with a recommendation, before any code.

## Open questions

1. Triangle budget: what total per tree, and what split between bark and leaves? Today's
   temperate trees spend 2,000+ on bark; something like 300 to 600 in all may be enough at
   game distance. Measure on screen first.
2. Crate shape within its workspace: a generator library (no Bevy dependency, or only
   `bevy_math`) plus the authoring app, or a single crate? The library is what the pipeline
   would call.
3. Leaf texture rendering in Rust: a 2D vector rasterizer (e.g. `tiny-skia`) on the CPU, or
   drawn on the GPU in the app. The command-line mode needs the CPU path either way.

## Decided

- Own repository and workspace; on Bevy main, named as in the migration ledger's Next item 0
  (`git` + `branch = "main"`, the commit held by `Cargo.lock`).
- UI: `bevy_immediate` from the fork `viridia/bevy_immediate`, branch `bevy-main`, with
  `bevy_feathers` widgets. `bevy_action_map` from its `main`. `bevy_remote_driver` from its
  own repo once extracted.
- Build the shell first, with a stub generator, before the growth-model survey. The shell
  is the `bevy_immediate` spike for Panoply's editor (X3), so its patterns (panel layout,
  parameter rows, settings, driver plans) are meant to be reused there.
