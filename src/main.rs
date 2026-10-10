#![allow(dead_code)]
use bevy::{
    asset::RenderAssetUsages,
    camera::visibility::RenderLayers,
    image::ImageSampler,
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
    settings::{SaveSettingsSync, SettingsPlugin},
    window::ExitCondition,
};
// TODO(bevy-0.20): restore
// use bevy_saga::{SagaPlugin, SagaVmResource, ScriptAsset};
// use bevy_inspector_egui::quick::WorldInspectorPlugin;
// use models::ModelsPlugin;
// use panoply_exemplar::ExemplarPlugin;
use panoply_core::{
    pan_rotate::pan_rotate_viewpoint, HudCamera, PrimaryCamera, Realm, ReservedLayers, Viewpoint,
};
use std::f32::consts::PI;
use window_settings::WindowSettingsPlugin;

// TODO(bevy-0.20): restore
// #[cfg(feature = "editor")]
// mod editor;
// mod actors;
// mod materials;
// mod models;
// mod portals;
// mod reflect_types;
// mod scenery;
// mod scripting;
// mod terrain;

mod diagnostics;
mod view;
mod window_settings;
mod world;

use crate::{diagnostics::ScreenDiagsPlugin, world::WorldPlugin};
// TODO(bevy-0.20): restore
// use crate::{
//     actors::ActorsPlugin, materials::MaterialsPlugin, portals::PortalPlugin,
//     reflect_types::ReflectTypesPlugin, scenery::SceneryPlugin, terrain::TerrainPlugin,
// };

// TODO(bevy-0.20): restore with editor
// #[derive(Resource)]
// struct EditorImages {
//     world: Handle<Image>,
//     terrain: Handle<Image>,
//     building: Handle<Image>,
//     quest: Handle<Image>,
//     play: Handle<Image>,
// }

fn main() {
    let mut app = App::new();
    // app.register_asset_source(
    //     "inline",
    //     AssetSource::build().with_reader(|| Box::new(InlineAssetReader)),
    // )
    app.add_plugins((
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Untitled Bevy Game".into(),
                    ..default()
                }),
                // Intercept exit so that settings can be saved.
                exit_condition: ExitCondition::DontExit,
                ..default()
            })
            .set(AssetPlugin::default()),
        SettingsPlugin::new("org.viridia.panoply"),
        // Must come after SettingsPlugin.
        WindowSettingsPlugin,
        ScreenDiagsPlugin,
    ))
    .init_resource::<view::viewport::ViewportInset>()
    // .insert_resource(RaycastBackendSettings {
    //     require_markers: true,
    //     ..default()
    // })
    // .insert_resource(settings)
    // .insert_resource(Msaa::Off)
    .insert_resource(Viewpoint {
        position: Vec3::new(0., 0., 0.),
        azimuth: 0.,
        camera_distance: 60.,
        elevation: PI * 0.25,
        ..default()
    })
    .init_resource::<ReservedLayers>()
    // TODO(bevy-0.20): restore with scripting
    // .init_resource::<TestScriptHandle>()
    // .add_systems(Startup, run_script)
    // .add_systems(Update, receive_script_assets)
    .add_systems(Startup, setup)
    .add_systems(Update, (rotate_shapes, nav_to_center, pan_rotate_viewpoint))
    .add_systems(
        PostUpdate,
        (
            view::viewport::update_viewport_inset,
            view::viewport::update_camera_viewport.after(view::viewport::update_viewport_inset),
        ),
    )
    .add_systems(Update, close_on_esc)
    .add_plugins((
        WorldPlugin,
        // TODO(bevy-0.20): restore
        // ReflectTypesPlugin,
        // ExemplarPlugin,
        // MaterialsPlugin,
        // TerrainPlugin,
        // SceneryPlugin,
        // ActorsPlugin,
        // PortalPlugin,
        // ModelsPlugin,
        // SagaPlugin,
        // WorldInspectorPlugin::new(),
    ));

    // #[cfg(feature = "editor")]
    // app.add_plugins(editor::EditorPlugin);

    app.run();

    println!("Exited!")
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
    assets: Res<AssetServer>,
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
            Name::new("DebugShape"),
            Mesh3d(shape),
            MeshMaterial3d(debug_material.clone()),
            Transform::from_xyz(
                0.0,
                2.0,
                -X_EXTENT / 2. + i as f32 / (num_shapes - 1) as f32 * X_EXTENT,
            )
            .with_rotation(Quat::from_rotation_x(-PI / 4.)),
            Shape,
            RenderLayers::from_layers(&[0, 1, 2, 3, 4, 5]),
        ));
    }

    // commands.insert_resource(AmbientLight {
    //     brightness: 1000.,
    //     color: Srgba {
    //         red: 0.2,
    //         green: 0.5,
    //         blue: 1.,
    //         alpha: 1.,
    //     }
    //     .into(),
    // });

    // TODO: Move to 'view' module
    // Ui Camera
    commands.spawn((
        Name::new("HudCamera"),
        Camera2d,
        Camera {
            // HUD goes on top of 3D
            order: 1,
            clear_color: ClearColorConfig::None,
            ..default()
        },
        HudCamera,
    ));

    // Primary Camera
    commands.spawn((
        Name::new("PrimaryCamera"),
        Camera3d::default(),
        Transform::from_xyz(100.0, 100., 150.0).looking_at(Vec3::ZERO, Vec3::Y),
        Camera {
            // Renders the 3d view first,
            order: 0,
            clear_color: ClearColorConfig::Custom(Color::BLACK),
            ..default()
        },
        EnvironmentMapLight {
            diffuse_map: assets.load("skybox_cubemap/skybox_diffuse.ktx2"),
            specular_map: assets.load("skybox_cubemap/skybox_specular.ktx2"),
            intensity: 200.0,
            ..Default::default()
        },
        RenderLayers::none(),
        PrimaryCamera,
    ));
}

