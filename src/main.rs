#![feature(impl_trait_in_assoc_type)]
#![allow(dead_code)]
use bevy::{
    asset::io::AssetSource,
    prelude::*,
    render::{
        render_asset::RenderAssetUsages,
        render_resource::{Extent3d, TextureDimension, TextureFormat},
        texture::ImageSampler,
        view::RenderLayers,
    },
};
use bevy_mod_picking::{debug::DebugPickingMode, DefaultPickingPlugins};
use bevy_mod_preferences::PreferencesPlugin;
use bevy_quill::QuillPlugin;
use bevy_quill_obsidian::ObsidianUiPlugin;
use models::ModelsPlugin;
use panoply_exemplar::ExemplarPlugin;
// use bevy_inspector_egui::quick::WorldInspectorPlugin;
use std::f32::consts::PI;
use world::Realm;

#[cfg(feature = "editor")]
mod editor;

extern crate directories;

mod actors;
mod diagnostics;
mod materials;
mod models;
mod msgpack;
mod portals;
mod random;
mod reflect_types;
mod scenery;
mod settings;
mod terrain;
mod view;
mod world;
use view::{HudCamera, PrimaryCamera, Viewpoint};

use crate::{
    actors::ActorsPlugin,
    diagnostics::ScreenDiagsPlugin,
    materials::{InlineAssetReader, MaterialsPlugin},
    msgpack::MsgpackExtPlugin,
    portals::PortalPlugin,
    reflect_types::ReflectTypesPlugin,
    scenery::SceneryPlugin,
    settings::{load_user_settings, update_window_settings, UserSettings, WindowSettings},
    terrain::TerrainPlugin,
    world::WorldPlugin,
};

#[derive(Resource)]
struct EditorImages {
    world: Handle<Image>,
    terrain: Handle<Image>,
    building: Handle<Image>,
    quest: Handle<Image>,
    play: Handle<Image>,
}

fn main() {
    let mut settings = UserSettings {
        window: WindowSettings {
            fullscreen: false,
            position: IVec2::new(0, 0),
            size: UVec2::new(800, 600),
        },
    };

    if let Some(s) = load_user_settings() {
        settings = s
    }

    let mut app = App::new();
    app.register_asset_source(
        "inline",
        AssetSource::build().with_reader(|| Box::new(InlineAssetReader)),
    )
    .add_plugins((
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Bevy Game".into(),
                    resolution: (settings.window.size.x as f32, settings.window.size.y as f32)
                        .into(),
                    position: WindowPosition::new(settings.window.position),
                    // mode: WindowMode::SizedFullscreen,
                    ..default()
                }),
                ..default()
            })
            .set(AssetPlugin::default()),
        ScreenDiagsPlugin,
        DefaultPickingPlugins,
        QuillPlugin,
        ObsidianUiPlugin,
        PreferencesPlugin::new("panoply"),
    ))
    .init_resource::<view::viewport::ViewportInset>()
    .insert_resource(DebugPickingMode::Disabled)
    .insert_resource(settings)
    // .insert_resource(Msaa::Off)
    .insert_resource(Viewpoint {
        position: Vec3::new(0., 0., 0.),
        azimuth: 0.,
        camera_distance: 20.,
        elevation: PI * 0.25,
        ..default()
    })
    .add_systems(Startup, setup)
    .add_systems(
        Update,
        (
            rotate_shapes,
            update_window_settings,
            nav_to_center,
            view::viewport::update_viewport_inset,
            view::viewport::update_camera_viewport.after(view::viewport::update_viewport_inset),
        ),
    )
    .add_systems(Update, close_on_esc)
    .add_plugins((
        MsgpackExtPlugin,
        ReflectTypesPlugin,
        ExemplarPlugin,
        MaterialsPlugin,
        WorldPlugin,
        TerrainPlugin,
        SceneryPlugin,
        ActorsPlugin,
        PortalPlugin,
        ModelsPlugin,
        // WorldInspectorPlugin::new(),
    ));

    #[cfg(feature = "editor")]
    app.add_plugins(editor::EditorPlugin);

    app.run();

    println!("Exited!")
}

