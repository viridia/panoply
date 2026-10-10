#![allow(dead_code)]
use bevy::{
    asset::RenderAssetUsages,
    camera::visibility::RenderLayers,
    dev_tools::infinite_grid::{InfiniteGrid, InfiniteGridPlugin},
    image::ImageSampler,
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
    settings::SettingsPlugin,
    window::ExitCondition,
};
// TODO(bevy-0.20): restore
// use bevy_inspector_egui::quick::WorldInspectorPlugin;
use bevy_remote_driver::RemoteDriverPlugin;
use panoply_core::{HudCamera, PrimaryCamera, Realm, ReservedLayers, Viewpoint};
use panoply_exemplar::ExemplarPlugin;
use std::f32::consts::PI;
use window_settings::WindowSettingsPlugin;

// TODO(bevy-0.20): restore
// #[cfg(feature = "editor")]
// mod editor;
// mod actors;
// mod portals;
// mod scenery;

mod diagnostics;
mod input;
mod materials;
mod models;
mod reflect_types;
mod terrain;
mod view;
mod window_settings;
mod world;

use crate::{
    diagnostics::ScreenDiagsPlugin, input::InputPlugin, materials::MaterialsPlugin,
    models::ModelsPlugin, reflect_types::ReflectTypesPlugin, terrain::TerrainPlugin,
    world::WorldPlugin,
};
// TODO(bevy-0.20): restore
// use crate::{actors::ActorsPlugin, portals::PortalPlugin, scenery::SceneryPlugin};

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
        // Under the remote driver, test plans get settings of their own, so a plan run never
        // changes the developer's saved window position or zoom.
        SettingsPlugin::new(if std::env::var_os("BEVY_REMOTE_DRIVER_PORT").is_some() {
            "org.viridia.panoply.driven"
        } else {
            "org.viridia.panoply"
        }),
        // Must come after SettingsPlugin.
        WindowSettingsPlugin,
        // Adds FrameTimeDiagnosticsPlugin, so must come before RemoteDriverPlugin.
        ScreenDiagsPlugin,
        RemoteDriverPlugin,
        InfiniteGridPlugin,
        InputPlugin,
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
        camera_distance: CAMERA_DISTANCE,
        elevation: PI * 0.25,
        ..default()
    })
    .init_resource::<ReservedLayers>()
    .add_systems(Startup, (setup, spawn_material_test, load_exemplar_check))
    .add_systems(Update, (run_exemplar_check, release_parcel_terrain_fx))
    .add_systems(Update, (rotate_shapes, nav_to_center))
    .add_systems(
        PostUpdate,
        (
            view::viewport::update_viewport_inset,
            view::viewport::update_camera_viewport.after(view::viewport::update_viewport_inset),
        ),
    )
    .add_plugins((
        ReflectTypesPlugin,
        ExemplarPlugin,
        MaterialsPlugin,
        WorldPlugin,
        ModelsPlugin,
        TerrainPlugin,
        // TODO(bevy-0.20): restore
        // SceneryPlugin,
        // ActorsPlugin,
        // PortalPlugin,
        // WorldInspectorPlugin::new(),
    ));

    // #[cfg(feature = "editor")]
    // app.add_plugins(editor::EditorPlugin);

    app.run();

    println!("Exited!")
}

/// Distance from the camera to the view center. Fixed in the game; the editor can zoom.
const CAMERA_DISTANCE: f32 = 20.;

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

    // Debug grid on the ground plane, visible in every realm.
    commands.spawn((
        Name::new("InfiniteGrid"),
        InfiniteGrid,
        RenderLayers::from_layers(&(0..16).collect::<Vec<_>>()),
    ));

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

