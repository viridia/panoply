//! Development diagnostics: the location readout, view readiness and its survey, and profiling
//! diagnostics. Render diagnostics record CPU time per render pass (GPU time needs Vulkan or
//! DX12); test plans read them, view readiness and the survey through the remote driver.

use bevy::{color::palettes, prelude::*};
use panoply_core::{metrics::PRECINCT_SIZE_F, Realm, ViewOverlay, Viewpoint};
use panoply_terrain::PARCEL_SIZE_F;

mod hitch;
mod survey;
mod view_ready;

pub use view_ready::ViewReady;

const FONT_SIZE: f32 = 16.0;

pub struct EditorDiagnosticsPlugin;

impl Plugin for EditorDiagnosticsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            bevy::render::diagnostic::RenderDiagnosticsPlugin,
            bevy::diagnostic::EntityCountDiagnosticsPlugin::default(),
        ))
        .init_resource::<ViewReady>()
        .add_systems(Startup, (spawn_location_text, survey::register))
        .add_systems(Update, update_location_text)
        .add_systems(Last, view_ready::update_view_ready);
        // The hitch log is opt-in: `PANOPLY_HITCH_LOG=1`.
        if std::env::var_os("PANOPLY_HITCH_LOG").is_some() {
            app.add_plugins(hitch::HitchLogPlugin);
        }
    }
}

/// The marker on the text showing where the view is centered.
#[derive(Component)]
pub struct LocationText;

fn spawn_location_text(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn((
        Name::new("LocationText"),
        LocationText,
        ViewOverlay,
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
