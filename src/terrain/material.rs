use bevy::{
    prelude::*,
    reflect::{TypePath, TypeUuid},
    render::render_resource::{AsBindGroup, ShaderRef},
};

// This is the struct that will be passed to your shader
#[derive(AsBindGroup, TypeUuid, TypePath, Debug, Clone)]
#[uuid = "c666955b-c00f-4de6-a4d6-49c9581d6139"]
pub struct TerrainMaterial {
    #[texture(1)]
    #[sampler(2)]
    noise: Handle<Image>,
    #[texture(3)]
    #[sampler(4)]
    grass: Handle<Image>,
    #[texture(5)]
    #[sampler(6)]
    dirt: Handle<Image>,
}

/// The Material trait is very configurable, but comes with sensible defaults for all methods.
/// You only need to implement functions for features that need non-default behavior. See the Material api docs for details!
impl Material for TerrainMaterial {
    fn fragment_shader() -> ShaderRef {
        "terrain/shaders/ground.wgsl".into()
    }
    fn vertex_shader() -> ShaderRef {
        "terrain/shaders/ground.wgsl".into()
    }
}

#[derive(Resource, Default)]
pub struct TerrainMaterials {
    pub ground: Handle<TerrainMaterial>,
}

pub fn create_materials(
    mut terrain_materials: ResMut<TerrainMaterials>,
    mut materials: ResMut<Assets<TerrainMaterial>>,
    asset_server: Res<AssetServer>,
) {
    terrain_materials.ground = materials.add(TerrainMaterial {
        noise: asset_server.load("terrain/textures/noise.png"),
        grass: asset_server.load("terrain/textures/grass.png"),
        dirt: asset_server.load("terrain/textures/dirt.png"),
    });
}
