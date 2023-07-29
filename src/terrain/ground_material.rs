use bevy::{
    prelude::*,
    reflect::{TypePath, TypeUuid},
    render::render_resource::{AsBindGroup, ShaderRef},
};

// This is the struct that will be passed to your shader
#[derive(AsBindGroup, TypeUuid, TypePath, Debug, Clone)]
#[uuid = "c666955b-c00f-4de6-a4d6-49c9581d6139"]
pub struct GroundMaterial {
    #[texture(1)]
    #[sampler(2)]
    noise: Handle<Image>,
    #[texture(3)]
    #[sampler(4)]
    grass: Handle<Image>,
    #[texture(5)]
    #[sampler(6)]
    dirt: Handle<Image>,
    #[texture(7)]
    #[sampler(8)]
    moss: Handle<Image>,

    #[uniform(9)]
    water_color: Color,
}

/// The Material trait is very configurable, but comes with sensible defaults for all methods.
/// You only need to implement functions for features that need non-default behavior. See the Material api docs for details!
impl Material for GroundMaterial {
    fn fragment_shader() -> ShaderRef {
        "terrain/shaders/ground.wgsl".into()
    }
    fn vertex_shader() -> ShaderRef {
        "terrain/shaders/ground.wgsl".into()
    }
}

#[derive(Resource, Default)]
pub struct TerrainMaterials {
    pub ground: Handle<GroundMaterial>,
}

pub fn create_materials(
    mut terrain_materials: ResMut<TerrainMaterials>,
    mut materials: ResMut<Assets<GroundMaterial>>,
    asset_server: Res<AssetServer>,
) {
    terrain_materials.ground = materials.add(GroundMaterial {
        noise: asset_server.load("terrain/textures/noise.png"),
        grass: asset_server.load("terrain/textures/grass.png"),
        dirt: asset_server.load("terrain/textures/dirt.png"),
        moss: asset_server.load("terrain/textures/moss.png"),
        water_color: Color::rgb(0.0, 0.1, 0.3),
    });
}