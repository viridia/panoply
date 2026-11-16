//! Scenery mode's wall tool: a press and drag paints a rectangle of wall tiles on the current tier
//! of the precinct the press was in, one tile a meter, at positions snapped to the tool's grid, so
//! that walls go down fast with no fiddling. Hovering shows the tile a click would place; dragging,
//! the rectangle, as the models themselves, with an outline around it. The list's `(erase)` row
//! erases the walls in the rectangle instead. Escape during a drag cancels it.
//!
//! Replace, on by default, first removes the walls the new ones would stand on: those within the
//! rectangle's tiles on the tier, but not fixtures.

use std::borrow::Cow;

use bevy::{
    app::Propagate,
    camera::visibility::RenderLayers,
    color::palettes::css,
    picking::{
        events::PointerState,
        pointer::{PointerButton, PointerId},
    },
    prelude::*,
    settings::{ReflectSettingsGroup, SaveSettingsDeferred, SettingsGroup},
};
use bevy_action_map::prelude::*;
use panoply_core::{Realm, Viewpoint};
use panoply_exemplar::Exemplar;
use panoply_overlays::{AddOverlay, DrawPlane, OverlayColor, OverlayShape, ShapeBuilder};
use panoply_precinct::SceneryInstanceId;
use panoply_scenery::{
    precinct::Precinct,
    scenery_element::{SceneryElement, SceneryElementRebuildAspects},
    PRECINCT_SIZE_F, WALL_TYPE,
};

use crate::{
    exemplar_list::{chosen_placeable, SceneryChoices},
    mode_selector::SceneryTool,
    precinct_edit::PrecinctEdits,
    scenery_pointer::{
        RotateLeft, RotateRight, SceneryKeys, SceneryPointer, SceneryPointerSystems, SceneryTier,
    },
    shell::SAVE_DELAY,
    toast::ShowToast,
};

/// The points a wall snaps to.
#[derive(Reflect, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum WallSnap {
    /// The middle of each meter's cell.
    #[default]
    Normal,
    /// The grid's lines, a meter apart.
    Offset,
    /// The middle of each quarter meter's cell.
    Quarter,
}

impl WallSnap {
    /// The spacing of the points, and how far they lie from the grid's lines.
    fn lattice(self) -> (f32, f32) {
        match self {
            WallSnap::Normal => (1.0, 0.5),
            WallSnap::Offset => (1.0, 0.0),
            WallSnap::Quarter => (0.25, 0.125),
        }
    }

    /// The point nearest `point`, in a precinct's coordinates, kept within the precinct.
    fn snap(self, point: Vec2) -> Vec2 {
        let (spacing, offset) = self.lattice();
        let snapped = ((point - offset) / spacing).round() * spacing + offset;
        snapped.clamp(
            Vec2::splat(offset),
            Vec2::splat(PRECINCT_SIZE_F - spacing + offset),
        )
    }
}

/// The wall tool's options, saved between runs.
#[derive(Resource, SettingsGroup, Reflect, Clone, Copy, PartialEq, Debug)]
#[reflect(Resource, SettingsGroup, Default)]
#[settings_group(group = "wall_tool")]
pub struct WallToolSettings {
    pub snap: WallSnap,
    /// Remove the walls the new ones would stand on.
    pub replace: bool,
    /// Quarter turns right from facing +z, 0 to 3.
    pub facing: u8,
}

impl Default for WallToolSettings {
    fn default() -> Self {
        Self {
            snap: WallSnap::default(),
            replace: true,
            facing: 0,
        }
    }
}

impl WallToolSettings {
    /// The facing walls are placed with, in degrees, as a precinct holds it.
    fn facing_degrees(self) -> f32 {
        (self.facing as f32 * -90.0).rem_euclid(360.0)
    }

    /// Turns by `turns` quarter turns, right for positive.
    pub(crate) fn turn(&mut self, turns: i8) {
        self.facing = (self.facing as i8 + turns).rem_euclid(4) as u8;
    }
}

/// A drag in progress, held from the press to the release.
#[derive(Clone, Copy, Debug)]
struct WallDrag {
    /// The precinct the press was in, and its coordinates.
    precinct: Entity,
    coords: IVec2,
    /// The tile where the press was, in the precinct's coordinates.
    anchor: Vec2,
    /// The tile at the rectangle's far corner, a whole number of meters from the anchor.
    corner: Vec2,
    /// Escape cancelled it: the release places nothing.
    cancelled: bool,
}

/// The drag in progress, if any; read by `WallDragKeys`.
#[derive(Resource, Default)]
struct WallDragState(Option<WallDrag>);

