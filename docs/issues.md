# Issues

Triage: findings awaiting the author's decision, whether to make each a chunk in
[roadmap.md](roadmap.md), a deferral or a decision, or to drop it. Each says whether it was
confirmed by running something or only by reading. Numbers are never reused.

**Next: I3.**

- **I1** The water shows a fine square grid (visible before and after D16). Almost certainly the
  debug `InfiniteGrid` (`main.rs`) seen through the transparent water: the same grid continues
  past the map edge in `lighting_bluff.png`. Not yet confirmed by hiding the grid.
- **I2** `ExemplarData`'s `Serialize` impl is `todo!()`; serializing an exemplar panics. Read,
  not run.