fn rotate_shapes(mut query: Query<&mut Transform, With<Shape>>, time: Res<Time>) {
    for mut transform in &mut query {
        transform.rotate_y(time.delta_secs() / 2.);
    }
}

// TODO(bevy-0.20): restore with scripting
// #[derive(Debug, Resource, Default)]
// struct TestScriptHandle(Handle<ScriptAsset>);
//
// fn run_script(server: Res<AssetServer>, mut handle: ResMut<TestScriptHandle>) {
//     let script = server.load::<ScriptAsset>("scripts/hello.saga");
//     handle.0 = script;
// }
//
// /** React when precinct assets change and update the scenery. */
// pub fn receive_script_assets(
//     // mut commands: Commands,
//     mut ev_asset: EventReader<AssetEvent<ScriptAsset>>,
//     assets: ResMut<Assets<ScriptAsset>>,
//     // asset_server: Res<AssetServer>,
//     r_vm: Res<SagaVmResource>,
// ) {
//     for ev in ev_asset.read() {
//         match ev {
//             AssetEvent::Added { id } => {
//                 info!("Script asset added: {:?}", id);
//                 let script = assets.get(*id).unwrap();
//                 let mut vm = r_vm.0.lock().unwrap();
//                 let result = script.call::<(), i32>(&mut vm, "hello", ()).unwrap();
//                 info!("Script result: {:?}", result);
//             }
//
//             AssetEvent::LoadedWithDependencies { id } | AssetEvent::Modified { id } => {
//                 info!("Script asset loaded: {:?}", id);
//
//                 // if let Some((precinct_entity, mut precinct, precinct_children)) =
//                 //     q_precincts.iter_mut().find(|r| r.1.asset.id() == *id)
//                 // {
//                 //     // TODO: Sync cutaway rects
//                 //     // TODO: Sync nav mesh, physics, light sources, particles, etc.
//                 //     // TODO: Sync actors
//
//                 //     let precinct_asset = assets.get(*id).unwrap();
//                 //     let floor_exemplars: Vec<Handle<Exemplar>> = precinct_asset
//                 //         .floor_types
//                 //         .iter()
//                 //         .map(|s| asset_server.load(s))
//                 //         .collect();
//
//                 // }
//             }
//
//             AssetEvent::Removed { id: _ } => {}
//
//             AssetEvent::Unused { id: _ } => {}
//         }
//     }
// }

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
    // let rname = "overland";
    let rname = "playground";
    if let Some(realm) = realms.iter().find(|r| r.1.name == rname) {
        println!("Navigating to [{}]", realm.1.name);
        viewpoint.realm = Some(realm.0);
    }
}

pub fn close_on_esc(input: Res<ButtonInput<KeyCode>>, mut commands: Commands) {
    if input.just_pressed(KeyCode::Escape) {
        commands.queue(SaveSettingsSync::IfChanged);
        commands.write_message(AppExit::Success);
    }
}
