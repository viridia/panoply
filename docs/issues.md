# Issues

Triage: findings awaiting the author's decision, whether to make each a chunk in
[roadmap.md](roadmap.md), a deferral or a decision, or to drop it. Each says whether it was
confirmed by running something or only by reading. Numbers are never reused.

**Next: I11.**

- **I1** The water shows a fine square grid (visible before and after D16). Almost certainly the
  debug `InfiniteGrid` (`main.rs`) seen through the transparent water: the same grid continues
  past the map edge in `lighting_bluff.png`. Not yet confirmed by hiding the grid.
- **I2** `ExemplarData`'s `Serialize` impl is `todo!()`; serializing an exemplar panics. Read, not
  run.
- **I4** A model and clip viewer, as faery's `catwalk` was (`../faery-catwalk`, Electron and
  three.js): load a glTF, pick a character in a multi-model file, play its clips from a menu.
  Undecided whether a Bevy version is worth it. If so, a candidate home is the editor's catalogs
  mode (X3), previewing an archetype as the game assembles it, colors and features included, rather
  than a separate tool. PAN24's repairs and X45's outside clips make it more pressing. A smaller
  step: a component that plays a named clip on an actor in the game, set through the remote protocol
  (as `TravelTo` is) or a debug key, which shows a clip with the game's outlines, crossfades and
  layers, which catwalk cannot.
- **I5** Landing from a fall feels like stepping onto a shock absorber. The fall itself is right: a
  probe of a 4 m drop showed speed growing at the set gravity until touchdown. The likely cause is
  the camera's chase (D42), which also trails in height and eases out over its last half metre, so
  the view keeps sinking for about a second after the actor lands. Candidates: a faster vertical
  catch-up, or none until PAN3j's landing clip, whose leg flex is part of the effect. Confirmed in
  the author's video of a fall from the playground's wooden stairs (at 2.45 m/s² gravity): the
  actor's fall accelerates steadily, but the camera did about 70% of its descent after touchdown,
  sliding for 1.2 s and creeping for 1.5 s more.
- **I6** Two female gait clips in `humanoid-armature.glb` look broken in catwalk: `WalkFastFemale`
  has stiff arms (perhaps a bad bone export), and `RunFemale` is broken. Female archetypes' `Run`
  plays `RunMale` meanwhile; `WalkFast` still plays `WalkFastFemale`. Seen by the author in catwalk.
- **I7** The editor icons' meta files (`assets/editor/icons/*.png.meta`) name the loader
  `bevy_render::texture::image_loader::ImageLoader`; Bevy finds a meta file's loader by its type
  path, and the image loader is now in `bevy_image`, so these loads may fail. Read, not run.
- **I8** Two precincts (`playground/p002-p000`, `holm-overland/p008-p012`) name the exemplar
  `exemplars/regional/marheim#Carlyle.Tepper`, whose file does not exist, probably a faery archetype
  path the converter's `mapArch` had no mapping for. Those actors would load no aspects and have no
  body. Read, not run.
- **I9** The cottage's corner roof tiles (`CottageRoofShinglesConvex`, `...Concave`) each collide as
  one straight tile's ramp, shifted 0.5 m along the ridge, as in faery, while their models span 2.5
  m both ways. A valley (concave) is the union of two ramps, so two descriptors fit it; a hip
  (convex) is their intersection, which no descriptor shape makes. PAN3j fixed the same copied ramp
  on the 2- and 3-wide tiles. Seen by the author in the F3 view; the corners read, not run.
