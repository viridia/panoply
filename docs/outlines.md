# Outlines

Outlines are central to the art style: art nouveau, specifically Mucha. This note records how
they are drawn and why, so the choice is not re-litigated each Bevy upgrade.

## Technique

Normal extrusion, also called an inverted hull: the mesh is drawn a second time, pushed out along
its normals, with front faces culled, so only a rim of the expanded back faces shows around the
silhouette. This is an aesthetic choice. Screen-space approaches (jump flooding, or edge
detection on the prepass's depth and normals) give a different line and are ruled out for
outlines.

Which objects have outlines:

- Actors and scenery: yes, by a GLTF material extra, `"outline": <width>`.
- Terrain contours: no.
- Flora: the leaf textures have outlines baked in; trunks use the outline material.

## Implementation

`materials::OutlineMaterial` is a small `Material` of its own (`src/materials/outline.rs`,
`outline.wesl`), not an extension of `StandardMaterial`:

- unlit, one color and one width (in the mesh's local units);
- front faces culled, in `specialize`;
- kept out of the depth prepass and shadow maps (`enable_prepass`, `enable_shadows`);
- vertex input goes through Bevy's `forward_io::Vertex` and `decompress_vertex`, and skinned
  meshes use `skinning::skin_model`, so a character's outline follows its pose.

Each outline is a **child entity** that shares the mesh (and `SkinnedMesh`, when there is one) of
the entity it outlines; see `models::process_material_extras`. An entity with two
`MeshMaterial3d` components fails wgpu validation in Bevy 0.20 (the shadow pass binds one
material's data under the other's pipeline), so a second material on the mesh entity is not an
option.

Hard-edged meshes with split normals, such as a cube, extrude each face separately and leave gaps
at the corners. Models that need continuous outlines want smoothed normals for extrusion.

## Related: x-ray gizmos

The editor once also used multiple materials for semi-transparent gizmos, partially occluded so
that one could see where a gizmo penetrates the scene. That is a different need from outlines,
never on the same object, and editor-only. When the editor returns, it should be a custom render
phase that depth-tests against the already-rendered scene (Bevy's
`examples/shader_advanced/custom_render_phase.rs` is the starting point, but with the view's depth
attachment loaded rather than none). That part of Bevy changes most between releases.

## Verifying

`plans/outlines.py` looks at the test objects from `spawn_material_test` (an outlined cube, the
camping model, the skinned humanoid) at the closest camera distance, against a light grey clear
color so that black outlines show.
