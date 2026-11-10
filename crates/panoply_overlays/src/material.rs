//! An overlay's two materials: one for where it is in front of the scene, and a fainter one for
//! where the scene hides it, so an overlay that runs into the ground or a wall still shows there.

use bevy::{
    mesh::MeshVertexBufferLayoutRef,
    pbr::{MaterialPipeline, MaterialPipelineKey},
    prelude::*,
    render::render_resource::{
        AsBindGroup, CompareFunction, RenderPipelineDescriptor, SpecializedMeshPipelineError,
    },
    shader::ShaderRef,
};

const SHADER: &str = "embedded://panoply_overlays/assets/overlay.wesl";

/// An overlay's color.
#[derive(Component, Clone, Copy, Debug, PartialEq, Reflect)]
#[reflect(Component, Default)]
pub struct OverlayColor {
    pub base: Color,
    /// The opacity, as a fraction of `base`'s, of the parts the scene hides; 0 hides them.
    pub underlay: f32,
}

impl Default for OverlayColor {
    fn default() -> Self {
        Self {
            base: Color::WHITE.with_alpha(0.5),
            underlay: 0.3,
        }
    }
}

/// Draws where the overlay is in front of the scene.
#[derive(Debug, Clone, AsBindGroup, Asset, TypePath, Default)]
pub(crate) struct OverlayMaterial {
    #[uniform(0)]
    pub(crate) color: LinearRgba,
}

/// Draws where the scene hides the overlay.
#[derive(Debug, Clone, AsBindGroup, Asset, TypePath, Default)]
pub(crate) struct UnderlayMaterial {
    #[uniform(0)]
    pub(crate) color: LinearRgba,
}

/// Added to an overlay's sort distance, so it draws after the scene's blended surfaces (water).
/// Sorted by its center among them, a large overlay would change places with each as the view
/// turned, and be tinted by them or not.
const DRAW_LAST: f32 = 1.0e6;

/// Blended, so overlays stay out of the prepass and do not disturb SSAO; nothing writes depth.
/// Depth is reversed-Z: greater is nearer.
fn specialize_depth(descriptor: &mut RenderPipelineDescriptor, compare: CompareFunction) {
    if let Some(depth_stencil) = descriptor.depth_stencil.as_mut() {
        depth_stencil.depth_write_enabled = Some(false);
        depth_stencil.depth_compare = Some(compare);
    }
    descriptor.primitive.cull_mode = None;
}

impl Material for OverlayMaterial {
    fn fragment_shader() -> ShaderRef {
        SHADER.into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Blend
    }

    fn depth_bias(&self) -> f32 {
        DRAW_LAST
    }

    fn specialize(
        _pipeline: &MaterialPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef,
        _key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        specialize_depth(descriptor, CompareFunction::GreaterEqual);
        Ok(())
    }
}

impl Material for UnderlayMaterial {
    fn fragment_shader() -> ShaderRef {
        SHADER.into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Blend
    }

    fn depth_bias(&self) -> f32 {
        DRAW_LAST
    }

    fn specialize(
        _pipeline: &MaterialPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef,
        _key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        specialize_depth(descriptor, CompareFunction::Less);
        Ok(())
    }
}
