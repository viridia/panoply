# Changelog

<!--
For agents. A chunk adds its own line here, at the top of the list, newest first:

- **PAN<n>** <what changed, for the reader>.

The reader is a player, a content author or a contributor, so the test for an entry is: what would
they notice? How it was done belongs in the commit message, why in decisions.md (a decision names
its chunk; entries carry no D or X numbers), and how it works in the code. No commit hashes; the
commit subject names the chunk. One line, id included, within the 100 columns prose wraps at. A
chunk split into lettered parts gets a line per part.

A `##` heading marks a real milestone only, such as PAN6, the first playable. Do not add one for a
month or a batch.
-->

- **PAN3d** Actors wear their exemplar's colors.
- **PAN3b** Actors play their idle clip, outlines following the pose.
- **PAN3a** An actor stands in the playground; actors and the precinct format are crates.
- **PAN13** Ledger tools: `scripts/verify.sh`, `xref.py`, `show.py` and `devfmt.py`, allowlisted.
- **PAN12** The migration ledger is split into separate ledgers and this changelog.

## Port to Bevy main, 2026-10-09 to 2026-11-01

Panoply last ran in full on Bevy 0.15. Everything but a minimal core was disabled, then restored
module by module, first on Bevy 0.20 and then on Bevy main.

- **PAN11** Trees block walking, and a canopy over forests keeps actors from falling between trunks.
- **PAN2** Walls, fixtures and floors are solid.
- Teleporting during startup no longer leaves outlines to hitch later.
- **PAN10** Plans can ask which parcels and precincts are still loading.
- **PAN9** Plans wait until the view has finished loading.
- **PAN7c** The location overlay, hitch log and profiling diagnostics live in the editor crate.
- **PAN7b** Materials, models and scenery are crates of their own.
- **PAN7a** Parcel systems live in the terrain crate.
- **PAN1** Editor shell: a side panel (T) with a Show menu for debug views.
- Physics on rapier 0.36, with collider debug views (F3).
- Plans run on the `bevy-remote-driver` package.
- Bevy main in place of Bevy releases.
- Flora size varies per biome entry.
- Trees from arbor_day can stand in for seedling's, for comparison.
- Terrain textures have mipmaps.
- Embedded shaders hot-reload in editor builds.
- Water and seabed colors as in the three.js version.
- Lighting as in the three.js version.
- Each exemplar catalog loads once, not once per label.
- Scrolling no longer hitches on pipeline compiles or bursts of scenes.
- Scenery, without actors.
- Parcels outside the view are hidden; flora is rotated at random.
- Hemisphere ambient light, and shadows fitted to the camera distance.
- Editor-only location overlay and zoom.
- Terrain.
- Exemplars.
- Input through `bevy_action_map`.
- Materials and models, with shaders in WESL.
- A minimal core on Bevy 0.20.
