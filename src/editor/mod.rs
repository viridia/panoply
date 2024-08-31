use bevy::prelude::*;
use bevy_mod_preferences::{PreferencesGroup, PreferencesKey};
use exemplars::ExemplarsHandleResource;
use lib::pick_plane::PlanePickBackend;
use ui::{
    mode_realm,
    mode_scenery::EditSceneryPlugin,
    mode_terrain::EditTerrainPlugin,
    zoom_selector::{update_zoom_level, ZoomLevel},
};
use undo::UndoStack;

use crate::terrain::terrain_groups::{
    TerrainGroupsAsset, TerrainGroupsHandle, TerrainGroupsLoader,
};

mod camera;
mod events;
mod exemplars;
mod lib;
pub mod renderers;
mod scenery;
mod terrain;
mod ui;
mod undo;
pub mod unsaved;

pub struct EditorPlugin;

#[derive(Resource, Reflect)]
#[reflect(@PreferencesGroup("editor"), @PreferencesKey("sidebar_width"))]
pub struct EditorSidebarWidth(pub f32);

#[derive(Resource, Default)]
pub struct SelectedParcel(pub Option<Entity>);

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
    Initial,
    #[default]
    Realm,
    Terrain,
    Scenery,
    Meta,
    Play,
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
            .insert_state(EditorMode::Initial)
            .enable_state_scoped_entities::<EditorMode>()
            .init_resource::<unsaved::UnsavedAssets>()
            .init_resource::<EditorSidebarWidth>()
            .init_resource::<SelectedParcel>()
            .init_resource::<TerrainDragState>()
            .init_resource::<ExemplarsHandleResource>()
            .init_resource::<TerrainGroupsHandle>()
            .init_resource::<ZoomLevel>()
            .init_resource::<UndoStack>()
            .register_type::<EditorSidebarWidth>()
            .register_type::<State<EditorMode>>()
            .register_type::<NextState<EditorMode>>()
            .register_type::<ZoomLevel>()
            .insert_state(ui::quick_nav::QuickNavOpen::default())
            .add_systems(OnEnter(EditorMode::Realm), mode_realm::enter)
            .add_systems(OnExit(EditorMode::Realm), mode_realm::exit)
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
                    (
                        renderers::create_terrain_thumbnails,
                        renderers::update_terrain_thumbnails,
                        renderers::assign_thumbnails_to_camera,
                    )
                        .chain(),
                    unsaved::receive_asset_saving,
                    update_zoom_level,
                ),
            )
            .add_plugins((EditSceneryPlugin, EditTerrainPlugin, PlanePickBackend));
    }
}
