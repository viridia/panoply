use bevy::prelude::*;
use bevy_mod_preferences::{PreferencesGroup, PreferencesKey, SetPreferencesChanged};
use exemplars::ExemplarsHandleResource;
use ui::{
    mode_realm,
    mode_scenery::{FloorTool, SceneryOverlay, SceneryTool, WallSnap},
    tool_floor_create, tool_floor_edit, tool_terrain_edit, tool_wall_create,
};

use crate::terrain::terrain_groups::{
    TerrainGroupsAsset, TerrainGroupsHandle, TerrainGroupsLoader,
};

mod camera;
mod events;
mod exemplars;
mod lib;
pub mod renderers;
mod ui;

pub struct EditorPlugin;

#[derive(Resource, Reflect)]
#[reflect(@PreferencesGroup("editor"), @PreferencesKey("sidebar_width"))]
pub struct EditorSidebarWidth(pub f32);

#[derive(Resource, Default)]
pub struct SelectedParcel(pub Option<Entity>);

#[derive(Resource, Default)]
pub struct SelectedPrecinct(pub Option<Entity>);

#[derive(Resource, Default, Clone, Copy, PartialEq)]
pub enum DragShape {
    /// When no cursor is shown
    #[default]
    None,

    /// When cursor is shown as a point.
    Point,

    /// When cursor is shown as a flat rectangle.
    FlatRect,

    /// When cursor is shown as a which conforms to the terrain.
    DecalRect,
}

#[derive(Resource, Default, Clone, PartialEq)]
pub(crate) struct TerrainDragState {
    pub(crate) dragging: bool,
    pub(crate) drag_shape: DragShape,
    pub(crate) parcel: Option<Entity>,
    pub(crate) anchor_pos: IVec2,
    pub(crate) cursor_pos: IVec2,
    pub(crate) anchor_height: i32,
}

#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Reflect)]
#[reflect(Default, @PreferencesGroup("editor"), @PreferencesKey("mode"))]
pub enum EditorMode {
    #[default]
    Realm,
    Terrain,
    Scenery,
    Meta,
    Play,
}

#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Reflect)]
#[reflect(Default, @PreferencesGroup("editor"), @PreferencesKey("terrain_tool"))]
pub(crate) enum TerrainTool {
    #[default]
    RaiseDraw,
    RaiseRect,
    LowerDraw,
    LowerRect,
    FlattenDraw,
    FlattenRect,
    DrawTrees,
    DrawShrubs,
    DrawHerbs,
    EraseFlora,
}

impl Default for EditorSidebarWidth {
    fn default() -> Self {
        Self(300.0)
    }
}

impl Plugin for EditorPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<TerrainGroupsAsset>()
            .register_asset_loader(TerrainGroupsLoader)
            .insert_state(EditorMode::default())
            .insert_state(TerrainTool::default())
            .insert_state(SceneryTool::default())
            .insert_state(FloorTool::default())
            .insert_state(WallSnap::default())
            .add_computed_state::<SceneryOverlay>()
            .enable_state_scoped_entities::<EditorMode>()
            .enable_state_scoped_entities::<TerrainTool>()
            .enable_state_scoped_entities::<SceneryTool>()
            .enable_state_scoped_entities::<FloorTool>()
            .enable_state_scoped_entities::<WallSnap>()
            .init_resource::<EditorSidebarWidth>()
            .init_resource::<SelectedParcel>()
            .init_resource::<SelectedPrecinct>()
            .init_resource::<TerrainDragState>()
            .init_resource::<ExemplarsHandleResource>()
            .init_resource::<TerrainGroupsHandle>()
            .register_type::<EditorSidebarWidth>()
            .register_type::<State<EditorMode>>()
            .register_type::<NextState<EditorMode>>()
            .register_type::<State<TerrainTool>>()
            .register_type::<NextState<TerrainTool>>()
            .register_type::<State<SceneryTool>>()
            .register_type::<NextState<SceneryTool>>()
            .register_type::<State<FloorTool>>()
            .register_type::<NextState<FloorTool>>()
            .register_type::<State<WallSnap>>()
            .register_type::<NextState<WallSnap>>()
            .insert_state(ui::quick_nav::QuickNavOpen::default())
            .add_systems(OnEnter(EditorMode::Realm), mode_realm::enter)
            .add_systems(OnExit(EditorMode::Realm), mode_realm::exit)
            .add_systems(OnEnter(EditorMode::Terrain), tool_terrain_edit::enter)
            .add_systems(OnExit(EditorMode::Terrain), tool_terrain_edit::exit)
            .add_systems(
                OnEnter(SceneryOverlay::FloorCreate),
                tool_floor_create::enter,
            )
            .add_systems(OnExit(SceneryOverlay::FloorCreate), tool_floor_create::exit)
            .add_systems(OnEnter(SceneryOverlay::FloorDraw), tool_floor_edit::enter)
            .add_systems(OnExit(SceneryOverlay::FloorDraw), tool_floor_edit::exit)
            .add_systems(OnEnter(SceneryOverlay::PlaceWall), tool_wall_create::enter)
            .add_systems(OnExit(SceneryOverlay::PlaceWall), tool_wall_create::exit)
            .add_systems(
                Startup,
                (
                    renderers::setup_thumbnail_realm,
                    renderers::setup_thumbnail_camera.after(renderers::setup_thumbnail_realm),
                    renderers::setup_thumbnail_observer,
                ),
            )
            .add_systems(PostStartup, ui::setup_editor_view)
            .add_systems(
                Update,
                (
                    camera::camera_controller,
                    watch_state_transitions,
                    (
                        renderers::create_terrain_thumbnails,
                        renderers::update_terrain_thumbnails,
                        renderers::assign_thumbnails_to_camera,
                    )
                        .chain(),
                    tool_terrain_edit::hover.run_if(in_state(EditorMode::Terrain)),
                    tool_floor_create::hover.run_if(in_state(SceneryOverlay::FloorCreate)),
                    tool_floor_edit::hover.run_if(in_state(SceneryOverlay::FloorDraw)),
                    tool_wall_create::hover.run_if(in_state(SceneryOverlay::PlaceWall)),
                ),
            );
    }
}

/// Mark preferences as changed whenever we change the editor mode.
fn watch_state_transitions(
    editor_mode: Res<State<EditorMode>>,
    terrain_tool: Res<State<TerrainTool>>,
    scenery_tool: Res<State<SceneryTool>>,
    floor_tool: Res<State<FloorTool>>,
    wall_snap: Res<State<WallSnap>>,
    mut commands: Commands,
) {
    if editor_mode.is_changed()
        || terrain_tool.is_changed()
        || scenery_tool.is_changed()
        || floor_tool.is_changed()
        || wall_snap.is_changed()
    {
        commands.push(SetPreferencesChanged);
    }
}
