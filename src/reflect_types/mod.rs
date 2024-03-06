use bevy::app::{App, Plugin};

mod hex_color;

pub use hex_color::HexColor;

pub struct ReflectTypesPlugin;

impl Plugin for ReflectTypesPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<HexColor>()
            .register_type::<Option<HexColor>>();
    }
}