/// TEMP(bevy-0.20): Spawns one object per custom material plus two GLTF models with material
/// extras, so that shader compilation errors show up at runtime. Remove once scenery and
/// actors are restored.
fn spawn_material_test(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut standard_materials: ResMut<Assets<StandardMaterial>>,
    mut outline_materials: ResMut<Assets<materials::OutlineMaterial>>,
    mut noisy_materials: ResMut<Assets<materials::FloorNoisyMaterial>>,
    flame: Res<materials::FlameMaterialHandle>,
    assets: Res<AssetServer>,
) {
    use bevy::{gltf::GltfAssetLabel, pbr::ExtendedMaterial, world_serialization::WorldAssetRoot};

    let all_layers = RenderLayers::from_layers(&(0..16).collect::<Vec<_>>());
    let cube = Mesh::from(Cuboid::default());
    let vertex_count = cube.count_vertices();
    let flame_cube = cube
        .clone()
        .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, vec![[0., 0.5, 0., 1.]; vertex_count]);
    let cube = meshes.add(cube);

    commands.spawn((
        Name::new("TestOutline"),
        Mesh3d(cube.clone()),
        MeshMaterial3d(
            standard_materials.add(StandardMaterial::from_color(Srgba::rgb(0.8, 0.7, 0.5))),
        ),
        Transform::from_xyz(-4., 1., 4.),
        all_layers.clone(),
        children![(
            Name::new("Outline"),
            Mesh3d(cube.clone()),
            MeshMaterial3d(outline_materials.add(materials::OutlineMaterial {
                width: 0.05,
                ..default()
            })),
            all_layers.clone(),
        )],
    ));
    commands.spawn((
        Name::new("TestFloorNoisy"),
        Mesh3d(cube),
        MeshMaterial3d(noisy_materials.add(ExtendedMaterial {
            base: StandardMaterial::default(),
            extension: materials::FloorNoisyMaterialExt {
                color: Srgba::rgb(0.6, 0.4, 0.2).into(),
                color_alt: Srgba::rgb(0.3, 0.3, 0.3).into(),
                roughness: 0.8,
                roughness_alt: 0.3,
                noise: assets.load("textures/noise_1.png"),
            },
        })),
        Transform::from_xyz(-4., 1., 0.),
        all_layers.clone(),
    ));
    commands.spawn((
        Name::new("TestFlame"),
        Mesh3d(meshes.add(flame_cube)),
        MeshMaterial3d(flame.0.clone()),
        Transform::from_xyz(-4., 1., -4.),
        all_layers.clone(),
    ));
    for (name, path, x) in [
        ("TestCamping", "scenery/models/camping.glb", 4.),
        ("TestHumanoid", "characters/humanoid-male.glb", 8.),
    ] {
        commands.spawn((
            Name::new(name),
            WorldAssetRoot(assets.load(GltfAssetLabel::Scene(0).from_asset(path))),
            Transform::from_xyz(x, 0., 0.),
            all_layers.clone(),
            models::PropagateRenderLayers,
        ));
    }
}

/// TEMP(bevy-0.20): Exemplars to load as a check on the exemplar loader, including one that
/// `extends` another. Only catalogs without aspects load until scenery and actors register their
/// aspect types. Remove once those are restored.
#[derive(Resource)]
struct ExemplarCheck(Vec<Handle<panoply_exemplar::Exemplar>>);

fn load_exemplar_check(mut commands: Commands, assets: Res<AssetServer>) {
    commands.insert_resource(ExemplarCheck(vec![
        assets.load("exemplars/weapons.exem.json#BattleAxe"),
        assets.load("exemplars/books.exem.json#FTP1"),
    ]));
}

/// Once every exemplar has loaded, logs each one's display name and what it extends, and spawns
/// `ExemplarCheckPassed` for a driver plan to wait on.
fn run_exemplar_check(
    mut commands: Commands,
    check: Option<Res<ExemplarCheck>>,
    exemplars: Res<Assets<panoply_exemplar::Exemplar>>,
) {
    let Some(check) = check else {
        return;
    };
    let Some(loaded) = check
        .0
        .iter()
        .map(|handle| exemplars.get(handle))
        .collect::<Option<Vec<_>>>()
    else {
        return;
    };
    let mut passed = true;
    for exemplar in loaded {
        let parent = exemplar
            .0
            .extends
            .as_ref()
            .map(|handle| exemplars.get(handle).map(|e| e.0.display_name.clone()));
        info!(
            "Exemplar check: {:?} extends {:?}",
            exemplar.0.display_name, parent
        );
        // An `extends` that never resolved would read as Some(None).
        passed &= !matches!(parent, Some(None));
    }
    if passed {
        commands.spawn(Name::new("ExemplarCheckPassed"));
    }
    commands.remove_resource::<ExemplarCheck>();
}

/// TEMP(bevy-0.20): Stands in for scenery, which normally writes each precinct's terrain effects
/// into its parcels and then clears `RebuildParcelTerrainFx`. Ground meshes and flora wait for
/// that marker to go, so without scenery they never appear. Remove when scenery is restored.
fn release_parcel_terrain_fx(
    mut commands: Commands,
    q_parcels: Query<Entity, With<panoply_terrain::RebuildParcelTerrainFx>>,
) {
    for parcel in &q_parcels {
        commands
            .entity(parcel)
            .remove::<panoply_terrain::RebuildParcelTerrainFx>();
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
    // let rname = "overland";
    let rname = "playground";
    if let Some(realm) = realms.iter().find(|r| r.1.name == rname) {
        println!("Navigating to [{}]", realm.1.name);
        viewpoint.realm = Some(realm.0);
    }
}