/// Escape cancels a drag. Above `AppControl`, whose Escape quits, and consumed, so cancelling a
/// drag never quits.
#[derive(InputContext)]
#[context(path = "editor.wall_drag", tick = Render, priority = 500)]
pub struct WallDragKeys;

/// Cancel the drag: its release places nothing.
#[derive(InputAction)]
#[action(path = "editor.wall_drag.cancel", output = bool, intent = Button)]
pub struct CancelWallDrag;

/// The outline around the tiles a release would fill: their cells, in the world's x and z, and
/// their height. Hidden while `None`.
#[derive(Component, Reflect, Clone, PartialEq, Default)]
#[reflect(Component)]
pub struct WallCursor(Option<(Rect, f32)>);

impl OverlayShape for WallCursor {
    fn draw(&self, builder: &mut ShapeBuilder) {
        let Some((cells, height)) = self.0 else {
            return;
        };
        builder
            .with_draw_plane(DrawPlane::XZ)
            .with_offset(height + CURSOR_LIFT)
            .with_stroke_width(CURSOR_STROKE)
            .stroke_rect(cells);
    }
}

/// The cursor's stroke, in meters.
const CURSOR_STROKE: f32 = 0.1;

/// How far above the tier the cursor is drawn, so its depth does not tie with a floor's.
const CURSOR_LIFT: f32 = 0.01;

/// The root of the preview's elements, at the origin of the precinct they would go in. Not a
/// precinct's child, so its rebuild leaves them alone, and its elements get no colliders.
#[derive(Component)]
struct WallPreview;

/// The tiles a release would fill, and what it would place there, which the outline and the
/// preview show; the preview keeps what it last showed, so it changes only when this does.
#[derive(Resource, Clone, PartialEq, Default)]
struct WallTarget {
    /// The precinct's coordinates, and the tiles, in its coordinates. Empty while there are none.
    coords: IVec2,
    tiles: Vec<Vec3>,
    /// The wall to place, or `None` for `(erase)` or for nothing placeable.
    exemplar: Option<AssetId<Exemplar>>,
    /// In radians.
    facing: f32,
}

pub struct WallToolPlugin;

impl Plugin for WallToolPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WallDragState>()
            .init_resource::<WallTarget>()
            .register_type::<WallToolSettings>()
            .add_overlay::<WallCursor>()
            .add_context::<WallDragKeys>(|context| {
                context.active_if(|drag: Res<WallDragState>| drag.0.is_some());
                context.bind::<CancelWallDrag>(KeyCode::Escape).consume();
            })
            .add_systems(Startup, |mut commands: Commands| {
                commands.spawn((Name::new("WallDragKeys"), WallDragKeys));
            })
            .add_systems(OnEnter(SceneryTool::PlaceWall), spawn_wall_tool)
            .add_systems(OnExit(SceneryTool::PlaceWall), clear_drag)
            .add_systems(
                Update,
                (rotate_keys, update_wall_tool, show_wall_target)
                    .chain()
                    .after(SceneryPointerSystems)
                    .run_if(in_state(SceneryTool::PlaceWall)),
            );
    }
}

fn spawn_wall_tool(mut commands: Commands) {
    commands.spawn((
        Name::new("WallCursor"),
        WallCursor::default(),
        OverlayColor {
            base: css::SILVER.with_alpha(0.9).into(),
            underlay: 0.8,
        },
        Propagate(RenderLayers::none()),
        Transform::default(),
        Visibility::default(),
        DespawnOnExit(SceneryTool::PlaceWall),
    ));
    commands.spawn((
        Name::new("WallPreview"),
        WallPreview,
        Transform::default(),
        Visibility::default(),
        Propagate(Pickable::IGNORE),
        DespawnOnExit(SceneryTool::PlaceWall),
    ));
}

fn clear_drag(mut drag: ResMut<WallDragState>) {
    drag.0 = None;
}

/// Turns the walls to place: the toolbar's buttons do the same.
fn rotate_keys(
    input: ContextActions<SceneryKeys>,
    mut settings: ResMut<WallToolSettings>,
    mut commands: Commands,
) {
    let turns = input.fired::<RotateRight>() as i8 - input.fired::<RotateLeft>() as i8;
    if turns != 0 {
        settings.turn(turns);
        commands.queue(SaveSettingsDeferred(SAVE_DELAY));
    }
}

