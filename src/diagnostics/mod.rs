use std::fmt::Write;

use bevy::{
    diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin},
    prelude::*,
};

pub struct ScreenDiagsPlugin;

const FONT_SIZE: f32 = 16.0;
const FONT_COLOR: Color = Color::RED;

const STRING_FORMAT: &str = "FPS: ";
const STRING_INITIAL: &str = "FPS: ...";
const STRING_MISSING: &str = "FPS: ???";

impl Plugin for ScreenDiagsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(FrameTimeDiagnosticsPlugin::default())
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
        let value = &mut text.sections[0].value;
        value.clear();

        if let Some(fps) = fps_diags {
            write!(value, "{}{:.0}", STRING_FORMAT, fps).unwrap();
        } else {
            value.clear();
            write!(value, "{}", STRING_MISSING).unwrap();
        }
    }
}

/// Get the current fps
pub fn extract_fps(diagnostics: &Res<DiagnosticsStore>) -> Option<f64> {
    diagnostics
        .get(FrameTimeDiagnosticsPlugin::FPS)
        .and_then(|fps| fps.average())
}

fn spawn_text(mut commands: Commands, asset_server: Res<AssetServer>) {
    // let font = asset_server.load("fonts/screen-diags-font.ttf");
    let font = asset_server.load("fonts/Rubik/Rubik-VariableFont_wght.ttf");
    commands
        .spawn(TextBundle {
            text: Text {
                sections: vec![TextSection {
                    value: STRING_INITIAL.to_string(),
                    style: TextStyle {
                        font,
                        font_size: FONT_SIZE,
                        color: FONT_COLOR,
                    },
                }],
                ..Default::default()
            },
            ..Default::default()
        })
        .insert(ScreenDiagsText);
}
