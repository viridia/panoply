//! The realm panel's map size control: eight buttons that move each of the map's four edges out or
//! in by a parcel. The buttons are laid out as the map is seen from the south, then the control is
//! turned with the camera's azimuth and squashed by its tilt, so each edge's buttons lie on the
//! side of the control where that edge lies on screen. Each arrow points the way its edge moves.
//!
//! A press resizes the view realm's terrain map, an edit (`terrain_map_edit.rs`).

use std::f32::consts::PI;

use bevy::{
    feathers::{constants::icons, controls::FeathersButton, display::icon},
    prelude::*,
    ui_widgets::Activate,
};
use panoply_core::Viewpoint;

use crate::{mode_selector::EditorMode, terrain_map_edit::TerrainMapEdits};

/// The grid's side cells, which hold the buttons' short sides, and its middle cell.
const CELL: f32 = 16.0;
const MIDDLE: f32 = 32.0;
const GAP: f32 = 2.0;
/// The grid's width and height.
const GRID: f32 = 4.0 * CELL + MIDDLE + 4.0 * GAP;
/// The width the control sweeps as it turns: the diagonal of the box around the outermost
/// buttons' far corners, hypot(MIDDLE, GRID), rounded up.
const SWEEP: f32 = 110.0;

/// The control's root, which reserves the space it sweeps, since layout ignores its transforms.
#[derive(Component, Default, Clone)]
struct MapSizeControl;

/// Squashes the turned grid vertically by the camera's tilt. A node of its own, since a
/// `UiTransform` scales before it rotates.
#[derive(Component, Default, Clone)]
struct MapSizeSquash;

/// The grid of buttons, turned with the camera.
#[derive(Component, Default, Clone)]
struct MapSizeRotor;

/// A button's change to the map's bounds, in parcels: one of `min` and `max` moves one edge by
/// one parcel.
#[derive(Component, Default, Clone, Debug)]
struct MapEdge {
    min: IVec2,
    max: IVec2,
}

impl MapEdge {
    /// The bounds after a press, or `None` if it would leave the map less than a parcel deep.
    /// Growing a map with no parcels makes it one parcel deep on the other axis too, so that it
    /// has a parcel to show.
    fn resized(&self, bounds: IRect) -> Option<IRect> {
        let mut resized = IRect {
            min: bounds.min + self.min,
            max: bounds.max + self.max,
        };
        let grows = self.min.cmplt(IVec2::ZERO).any() || self.max.cmpgt(IVec2::ZERO).any();
        if grows {
            resized.max = resized.max.max(resized.min + IVec2::ONE);
        } else if resized.size().cmplt(IVec2::ONE).any() {
            return None;
        }
        Some(resized)
    }

    /// The arrow's clockwise turn from pointing right, as a fraction of a full turn: it points the
    /// way the edge moves, with +y down the grid.
    fn arrow_turn(&self) -> f32 {
        match (self.min + self.max).signum().to_array() {
            [1, 0] => 0.0,
            [0, 1] => 0.25,
            [-1, 0] => 0.5,
            _ => 0.75,
        }
    }
}

pub struct MapSizePlugin;

impl Plugin for MapSizePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, follow_camera.run_if(in_state(EditorMode::Realm)));
    }
}

/// The control, as faery's: each edge's two buttons on a line out from the middle, the outer one
/// growing the map.
pub(crate) fn map_size_control() -> impl Scene {
    let (n, s, w, e) = (IVec2::NEG_Y, IVec2::Y, IVec2::NEG_X, IVec2::X);
    bsn! {
        #MapSizeControl
        MapSizeControl
        Node {
            width: px(SWEEP),
            height: px(SWEEP),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            align_self: AlignSelf::Center,
        }
        Children [
            MapSizeSquash
            Node { width: px(GRID), height: px(GRID), flex_shrink: 0.0 }
            UiTransform
            Children [
                #MapSizeRotor
                MapSizeRotor
                Node {
                    display: Display::Grid,
                    width: percent(100),
                    height: percent(100),
                    grid_template_columns: {tracks()},
                    grid_template_rows: {tracks()},
                    column_gap: px(GAP),
                    row_gap: px(GAP),
                }
                UiTransform
                Children [
                    @edge_button("GrowNorth", 3, 1, n, IVec2::ZERO)
                    --
                    @edge_button("ShrinkNorth", 3, 2, s, IVec2::ZERO)
                    --
                    @edge_button("GrowWest", 1, 3, w, IVec2::ZERO)
                    --
                    @edge_button("ShrinkWest", 2, 3, e, IVec2::ZERO)
                    --
                    @edge_button("ShrinkEast", 4, 3, IVec2::ZERO, w)
                    --
                    @edge_button("GrowEast", 5, 3, IVec2::ZERO, e)
                    --
                    @edge_button("ShrinkSouth", 3, 4, IVec2::ZERO, n)
                    --
                    @edge_button("GrowSouth", 3, 5, IVec2::ZERO, s)
                ]
            ]
        ]
    }
}

