//! Scenery mode's panel, laid out as faery's: a grid whose left column holds the tool palette, the
//! layers' Cut ▲ ▼, then New… and Clone…; whose right column holds the chosen tool's options, then
//! its exemplars. The palette is a segmented grid of buttons, as terrain mode's is: each tool button
//! chooses its [`SceneryTool`].

use bevy::{
    ecs::system::SystemParam,
    feathers::{
        constants::icons,
        controls::{ButtonVariant, FeathersButton},
        display::icon,
        rounded_corners::RoundedCorners,
        theme::ThemedText,
    },
    prelude::*,
    settings::SaveSettingsDeferred,
    ui::InteractionDisabled,
};
use bevy_immediate::{
    attach::{BevyImmediateAttachPlugin, ImmediateAttach},
    ui::{activated::ImmUiActivated, CapsUiFeathers},
    Imm,
};

use crate::{
    exemplar_list::{exemplar_chooser, placed_type, SceneryChoices},
    mode_selector::{EditorModeSettings, SceneryTool},
    shell::SAVE_DELAY,
};

/// The panel's root.
#[derive(Component, Default, Clone)]
pub struct SceneryPanel;

/// The root of the immediate-mode palette.
#[derive(Component, Default, Clone)]
struct SceneryPalette;

/// The immediate-mode root holding the chosen tool's exemplars.
#[derive(Component, Default, Clone)]
struct SceneryExemplars;

/// Each palette button's size, as terrain mode's.
const BUTTON_WIDTH: f32 = 44.0;
const BUTTON_HEIGHT: f32 = 44.0;
const COLUMNS: usize = 2;

/// What a palette button does: choose a tool, or turn the selection.
#[derive(Clone, Copy, PartialEq)]
enum PaletteAction {
    Tool(SceneryTool),
    Rotate,
}

/// The palette's buttons, in rows of two: a name, its icon and the icon's height, and what it
/// does. Each icon keeps its own aspect, at the height that fits it in 40 by 32.
const BUTTONS: [(&str, &str, f32, PaletteAction); 10] = {
    use PaletteAction::{Rotate, Tool};
    [
        (
            "PlaceFloor",
            "editor/icons/floor-draw.png",
            28.,
            Tool(SceneryTool::PlaceFloor),
        ),
        (
            "PlaceWall",
            "editor/icons/wall-draw.png",
            32.,
            Tool(SceneryTool::PlaceWall),
        ),
        (
            "PlaceFixture",
            "editor/icons/furnishing-draw.png",
            32.,
            Tool(SceneryTool::PlaceFixture),
        ),
        (
            "PlaceActor",
            "editor/icons/actor.png",
            32.,
            Tool(SceneryTool::PlaceActor),
        ),
        (
            "DrawTerrainFx",
            "editor/icons/road-draw.png",
            28.,
            Tool(SceneryTool::DrawTerrainFx),
        ),
        (
            "Interact",
            "editor/icons/machine.png",
            32.,
            Tool(SceneryTool::Interact),
        ),
        (
            "Layers",
            "editor/icons/layers.png",
            32.,
            Tool(SceneryTool::Layers),
        ),
        (
            "RectSelect",
            "editor/icons/rect-select.png",
            32.,
            Tool(SceneryTool::RectSelect),
        ),
        ("RotateCcw", "editor/icons/rotate-ccw.png", 26., Rotate),
        ("RotateCw", "editor/icons/rotate-cw.png", 26., Rotate),
    ]
};

pub struct SceneryPanelPlugin;

impl Plugin for SceneryPanelPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            BevyImmediateAttachPlugin::<CapsUiFeathers, SceneryPalette>::new(),
            BevyImmediateAttachPlugin::<CapsUiFeathers, SceneryExemplars>::new(),
        ));
    }
}

