use bevy::{
    prelude::*,
    reflect::{TypePath, TypeUuid},
    render::render_resource::{AsBindGroup, ShaderRef},
};

// This is the struct that will be passed to your shader
#[derive(AsBindGroup, TypeUuid, TypePath, Debug, Clone)]
#[uuid = "e9fd10a4-0a9d-409a-ae07-0142783fc98d"]
pub struct WaterMaterial {
    #[uniform(1)]
    pub water_color: Color,

    #[uniform(2)]
    pub sky_color: [Color; 2],
    // #[texture(1)]
    // #[sampler(2)]
    // pub noise: Handle<Image>,
}

impl Material for WaterMaterial {
    fn fragment_shader() -> ShaderRef {
        "terrain/shaders/water.wgsl".into()
    }
    fn vertex_shader() -> ShaderRef {
        "terrain/shaders/water.wgsl".into()
    }
    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Blend
    }
}

#[derive(Resource, Default)]
pub struct WaterMaterialResource {
    pub handle: Handle<WaterMaterial>,
}

pub fn create_water_material(
    mut materials: ResMut<Assets<WaterMaterial>>,
    mut resource: ResMut<WaterMaterialResource>,
) {
    resource.handle = materials.add(WaterMaterial {
        water_color: Color::rgb(0.0, 0.3, 0.3),
        sky_color: [Color::rgb(0.7, 0.8, 0.9), Color::rgb(0.6, 0.7, 0.9)],
    });
}
