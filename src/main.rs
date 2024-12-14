#![allow(dead_code)]
use bevy::{
    asset::io::AssetSource,
    ecs::world::Command,
    image::ImageSampler,
    prelude::*,
    render::{
        render_asset::RenderAssetUsages,
        render_resource::{Extent3d, TextureDimension, TextureFormat},
        view::RenderLayers,
    },
};
use bevy_saga::{SagaPlugin, SagaVmResource, ScriptAsset};
use bevy_user_prefs::{AutosavePrefsPlugin, Preferences, SavePreferences};
// use bevy_inspector_egui::quick::WorldInspectorPlugin;
use models::ModelsPlugin;
use panoply_core::{HudCamera, PrimaryCamera, Realm, ReservedLayers, Viewpoint};
use panoply_exemplar::ExemplarPlugin;
use std::f32::consts::PI;
use window_settings::{load_window_settings, WindowSettingsPlugin};

// #[cfg(feature = "editor")]
// mod editor;

mod actors;
mod diagnostics;
mod materials;
mod models;
mod portals;
mod reflect_types;
mod scenery;
mod scripting;
mod terrain;
mod view;
mod window_settings;
mod world;

use crate::{
    actors::ActorsPlugin,
    diagnostics::ScreenDiagsPlugin,
    materials::{InlineAssetReader, MaterialsPlugin},
    portals::PortalPlugin,
    reflect_types::ReflectTypesPlugin,
    scenery::SceneryPlugin,
    terrain::TerrainPlugin,
    window_settings::update_window_settings,
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
    let mut prefs = Preferences::new("org.viridia.panoply");
    let mut window = Window {
        title: "Untitled Bevy Game".into(),
        ..default()
    };
    load_window_settings(&mut prefs, &mut window);

    let mut app = App::new();
    app.register_asset_source(
        "inline",
        AssetSource::build().with_reader(|| Box::new(InlineAssetReader)),
    )
    .add_plugins((
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(window),
                ..default()
            })
            .set(AssetPlugin::default()),
        ScreenDiagsPlugin,
        AutosavePrefsPlugin,
        WindowSettingsPlugin,
        // PreferencesPlugin::new("panoply"),
    ))
    .insert_resource(prefs)
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
    .init_resource::<TestScriptHandle>()
    .add_systems(Startup, (setup, run_script))
    .add_systems(
        Update,
        (rotate_shapes, update_window_settings, nav_to_center),
    )
    .add_systems(
        PostUpdate,
        (
            view::viewport::update_viewport_inset,
            view::viewport::update_camera_viewport.after(view::viewport::update_viewport_inset),
        ),
    )
    .add_systems(Update, close_on_esc)
    .add_systems(Update, receive_script_assets)
    // .add_systems(Update, watch_prefs_changes)
    .add_plugins((
        ReflectTypesPlugin,
        ExemplarPlugin,
        MaterialsPlugin,
        WorldPlugin,
        TerrainPlugin,
        SceneryPlugin,
        ActorsPlugin,
        PortalPlugin,
        ModelsPlugin,
        // ScriptsPlugin,
        SagaPlugin,
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
        RayCastPickable,
    ));
}

fn rotate_shapes(mut query: Query<&mut Transform, With<Shape>>, time: Res<Time>) {
    for mut transform in &mut query {
        transform.rotate_y(time.delta_secs() / 2.);
    }
}

#[derive(Debug, Resource, Default)]
struct TestScriptHandle(Handle<ScriptAsset>);

fn run_script(server: Res<AssetServer>, mut handle: ResMut<TestScriptHandle>) {
    let script = server.load::<ScriptAsset>("scripts/hello.saga");
    handle.0 = script;
}

/** React when precinct assets change and update the scenery. */
pub fn receive_script_assets(
    // mut commands: Commands,
    mut ev_asset: EventReader<AssetEvent<ScriptAsset>>,
    assets: ResMut<Assets<ScriptAsset>>,
    // asset_server: Res<AssetServer>,
    r_vm: Res<SagaVmResource>,
) {
    for ev in ev_asset.read() {
        match ev {
            AssetEvent::Added { id } => {
                info!("Script asset added: {:?}", id);
                let script = assets.get(*id).unwrap();
                let mut vm = r_vm.0.lock().unwrap();
                let result = script.call::<(), i32>(&mut vm, "hello", ()).unwrap();
                info!("Script result: {:?}", result);
            }

            AssetEvent::LoadedWithDependencies { id } | AssetEvent::Modified { id } => {
                info!("Script asset loaded: {:?}", id);

                // if let Some((precinct_entity, mut precinct, precinct_children)) =
                //     q_precincts.iter_mut().find(|r| r.1.asset.id() == *id)
                // {
                //     // TODO: Sync cutaway rects
                //     // TODO: Sync nav mesh, physics, light sources, particles, etc.
                //     // TODO: Sync actors

                //     let precinct_asset = assets.get(*id).unwrap();
                //     let floor_exemplars: Vec<Handle<Exemplar>> = precinct_asset
                //         .floor_types
                //         .iter()
                //         .map(|s| asset_server.load(s))
                //         .collect();

                // }
            }

            AssetEvent::Removed { id: _ } => {}

            AssetEvent::Unused { id: _ } => {}
        }
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
            viewpoint.realm = Some(entity);
            // viewpoint.set_camera_distance(10., 10.);
        }
    }
}

pub fn close_on_esc(input: Res<ButtonInput<KeyCode>>, mut commands: Commands) {
    if input.just_pressed(KeyCode::Escape) {
        commands.queue(SavePreferences::IfChanged);
        commands.queue(AppExitCmd);
    }
}

pub struct AppExitCmd;

impl Command for AppExitCmd {
    fn apply(self, world: &mut World) {
        world.send_event(AppExit::Success);
    }
}