/// The panel, which `PanelBody` shows while in scenery mode. Grid rows: the palette spans the
/// first two, beside the tool's options and then the top of the exemplars, which reach down beside
/// Cut ▲ ▼, New… and Clone… and through the last row, which takes the rest of the panel's height.
pub(crate) fn scenery_panel() -> impl Scene {
    bsn! {
        #SceneryPanel
        SceneryPanel
        Node {
            display: Display::Grid,
            grid_template_columns: {
                vec![RepeatedGridTrack::auto(1), RepeatedGridTrack::flex(1, 1.0)]
            },
            grid_template_rows: {
                vec![RepeatedGridTrack::auto(5), RepeatedGridTrack::flex(1, 1.0)]
            },
            column_gap: px(4),
            row_gap: px(4),
            flex_grow: 1.0,
            min_height: px(0),
        }
        Children [
            SceneryPalette
            Node {
                display: Display::Grid,
                grid_column: {GridPlacement::start(1)},
                grid_row: {GridPlacement::start_span(1, 2)},
                grid_template_columns: {
                    vec![RepeatedGridTrack::px(COLUMNS as u16, BUTTON_WIDTH)]
                },
                // The 1-pixel seams of a segmented button.
                column_gap: px(1),
                row_gap: px(1),
                align_self: AlignSelf::Start,
            }
            --
            // TODO(PAN5): the floor tool's shapes and the wall tool's snap.
            #ToolOptions
            Node {
                grid_column: {GridPlacement::start(2)},
                grid_row: {GridPlacement::start(1)},
                height: px(32),
            }
            --
            // Into the last row, which flexes: an item spanning a flexible track gives its height
            // to that track, so the list's length leaves the left column's rows alone.
            SceneryExemplars
            #Exemplars
            Node {
                grid_column: {GridPlacement::start(2)},
                grid_row: {GridPlacement::start_end(2, -1)},
                flex_direction: FlexDirection::Column,
                min_width: px(0),
                min_height: px(0),
            }
            --
            // TODO(PAN5): the layer to cut at, and its Cut toggle.
            Node {
                grid_column: {GridPlacement::start(1)},
                grid_row: {GridPlacement::start(3)},
                column_gap: px(1),
            }
            Children [
                @FeathersButton {
                    @caption: bsn! { Text("Cut") ThemedText },
                    @corners: RoundedCorners::Left,
                }
                #Cut
                Node { flex_grow: 1.0 }
                InteractionDisabled
                --
                @FeathersButton {
                    @caption: bsn! {
                        @icon(icons::CHEVRON_DOWN)
                        Node { height: px(10) }
                        UiTransform { rotation: {Rot2::turn_fraction(0.5)} }
                    },
                    @corners: RoundedCorners::None,
                }
                #LayerUp
                InteractionDisabled
                --
                @FeathersButton {
                    @caption: bsn! {
                        @icon(icons::CHEVRON_DOWN)
                        Node { height: px(10) }
                    },
                    @corners: RoundedCorners::Right,
                }
                #LayerDown
                InteractionDisabled
            ]
            --
            // TODO(PAN28c): New and Clone act on the chosen exemplar.
            @FeathersButton { @caption: bsn! { Text("New…") ThemedText } }
            #New
            Node { grid_column: {GridPlacement::start(1)}, grid_row: {GridPlacement::start(4)} }
            InteractionDisabled
            --
            @FeathersButton { @caption: bsn! { Text("Clone…") ThemedText } }
            #Clone
            Node { grid_column: {GridPlacement::start(1)}, grid_row: {GridPlacement::start(5)} }
            InteractionDisabled
        ]
    }
}

#[derive(SystemParam)]
struct SceneryExemplarsParams<'w> {
    settings: Res<'w, EditorModeSettings>,
    choices: Res<'w, SceneryChoices>,
}

impl ImmediateAttach<CapsUiFeathers> for SceneryExemplars {
    type Params = SceneryExemplarsParams<'static>;

    /// The chosen tool's chooser, keyed by the tool, so choosing another replaces it; none for a
    /// tool that places no exemplars.
    fn construct(ui: &mut Imm<CapsUiFeathers>, params: &mut SceneryExemplarsParams) {
        let tool = params.settings.scenery_tool;
        if placed_type(tool).is_none() {
            return;
        }
        let selected = params.choices.choice(tool);
        ui.ch_id(tool)
            .on_spawn_apply_scene(move || exemplar_chooser(tool, selected.clone()));
    }
}

#[derive(SystemParam)]
struct SceneryPaletteParams<'w, 's> {
    settings: ResMut<'w, EditorModeSettings>,
    /// The tool the buttons last showed: `is_changed` would miss the palette's own changes.
    shown: Local<'s, Option<SceneryTool>>,
    commands: Commands<'w, 's>,
}

impl ImmediateAttach<CapsUiFeathers> for SceneryPalette {
    type Params = SceneryPaletteParams<'static, 'static>;

    /// A button per tool, each named for its tool, the chosen one highlighted, then the rotate
    /// buttons. The grid's outer corners are rounded.
    fn construct(ui: &mut Imm<CapsUiFeathers>, params: &mut SceneryPaletteParams) {
        // Edit a copy: writing through `ResMut` every frame would mark the settings changed every
        // frame.
        let mut settings = *params.settings;
        let changed = *params.shown != Some(settings.scenery_tool);
        *params.shown = Some(settings.scenery_tool);
        let last = BUTTONS.len() - 1;
        for (index, &(name, image, height, action)) in BUTTONS.iter().enumerate() {
            let selected = action == PaletteAction::Tool(settings.scenery_tool);
            let corners = match index {
                0 => RoundedCorners::TopLeft,
                i if i == COLUMNS - 1 => RoundedCorners::TopRight,
                i if i == last + 1 - COLUMNS => RoundedCorners::BottomLeft,
                i if i == last => RoundedCorners::BottomRight,
                _ => RoundedCorners::None,
            };
            let mut button = ui
                .ch_id(name)
                .on_spawn_apply_scene(move || {
                    bsn! {
                        @FeathersButton {
                            @caption: bsn! {
                                Node { height: px(height) }
                                ImageNode { image: image }
                            },
                            @corners: corners,
                            @variant: ButtonVariant::selected(selected),
                        }
                        Node {
                            width: px(BUTTON_WIDTH),
                            height: px(BUTTON_HEIGHT),
                            padding: px(0),
                        }
                    }
                })
                .on_spawn_insert(move || Name::new(name))
                .on_change_insert(changed, move || ButtonVariant::selected(selected));
            match action {
                PaletteAction::Tool(tool) => {
                    if button.activated() {
                        settings.scenery_tool = tool;
                    }
                }
                // TODO(PAN5): enabled while scenery is selected, turning it.
                PaletteAction::Rotate => {
                    button.on_spawn_insert(|| InteractionDisabled);
                }
            }
        }
        if params.settings.set_if_neq(settings) {
            params.commands.queue(SaveSettingsDeferred(SAVE_DELAY));
        }
    }
}