/// The tiles from `anchor` to `corner`, a meter apart, at `height`.
fn tiles(anchor: Vec2, corner: Vec2, height: f32) -> Vec<Vec3> {
    let (min, max) = (anchor.min(corner), anchor.max(corner));
    let count = (max - min).round().as_ivec2() + IVec2::ONE;
    (0..count.y)
        .flat_map(|z| (0..count.x).map(move |x| IVec2::new(x, z)))
        .map(|step| (min + step.as_vec2()).extend(height).xzy())
        .collect()
}

/// The cells of `tiles`, which are a meter wide, in the tiles' coordinates.
fn cells(tiles: &[Vec3]) -> Rect {
    let rect = tiles
        .iter()
        .fold(Rect::EMPTY, |rect, tile| rect.union_point(tile.xz()));
    rect.inflate(0.5)
}

/// Follows the pointer and the tool's options; starts, cancels and ends drags, editing the
/// precinct as one ends.
fn update_wall_tool(
    pointer: Res<SceneryPointer>,
    tier: Res<SceneryTier>,
    settings: Res<WallToolSettings>,
    choices: Res<SceneryChoices>,
    pointers: Res<PointerState>,
    keys: ContextActions<WallDragKeys>,
    mut drag: ResMut<WallDragState>,
    mut was_pressed: Local<bool>,
    ui_nodes: Query<(), With<Node>>,
    precincts: Query<&Precinct>,
    server: Res<AssetServer>,
    exemplars: Res<Assets<Exemplar>>,
    mut edits: PrecinctEdits,
    mut target: ResMut<WallTarget>,
    mut commands: Commands,
) {
    // A press in the view, not on the panel or a widget over the view.
    let pressed = pointers
        .get(PointerId::Mouse, PointerButton::Primary)
        .is_some_and(|state| state.pressing.keys().any(|e| !ui_nodes.contains(*e)));
    let just_pressed = pressed && !*was_pressed;
    let released = !pressed && *was_pressed;
    *was_pressed = pressed;
    let height = tier.height();
    let local = |coords: IVec2, point: Vec3| point.xz() - coords.as_vec2() * PRECINCT_SIZE_F;

    if keys.fired::<CancelWallDrag>() {
        if let Some(drag) = drag.0.as_mut() {
            drag.cancelled = true;
        }
    }
    if just_pressed {
        drag.0 = pointer
            .precinct
            .zip(pointer.point)
            .map(|((entity, coords), point)| {
                let anchor = settings.snap.snap(local(coords, point));
                WallDrag {
                    precinct: entity,
                    coords,
                    anchor,
                    corner: anchor,
                    cancelled: false,
                }
            });
    } else if let (Some(drag), Some(point)) = (drag.0.as_mut(), pointer.point) {
        // The snapped point, as the anchor was, so the two agree on a point on the grid's lines;
        // then a whole number of meters from the anchor, so the drag fills whole tiles, within
        // the precinct.
        let (spacing, offset) = settings.snap.lattice();
        let (low, high) = (offset, PRECINCT_SIZE_F - spacing + offset);
        let snapped = settings.snap.snap(local(drag.coords, point));
        let step = (snapped - drag.anchor).round();
        let lowest = (Vec2::splat(low) - drag.anchor).ceil();
        let highest = (Vec2::splat(high) - drag.anchor).floor();
        drag.corner = drag.anchor + step.clamp(lowest, highest);
    }

    let wall = chosen_placeable(&choices, SceneryTool::PlaceWall, &server, &exemplars);
    if released {
        if let Some(done) = drag.0.take().filter(|drag| !drag.cancelled) {
            match &wall {
                Ok(exemplar) => {
                    if let Ok(precinct) = precincts.get(done.precinct) {
                        let tiles = tiles(done.anchor, done.corner, height);
                        let walls = wall_types(&server, &exemplars, &edits, &precinct.asset);
                        let path = exemplar
                            .and_then(|id| server.get_path(id))
                            .map(|path| path.to_string());
                        let facing = settings.facing_degrees();
                        let replace = settings.replace || exemplar.is_none();
                        edits.edit(&precinct.asset, |asset| {
                            place_walls(asset, &tiles, path, facing, replace, &walls)
                        });
                    }
                }
                Err(Some(reason)) => {
                    commands.write_message(ShowToast(reason.clone().into()));
                }
                Err(None) => {}
            }
        }
    }

    // The tiles a release would fill: the drag's, or hovering, the one under the pointer.
    let (coords, tiles) = match drag.0 {
        Some(drag) if !drag.cancelled => (drag.coords, tiles(drag.anchor, drag.corner, height)),
        Some(_) => (IVec2::ZERO, Vec::new()),
        None => pointer.precinct.zip(pointer.point).map_or(
            (IVec2::ZERO, Vec::new()),
            |((_, coords), point)| {
                let tile = settings.snap.snap(local(coords, point));
                (coords, tiles(tile, tile, height))
            },
        ),
    };
    target.set_if_neq(WallTarget {
        coords,
        tiles,
        exemplar: wall.ok().flatten(),
        facing: settings.facing_degrees().to_radians(),
    });
}

