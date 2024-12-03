use bevy::{
    color::palettes,
    diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin},
    prelude::*,
};

pub struct ScreenDiagsPlugin;

const FONT_SIZE: f32 = 16.0;
const FONT_COLOR: Srgba = palettes::basic::RED;

const STRING_FORMAT: &str = "FPS: ";
const STRING_INITIAL: &str = "FPS: ...";
const STRING_MISSING: &str = "FPS: ???";

impl Plugin for ScreenDiagsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(FrameTimeDiagnosticsPlugin)
            .add_systems(Startup, spawn_text)
            .add_systems(Update, update);
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
                font,
                font_size: FONT_SIZE,
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
