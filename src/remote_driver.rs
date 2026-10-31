//! Lets `bevy-remote-driver` run plans against the game.

use bevy::{
    diagnostic::FrameTimeDiagnosticsPlugin,
    prelude::*,
    remote::{RemotePlugin, http::RemoteHttpPlugin},
};

/// Enables the Bevy Remote Protocol on the port the driver gives in `BEVY_REMOTE_DRIVER_PORT`, and
/// the frame count it reads. Without the variable it adds nothing, so a normal start opens no port.
pub struct RemoteDriverPlugin;

impl Plugin for RemoteDriverPlugin {
    fn build(&self, app: &mut App) {
        if let Ok(port) = std::env::var("BEVY_REMOTE_DRIVER_PORT") {
            let port = port.trim().parse().expect("BEVY_REMOTE_DRIVER_PORT is not a port");
            app.add_plugins((RemotePlugin::default(), RemoteHttpPlugin::default().with_port(port)));
            // `ScreenDiagsPlugin` adds it when it comes first.
            if !app.is_plugin_added::<FrameTimeDiagnosticsPlugin>() {
                app.add_plugins(FrameTimeDiagnosticsPlugin::default());
            }
        }
    }
}
