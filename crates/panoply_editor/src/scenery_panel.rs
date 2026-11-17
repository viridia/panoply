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
    ui_widgets::Activate,
};
use bevy_immediate::{
    attach::{BevyImmediateAttachPlugin, ImmediateAttach},
    ui::{activated::ImmUiActivated, CapsUiFeathers},
    Imm,
};

use crate::{
    exemplar_list::{has_list, tool_chooser, SceneryChoices},
    mode_selector::{EditorModeSettings, SceneryTool},
    scenery_pointer::{step_tier, SceneryTier},
    scenery_properties::SceneryProperties,
    shell::SAVE_DELAY,
    wall_tool::{WallSnap, WallToolSettings},
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

/// The palette's buttons, in rows of two: a name, its icon and the icon's height, and the tool it
/// chooses. Each icon keeps its own aspect, at the height that fits it in 40 by 32.
const BUTTONS: [(&str, &str, f32, SceneryTool); 8] = [
    (
        "PlaceFloor",
        "editor/icons/floor-draw.png",
        28.,
        SceneryTool::PlaceFloor,
    ),
    (
        "PlaceWall",
        "editor/icons/wall-draw.png",
        32.,
        SceneryTool::PlaceWall,
    ),
    (
        "PlaceFixture",
        "editor/icons/furnishing-draw.png",
        32.,
        SceneryTool::PlaceFixture,
    ),
    (
        "PlaceActor",
        "editor/icons/actor.png",
        32.,
        SceneryTool::PlaceActor,
    ),
    (
        "DrawTerrainFx",
        "editor/icons/road-draw.png",
        28.,
        SceneryTool::DrawTerrainFx,
    ),
    (
        "Interact",
        "editor/icons/machine.png",
        32.,
        SceneryTool::Interact,
    ),
    (
        "Layers",
        "editor/icons/layers.png",
        32.,
        SceneryTool::Layers,
    ),
    (
        "RectSelect",
        "editor/icons/rect-select.png",
        32.,
        SceneryTool::RectSelect,
    ),
];

/// The wall toolbar's buttons' size, and their icons' height.
const OPTION_WIDTH: f32 = 32.0;
const OPTION_HEIGHT: f32 = 28.0;
const OPTION_ICON: f32 = 18.0;

/// The snap buttons: a name, an icon, and the snap each chooses.
const SNAPS: [(&str, &str, WallSnap); 3] = [
    (
        "SnapNormal",
        "editor/icons/grid-normal.png",
        WallSnap::Normal,
    ),
    (
        "SnapOffset",
        "editor/icons/grid-offset.png",
        WallSnap::Offset,
    ),
    (
        "SnapQuarter",
        "editor/icons/grid-fine.png",
        WallSnap::Quarter,
    ),
];

pub struct SceneryPanelPlugin;

impl Plugin for SceneryPanelPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            BevyImmediateAttachPlugin::<CapsUiFeathers, SceneryPalette>::new(),
            BevyImmediateAttachPlugin::<CapsUiFeathers, SceneryExemplars>::new(),
            BevyImmediateAttachPlugin::<CapsUiFeathers, WallToolbar>::new(),
        ))
        .add_systems(Update, show_tool_options);
    }
}

/// Whether `tool` has options above its exemplars.
fn has_options(tool: SceneryTool) -> bool {
    // TODO(PAN95): the floor tool's shapes.
    tool == SceneryTool::PlaceWall
}

