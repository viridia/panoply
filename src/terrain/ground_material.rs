use bevy::{
    prelude::*,
    reflect::TypePath,
    render::render_resource::{AsBindGroup, ShaderRef},
};

// This is the struct that will be passed to your shader
#[derive(AsBindGroup, TypePath, Debug, Clone, Asset)]
pub struct GroundMaterial {
    #[texture(1)]
    #[sampler(2)]
    pub noise: Handle<Image>,
    #[texture(3)]
    #[sampler(4)]
    pub grass: Handle<Image>,
    #[texture(5)]
    #[sampler(6)]
    pub dirt: Handle<Image>,
    #[texture(7)]
    #[sampler(8)]
    pub moss: Handle<Image>,

    #[texture(9, sample_type = "u_int")]
    #[sampler(10)]
    pub biomes: Handle<Image>,

    #[uniform(11)]
    pub water_color: Color,

    #[uniform(12)]
    pub realm_offset: Vec2,
}

impl Material for GroundMaterial {
    fn fragment_shader() -> ShaderRef {
        "terrain/shaders/ground.wgsl".into()
    }

    fn vertex_shader() -> ShaderRef {
        "terrain/shaders/ground.wgsl".into()
    }
}
