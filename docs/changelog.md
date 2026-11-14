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

- **PAN28k** Features and colors are a row per slot, marked in the models; maps save in key order.
- **PAN28j** Dragged numbers round to an annotated precision; aspects open from their own button.
- **PAN28i** Maps edit as entries: a key input, its value's widget and a ×; + adds, keys rename.
- **PAN28m** Property rows align in columns, collapse with summaries; aspects have icons.
- **PAN28l** The property editor's next look is chosen: aligned columns, collapsing panes, icons.
- **PAN28d** Aspects are added and removed, and an exemplar's extends, name and abstract edited.
- **PAN28h** An Armature names its node and clips file, each defaulting to the Skin's; clips picked.
- **PAN28g** Actors' skins and walls' and fixtures' models are picked from lists by file, not typed.
- **PAN28f** Lists add and remove items, enums switch variant; colliders' sizes follow their shape.
- **PAN28c** Optional fields: × removes, a menu adds; text edits on Enter; abstract exemplars list.
- **PAN28b** Save writes edited exemplars to their catalogs, changing only the lines edited.
- **PAN28a** Scenery mode shows the chosen exemplar's properties; its numbers edit live, undoably.
- **PAN41** Scenery mode lists the walls, fixtures, actors and terrain effects to place, each kept.
- **PAN73** Scenery mode has its panel, whose palette chooses a tool; later chunks add the tools.
- **PAN45c** A chest offers Open or Close as it is shut or open, and nothing while locked.
- **PAN45b** Formulas in exemplars and precincts compile as they load; an error names its file line.
- **PAN45a** SAGA's compiler is part of Panoply, hardened against mods' scripts; `.saga` highlights.
- **PAN69** The editor's filter boxes say "Filter…" while empty; a double-click in Go To travels.
- **PAN70** Checked that reloading a character file still needs a rebuild; nothing changes in play.
- **PAN68** Built against a newer Bevy main, which brings fixes to the editor's lists and inputs.
- **PAN67** Portals shimmer again: their glow drifts off the arch, as in the old engine.
- **PAN38c** Terrain mode's sculpting tools show where they will act, and draw the parcel grid.
- **PAN38b** In terrain mode a click selects a parcel, showing its contour, which Rotate turns.
- **PAN66c** Under overload the game slows down, at worst, rather than freezing.
- **PAN66b** Actors away from the view hold still while their ground is unloaded, not falling.
- **PAN66a** An actor caught inside the ground is pushed out and lands, rather than sticking there.
- **PAN38a** Terrain mode has its panel, whose palette chooses a tool; later parts add the tools.
- **PAN37** The realm panel's map size control grows or shrinks the map a parcel at a side.
- **PAN36** In realm mode the editor draws the map's parcel and precinct grid and its border.
- **PAN35** The realm panel can create a new, empty realm, and change to any realm from a list.
- **PAN56** The editor reopens at the realm, place and heading it was left at.
- **PAN65** Light hair is softer in shade, its lit side falling away gently instead of cutting off.
- **PAN3n** Hair shines in a soft band across its strands, moving with the sun.
- **PAN44** Quitting the editor with unsaved edits asks first: Save, Discard or Cancel.
- **PAN58** The editor briefly shows what Ctrl+Z just undid or redid, and when a save has failed.
- **PAN57** After Save the editor waits a moment for the file to reload, so no edit is lost to it.
- **PAN64** Chests, dressers and sarcophagi make sounds as they open and shut.
- **PAN63** The windmill turns, and a sarcophagus lid waits a moment before it moves.
- **PAN62** In play the cursor hides after a second at rest over the scene, and returns on moving.
- **PAN61** Dressers and sarcophagi open and shut as chests do.
- **PAN60** In play the pointer is a game cursor, which changes over things you can use.
- **PAN21** Facing a chest shows a prompt at the bottom of the view, and E opens and shuts it.
- **PAN42** Ctrl+Z and Ctrl+Shift+Z undo and redo adding and removing nav points in the editor.
- **PAN54** Snow and moss meet other ground in clumps of many sizes; slopes bare a little sooner.
- **PAN55** The camera sits closer to the action, and the sun falls at an angle across buildings.
- **PAN52** Corners and creases deepen in shade, so walls away from the sun show their relief.
- **PAN53** The ground shows each parcel's biome, snow and tundra among them, mixing in patches.
- **PAN51** Carlyle Tepper appears; `verify.sh` fails when an asset names a path that is not there.
- **PAN50** Scenery and props are Blender 5.2's: highlights shift, and the keep's walnut has grain.
- **PAN49** A character file re-exported while the game runs rebuilds its actors, not crashing.
- **PAN48** Character files are Blender 5.2's; their `.glb`s are up to 70% smaller, junk clips gone.
- **PAN29** Realm mode's panel shows the map's stats and edits nav points; Save writes them.
- **PAN27** Buttons atop the editor panel choose the editor mode or play, kept across restarts.
- **PAN22** F2 switches play and edit: WASD and arrows move the player, or the camera; Go To too.
- **PAN26** G opens Go To: type to filter the named places, then Go or Enter travels to one.
- **PAN33** Typing in a text field or a dialog no longer also drives the game behind it.
- **PAN18** W and S held together now stop the player's actor; before, W won.
- **PAN25** A contributor finds the 51 named places to travel to in `panoply_core`, as `NavPoint`.
- **PAN32** The early debug grid no longer draws over the world, including across the water.
- **PAN3m** An actor that falls off the map or through a hole is put back where it last stood.
- **PAN3p** Standing actors cost almost nothing; `plans/perf.py` logs its frame times to a file.
- **PAN3k** E logs what lies within 1.5 m ahead of the player's actor, passing through canopies.
- **PAN31** The smoke test's window title names the part of the test running.
- **PAN14** `AGENTS.md` gives the driver workflow, from launching to a chunk's final plan run.
- **PAN8** The default plan run is three launches, each window titled with its plan's name.
- **PAN3j** Actors fall, land, turn on the spot, and idle in the stance their aspects choose.
- **PAN3l** Precincts place their NPCs, facing as in faery; archetypes can scale an actor.
- **PAN3i** Actors walk, fast walk and run by speed, crossfading; models face the way they walk.
- **PAN3h** Actors travel to a point at a pace, turning toward it; the player's keys cut it short.
- **PAN3g** WASD walks the player relative to the camera, which follows; steep slopes block it.
- **PAN3f** Actors fall under gravity and land on the ground; archetypes set their collision size.
- **PAN3o** Actors hold weapons and tools in their hands; item exemplars carry model and slot.
- **PAN3e** Actors wear their hair, beards and hats, colored and outlined.
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