/// Shows the [`WallTarget`]: the outline around its tiles, and the chosen wall at each, none for
/// `(erase)`.
fn show_wall_target(
    target: Res<WallTarget>,
    viewpoint: Res<Viewpoint>,
    realms: Query<&Realm>,
    server: Res<AssetServer>,
    mut cursors: Query<(&mut WallCursor, &mut Propagate<RenderLayers>)>,
    mut previews: Query<(Entity, &mut Transform, Option<&Children>), With<WallPreview>>,
    mut elements: Query<&mut SceneryElement>,
    mut shown: Local<WallTarget>,
    mut commands: Commands,
) {
    let layer = viewpoint
        .realm
        .and_then(|e| realms.get(e).ok())
        .map_or(RenderLayers::none(), |r| r.layer.clone());
    let origin = target.coords.as_vec2() * PRECINCT_SIZE_F;
    let outline = target.tiles.first().map(|tile| {
        let cells = cells(&target.tiles);
        (
            Rect::from_corners(cells.min + origin, cells.max + origin),
            tile.y,
        )
    });
    for (mut cursor, mut propagate) in &mut cursors {
        cursor.set_if_neq(WallCursor(outline));
        propagate.set_if_neq(Propagate(layer.clone()));
    }

    let wanted = match target.exemplar {
        Some(_) => target.clone(),
        None => WallTarget::default(),
    };
    if *shown == wanted {
        return;
    }
    let Ok((root, mut transform, children)) = previews.single_mut() else {
        return;
    };
    transform.translation = (wanted.coords.as_vec2() * PRECINCT_SIZE_F).extend(0.).xzy();
    let mut existing: Vec<Entity> = children.into_iter().flatten().copied().collect();
    if shown.exemplar != wanted.exemplar {
        for child in existing.drain(..) {
            commands.entity(child).despawn();
        }
    }
    let handle = wanted.exemplar.and_then(|id| server.get_id_handle(id));
    for (index, &position) in wanted.tiles.iter().enumerate() {
        let element_transform = Transform::from_translation(position)
            .with_rotation(Quat::from_rotation_y(wanted.facing));
        if let Some(&child) = existing.get(index) {
            if let Ok(mut element) = elements.get_mut(child) {
                element.position = position;
                element.facing = wanted.facing;
            }
            commands.entity(child).insert(element_transform);
            continue;
        }
        let Some(handle) = handle.clone() else {
            break;
        };
        commands.spawn((
            Name::new("WallPreviewElement"),
            SceneryElement {
                iid: SceneryInstanceId::None,
                exemplar: handle,
                facing: wanted.facing,
                position,
            },
            element_transform,
            Visibility::Inherited,
            layer.clone(),
            SceneryElementRebuildAspects,
            ChildOf(root),
        ));
    }
    for &child in existing.iter().skip(wanted.tiles.len()) {
        commands.entity(child).despawn();
    }
    *shown = wanted;
}

/// Whether each of the precinct's scenery types is a wall; a type whose exemplar is not loaded
/// counts as none.
fn wall_types(
    server: &AssetServer,
    exemplars: &Assets<Exemplar>,
    edits: &PrecinctEdits,
    precinct: &Handle<panoply_precinct::PrecinctAsset>,
) -> Vec<bool> {
    let Some(asset) = edits.get(precinct) else {
        return Vec::new();
    };
    asset
        .scenery_types
        .iter()
        .map(|path| {
            server
                .get_handle::<Exemplar>(path.clone())
                .and_then(|handle| exemplars.get(&handle))
                .is_some_and(|exemplar| exemplar.0.meta_type == WALL_TYPE)
        })
        .collect()
}

