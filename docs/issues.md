# Issues

Triage: findings awaiting the author's decision, whether to make each a chunk in
[roadmap.md](roadmap.md), a deferral or a decision, or to drop it. Each says whether it was
confirmed by running something or only by reading. Numbers are never reused.

**Next: I6.**

- **I1** The water shows a fine square grid (visible before and after D16). Almost certainly the
  debug `InfiniteGrid` (`main.rs`) seen through the transparent water: the same grid continues
  past the map edge in `lighting_bluff.png`. Not yet confirmed by hiding the grid.
- **I2** `ExemplarData`'s `Serialize` impl is `todo!()`; serializing an exemplar panics. Read,
  not run.
- **I3** Nothing draws the flame material (`FlameMaterial`, for the `flame` material extra): no
  model in `assets/scenery/models` has the extra, and PAN3a removed the test cube that used it. A
  shader break would go unseen until a model with flames is placed. Read, not run.
- **I4** A model and clip viewer, as faery's `catwalk` was (`../faery-catwalk`, Electron and
  three.js): load a glTF, pick a character in a multi-model file, play its clips from a menu.
  Undecided whether a Bevy version is worth it. If so, a candidate home is the editor's catalogs
  mode (X3), previewing an archetype as the game assembles it, colors and features included, rather
  than a separate tool.
- **I5** Landing from a fall feels like stepping onto a shock absorber. The fall itself is right: a
  probe of a 4 m drop showed speed growing at the set gravity until touchdown. The likely cause is
  the camera's chase (D42), which also trails in height and eases out over its last half metre, so
  the view keeps sinking for about a second after the actor lands. Candidates: a faster vertical
  catch-up, or none until PAN3j's landing clip, whose leg flex is part of the effect. Confirmed in
  the author's video of a fall from the playground's wooden stairs (at 2.45 m/s² gravity): the
  actor's fall accelerates steadily, but the camera did about 70% of its descent after touchdown,
  sliding for 1.2 s and creeping for 1.5 s more.
