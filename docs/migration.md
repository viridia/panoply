# Bevy 0.20 migration ledger

The project was last fully working on Bevy 0.15. Rather than port everything at once, all modules
were disabled except a minimal core, and are being restored one group (a "chunk") at a time.
Disabled code stays in place, marked `TODO(bevy-0.20)`; `grep -rn "TODO(bevy-0.20)"` lists what
remains. Excluded crates are listed under `[workspace] exclude` in the root `Cargo.toml`.

## Landed

| Commit    | Chunk                                                                                   |
| --------- | --------------------------------------------------------------------------------------- |
| `6f0a2db` | Checkpoint of the abandoned 0.16-dev WIP port.                                          |
| `6a04afa` | Minimal core on 0.20: `panoply_core`, `world`, `view`, `diagnostics`, window settings.  |
| `0f32a52` | `reflect_types`, `materials`, `models`; shaders ported to WESL; infinite grid.          |

## Next

1. Input: replace bespoke handling (`panoply_core::pan_rotate`, `close_on_esc`) with
   `bevy_action_map`.
2. `panoply_exemplar` (port as-is; see D4).
3. `panoply_terrain` + `src/terrain` + `scenery`: first break the `scenery` <-> `terrain` cycle,
   and consolidate code duplicated between the crate and `src/terrain` (`water_mesh`,
   `terrain_map`, ground material). Port `assets/shaders/lib/*` and terrain shaders to WESL.
4. `actors`, `portals`.
5. `editor` (see X3).

When `scenery` and `actors` are back, remove `spawn_material_test` from `main.rs`.

## Decisions

- **D1** Build against crates.io Bevy releases, not git main.
- **D2** Settings use `bevy_settings` (`SettingsGroup` resources, `settings.toml`).
  `bevy_prefs_lite` and `bevy_basic_prefs` are abandoned. Window position falls back to winit's
  `outer_position()` via `bevy::winit::WINIT_WINDOWS` (a stopgap API, Bevy issue #17667).
- **D3** Shaders are WESL (`.wesl`); naga_oil is gone from Bevy. Material bindings use
  `@group(constants::MATERIAL_BIND_GROUP)`. Bevy modules are `bevy_pbr::render::*` etc.
- **D4** Exemplars stay as the composition mechanism for now. Keep the `panoply_exemplar` API the
  boundary, so that scenery/actors depend on "entity with aspects", not on exemplar internals.
- **D5** GLTF models spawn via `WorldAssetRoot`; per-model render layers are copied to descendants
  in an observer on `WorldInstanceReady` (`models::copy_model_render_layers`).
- **D6** Removed the "inline" asset source (`InlineAssetReader` / `InlineAssetParams`); it had
  been unused since `d0d2b9b`.
- **D7** Asset preprocessing happens in `packages/build-assets/pipeline.ts`, run by `overrun` (a
  TypeScript build orchestrator in the style of grunt/gulp), not in Bevy's asset processor.
  Bevy's processor handles 1:1 transformations well, but not merging or splitting files, which
  this pipeline needs. In watch mode (`npm run build:watch`), overrun notices Blender exports,
  writes the processed output into `assets/`, and Bevy hot-reloads it.
- **D8** `crates/bevy_saga` was a failed experiment, never integrated into the game, and has
  been removed (along with `assets/scripts/` and `src/scripting.rs`). Compiling to wasmtime is a dead end: its support for garbage-collected
  types is too immature to use. Scripting and modding will build on the SAGA prototypes in
  `bevy_reactor` instead, a bytecode VM interpreter with a possible later move to something like
  Cranelift (see X7).

## Deferred

- **X1** Migrate exemplars to BSN. *Gate:* BSN assets released, and BSN supports shared
  (non-instanced) templates. Includes updating the exemplar migration script, which lives in the
  `faery` repo (the earlier TypeScript/three.js engine), to emit BSN.
- **X2** Synthesized shared assets (successor to D6). *Gate:* a restored module needs them.
  Consider deterministic `AssetId`s from hashed params instead of a fake asset source.
- **X3** Editor UI toolkit: `bevy_ui_widgets` / `bevy_feathers`, or `bevy_reactor` as a git
  dependency (not on crates.io; prototype). *Gate:* editor restoration. Also replace
  `bevy_mod_picking` with `bevy_picking`, `@PreferencesGroup` with `SettingsGroup`.
- **X4** Replace `PropagateRenderLayers` with Bevy's `Propagate<RenderLayers>`. *Gate:* scenery,
  actors and flora restored (they are the callers).
- **X5** `bevy-inspector-egui` (pinned bevy 0.15). *Gate:* a compatible release is needed.
- **X6** Revisit asset bundling. The GLTF layout carries three.js-era assumptions, made to cut
  HTTP requests: small models (e.g. flora) are combined into one file, and large Blender
  catalogs are never split. Under Bevy, per-model files and labeled sub-assets may suit better.
  *Gate:* restoring terrain/flora/scenery, or a change to the asset pipeline (see D7).
- **X7** Scripting and modding using the SAGA language from `bevy_reactor` (SAGA: "Scripts for
  Animated Graphic Adventures"). *Gate:* the `bevy_reactor` SAGA prototype is ready to integrate.

## Notes

- `assets/shaders/outline.wgsl` is an unused duplicate of `src/materials/outline.wesl`.
- With `dynamic_linking`, run via `cargo run`; the bare binary can't find `libstd`.