/// The grid's columns, or rows: two side cells either side of the middle.
fn tracks() -> Vec<RepeatedGridTrack> {
    vec![
        RepeatedGridTrack::px(2, CELL),
        RepeatedGridTrack::px(1, MIDDLE),
        RepeatedGridTrack::px(2, CELL),
    ]
}

/// A button in the grid's `column` and `row` (from 1) that moves the map's bounds by `min` and
/// `max`.
fn edge_button(name: &'static str, column: i16, row: i16, min: IVec2, max: IVec2) -> impl Scene {
    let turn = MapEdge { min, max }.arrow_turn();
    bsn! {
        @FeathersButton {
            @caption: bsn! {
                @icon(icons::CHEVRON_RIGHT)
                Node { height: px(10) }
                UiTransform { rotation: {Rot2::turn_fraction(turn)} }
            }
        }
        Node {
            grid_column: {GridPlacement::start(column)},
            grid_row: {GridPlacement::start(row)},
            height: Val::Auto,
            padding: px(0),
        }
        Name({name})
        MapEdge { min: {min}, max: {max} }
        on(resize_map)
    }
}

/// Turns the control with the camera's azimuth and squashes it by its tilt, as the ground is
/// foreshortened on screen.
fn follow_camera(
    viewpoint: Res<Viewpoint>,
    mut controls: Query<&mut Node, With<MapSizeControl>>,
    mut squashes: Query<&mut UiTransform, (With<MapSizeSquash>, Without<MapSizeRotor>)>,
    mut rotors: Query<&mut UiTransform, With<MapSizeRotor>>,
) {
    let squash = viewpoint.elevation.sin();
    for mut node in &mut controls {
        let height = px(SWEEP * squash);
        if node.height != height {
            node.height = height;
        }
    }
    for mut transform in &mut squashes {
        transform.set_if_neq(UiTransform::from_scale(Vec2::new(1.0, squash)));
    }
    // The grid's top is north (-z), which the camera shows at the top when it faces north, at an
    // azimuth of PI.
    let rotation = Rot2::radians(viewpoint.azimuth + PI);
    for mut transform in &mut rotors {
        transform.set_if_neq(UiTransform::from_rotation(rotation));
    }
}

/// Observer on each button: resizes the view realm's terrain map.
fn resize_map(activate: On<Activate>, edges: Query<&MapEdge>, mut edits: TerrainMapEdits) {
    let Ok(edge) = edges.get(activate.event_target()) else {
        return;
    };
    edits.edit("Resize map", |map| {
        let Some(bounds) = edge.resized(map.bounds) else {
            return false;
        };
        map.resize(bounds);
        true
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    const GROW_EAST: MapEdge = MapEdge {
        min: IVec2::ZERO,
        max: IVec2::X,
    };
    const SHRINK_NORTH: MapEdge = MapEdge {
        min: IVec2::Y,
        max: IVec2::ZERO,
    };

    #[test]
    fn a_press_moves_one_edge() {
        let bounds = IRect::new(-2, -2, 2, 2);
        assert_eq!(GROW_EAST.resized(bounds), Some(IRect::new(-2, -2, 3, 2)));
        assert_eq!(SHRINK_NORTH.resized(bounds), Some(IRect::new(-2, -1, 2, 2)));
    }

    #[test]
    fn a_map_keeps_at_least_one_parcel() {
        assert_eq!(SHRINK_NORTH.resized(IRect::new(0, 0, 3, 1)), None);
    }

    #[test]
    fn growing_an_empty_map_makes_one_parcel() {
        assert_eq!(GROW_EAST.resized(IRect::default()), Some(IRect::new(0, 0, 1, 1)));
    }

    #[test]
    fn arrows_point_the_way_the_edge_moves() {
        assert_eq!(GROW_EAST.arrow_turn(), 0.0);
        assert_eq!(SHRINK_NORTH.arrow_turn(), 0.25);
    }
}