fn load_assets_system(mut commands: Commands, assets: Res<AssetServer>) {
    commands.insert_resource(EditorImages {
        world: assets.load("editor/icons/world.png"),
        terrain: assets.load("editor/icons/terrain.png"),
        building: assets.load("editor/icons/building.png"),
        quest: assets.load("editor/icons/quest.png"),
        play: assets.load("editor/icons/play.png"),
    });
}

/// A marker component for our shapes so we can query them separately from the ground plane
#[derive(Component)]
struct Shape;

const X_EXTENT: f32 = 14.5;

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let debug_material = materials.add(StandardMaterial {
        base_color_texture: Some(images.add(uv_debug_texture())),
        ..default()
    });

    let shapes = [
        meshes.add(Cuboid::default()),
        meshes.add(Capsule3d::default()),
        meshes.add(Torus::default()),
        meshes.add(Cylinder::default()),
        meshes.add(Sphere::default().mesh().ico(5).unwrap()),
        meshes.add(Sphere::default().mesh().uv(32, 18)),
    ];

    let num_shapes = shapes.len();

    for (i, shape) in shapes.into_iter().enumerate() {
        commands.spawn((
            PbrBundle {
                mesh: shape,
                material: debug_material.clone(),
                transform: Transform::from_xyz(
                    0.0,
                    2.0,
                    -X_EXTENT / 2. + i as f32 / (num_shapes - 1) as f32 * X_EXTENT,
                )
                .with_rotation(Quat::from_rotation_x(-PI / 4.)),
                ..default()
            },
            Shape,
        ));
    }

    commands.insert_resource(AmbientLight {
        brightness: 0.5 * 1000.,
        color: Srgba {
            red: 0.5,
            green: 0.7,
            blue: 1.,
            alpha: 1.,
        }
        .into(),
    });

    // TODO: Move to 'view' module
    // Ui Camera
    commands.spawn((
        Camera2dBundle {
            camera: Camera {
                // HUD goes on top of 3D
                order: 1,
                clear_color: ClearColorConfig::None,
                ..default()
            },
            ..default()
        },
        HudCamera, // UiCameraConfig { show_ui: true },
    ));

    // Primary Camera
    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_xyz(100.0, 100., 150.0).looking_at(Vec3::ZERO, Vec3::Y),
            camera: Camera {
                // Renders the 3d view first,
                order: 0,
                clear_color: ClearColorConfig::Custom(Color::BLACK),
                ..default()
            },
            // tonemapping: Tonemapping::AcesFitted,
            ..default()
        },
        RenderLayers::none(),
        PrimaryCamera,
    ));
}

fn rotate_shapes(mut query: Query<&mut Transform, With<Shape>>, time: Res<Time>) {
    for mut transform in &mut query {
        transform.rotate_y(time.delta_seconds() / 2.);
    }
}

/// Creates a colorful test pattern
fn uv_debug_texture() -> Image {
    const TEXTURE_SIZE: usize = 8;

    let mut palette: [u8; 32] = [
        255, 102, 159, 255, 255, 159, 102, 255, 236, 255, 102, 255, 121, 255, 102, 255, 102, 255,
        198, 255, 102, 198, 255, 255, 121, 102, 255, 255, 236, 102, 255, 255,
    ];

    let mut texture_data = [0; TEXTURE_SIZE * TEXTURE_SIZE * 4];
    for y in 0..TEXTURE_SIZE {
        let offset = TEXTURE_SIZE * y * 4;
        texture_data[offset..(offset + TEXTURE_SIZE * 4)].copy_from_slice(&palette);
        palette.rotate_right(4);
    }

    let mut res = Image::new_fill(
        Extent3d {
            width: TEXTURE_SIZE as u32,
            height: TEXTURE_SIZE as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &texture_data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    res.sampler = ImageSampler::nearest();
    res
}

fn nav_to_center(mut viewpoint: ResMut<Viewpoint>, realms: Query<(Entity, &Realm), Added<Realm>>) {
    for (entity, realm) in realms.iter() {
        if realm.name == "overland" {
            println!("Navigating to [overland]");
            viewpoint.realm = Some(entity)
        }
    }
}

pub fn close_on_esc(input: Res<ButtonInput<KeyCode>>, mut exit: EventWriter<AppExit>) {
    if input.just_pressed(KeyCode::Escape) {
        exit.send(AppExit::Success);
    }
}
