//! `driver.diagnostics`: the latest value of each diagnostic, by path (DD3.4).
//!
//! A stopgap: nothing in `bevy_diagnostic` is reflected, so BRP cannot read the store itself.

use bevy_diagnostic::DiagnosticsStore;
use bevy_ecs::system::{In, Res};
use bevy_remote::BrpResult;
use serde_json::{Map, Value};

pub(crate) const METHOD: &str = "driver.diagnostics";

pub(crate) fn process_request(In(_): In<Option<Value>>, store: Res<DiagnosticsStore>) -> BrpResult {
    let values: Map<String, Value> = store
        .iter()
        .filter(|diagnostic| diagnostic.is_enabled)
        .filter_map(|diagnostic| Some((diagnostic.path().to_string(), diagnostic.value()?.into())))
        .collect();
    Ok(Value::Object(values))
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_app::{App, TaskPoolPlugin};
    use bevy_diagnostic::{DiagnosticsPlugin, FrameCountPlugin, FrameTimeDiagnosticsPlugin};
    use bevy_time::TimePlugin;

    fn frame_count(app: &mut App) -> f64 {
        let values = app
            .world_mut()
            .run_system_cached_with(process_request, None)
            .unwrap()
            .unwrap();
        values["frame_count"].as_f64().unwrap()
    }

    #[test]
    fn the_frame_count_advances_one_per_update() {
        let mut app = App::new();
        app.add_plugins((
            TaskPoolPlugin::default(),
            TimePlugin,
            FrameCountPlugin,
            DiagnosticsPlugin,
            FrameTimeDiagnosticsPlugin::default(),
        ));
        app.update();
        let before = frame_count(&mut app);

        app.update();
        app.update();

        assert_eq!(frame_count(&mut app), before + 2.0);
    }
}
