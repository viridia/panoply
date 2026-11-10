//! Each editor mode's overlays, spawned as the mode is entered and despawned as it is left. Realm
//! mode's: a grid on the ground at parcel and precinct boundaries, and the realm's border.

use bevy::{app::Propagate, camera::visibility::RenderLayers, prelude::*};
use panoply_core::{metrics::PRECINCT_SIZE, Realm, Viewpoint};
use panoply_overlays::{AddOverlay, DrawPlane, OverlayColor, OverlayShape, ShapeBuilder};
use panoply_terrain::PARCEL_SIZE;

use crate::mode_selector::EditorMode;

/// The grid's height, in meters.
const GRID_HEIGHT: f32 = 0.05;

/// The view realm's map, in parcels; empty while the view is in no realm.
#[derive(Component, Clone, PartialEq, Default)]
pub struct RealmGrid {
    parcels: IRect,
}

impl OverlayShape for RealmGrid {
    fn draw(&self, builder: &mut ShapeBuilder) {
        // A realm with no parcels has no map to outline.
        if self.parcels.is_empty() {
            return;
        }
        let map = self.parcels.as_rect();
        let map = Rect::from_corners(map.min * PARCEL_SIZE as f32, map.max * PARCEL_SIZE as f32);
        // Just above flat ground at 0, where its depth would tie with the ground's.
        builder.with_draw_plane(DrawPlane::XZ).with_offset(GRID_HEIGHT);
        builder.with_stroke_width(0.1).grid(map, PARCEL_SIZE as f32);
        // Over the parcel lines, so a precinct's are brighter as well as wider.
        builder.with_stroke_width(0.3).grid(map, PRECINCT_SIZE as f32);
        // Outside the map, so the border does not hide its outer parcels' edges.
        builder.with_stroke_width(0.5).stroke_rect(map.inflate(0.5));
    }
}

pub struct ModeOverlaysPlugin;

impl Plugin for ModeOverlaysPlugin {
    fn build(&self, app: &mut App) {
        app.add_overlay::<RealmGrid>()
            .add_systems(OnEnter(EditorMode::Realm), spawn_realm_grid)
            .add_systems(Update, update_realm_grid.run_if(in_state(EditorMode::Realm)));
    }
}

fn spawn_realm_grid(mut commands: Commands) {
    commands.spawn((
        Name::new("RealmGrid"),
        RealmGrid::default(),
        OverlayColor {
            base: Color::WHITE.with_alpha(0.15),
            underlay: 0.3,
        },
        Propagate(RenderLayers::none()),
        Transform::default(),
        Visibility::default(),
        DespawnOnExit(EditorMode::Realm),
    ));
}

/// Follows the view's realm: its map and its render layer.
fn update_realm_grid(
    viewpoint: Res<Viewpoint>,
    realms: Query<&Realm>,
    mut grids: Query<(&mut RealmGrid, &mut Propagate<RenderLayers>)>,
) {
    let realm = viewpoint.realm.and_then(|e| realms.get(e).ok());
    let parcels = realm.map_or(IRect::default(), |r| r.parcel_bounds);
    let layer = realm.map_or(RenderLayers::none(), |r| r.layer.clone());
    for (mut grid, mut propagate) in &mut grids {
        grid.set_if_neq(RealmGrid { parcels });
        propagate.set_if_neq(Propagate(layer.clone()));
    }
}
