#![allow(dead_code)]
use bevy::{
    asset::ChangeWatcher,
    core_pipeline::{clear_color::ClearColorConfig, tonemapping::Tonemapping},
    pbr::CascadeShadowConfigBuilder,
    prelude::*,
    render::{
        render_resource::{Extent3d, TextureDimension, TextureFormat},
        texture::ImageSampler,
    },
};
use bevy_egui::{egui, EguiContexts, EguiPlugin};
use egui::Frame;
use std::{f32::consts::PI, time::Duration};
use world::Realm;

extern crate directories;

mod diagnostics;
mod editor;
mod instancing;
mod random;
mod settings;
mod terrain;
mod view;
mod world;
use view::{PrimaryCamera, Viewpoint};

use crate::{
    diagnostics::ScreenDiagsPlugin,
    settings::{load_user_settings, update_window_settings, UserSettings, WindowSettings},
    terrain::TerrainPlugin,
    view::{update_camera_viewport, ViewportInset},
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

enum EditorState {
    World,
    Terrain,
    Scenery,
    Meta,
    Play,
}

#[derive(Resource)]
struct ToolState {
    state: EditorState,
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

    App::new()
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
                .set(AssetPlugin {
                    watch_for_changes: ChangeWatcher::with_delay(Duration::from_millis(200)),
                    ..Default::default()
                }),
            EguiPlugin,
            ScreenDiagsPlugin,
            // ImagePlugin::default_nearest(),
        ))
        .insert_resource(settings)
        .init_resource::<ViewportInset>()
        .insert_resource(Viewpoint {
            position: Vec3::new(0., 0., 0.),
            azimuth: 0.,
            camera_distance: 20.,
            elevation: PI * 0.25,
            ..default()
        })
        .insert_resource(ToolState {
            state: EditorState::World,
        })
        .add_systems(Startup, (setup, load_assets_system))
        .add_systems(
            Update,
            (
                // editor_ui_system,
                update_camera_viewport,
                rotate_shapes,
                editor::camera_controller,
                update_window_settings,
                nav_to_center,
            ),
        )
        .add_systems(Update, bevy::window::close_on_esc)
        .add_plugins((WorldPlugin, TerrainPlugin))
        .run();

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

fn editor_ui_system(
    mut contexts: EguiContexts,
    mut viewport_inset: ResMut<ViewportInset>,
    mut tool_state_res: ResMut<ToolState>,
    images: Res<EditorImages>,
) {
    let world_texture_id = contexts.add_image(images.world.clone());
    let terrain_texture_id = contexts.add_image(images.terrain.clone());
    let building_texture_id = contexts.add_image(images.building.clone());
    let quest_texture_id = contexts.add_image(images.quest.clone());
    let play_texture_id = contexts.add_image(images.play.clone());
    let ctx = contexts.ctx_mut();
    let tool_state = tool_state_res.as_mut();

    viewport_inset.left = egui::SidePanel::left("left_panel")
        .resizable(true)
        .default_width(300.)
        .min_width(300.)
        .frame(Frame {
            inner_margin: egui::Margin {
                left: 4.,
                right: 4.,
                top: 4.,
                bottom: 4.,
            },
            fill: ctx.style().visuals.window_fill,

            ..default()
        })
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 1.;

                if ui
                    .add(
                        egui::ImageButton::new(
                            world_texture_id,
                            bevy_egui::egui::Vec2::new(32., 26.),
                        )
                        .selected(matches!(tool_state.state, EditorState::World)),
                    )
                    .clicked()
                {
                    tool_state.state = EditorState::World
                }

                if ui
                    .add(
                        egui::ImageButton::new(
                            terrain_texture_id,
                            bevy_egui::egui::Vec2::new(32., 26.),
                        )
                        .selected(matches!(tool_state.state, EditorState::Terrain)),
                    )
                    .clicked()
                {
                    tool_state.state = EditorState::Terrain
                };

                if ui
                    .add(
                        egui::ImageButton::new(
                            building_texture_id,
                            bevy_egui::egui::Vec2::new(32., 26.),
                        )
                        .selected(matches!(tool_state.state, EditorState::Scenery)),
                    )
                    .clicked()
                {
                    tool_state.state = EditorState::Scenery
                };

                if ui
                    .add(
                        egui::ImageButton::new(
                            quest_texture_id,
                            bevy_egui::egui::Vec2::new(28., 26.),
                        )
                        .selected(matches!(tool_state.state, EditorState::Meta)),
                    )
                    .clicked()
                {
                    tool_state.state = EditorState::Meta
                };

                if ui
                    .add(
                        egui::ImageButton::new(
                            play_texture_id,
                            bevy_egui::egui::Vec2::new(28., 26.),
                        )
                        .selected(matches!(tool_state.state, EditorState::Play)),
                    )
                    .clicked()
                {
                    tool_state.state = EditorState::Play
                };
            });

            egui::Grid::new("tools")
                .spacing(bevy_egui::egui::Vec2 { x: 0., y: 0. })
                .show(ui, |ui| {
                    ui.button("Hello").clicked();
                    ui.button("Hello").clicked();
                    ui.end_row();

                    ui.button("Hello").clicked();
                    ui.button("Hello").clicked();
                    ui.button("Hello").clicked();
                    ui.end_row();

                    // ui.horizontal(|ui| {
                    //     ui.label("Same");
                    //     ui.label("cell");
                    // });
                    // ui.label("Third row, second column");
                    // ui.end_row();
                });

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
            intensity: 5000.0,
            range: 20.,
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_xyz(8.0, 16.0, 8.0),
        ..default()
    });

    commands.insert_resource(AmbientLight {
        brightness: 2.0,
        color: Color::Rgba {
            red: 0.5,
            green: 0.5,
            blue: 1.,
            alpha: 1.,
        },
    });

    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            shadows_enabled: true,
            color: Color::Rgba {
                red: 1.,
                green: 1.,
                blue: 1.,
                alpha: 1.,
            },
            illuminance: 17000.,
            ..default()
        },
        transform: Transform {
            translation: Vec3::new(0.0, 2.0, 0.0),
            rotation: Quat::from_rotation_x(-PI / 3.),
            ..default()
        },
        // The default cascade config is designed to handle large scenes.
        // As this example has a much smaller world, we can tighten the shadow
        // bounds for better visual quality.
        cascade_shadow_config: CascadeShadowConfigBuilder {
            first_cascade_far_bound: 4.0,
            maximum_distance: 40.0,
            ..default()
        }
        .into(),
        ..default()
    });

    // Primary Camera
    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_xyz(100.0, 100., 150.0).looking_at(Vec3::ZERO, Vec3::Y),
            camera: Camera {
                // Renders the right camera after the left camera, which has a default priority of 0
                hdr: true,
                // order: 1,
                ..default()
            },
            camera_3d: Camera3d {
                // don't clear on the second camera because the first camera already cleared the window
                clear_color: ClearColorConfig::Custom(Color::BLACK),
                ..default()
            },
            tonemapping: Tonemapping::AcesFitted,
            ..default()
        },
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
    );
    res.sampler_descriptor = ImageSampler::nearest();
    res
}

fn nav_to_center(mut viewpoint: ResMut<Viewpoint>, realms: Query<(Entity, &Realm), Added<Realm>>) {
    for (entity, realm) in realms.iter() {
        if realm.name == "playground" {
            println!("Navigating to [playground]");
            viewpoint.realm = Some(entity)
        }
    }
}
