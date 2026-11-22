//! The aperture's material: the portal camera's image, read at each fragment's place on screen,
//! so that the aperture shows it as a window would. The camera renders only the aperture's
//! rectangle on screen, into the corner of its image.

use bevy::{
    mesh::MeshVertexBufferLayoutRef,
    pbr::{MaterialPipeline, MaterialPipelineKey},
    prelude::*,
    render::render_resource::{AsBindGroup, RenderPipelineDescriptor, SpecializedMeshPipelineError},
    shader::ShaderRef,
};

const SHADER: &str = "embedded://panoply_portals/assets/aperture.wesl";

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub(crate) struct ApertureMaterial {
    /// Where the image's corner lies in the window, in physical pixels, in `xy`.
    #[uniform(0)]
    pub(crate) origin: Vec4,
    #[texture(1)]
    pub(crate) image: Handle<Image>,
}

impl Material for ApertureMaterial {
    fn fragment_shader() -> ShaderRef {
        SHADER.into()
    }

    /// Seen from both sides, for a portal open from both.
    fn specialize(
        _pipeline: &MaterialPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef,
        _key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        descriptor.primitive.cull_mode = None;
        Ok(())
    }
}
