//! `driver.locate`: where a UI node is drawn, so a client can click on it (DD3.3).

use bevy_camera::{NormalizedRenderTarget, RenderTarget};
use bevy_ecs::{
    entity::{ContainsEntity, Entity},
    query::With,
    system::{In, Query},
};
use bevy_remote::{BrpError, BrpResult, builtin_methods::parse_some, error_codes};
use bevy_ui::{ComputedUiTargetCamera, UiGlobalTransform};
use bevy_window::{PrimaryWindow, Window};
use serde_json::{Value, json};

use crate::select::{PathParams, Selector};

pub(crate) const METHOD: &str = "driver.locate";

fn refused(message: String) -> BrpError {
    BrpError {
        code: error_codes::INVALID_PARAMS,
        message,
        data: None,
    }
}

pub(crate) fn process_request(
    In(params): In<Option<Value>>,
    selector: Selector,
    nodes: Query<(&UiGlobalTransform, &ComputedUiTargetCamera)>,
    targets: Query<&RenderTarget>,
    primary: Query<Entity, With<PrimaryWindow>>,
    windows: Query<&Window>,
) -> BrpResult {
    let PathParams { path } = parse_some(params)?;
    let shown = path.join("/");
    let entity = selector.select_one(&path)?;
    let Ok((transform, camera)) = nodes.get(entity) else {
        return Err(refused(format!("{shown} is not a UI node")));
    };
    let primary = primary.single().ok();
    let window = match camera.get() {
        Some(camera) => match targets.get(camera).ok().and_then(|t| t.normalize(primary)) {
            Some(NormalizedRenderTarget::Window(window)) => Some(window.entity()),
            _ => return Err(refused(format!("{shown} is not drawn to a window"))),
        },
        None => primary,
    };
    let Some((window, scale)) = window.and_then(|w| Some((w, windows.get(w).ok()?.scale_factor())))
    else {
        return Err(refused(format!("{shown} has no window to be drawn in")));
    };
    // The transform is in physical pixels, and `CursorMoved` takes logical ones.
    let logical = transform.translation / scale;
    Ok(json!({ "window": window, "logical": [logical.x, logical.y] }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_ecs::{name::Name, world::World};
    use bevy_math::Vec2;
    use bevy_window::WindowResolution;

    fn locate(world: &mut World, path: &str) -> BrpResult {
        world
            .run_system_cached_with(process_request, Some(json!({ "path": [path] })))
            .unwrap()
    }

    #[test]
    fn the_centre_is_reported_in_logical_pixels() {
        let mut world = World::new();
        let resolution = WindowResolution::new(1600, 1200).with_scale_factor_override(2.0);
        let window = world
            .spawn((
                Window {
                    resolution,
                    ..Default::default()
                },
                PrimaryWindow,
            ))
            .id();
        world.spawn((
            Name::new("Start"),
            UiGlobalTransform::from_translation(Vec2::new(400.0, 300.0)),
            ComputedUiTargetCamera::default(),
        ));

        let found = locate(&mut world, "Start").unwrap();

        assert_eq!(found["window"], json!(window));
        assert_eq!(found["logical"], json!([200.0, 150.0]));
    }

    #[test]
    fn an_entity_that_is_not_a_node_is_refused() {
        let mut world = World::new();
        world.spawn((Window::default(), PrimaryWindow));
        world.spawn(Name::new("Start"));

        let err = locate(&mut world, "Start").unwrap_err();

        assert!(err.message.contains("not a UI node"), "{}", err.message);
    }

    #[test]
    fn several_matches_are_listed_in_the_error() {
        let mut world = World::new();
        world.spawn(Name::new("Start"));
        world.spawn(Name::new("Start"));

        let err = locate(&mut world, "Start").unwrap_err();

        assert!(
            err.message.contains("matched 2 entities"),
            "{}",
            err.message
        );
        assert_eq!(err.data.unwrap().as_array().unwrap().len(), 2);
    }
}
