use bevy::prelude::*;
use bevy_mod_preferences::{Preferences, ReflectPreferences};
use ui::mode_realm;

mod camera;
mod ui;

pub struct EditorPlugin;

#[derive(Resource)]
pub struct EditorSidebarWidth(pub f32);

#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Reflect)]
enum EditorState {
    #[default]
    Realm,
    Terrain,
    Scenery,
    Meta,
    Play,
}

#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Reflect)]
enum TerrainTool {
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

#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Reflect)]
enum SceneryTool {
    #[default]
    FloorDraw,
    WallDraw,
    FixtureDraw,
    ActorPlacement,
    TerrainFxDraw,
    SceneryEdit,
    EditLayers,
    SceneryRect,
}

#[derive(Resource, Default, Reflect, PartialEq)]
#[reflect(Default, Preferences)]
struct EditorPrefs {
    pub sidebar_width: f32,
    pub mode: EditorState,
}

impl Preferences for EditorPrefs {}

impl Default for EditorSidebarWidth {
    fn default() -> Self {
        Self(300.0)
    }
}

impl Plugin for EditorPlugin {
    fn build(&self, app: &mut App) {
        app.insert_state(EditorState::default())
            .insert_state(TerrainTool::default())
            .insert_state(SceneryTool::default())
            .enable_state_scoped_entities::<EditorState>()
            .enable_state_scoped_entities::<TerrainTool>()
            .enable_state_scoped_entities::<SceneryTool>()
            .init_resource::<EditorSidebarWidth>()
            .init_resource::<EditorPrefs>()
            .insert_state(ui::quick_nav::QuickNavOpen::default())
            .add_systems(PostStartup, ui::setup_editor_view)
            .add_systems(Update, camera::camera_controller)
            .add_systems(OnEnter(EditorState::Realm), mode_realm::enter)
            .add_systems(OnExit(EditorState::Realm), mode_realm::exit);
    }
}
