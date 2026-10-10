//! Drive a running Bevy app from outside, for automated tests.
//!
//! Add [`RemoteDriverPlugin`] to your app, and a test client can find entities by name, wait for
//! scenes to finish spawning, count frames and click on UI nodes, all over the [Bevy Remote
//! Protocol](bevy_remote). Everything the protocol already does, such as querying components,
//! writing input messages and taking screenshots, the client does through the protocol's own
//! methods.
//!
//! ```ignore
//! App::new()
//!     .add_plugins((DefaultPlugins, RemoteDriverPlugin))
//!     .run();
//! ```
//!
//! The plugin does nothing unless the app is started with `BEVY_REMOTE_DRIVER_PORT` set, so it can
//! stay in every build: a player who starts the game normally gets no open port.
//!
//! ```sh
//! BEVY_REMOTE_DRIVER_PORT=15702 cargo run
//! ```
//!
//! # Naming entities for a test
//!
//! A test selects entities by a path of [`Name`](bevy_ecs::name::Name)s, such as
//! `["Settings", "Audio", "Volume"]`: an entity named `Volume` somewhere below one named `Audio`,
//! itself somewhere below `Settings`. Entities without a name in between are skipped, so a name is
//! needed only on what a test refers to. In a BSN scene, `#Volume` gives an entity its name.
//!
//! # Plugins the driver adds
//!
//! When it is active, the plugin adds [`RemotePlugin`] and [`RemoteHttpPlugin`] itself, so do not
//! add them as well. It also adds [`FrameTimeDiagnosticsPlugin`] unless the app already has it,
//! which is how a test counts frames. If your app adds that plugin too, add it before this one.

#![forbid(unsafe_code)]

mod diagnostics;
mod locate;
mod ready;
mod select;

use bevy_app::{App, Plugin};
use bevy_diagnostic::FrameTimeDiagnosticsPlugin;
use bevy_remote::{RemotePlugin, http::RemoteHttpPlugin};

pub use ready::SceneReady;

const PORT_VARIABLE: &str = "BEVY_REMOTE_DRIVER_PORT";

/// Lets a test client drive this app over the Bevy Remote Protocol.
///
/// Inactive unless `BEVY_REMOTE_DRIVER_PORT` is set when the app starts. See the [crate
/// documentation](crate) for what it adds when active.
pub struct RemoteDriverPlugin;

impl Plugin for RemoteDriverPlugin {
    fn build(&self, app: &mut App) {
        if let Some(port) = parse_port(std::env::var(PORT_VARIABLE)) {
            install(app, port);
        }
    }
}

// A malformed value is a mistake in the test setup, and running without the port would turn it into
// a client timing out on a connection with no word as to why.
fn parse_port(value: Result<String, std::env::VarError>) -> Option<u16> {
    match value {
        Err(std::env::VarError::NotPresent) => None,
        Err(err) => panic!("{PORT_VARIABLE} is not readable: {err}"),
        Ok(text) => Some(
            text.trim()
                .parse()
                .unwrap_or_else(|_| panic!("{PORT_VARIABLE} is {text:?}, which is not a port")),
        ),
    }
}

fn install(app: &mut App, port: u16) {
    // `is_plugin_added` sees only plugins added before this one, which is the ordering rule the
    // crate docs state.
    if !app.is_plugin_added::<FrameTimeDiagnosticsPlugin>() {
        app.add_plugins(FrameTimeDiagnosticsPlugin::default());
    }
    app.add_plugins((
        RemotePlugin::default()
            .with_method_main(select::METHOD, select::process_request)
            .with_method_main(locate::METHOD, locate::process_request)
            .with_method_main(diagnostics::METHOD, diagnostics::process_request),
        RemoteHttpPlugin::default().with_port(port),
    ))
    .register_type::<SceneReady>()
    .add_observer(ready::mark_ready);
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_remote::RemoteMethods;
    use std::env::VarError;

    #[test]
    fn an_unset_variable_leaves_the_driver_off() {
        assert_eq!(parse_port(Err(VarError::NotPresent)), None);
        assert_eq!(parse_port(Ok("15702".into())), Some(15702));
        assert_eq!(parse_port(Ok(" 15702\n".into())), Some(15702));
    }

    #[test]
    #[should_panic(expected = "which is not a port")]
    fn a_malformed_port_is_refused() {
        parse_port(Ok("fifteen".into()));
    }

    #[test]
    fn installing_registers_the_methods() {
        let mut app = App::new();
        install(&mut app, 0);
        let methods = app.world().resource::<RemoteMethods>();
        for method in [select::METHOD, locate::METHOD, diagnostics::METHOD] {
            assert!(methods.get(method).is_some(), "{method} is not registered");
        }
    }
}
