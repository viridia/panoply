# Asset BSN

What Panoply needs from BSN as an asset format, which has not landed (X1). Macro BSN (`bsn!`) can
embed Rust expressions and asset BSN cannot, so their features will differ even where their syntax
matches. These are the questions to put to the people building it, most blocking first, with Cart's
answers of 2026-11 and what they leave open.

1. **Parameters.** In macro BSN a parameterized scene is a Rust function, as
   `fn chooser(autofocus: bool, items: Vec<String>, selected: Option<String>) -> impl Scene` in
   Panoply's editor. What is the asset form? Panoply's case: one goal subtree used with different
   arguments, as faery's Ivy runs `UseCuttingBoard` on two worktables, each with its own fixture and
   distance ([goal-format.md](goal-format.md)).

   *Planned:* parameters become part of the syntax, so a scene knows its own, which allows them in
   assets and lets parameterized scenes share cached assets. A "scene function", a Rust function
   returning a scene that does not know its parameters, stays as a separate thing.
2. **Values without Rust expressions.** Which literal forms assets have: lists, maps, enum variants,
   nested structs, vectors, colors. Whether an app can parse literals of its own, as `FromTemplate`
   turns an asset path into a `Handle`: Panoply's are SAGA formulas, written as strings and compiled
   by the game, and references to fixtures placed in the world.

   *Answered:* the same as `bsn!`: structs, nested (vectors and colors are structs), enums, arrays
   and primitives. Arrays serve lists and sets. Map syntax is undecided: `{ "key": value }` is
   ambiguous with a Rust block expression in `bsn!`, and could be claimed for maps if `bsn!` stops
   needing `{}`. *Open:* app-parsed literals.
3. **Including and patching.** Whether an asset scene can include another and patch it. Patches
   merge by field: does a list field replace or append, and how is an inherited component or child
   removed? Can a patch reach an entity other than the root, by `#Name`?

   *Answered:* including and patching will be supported. *Open:* lists, removal, and patching by
   `#Name`.
4. **Many scenes per file.** Panoply's catalogs hold hundreds of prototypes per file, addressed as
   `file#Label`. Bevy #10549, a label requested before its file has loaded starting a full load of
   the file, is why Panoply loads catalogs by their root (D27).

   *Planned soon:* a new syntax for scenes, expected also to resolve scene flattening. *Open:*
   #10549.
5. **Unknown types.** Whether a component of an unregistered type can be skipped with a warning, not
   fail the load (D25), and kept on save (X20), for mods and modules not loaded.

   *Answered:* skipping is acceptable, as an `AssetLoader` setting. *Open:* keeping them on save.
6. **Writing.** A serializer from reflected values, a printer that keeps a file's formatting and
   comments, and an AST that an editor other than Bevy's can use. Panoply's in-game editor saves a
   catalog by merging its edits into the file (D73).
7. **Errors and tooling.** Spans in errors; recovery, so one bad entity does not fail the file; and
   validation while writing in a text editor, as the JSON Schema `bevy_remote` exports from the type
   registry gives JSON.
8. **Hot reload** reaching the instances already spawned from a scene asset.

`bevy_bsn`, at the commit Panoply builds, is a text parser on `nom`, behind an optional feature and
called by nothing; `bsn!` parses with `syn`. It is no evidence of what asset BSN will hold.
