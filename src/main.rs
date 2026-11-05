#![allow(dead_code)]
use bevy::{
    camera::visibility::RenderLayers,
    prelude::*,
    settings::SettingsPlugin,
    window::ExitCondition,
};
// TODO(X5): restore
// use bevy_inspector_egui::quick::WorldInspectorPlugin;
use panoply_core::{HudCamera, PanoplyCorePlugin, PrimaryCamera, Realm, ReservedLayers, Viewpoint};
use panoply_exemplar::ExemplarPlugin;
use std::f32::consts::PI;
use window_settings::WindowSettingsPlugin;

// REFERENCE(PAN4, PAN5): uncompiled, deleted as the rewrites replace them.
// #[cfg(feature = "editor")]
// mod editor;
// mod portals;

mod diagnostics;
mod input;
mod remote_driver;
mod view;
mod window_settings;

use crate::{
    diagnostics::ScreenDiagsPlugin, input::InputPlugin, remote_driver::RemoteDriverPlugin,
};
use panoply_actors::ActorsPlugin;
use panoply_render::PanoplyRenderPlugin;
use panoply_scenery::PanoplySceneryPlugin;
use panoply_terrain::PanoplyTerrainPlugin;
// TODO(PAN4): restore
// use crate::portals::PortalPlugin;

// REFERENCE(PAN5)
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
        // Plans run with `BEVY_SETTINGS_DIR` set to a directory of their own, so a plan run never
        // changes the developer's saved window position or zoom.
        SettingsPlugin::new("org.viridia.panoply"),
        // Must come after SettingsPlugin.
        WindowSettingsPlugin,
        // Adds FrameTimeDiagnosticsPlugin, so must come before RemoteDriverPlugin.
        ScreenDiagsPlugin,
        RemoteDriverPlugin,
        InputPlugin,
    ))
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
    .add_systems(Startup, setup)
    .add_systems(Update, nav_to_center)
    .add_systems(PostUpdate, view::viewport::update_camera_viewport)
    .add_plugins((
        ExemplarPlugin,
        PanoplyCorePlugin,
        PanoplyRenderPlugin,
        PanoplyTerrainPlugin,
        PanoplySceneryPlugin,
        ActorsPlugin,
        // TODO(PAN4, X5): restore
        // PortalPlugin,
        // WorldInspectorPlugin::new(),
    ));

    #[cfg(feature = "editor")]
    app.add_plugins(panoply_editor::EditorPlugin);

    app.run();

    println!("Exited!")
}

/// Distance from the camera to the view center. Fixed in the game; the editor can zoom.
const CAMERA_DISTANCE: f32 = 20.;

fn setup(mut commands: Commands) {
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
        // No camera-wide EnvironmentMapLight: image-based lighting costs about 4 ms a frame
        // whatever the cubemap size. Ambient light comes from `HemisphereLight`; shiny objects
        // should get `LightProbe` volumes instead.
        RenderLayers::none(),
        PrimaryCamera,
        // Textures are painted to be seen as they are, as in the three.js version (which had no
        // tone mapping). Bevy's default, TonyMcMapface, desaturates the palette. Lighting is kept
        // low enough (`Sunlight`, `HemisphereLight`) that little clips. Linear rather than None,
        // which is the same mapping but turns off deband dithering.
        bevy::render::view::Tonemapping::Linear,
        // Sharper shadow edges than the default Gaussian filter, and cheaper (about 0.5 ms).
        bevy::light::ShadowFilteringMethod::Hardware2x2,
    ));
}

fn nav_to_center(mut viewpoint: ResMut<Viewpoint>, realms: Query<(Entity, &Realm), Added<Realm>>) {
    // let rname = "overland";
    let rname = "playground";
    if let Some(realm) = realms.iter().find(|r| r.1.name == rname) {
        println!("Navigating to [{}]", realm.1.name);
        viewpoint.realm = Some(realm.0);
    }
}
