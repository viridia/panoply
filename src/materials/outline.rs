use bevy::{
    mesh::MeshVertexBufferLayoutRef,
    pbr::{MaterialPipeline, MaterialPipelineKey},
    prelude::*,
    render::render_resource::{
        AsBindGroup, Face, RenderPipelineDescriptor, SpecializedMeshPipelineError,
    },
    shader::ShaderRef,
};

/// An outline drawn by normal extrusion (an inverted hull): the mesh is redrawn slightly
/// expanded along its normals, with front faces culled, so only the rim behind the original
/// shows. Put it on a child entity that shares the mesh, never as a second material on the mesh
/// entity itself (see `models::process_material_extras`).
///
/// Unlit, and kept out of the depth prepass and shadow maps: an outline neither casts shadows
/// nor needs lighting.
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct OutlineMaterial {
    /// Line color.
    #[uniform(0)]
    pub color: LinearRgba,

    /// Extrusion distance along the normal, in the mesh's local units.
    #[uniform(1)]
    pub width: f32,
}

impl Default for OutlineMaterial {
    fn default() -> Self {
        Self {
            color: LinearRgba::BLACK,
            width: 0.015,
        }
    }
}

impl Material for OutlineMaterial {
    fn vertex_shader() -> ShaderRef {
        "embedded://panoply/materials/outline.wesl".into()
    }

    fn fragment_shader() -> ShaderRef {
        "embedded://panoply/materials/outline.wesl".into()
    }

    fn enable_prepass() -> bool {
        false
    }

    fn enable_shadows() -> bool {
        false
    }

    fn specialize(
        _pipeline: &MaterialPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef,
        _key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        descriptor.primitive.cull_mode = Some(Face::Front);
        Ok(())
    }
}
