//! The nav points in a [`Chooser`], and travel to one by name.

use bevy::prelude::*;
use panoply_core::{GotoLocation, NavPointsAsset, NavPointsHandleResource};

use crate::chooser::{chooser, Chooser};

/// A chooser that lists the nav points as they change.
#[derive(Component, Default, Clone)]
pub struct NavPointChooser;

pub struct NavPointListPlugin;

impl Plugin for NavPointListPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, list_nav_points);
    }
}

/// A chooser of the nav points. Its filter box must not take the focus: in the panel, the keys
/// that move the view would type into it.
pub fn nav_point_chooser() -> impl Scene {
    bsn! {
        @chooser(false, Vec::new(), None)
        NavPointChooser
    }
}

/// Copies the nav points' names to each [`NavPointChooser`] that differs.
fn list_nav_points(
    handle: Res<NavPointsHandleResource>,
    assets: Res<Assets<NavPointsAsset>>,
    mut choosers: Query<&mut Chooser, With<NavPointChooser>>,
) {
    let Some(points) = assets.get(&handle.0) else {
        return;
    };
    for mut chooser in &mut choosers {
        if !chooser.items.iter().eq(points.0.iter().map(|p| &p.name)) {
            chooser.items = points.0.iter().map(|p| p.name.clone()).collect();
        }
    }
}

/// The nav points' names, for a chooser opened over them.
pub(crate) fn nav_point_names(
    handle: &NavPointsHandleResource,
    assets: &Assets<NavPointsAsset>,
) -> Vec<String> {
    assets
        .get(&handle.0)
        .map(|points| points.0.iter().map(|p| p.name.clone()).collect())
        .unwrap_or_default()
}

/// Moves the viewpoint to a nav point, by name.
#[derive(bevy::ecs::system::SystemParam)]
pub(crate) struct Travel<'w> {
    handle: Res<'w, NavPointsHandleResource>,
    assets: Res<'w, Assets<NavPointsAsset>>,
    gotos: MessageWriter<'w, GotoLocation>,
}

impl Travel<'_> {
    /// Moves the viewpoint to the point named `name`, if there is one.
    pub(crate) fn to(&mut self, name: &str) {
        let point = self
            .assets
            .get(&self.handle.0)
            .and_then(|points| points.0.iter().find(|p| p.name == name));
        if let Some(point) = point {
            self.gotos.write(GotoLocation {
                realm: point.realm.clone(),
                position: point.pos,
                ..default()
            });
        }
    }
}