/// Shows the options row for a tool that has options; otherwise the exemplars take its place.
fn show_tool_options(
    settings: Res<EditorModeSettings>,
    mut options: Query<&mut Node, (With<WallToolbar>, Without<SceneryExemplars>)>,
    mut exemplars: Query<&mut Node, (With<SceneryExemplars>, Without<WallToolbar>)>,
) {
    let shown = has_options(settings.scenery_tool);
    for mut node in &mut options {
        let display = if shown { Display::Flex } else { Display::None };
        if node.display != display {
            node.display = display;
        }
    }
    for mut node in &mut exemplars {
        let rows = GridPlacement::start_end(if shown { 2 } else { 1 }, 6);
        if node.grid_row != rows {
            node.grid_row = rows;
        }
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
            // The chosen tool's options, shown only for a tool that has some
            // (`show_tool_options`).
            WallToolbar
            #ToolOptions
            Node {
                grid_column: {GridPlacement::start(2)},
                grid_row: {GridPlacement::start(1)},
                height: px(32),
                column_gap: px(8),
                align_items: AlignItems::Center,
            }
            --
            // Down to the bottom of Clone…. An auto row grows to the max-content height of what
            // spans it before a flexible row gets any space, so the list would push the
            // properties out; `min_height: 0` does not stop that, since it lowers only the
            // minimum. A zero height keeps the list out of the rows' sizing, and the percentage
            // minimum, resolved against the grid area once the rows are sized, fills the area.
            SceneryExemplars
            #Exemplars
            Node {
                grid_column: {GridPlacement::start(2)},
                grid_row: {GridPlacement::start_end(2, 6)},
                flex_direction: FlexDirection::Column,
                min_width: px(0),
                min_height: percent(100),
                height: px(0),
            }
            --
            // The current tier, which ▲ ▼ step. TODO(PAN76a): Cut, which hides what is above it.
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
                #TierUp
                on(|_: On<Activate>, mut tier: ResMut<SceneryTier>| step_tier(&mut tier, 1))
                --
                @FeathersButton {
                    @caption: bsn! {
                        @icon(icons::CHEVRON_DOWN)
                        Node { height: px(10) }
                    },
                    @corners: RoundedCorners::Right,
                }
                #TierDown
                on(|_: On<Activate>, mut tier: ResMut<SceneryTier>| step_tier(&mut tier, -1))
            ]
            --
            // TODO(PAN28e): New and Clone act on the chosen exemplar.
            @FeathersButton { @caption: bsn! { Text("New…") ThemedText } }
            #New
            Node { grid_column: {GridPlacement::start(1)}, grid_row: {GridPlacement::start(4)} }
            InteractionDisabled
            --
            @FeathersButton { @caption: bsn! { Text("Clone…") ThemedText } }
            #Clone
            Node { grid_column: {GridPlacement::start(1)}, grid_row: {GridPlacement::start(5)} }
            InteractionDisabled
            --
            // The chosen exemplar's properties, across both columns in the last row, which takes
            // the rest of the panel's height.
            #Properties
            SceneryProperties
            Node {
                grid_column: {GridPlacement::start_end(1, -1)},
                grid_row: {GridPlacement::start(6)},
                flex_direction: FlexDirection::Column,
                min_height: px(0),
            }
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
    /// tool that has no list.
    fn construct(ui: &mut Imm<CapsUiFeathers>, params: &mut SceneryExemplarsParams) {
        let tool = params.settings.scenery_tool;
        if !has_list(tool) {
            return;
        }
        let selected = params.choices.choice(tool);
        ui.ch_id(tool)
            .on_spawn_apply_scene(move || tool_chooser(tool, selected.clone()));
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

    /// A button per tool, each named for its tool, the chosen one highlighted. The grid's outer
    /// corners are rounded.
    fn construct(ui: &mut Imm<CapsUiFeathers>, params: &mut SceneryPaletteParams) {
        // Edit a copy: writing through `ResMut` every frame would mark the settings changed every
        // frame.
        let mut settings = *params.settings;
        let changed = *params.shown != Some(settings.scenery_tool);
        *params.shown = Some(settings.scenery_tool);
        let last = BUTTONS.len() - 1;
        for (index, &(name, image, height, tool)) in BUTTONS.iter().enumerate() {
            let selected = tool == settings.scenery_tool;
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
            if button.activated() {
                settings.scenery_tool = tool;
            }
        }
        if params.settings.set_if_neq(settings) {
            params.commands.queue(SaveSettingsDeferred(SAVE_DELAY));
        }
    }
}

/// The wall tool's options: the snap, Replace, and the rotate buttons.
#[derive(Component, Default, Clone)]
struct WallToolbar;

#[derive(SystemParam)]
struct WallToolbarParams<'w, 's> {
    settings: ResMut<'w, WallToolSettings>,
    /// The options the buttons last showed: `is_changed` would miss the toolbar's own changes.
    shown: Local<'s, Option<WallToolSettings>>,
    commands: Commands<'w, 's>,
}

impl ImmediateAttach<CapsUiFeathers> for WallToolbar {
    type Params = WallToolbarParams<'static, 'static>;

    /// The snap buttons, segmented, the chosen one highlighted; Replace, highlighted while on; and
    /// Rotate left and right, segmented. Each button is named for what it does.
    fn construct(ui: &mut Imm<CapsUiFeathers>, params: &mut WallToolbarParams) {
        let mut settings = *params.settings;
        let changed = *params.shown != Some(settings);
        *params.shown = Some(settings);
        ui.ch_id("snap").on_spawn_insert(segmented).add(|ui| {
            for (index, &(name, image, snap)) in SNAPS.iter().enumerate() {
                let corners = match index {
                    0 => RoundedCorners::Left,
                    i if i == SNAPS.len() - 1 => RoundedCorners::Right,
                    _ => RoundedCorners::None,
                };
                let selected = settings.snap == snap;
                if option_button(ui, name, image, corners, selected, changed, false) {
                    settings.snap = snap;
                }
            }
        });
        let replace = settings.replace;
        if option_button(
            ui,
            "Replace",
            "editor/icons/lucide/eraser.png",
            RoundedCorners::All,
            replace,
            changed,
            false,
        ) {
            settings.replace = !replace;
        }
        ui.ch_id("rotate").on_spawn_insert(segmented).add(|ui| {
            let left = "editor/icons/rotate-view.png";
            if option_button(
                ui,
                "RotateLeft",
                left,
                RoundedCorners::Left,
                false,
                false,
                true,
            ) {
                settings.turn(-1);
            }
            if option_button(
                ui,
                "RotateRight",
                left,
                RoundedCorners::Right,
                false,
                false,
                false,
            ) {
                settings.turn(1);
            }
        });
        if params.settings.set_if_neq(settings) {
            params.commands.queue(SaveSettingsDeferred(SAVE_DELAY));
        }
    }
}

/// A row of buttons joined by 1-pixel seams.
fn segmented() -> Node {
    Node {
        column_gap: px(1),
        ..default()
    }
}

/// One of the wall toolbar's buttons, named `name`, its icon mirrored if `mirrored`, as the rotate
/// buttons share one; highlighted while `selected`, updated when `changed`. Returns whether it was
/// clicked.
fn option_button(
    ui: &mut Imm<CapsUiFeathers>,
    name: &'static str,
    image: &'static str,
    corners: RoundedCorners,
    selected: bool,
    changed: bool,
    mirrored: bool,
) -> bool {
    let flip = if mirrored {
        UiTransform::from_scale(Vec2::new(-1.0, 1.0))
    } else {
        UiTransform::IDENTITY
    };
    ui.ch_id(name)
        .on_spawn_apply_scene(move || {
            bsn! {
                @FeathersButton {
                    @caption: bsn! {
                        Node { height: px(OPTION_ICON) }
                        ImageNode { image: image }
                        UiTransform { scale: {flip.scale}, rotation: {flip.rotation} }
                    },
                    @corners: corners,
                    @variant: ButtonVariant::selected(selected),
                }
                Node {
                    width: px(OPTION_WIDTH),
                    height: px(OPTION_HEIGHT),
                    padding: px(0),
                }
            }
        })
        .on_spawn_insert(move || Name::new(name))
        .on_change_insert(changed, move || ButtonVariant::selected(selected))
        .activated()
}