/// Places a wall of the exemplar at `path` at each of `tiles`, facing `facing` degrees, first
/// removing, if `replace`, the walls among them; with no exemplar, removes them only. `walls` says
/// which of the precinct's scenery types are walls. Returns the edit's name, or `None` if it would
/// change nothing.
fn place_walls(
    precinct: &mut panoply_precinct::PrecinctAsset,
    tiles: &[Vec3],
    path: Option<String>,
    facing: f32,
    replace: bool,
    walls: &[bool],
) -> Option<Cow<'static, str>> {
    let height = tiles.first()?.y;
    // Just inside the tiles' cells, so a wall on a neighboring cell's edge stays.
    let area = cells(tiles).inflate(-0.01);
    let removed = if replace {
        precinct
            .remove_scenery_elements(|_, index, position| {
                walls.get(index).copied().unwrap_or(false)
                    && (position.y - height).abs() < 0.5
                    && area.contains(position.xz())
            })
            .len()
    } else {
        0
    };
    let Some(path) = path else {
        return (removed > 0).then(|| plural("Erase", removed).into());
    };
    let index = precinct
        .scenery_type_index(&path)
        .unwrap_or_else(|| precinct.add_scenery_type(path));
    for &tile in tiles {
        precinct.add_scenery_element(index, facing, tile, None);
    }
    Some(plural("Place", tiles.len()).into())
}

/// "Place 1 wall", "Place 6 walls".
fn plural(verb: &str, count: usize) -> String {
    match count {
        1 => format!("{verb} 1 wall"),
        _ => format!("{verb} {count} walls"),
    }
}

#[cfg(test)]
mod tests {
    use panoply_precinct::PrecinctAsset;

    use super::*;

    #[test]
    fn each_snap_takes_the_nearest_of_its_points() {
        let point = Vec2::new(3.2, 7.9);
        assert_eq!(WallSnap::Normal.snap(point), Vec2::new(3.5, 7.5));
        assert_eq!(WallSnap::Offset.snap(point), Vec2::new(3.0, 8.0));
        assert_eq!(WallSnap::Quarter.snap(point), Vec2::new(3.125, 7.875));
        // Kept within the precinct.
        assert_eq!(
            WallSnap::Normal.snap(Vec2::new(-1.0, 70.0)),
            Vec2::new(0.5, 63.5)
        );
    }

    #[test]
    fn a_drag_fills_a_tile_a_meter() {
        let tiles = tiles(Vec2::new(2.5, 4.5), Vec2::new(0.5, 4.5), 0.02);
        assert_eq!(
            tiles,
            [0.5, 1.5, 2.5].map(|x| Vec3::new(x, 0.02, 4.5)).to_vec()
        );
        assert_eq!(cells(&tiles), Rect::new(0.0, 4.0, 3.0, 5.0));
    }

    #[test]
    fn facing_turns_in_quarter_turns() {
        let mut settings = WallToolSettings::default();
        settings.turn(-1);
        assert_eq!(settings.facing, 3);
        assert_eq!(settings.facing_degrees(), 90.0);
        settings.turn(2);
        assert_eq!(settings.facing, 1);
        assert_eq!(settings.facing_degrees(), 270.0);
    }

    /// Replace removes the walls on the tiles' cells and tier, but not fixtures, walls on another
    /// tier, or walls on the next cell.
    #[test]
    fn replace_removes_only_the_walls_underneath() {
        let mut precinct = PrecinctAsset {
            scenery_types: vec!["walls#Old".into(), "fixtures#Chair".into()],
            ..default()
        };
        let at = |x, y| Vec3::new(x, y, 0.5);
        precinct.add_scenery_element(0, 0., at(0.5, 0.02), None);
        precinct.add_scenery_element(1, 0., at(1.5, 0.02), None);
        precinct.add_scenery_element(0, 0., at(1.5, 1.02), None);
        precinct.add_scenery_element(0, 0., at(2.5, 0.02), None);
        let tiles = [at(0.5, 0.02), at(1.5, 0.02)];
        let label = place_walls(
            &mut precinct,
            &tiles,
            Some("walls#New".into()),
            0.,
            true,
            &[true, false],
        );
        assert_eq!(label.as_deref(), Some("Place 2 walls"));
        let kept: Vec<(usize, Vec3)> = precinct
            .scenery
            .iter()
            .map(|s| (s.id, s.position))
            .collect();
        assert_eq!(
            kept,
            [
                (1, at(1.5, 0.02)),
                (0, at(1.5, 1.02)),
                (0, at(2.5, 0.02)),
                (2, at(0.5, 0.02)),
                (2, at(1.5, 0.02)),
            ]
        );
    }

    #[test]
    fn erasing_nothing_is_no_edit() {
        let mut precinct = PrecinctAsset::default();
        let tiles = [Vec3::new(0.5, 0.02, 0.5)];
        assert_eq!(
            place_walls(&mut precinct, &tiles, None, 0., true, &[]),
            None
        );
    }
}
