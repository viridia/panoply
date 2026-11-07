//! The mode selector: a segmented button atop the editor panel choosing the editor mode (realm,
//! terrain, scenery, catalogs) or play. F2 switches between play and the last editor mode. The
//! choice is saved, so a restart returns to it.

use bevy::{
    ecs::system::SystemParam,
    feathers::{
        controls::{ButtonVariant, FeathersButton},
        rounded_corners::RoundedCorners,
    },
    prelude::*,
    settings::{ReflectSettingsGroup, SaveSettingsDeferred, SettingsGroup},
};
use bevy_action_map::prelude::*;
use bevy_immediate::{
    attach::{BevyImmediateAttachPlugin, ImmediateAttach},
    ui::{activated::ImmUiActivated, CapsUiFeathers},
    Imm,
};
use panoply_core::GameMode;

use crate::shell::{EditorShellControl, SAVE_DELAY};

/// The editor's modes, each with its own tools; a sub-state of editing (D53).
#[derive(SubStates, Reflect, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[source(GameMode = GameMode::Edit)]
pub enum EditorMode {
    /// Realms and their named places.
    #[default]
    Realm,
    /// Terrain shape and flora.
    Terrain,
    /// Walls, floors, fixtures and actors.
    Scenery,
    /// Quests, dialogues, items and the other catalogs.
    Catalogs,
}

/// Switch between play and the last editor mode.
#[derive(InputAction)]
#[action(path = "editor.toggle_mode", output = bool, intent = Button)]
pub struct ToggleGameMode;

/// The mode chosen, saved between runs. The states follow it (`apply_mode`); while playing,
/// `editor_mode` is the one F2 returns to.
#[derive(Resource, SettingsGroup, Reflect, Clone, Copy, PartialEq, Debug, Default)]
#[reflect(Resource, SettingsGroup, Default)]
#[settings_group(group = "editor_mode")]
pub struct EditorModeSettings {
    pub game_mode: GameMode,
    pub editor_mode: EditorMode,
}

/// The root of the immediate-mode selector.
#[derive(Component, Default, Clone)]
pub(crate) struct ModeSelector;

/// Each button's width, fixed so the selector's width does not depend on its icons'. The widest
/// icon (`world.png`, 36.6 wide at the selector's 24 high) is a little cramped inside the padding.
const BUTTON_WIDTH: f32 = 44.0;

/// The selector's width: its buttons and the 1-pixel seams between them (`shell.rs`).
pub(crate) const SELECTOR_WIDTH: f32 =
    BUTTONS.len() as f32 * BUTTON_WIDTH + (BUTTONS.len() - 1) as f32;

/// The selector's buttons, in order: a name, its icon, and the mode it selects. Play is last.
const BUTTONS: [(&str, &str, Option<EditorMode>); 5] = [
    ("RealmMode", "editor/icons/world.png", Some(EditorMode::Realm)),
    ("TerrainMode", "editor/icons/terrain.png", Some(EditorMode::Terrain)),
    ("SceneryMode", "editor/icons/building.png", Some(EditorMode::Scenery)),
    ("CatalogsMode", "editor/icons/quest.png", Some(EditorMode::Catalogs)),
    ("PlayMode", "editor/icons/play.png", None),
];

pub struct ModeSelectorPlugin;

impl Plugin for ModeSelectorPlugin {
    fn build(&self, app: &mut App) {
        app.add_sub_state::<EditorMode>()
            // So plans can read the mode.
            .register_type_mutable_state::<EditorMode>()
            .add_plugins(BevyImmediateAttachPlugin::<CapsUiFeathers, ModeSelector>::new())
            .add_systems(
                Update,
                (
                    toggle_game_mode,
                    apply_mode.run_if(resource_changed::<EditorModeSettings>),
                )
                    .chain(),
            );
    }
}

/// Sets the states to the mode chosen, at startup from the saved settings and after each change.
fn apply_mode(
    settings: Res<EditorModeSettings>,
    mut game_mode: ResMut<NextState<GameMode>>,
    mut editor_mode: ResMut<NextState<EditorMode>>,
) {
    game_mode.set_if_different(settings.game_mode);
    // A sub-state entering with its parent takes a pending value over its default.
    if settings.game_mode == GameMode::Edit {
        editor_mode.set_if_different(settings.editor_mode);
    }
}

fn toggle_game_mode(
    input: ContextActions<EditorShellControl>,
    mut settings: ResMut<EditorModeSettings>,
    mut commands: Commands,
) {
    if input.fired::<ToggleGameMode>() {
        settings.game_mode = match settings.game_mode {
            GameMode::Play => GameMode::Edit,
            GameMode::Edit => GameMode::Play,
        };
        commands.queue(SaveSettingsDeferred(SAVE_DELAY));
    }
}

#[derive(SystemParam)]
pub(crate) struct ModeSelectorParams<'w, 's> {
    settings: ResMut<'w, EditorModeSettings>,
    /// The mode the buttons last showed: `is_changed` would miss the selector's own changes.
    shown: Local<'s, Option<EditorModeSettings>>,
    commands: Commands<'w, 's>,
}

impl ImmediateAttach<CapsUiFeathers> for ModeSelector {
    type Params = ModeSelectorParams<'static, 'static>;

    /// A button per mode, each named for its mode, the chosen one highlighted.
    fn construct(ui: &mut Imm<CapsUiFeathers>, params: &mut ModeSelectorParams) {
        // Edit a copy: writing through `ResMut` every frame would mark the settings changed every
        // frame.
        let mut settings = *params.settings;
        let changed = *params.shown != Some(settings);
        *params.shown = Some(settings);
        for (index, &(name, image, mode)) in BUTTONS.iter().enumerate() {
            let selected = match mode {
                Some(mode) => settings.game_mode == GameMode::Edit && settings.editor_mode == mode,
                None => settings.game_mode == GameMode::Play,
            };
            let corners = match index {
                0 => RoundedCorners::Left,
                i if i == BUTTONS.len() - 1 => RoundedCorners::Right,
                _ => RoundedCorners::None,
            };
            let mut button = ui
                .ch_id(name)
                .on_spawn_apply_scene(move || {
                    bsn! {
                        @FeathersButton {
                            @caption: bsn! {
                                Node { height: px(24) }
                                ImageNode { image: image }
                            },
                            @corners: corners,
                            @variant: ButtonVariant::selected(selected),
                        }
                        Node {
                            width: px(BUTTON_WIDTH),
                            height: px(32),
                            padding: UiRect::horizontal(px(4)),
                            flex_shrink: 0.0,
                        }
                    }
                })
                .on_spawn_insert(move || Name::new(name))
                .on_change_insert(changed, move || ButtonVariant::selected(selected));
            if button.activated() {
                match mode {
                    Some(mode) => {
                        settings.game_mode = GameMode::Edit;
                        settings.editor_mode = mode;
                    }
                    None => settings.game_mode = GameMode::Play,
                }
            }
        }
        if params.settings.set_if_neq(settings) {
            params.commands.queue(SaveSettingsDeferred(SAVE_DELAY));
        }
    }
}
