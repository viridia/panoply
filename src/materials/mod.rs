use bevy::{
    asset::embedded_asset, pbr::ExtendedMaterial, prelude::*, render::render_resource::Face,
};

mod flame;
mod floor_noisy;
mod outline;

pub use flame::FlameMaterial;
pub use floor_noisy::{FloorNoisyMaterial, FloorNoisyMaterialExt};
pub use outline::{OutlineMaterial, OutlineMaterialExtension};

#[derive(Debug, Clone, Resource, Default)]
pub struct OutlineMaterialHandle(pub Handle<OutlineMaterial>);

#[derive(Debug, Clone, Resource, Default)]
pub struct BlackMaterialHandle(pub Handle<StandardMaterial>);

#[derive(Debug, Clone, Resource, Default)]
pub struct FlameMaterialHandle(pub Handle<FlameMaterial>);

pub struct MaterialsPlugin;

impl Plugin for MaterialsPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "flame.wesl");
        embedded_asset!(app, "flame_prepass.wesl");
        embedded_asset!(app, "outline.wesl");
        app.init_resource::<OutlineMaterialHandle>()
            .init_resource::<BlackMaterialHandle>()
            .init_resource::<FlameMaterialHandle>()
            .add_plugins(MaterialPlugin::<FloorNoisyMaterial>::default())
            .add_plugins(MaterialPlugin::<OutlineMaterial>::default())
            .add_plugins(MaterialPlugin::<FlameMaterial>::default())
            .add_systems(
                Startup,
                (
                    create_outline_material,
                    create_black_material,
                    create_flame_material,
                ),
            );
    }
}

fn create_outline_material(
    mut materials: ResMut<Assets<OutlineMaterial>>,
    mut r_outline: ResMut<OutlineMaterialHandle>,
) {
    r_outline.0 = materials.add(ExtendedMaterial {
        base: StandardMaterial {
            base_color: Color::BLACK,
            unlit: true,
            cull_mode: Some(Face::Front),
            ..Default::default()
        },
        extension: OutlineMaterialExtension { width: 0.015 },
    });
}

fn create_black_material(
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut r_black: ResMut<BlackMaterialHandle>,
) {
    r_black.0 = materials.add(StandardMaterial {
        base_color: Srgba::BLACK.into(),
        unlit: true,
        ..Default::default()
    });
}

pub fn create_flame_material(
    mut materials: ResMut<Assets<FlameMaterial>>,
    mut resource: ResMut<FlameMaterialHandle>,
    asset_server: Res<AssetServer>,
) {
    resource.0 = materials.add(FlameMaterial {
        noise: asset_server.load("textures/noise_1.png"),
    });
}
