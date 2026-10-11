use bevy::{
    color::palettes,
    diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin},
    prelude::*,
};
use panoply_core::{metrics::PRECINCT_SIZE_F, Realm, Viewpoint};
use panoply_terrain::PARCEL_SIZE_F;

#[cfg(feature = "editor")]
mod hitch;

pub struct ScreenDiagsPlugin;

const FONT_SIZE: f32 = 16.0;
const FONT_COLOR: Srgba = palettes::basic::RED;

const STRING_FORMAT: &str = "FPS: ";
const STRING_INITIAL: &str = "FPS: ...";
const STRING_MISSING: &str = "FPS: ???";

impl Plugin for ScreenDiagsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(FrameTimeDiagnosticsPlugin::default())
            .add_systems(Startup, spawn_text)
            .add_systems(Update, update);

        // The location readout and profiling diagnostics are development aids, so they ship only
        // with the editor. Render diagnostics record CPU time per render pass (GPU time needs
        // Vulkan or DX12); test plans read both through the remote driver.
        #[cfg(feature = "editor")]
        app.add_plugins((
            bevy::render::diagnostic::RenderDiagnosticsPlugin,
            bevy::diagnostic::EntityCountDiagnosticsPlugin::default(),
            hitch::HitchLogPlugin,
        ))
        .add_systems(Startup, spawn_location_text)
        .add_systems(Update, update_location_text);
    }
}

/// The marker on the text to be updated.
#[derive(Component)]
pub struct ScreenDiagsText;

fn update(
    diagnostics: Res<DiagnosticsStore>,
    mut text_query: Query<&mut Text, With<ScreenDiagsText>>,
) {
    let fps_diags = extract_fps(&diagnostics);

    for mut text in text_query.iter_mut() {
        if let Some(fps) = fps_diags {
            text.0 = format!("{}{:.0}", STRING_FORMAT, fps);
        } else {
            text.0 = STRING_MISSING.to_string();
        }
    }
}

/// Get the current fps
pub fn extract_fps(diagnostics: &Res<DiagnosticsStore>) -> Option<f64> {
    diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FPS)
        .and_then(|fps| fps.smoothed())
}

fn spawn_text(mut commands: Commands, asset_server: Res<AssetServer>) {
    // let font = asset_server.load("fonts/screen-diags-font.ttf");
    let font = asset_server.load("fonts/Rubik/Rubik-VariableFont_wght.ttf");
    commands
        .spawn((
            Text::new(STRING_INITIAL),
            TextFont {
                font: FontSource::Handle(font),
                font_size: FontSize::Px(FONT_SIZE),
                ..default()
            },
            TextColor(FONT_COLOR.into()),
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(10.0),
                top: Val::Px(10.0),
                left: Val::Auto,
                ..default()
            },
        ))
        .insert(ScreenDiagsText);
}

/// The marker on the text showing where the view is centered.
#[derive(Component)]
pub struct LocationText;

fn spawn_location_text(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn((
        Name::new("LocationText"),
        LocationText,
        Text::new(""),
        TextFont {
            font: FontSource::Handle(asset_server.load("fonts/Rubik/Rubik-VariableFont_wght.ttf")),
            font_size: FontSize::Px(FONT_SIZE),
            ..default()
        },
        TextColor(palettes::basic::YELLOW.into()),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(10.0),
            top: Val::Px(10.0),
            ..default()
        },
    ));
}

/// Shows the realm and view center, and the parcel and precinct it falls in, so that a problem
/// spotted on screen can be located in the map.
fn update_location_text(
    viewpoint: Res<Viewpoint>,
    q_realms: Query<&Realm>,
    mut q_text: Query<&mut Text, With<LocationText>>,
) {
    let Ok(mut text) = q_text.single_mut() else {
        return;
    };
    let realm = viewpoint
        .realm
        .and_then(|e| q_realms.get(e).ok())
        .map_or("(none)", |r| r.name.as_str());
    let p = viewpoint.position;
    let parcel = (p.xz() / PARCEL_SIZE_F).floor();
    let precinct = (p.xz() / PRECINCT_SIZE_F).floor();
    let value = format!(
        "{realm}  x {:.1}  z {:.1}\nparcel {} {}  precinct {} {}\nzoom {:.0}",
        p.x,
        p.z,
        parcel.x,
        parcel.y,
        precinct.x,
        precinct.y,
        viewpoint.camera_distance(),
    );
    if text.0 != value {
        text.0 = value;
    }
}
