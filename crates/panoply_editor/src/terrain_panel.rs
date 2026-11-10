//! Terrain mode's panel, laid out as faery's: on the left the tool palette, then the contour's Has
//! water and Has terrain, Terrain Groups and Biomes; on the right the contour list. The palette is
//! a segmented grid of buttons, as the mode selector is: each tool button chooses its
//! [`TerrainTool`], and the rotate buttons act on the selected parcel.

use bevy::{
    ecs::system::SystemParam,
    feathers::{
        containers::{group, group_body},
        controls::{ButtonVariant, FeathersButton},
        display::label,
        rounded_corners::RoundedCorners,
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
    mode_selector::{EditorModeSettings, TerrainTool},
    parcel_selection::SelectedParcelEdits,
    shell::SAVE_DELAY,
};

/// The panel's root.
#[derive(Component, Default, Clone)]
pub struct TerrainPanel;

/// The root of the immediate-mode palette.
#[derive(Component, Default, Clone)]
struct TerrainPalette;

/// Each palette button's size: the mode selector's width, taller to show the icons large enough
/// that their downscaling does not alias.
const BUTTON_WIDTH: f32 = 44.0;
const BUTTON_HEIGHT: f32 = 44.0;
const COLUMNS: usize = 3;

/// What a palette button does: choose a tool, or turn the selected parcel's contour by a number of
/// quarter turns.
#[derive(Clone, Copy, PartialEq)]
enum PaletteAction {
    Tool(TerrainTool),
    Rotate(u8),
}

/// The palette's buttons, in rows of three: a name, its icon and the icon's height, and what it
/// does. Each icon keeps its own aspect, at the height that fits it in 40 by 32. The PNGs are
/// exported at twice these heights, 1:1 on a high-density display, with black in their transparent
/// pixels: filtering blends that colour into an icon's edge, and Inkscape's default white shows as
/// a light fringe.
const BUTTONS: [(&str, &str, f32, PaletteAction); 12] = {
    use PaletteAction::{Rotate, Tool};
    [
        ("RaiseDraw", "editor/icons/raise-draw.png", 32., Tool(TerrainTool::RaiseDraw)),
        ("LowerDraw", "editor/icons/lower-draw.png", 32., Tool(TerrainTool::LowerDraw)),
        ("FlattenDraw", "editor/icons/flatten-draw.png", 32., Tool(TerrainTool::FlattenDraw)),
        ("RaiseRect", "editor/icons/raise-rect.png", 32., Tool(TerrainTool::RaiseRect)),
        ("LowerRect", "editor/icons/lower-rect.png", 32., Tool(TerrainTool::LowerRect)),
        ("FlattenRect", "editor/icons/flatten-rect.png", 32., Tool(TerrainTool::FlattenRect)),
        ("DrawTrees", "editor/icons/pine.png", 32., Tool(TerrainTool::DrawTrees)),
        ("DrawShrubs", "editor/icons/shrub.png", 32., Tool(TerrainTool::DrawShrubs)),
        ("DrawHerbs", "editor/icons/herb.png", 32., Tool(TerrainTool::DrawHerbs)),
        ("EraseFlora", "editor/icons/chop.png", 32., Tool(TerrainTool::EraseFlora)),
        // Three quarter turns one way are one the other.
        ("RotateCcw", "editor/icons/rotate-ccw.png", 26., Rotate(3)),
        ("RotateCw", "editor/icons/rotate-cw.png", 26., Rotate(1)),
    ]
};

pub struct TerrainPanelPlugin;

impl Plugin for TerrainPanelPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(BevyImmediateAttachPlugin::<CapsUiFeathers, TerrainPalette>::new());
    }
}

/// The panel, which `PanelBody` shows while in terrain mode. Has water and Has terrain, the groups,
/// the biomes and the contour list are empty places for now.
pub(crate) fn terrain_panel() -> impl Scene {
    bsn! {
        #TerrainPanel
        TerrainPanel
        Node { column_gap: px(4), flex_grow: 1.0, min_height: px(0) }
        Children [
            Node { flex_direction: FlexDirection::Column, row_gap: px(4), flex_shrink: 0.0 }
            Children [
                TerrainPalette
                Node {
                    display: Display::Grid,
                    grid_template_columns: {
                        vec![RepeatedGridTrack::px(COLUMNS as u16, BUTTON_WIDTH)]
                    },
                    // The 1-pixel seams of a segmented button.
                    column_gap: px(1),
                    row_gap: px(1),
                }
                --
                #ContourFlags
                Node { flex_direction: FlexDirection::Column }
                --
                @label("Terrain Groups")
                --
                @list_place()
                #TerrainGroups
                --
                @label("Biomes")
                --
                @list_place()
                #Biomes
            ]
            --
            @list_place()
            #ContourList
            Node { flex_grow: 1.0, min_width: px(0) }
        ]
    }
}

/// An empty group box that a list will fill, sharing the column's height with its siblings.
fn list_place() -> impl Scene {
    bsn! {
        @group()
        Node { flex_grow: 1.0, min_height: px(0) }
        Children [
            @group_body()
            Node { flex_grow: 1.0, min_height: px(0) }
        ]
    }
}

#[derive(SystemParam)]
struct TerrainPaletteParams<'w, 's> {
    settings: ResMut<'w, EditorModeSettings>,
    parcel: SelectedParcelEdits<'w, 's>,
    /// The tool the buttons last showed, and whether the rotate buttons were enabled: `is_changed`
    /// would miss the palette's own changes.
    shown: Local<'s, Option<(TerrainTool, bool)>>,
    commands: Commands<'w, 's>,
}

impl ImmediateAttach<CapsUiFeathers> for TerrainPalette {
    type Params = TerrainPaletteParams<'static, 'static>;

    /// A button per tool, each named for its tool, the chosen one highlighted, then the rotate
    /// buttons, disabled while no parcel is selected. The grid's outer corners are rounded.
    fn construct(ui: &mut Imm<CapsUiFeathers>, params: &mut TerrainPaletteParams) {
        // Edit a copy: writing through `ResMut` every frame would mark the settings changed every
        // frame.
        let mut settings = *params.settings;
        let can_rotate = params.parcel.parcel().is_some();
        let shown = Some((settings.terrain_tool, can_rotate));
        let changed = *params.shown != shown;
        *params.shown = shown;
        let mut turns = None;
        let last = BUTTONS.len() - 1;
        for (index, &(name, image, height, action)) in BUTTONS.iter().enumerate() {
            let selected = action == PaletteAction::Tool(settings.terrain_tool);
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
                        settings.terrain_tool = tool;
                    }
                }
                PaletteAction::Rotate(by) => {
                    let update = button.will_be_spawned() || changed;
                    button = button.at_this_moment_apply_commands_if(
                        |commands| {
                            if can_rotate {
                                commands.remove::<InteractionDisabled>();
                            } else {
                                commands.insert(InteractionDisabled);
                            }
                        },
                        || update,
                    );
                    if button.activated() {
                        turns = Some(by);
                    }
                }
            }
        }
        if let Some(turns) = turns {
            params.parcel.rotate(turns);
        }
        if params.settings.set_if_neq(settings) {
            params.commands.queue(SaveSettingsDeferred(SAVE_DELAY));
        }
    }
}
