use bevy::prelude::*;
use bevy_mod_preferences::{PreferencesGroup, PreferencesKey, SetPreferencesChanged};
use ui::{mode_realm, mode_terrain};

mod camera;
mod ui;

pub struct EditorPlugin;

#[derive(Resource, Reflect)]
#[reflect(@PreferencesGroup("editor"), @PreferencesKey("sidebar_width"))]
pub struct EditorSidebarWidth(pub f32);

#[derive(Resource, Default)]
pub struct SelectedParcel(pub Option<Entity>);

#[derive(Resource, Default, Clone, PartialEq)]
pub enum ParcelCursor {
    /// When no cursor is shown
    #[default]
    None,

    /// When cursor is shown as a point.
    Point((Entity, IVec2)),

    /// When cursor is shown as a flat rectangle.
    FlatRect((Entity, IRect, f32)),

    /// When cursor is shown as a which conforms to the terrain.
    DecalRect((Entity, IRect)),
}

#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Reflect)]
#[reflect(Default, @PreferencesGroup("editor"), @PreferencesKey("mode"))]
enum EditorMode {
    #[default]
    Realm,
    Terrain,
    Scenery,
    Meta,
    Play,
}

#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Reflect)]
#[reflect(Default, @PreferencesGroup("editor"), @PreferencesKey("terrain_tool"))]
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
#[reflect(Default, @PreferencesGroup("editor"), @PreferencesKey("scenery_tool"))]
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

impl Default for EditorSidebarWidth {
    fn default() -> Self {
        Self(300.0)
    }
}

impl Plugin for EditorPlugin {
    fn build(&self, app: &mut App) {
        app.insert_state(EditorMode::default())
            .insert_state(TerrainTool::default())
            .insert_state(SceneryTool::default())
            .enable_state_scoped_entities::<EditorMode>()
            .enable_state_scoped_entities::<TerrainTool>()
            .enable_state_scoped_entities::<SceneryTool>()
            .init_resource::<EditorSidebarWidth>()
            .init_resource::<SelectedParcel>()
            .init_resource::<ParcelCursor>()
            .register_type::<EditorSidebarWidth>()
            .register_type::<State<EditorMode>>()
            .register_type::<NextState<EditorMode>>()
            .register_type::<State<TerrainTool>>()
            .register_type::<NextState<TerrainTool>>()
            .register_type::<State<SceneryTool>>()
            .register_type::<NextState<SceneryTool>>()
            .insert_state(ui::quick_nav::QuickNavOpen::default())
            .add_systems(PostStartup, ui::setup_editor_view)
            .add_systems(Update, camera::camera_controller)
            .add_systems(OnEnter(EditorMode::Realm), mode_realm::enter)
            .add_systems(OnExit(EditorMode::Realm), mode_realm::exit)
            .add_systems(OnEnter(EditorMode::Terrain), mode_terrain::enter)
            .add_systems(OnExit(EditorMode::Terrain), mode_terrain::exit)
            .add_systems(
                Update,
                (
                    watch_state_transitions,
                    mode_terrain::hover.run_if(in_state(EditorMode::Terrain)),
                ),
            );
    }
}

/// Mark preferences as changed whenever we change the editor mode.
fn watch_state_transitions(
    editor_mode: Res<State<EditorMode>>,
    terrain_tool: Res<State<TerrainTool>>,
    scenery_tool: Res<State<SceneryTool>>,
    mut commands: Commands,
) {
    if editor_mode.is_changed() || terrain_tool.is_changed() || scenery_tool.is_changed() {
        commands.push(SetPreferencesChanged);
    }
}
