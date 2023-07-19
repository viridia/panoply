use bevy::{
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};
use bevy_egui::{egui, EguiContexts, EguiPlugin};
use std::f32::consts::PI;

extern crate directories;

mod editor;
mod settings;
mod view;
use view::{PrimaryCamera, Viewpoint};

use crate::{
    settings::{load_user_settings, update_window_settings, UserSettings, WindowSettings},
    view::{update_camera_viewport, ViewportInset},
};

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

    App::new()
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Bevy Game".into(),
                    resolution: (settings.window.size.x as f32, settings.window.size.y as f32)
                        .into(),
                    position: WindowPosition::new(settings.window.position),
                    // mode: WindowMode::SizedFullscreen,
                    ..default()
                }),
                ..default()
            }),
            EguiPlugin,
            // ImagePlugin::default_nearest(),
        ))
        .insert_resource(settings)
        .init_resource::<ViewportInset>()
        .insert_resource(Viewpoint {
            position: Vec3::new(0., 0., 0.),
            azimuth: 0.,
            camera_distance: 32.,
            elevation: PI * 0.25,
            ..default()
        })
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                editor_ui_system,
                update_camera_viewport,
                rotate,
                editor::editor_camera_controller,
                update_window_settings,
            ),
        )
        .add_systems(Update, bevy::window::close_on_esc)
        .run();

    println!("Exited!")
}

fn editor_ui_system(mut contexts: EguiContexts, mut viewport_inset: ResMut<ViewportInset>) {
    let ctx = contexts.ctx_mut();

    viewport_inset.left = egui::SidePanel::left("left_panel")
        .resizable(true)
        .default_width(300.)
        .min_width(300.)
        .show(ctx, |ui| {
            ui.allocate_rect(ui.available_rect_before_wrap(), egui::Sense::hover());
        })
        .response
        .rect
        .width();
    viewport_inset.right = egui::SidePanel::right("right_panel")
        .resizable(true)
        .default_width(300.)
        .min_width(300.)
        .show(ctx, |ui| {
            ui.allocate_rect(ui.available_rect_before_wrap(), egui::Sense::hover());
        })
        .response
        .rect
        .width();
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
        meshes.add(shape::Cube::default().into()),
        meshes.add(shape::Box::default().into()),
        meshes.add(shape::Capsule::default().into()),
        meshes.add(shape::Torus::default().into()),
        meshes.add(shape::Cylinder::default().into()),
        meshes.add(shape::Icosphere::default().try_into().unwrap()),
        meshes.add(shape::UVSphere::default().into()),
    ];

    let num_shapes = shapes.len();

    for (i, shape) in shapes.into_iter().enumerate() {
        commands.spawn((
            PbrBundle {
                mesh: shape,
                material: debug_material.clone(),
                transform: Transform::from_xyz(
                    -X_EXTENT / 2. + i as f32 / (num_shapes - 1) as f32 * X_EXTENT,
                    2.0,
                    0.0,
                )
                .with_rotation(Quat::from_rotation_x(-PI / 4.)),
                ..default()
            },
            Shape,
        ));
    }

    commands.spawn(PointLightBundle {
        point_light: PointLight {
            intensity: 9000.0,
            range: 30.,
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_xyz(8.0, 16.0, 8.0),
        ..default()
    });

    // ground plane
    commands.spawn(PbrBundle {
        mesh: meshes.add(shape::Plane::from_size(50.0).into()),
        material: materials.add(Color::SILVER.into()),
        ..default()
    });

    // Primary Camera
    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_xyz(100.0, 100., 150.0).looking_at(Vec3::ZERO, Vec3::Y),
            camera: Camera {
                // Renders the right camera after the left camera, which has a default priority of 0
                order: 1,
                ..default()
            },
            camera_3d: Camera3d {
                // don't clear on the second camera because the first camera already cleared the window
                // clear_color: ClearColorConfig::None,
                ..default()
            },
            ..default()
        },
        PrimaryCamera,
    ));
}

fn rotate(mut query: Query<&mut Transform, With<Shape>>, time: Res<Time>) {
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

    Image::new_fill(
        Extent3d {
            width: TEXTURE_SIZE as u32,
            height: TEXTURE_SIZE as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &texture_data,
        TextureFormat::Rgba8UnormSrgb,
    )
}
