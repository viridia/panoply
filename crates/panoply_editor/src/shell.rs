//! The editor's shell: a panel at the left of the game's view, split from it by a handle that
//! drags. T shows or hides the panel, and G opens the Go To dialog (`goto`). Atop the panel are the
//! mode selector (`mode_selector`), the save button, which writes every unsaved asset (`unsaved`),
//! and the Show menu, which turns debug views on and off. Below them, [`PanelBody`] holds the
//! current mode's panel.

use std::time::Duration;

use bevy::{
    ecs::system::SystemParam,
    feathers::{
        controls::{
            FeathersButton, FeathersCheckbox, FeathersMenu, FeathersMenuPopup,
            FeathersMenuToolButton, FeathersPane, FeathersSplitPane, FeathersSplitPaneHandle,
        },
        display::icon,
        theme::{ThemeBackgroundColor, ThemedText},
        tokens,
    },
    prelude::*,
    settings::{ReflectSettingsGroup, SaveSettingsDeferred, SettingsGroup},
    ui::{InteractionDisabled, UiGlobalTransform, UiSystems},
    ui_widgets::{split_pane_self_update, Activate, Pane, ValueChange},
    window::PrimaryWindow,
};
use bevy_action_map::prelude::*;
use bevy_immediate::{
    attach::{BevyImmediateAttachPlugin, ImmediateAttach},
    ui::{checked::ImmUiChecked, CapsUiFeathers},
    Imm, ImmEntity,
};
use panoply_core::{realm_physics::collision_groups, GameViewNode, ViewOverlay, ViewportInset};
use rapier3d::prelude::Group;

use crate::{
    debug_views::DebugViews,
    goto::OpenGotoDialog,
    mode_selector::{EditorMode, ModeSelector, ToggleGameMode, SELECTOR_WIDTH},
    realm_panel::realm_panel,
    scenery_panel::scenery_panel,
    terrain_panel::terrain_panel,
    unsaved::{SaveAssets, UnsavedAssets},
};

/// Narrowest either pane may be, in logical pixels.
const MIN_PANE_WIDTH: f32 = 200.0;

/// The top row's padding and the gaps between its parts.
const ROW_SPACING: f32 = 8.0;
/// The save button and the Show menu's button are square, as tall as the mode selector.
const ROW_BUTTON: f32 = 32.0;

/// Narrowest the panel may be: wide enough for its top row unsquished. The row's padding at both
/// ends, the selector, then the gaps around the slack between it and the two buttons.
const MIN_PANEL_WIDTH: f32 =
    2.0 * ROW_SPACING + SELECTOR_WIDTH + 3.0 * ROW_SPACING + 2.0 * ROW_BUTTON;

pub(crate) const SAVE_DELAY: Duration = Duration::from_millis(500);

/// The panel's layout, saved between runs.
#[derive(Resource, SettingsGroup, Reflect, Clone, PartialEq, Debug)]
#[reflect(Resource, SettingsGroup, Default)]
#[settings_group(group = "editor_panel")]
pub struct EditorPanelSettings {
    /// Whether the panel shows. Hidden by default, so plans, which start from empty settings, see
    /// the game in the whole window.
    pub visible: bool,
    /// The panel's width, in logical pixels.
    pub width: f32,
}

impl Default for EditorPanelSettings {
    fn default() -> Self {
        Self {
            visible: false,
            width: MIN_PANEL_WIDTH,
        }
    }
}

/// Show or hide the editor panel.
#[derive(InputAction)]
#[action(path = "editor.toggle_panel", output = bool, intent = Button)]
pub struct TogglePanel;

/// Editor shell shortcuts.
#[derive(InputContext)]
#[context(path = "editor.shell", tick = Render)]
pub struct EditorShellControl;

/// The split pane's pane holding the panel.
#[derive(Component, Default, Clone)]
struct PanelPane;

/// The handle between the panel and the view.
#[derive(Component, Default, Clone)]
struct PanelHandle;

/// The pane the game's view fills: the primary camera's viewport follows its rectangle.
#[derive(Component, Default, Clone)]
struct ViewPane;

/// The root of the immediate-mode panel body, below the mode selector: it shows the current
/// mode's panel. Built each frame from the mode, so the mode the app starts in has its panel
/// without waiting on state transitions, which run before the shell spawns.
#[derive(Component, Default, Clone)]
pub(crate) struct PanelBody;

/// The save button, disabled while nothing is unsaved.
#[derive(Component, Default, Clone)]
struct SaveButton;

/// The root of the immediate-mode Show menu.
#[derive(Component, Default, Clone)]
struct ShowMenu;

pub struct EditorShellPlugin;

impl Plugin for EditorShellPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            BevyImmediateAttachPlugin::<CapsUiFeathers, ShowMenu>::new(),
            BevyImmediateAttachPlugin::<CapsUiFeathers, PanelBody>::new(),
        ))
        .add_context::<EditorShellControl>(|context| {
            context.bind::<TogglePanel>(KeyCode::KeyT);
            context.bind::<OpenGotoDialog>(KeyCode::KeyG);
            // Not Tab, which feathers' tab navigation takes.
            context.bind::<ToggleGameMode>(KeyCode::F2);
        })
        .add_systems(
            Startup,
            (
                |mut commands: Commands| {
                    commands.spawn((Name::new("EditorShellControl"), EditorShellControl));
                },
                shell.spawn(),
            ),
        )
        .add_systems(
            Update,
            (
                toggle_panel,
                show_panel,
                fit_panel_width,
                adopt_view_overlays,
                enable_save.run_if(resource_changed::<UnsavedAssets>),
            ),
        )
        .add_systems(PostUpdate, follow_view_pane.after(UiSystems::Layout));
    }
}

fn shell() -> impl Scene {
    bsn! {
        #EditorShell
        Node {
            width: percent(100),
            height: percent(100),
            min_height: px(0),
        }
        Pickable::IGNORE
        Children [
            @FeathersSplitPane
            Node {
                width: percent(100),
                height: percent(100),
                min_height: px(0),
            }
            Pickable::IGNORE
            on(split_pane_self_update)
            on(save_panel_width)
            Children [
                @FeathersPane { @size: 1.0, @min_size: MIN_PANEL_WIDTH }
                #EditorPanel
                PanelPane
                ThemeBackgroundColor(tokens::WINDOW_BG)
                Children [
                    Node {
                        align_items: AlignItems::Center,
                        column_gap: px(ROW_SPACING),
                        padding: UiRect::all(px(ROW_SPACING)),
                    }
                    Children [
                        ModeSelector
                        Node { column_gap: px(1) }
                        --
                        // The space between the selector and the buttons takes up the slack.
                        Node { flex_grow: 1.0 }
                        --
                        @FeathersButton {
                            @caption: bsn! { @icon("editor/icons/save.png") Node { height: px(20) } },
                        }
                        // Square, as tall as the mode selector's buttons.
                        Node {
                            width: px(ROW_BUTTON),
                            height: px(ROW_BUTTON),
                            padding: UiRect::ZERO,
                        }
                        #Save
                        SaveButton
                        InteractionDisabled
                        on(|_: On<Activate>, mut saves: MessageWriter<SaveAssets>| {
                            saves.write(SaveAssets);
                        })
                        --
                        ShowMenu
                        Node
                    ]
                    --
                    Node {
                        flex_direction: FlexDirection::Column,
                        flex_grow: 1.0,
                        min_height: px(0),
                        padding: UiRect { top: px(0), ..UiRect::all(px(8)) },
                    }
                    #PanelBody
                    PanelBody
                ]
                --
                @FeathersSplitPaneHandle
                PanelHandle
                --
                @FeathersPane { @size: 3.0, @min_size: MIN_PANE_WIDTH }
                #EditorView
                ViewPane
                GameViewNode
                Pickable::IGNORE
            ]
        ]
    }
}

fn enable_save(
    unsaved: Res<UnsavedAssets>,
    button: Single<(Entity, Has<InteractionDisabled>), With<SaveButton>>,
    mut commands: Commands,
) {
    let (button, disabled) = *button;
    match (unsaved.is_empty(), disabled) {
        (false, true) => {
            commands.entity(button).remove::<InteractionDisabled>();
        }
        (true, false) => {
            commands.entity(button).insert(InteractionDisabled);
        }
        _ => {}
    }
}

fn toggle_panel(
    input: ContextActions<EditorShellControl>,
    mut settings: ResMut<EditorPanelSettings>,
    mut commands: Commands,
) {
    if input.fired::<TogglePanel>() {
        settings.visible = !settings.visible;
        commands.queue(SaveSettingsDeferred(SAVE_DELAY));
    }
}

/// Hides the panel and its handle while the panel is off, so the view pane fills the window.
fn show_panel(
    settings: Res<EditorPanelSettings>,
    mut nodes: Query<&mut Node, Or<(With<PanelPane>, With<PanelHandle>)>>,
) {
    if !settings.is_changed() {
        return;
    }
    let display = if settings.visible {
        Display::Flex
    } else {
        Display::None
    };
    for mut node in &mut nodes {
        node.display = display;
    }
}

/// Keeps the panel at its saved width. Pane sizes are flex weights, so with weights in pixels
/// that sum to the space the two panes share, the panel is exactly `width` wide. The weights are
/// reset when that space changes (the window resized, or the panel shown) or the width does.
fn fit_panel_width(
    settings: Res<EditorPanelSettings>,
    mut panel: Single<(&mut Pane, &ComputedNode), (With<PanelPane>, Without<ViewPane>)>,
    mut view: Single<(&mut Pane, &ComputedNode), With<ViewPane>>,
    mut last_total: Local<f32>,
) {
    if !settings.visible {
        *last_total = 0.0;
        return;
    }
    let logical_width = |node: &ComputedNode| node.size().x * node.inverse_scale_factor();
    let total = logical_width(panel.1) + logical_width(view.1);
    if total < MIN_PANEL_WIDTH + MIN_PANE_WIDTH {
        return;
    }
    if (total - *last_total).abs() < 0.5 && !settings.is_changed() {
        return;
    }
    *last_total = total;

    let width = settings
        .width
        .clamp(MIN_PANEL_WIDTH, total - MIN_PANE_WIDTH);
    if (panel.0.size - width).abs() >= 0.5 {
        panel.0.size = width;
    }
    if (view.0.size - (total - width)).abs() >= 0.5 {
        view.0.size = total - width;
    }
}

/// Observer on the split pane: a finished drag proposes the panes' sizes, and with weights in
/// pixels (see [`fit_panel_width`]) the first is the panel's new width.
fn save_panel_width(
    change: On<ValueChange<Vec<f32>>>,
    mut settings: ResMut<EditorPanelSettings>,
    mut commands: Commands,
) {
    if !change.is_final {
        return;
    }
    let Some(&width) = change.value.first() else {
        return;
    };
    let width = width.round();
    if settings.width != width {
        settings.width = width;
        commands.queue(SaveSettingsDeferred(SAVE_DELAY));
    }
}

/// Moves overlays on the game's view (location, frame time) into the view pane, so they keep to
/// the view beside the panel.
fn adopt_view_overlays(
    view: Single<Entity, With<ViewPane>>,
    overlays: Query<Entity, (With<ViewOverlay>, Without<ChildOf>)>,
    mut commands: Commands,
) {
    for overlay in &overlays {
        commands.entity(*view).add_child(overlay);
    }
}

/// Insets the primary camera's viewport to the view pane while the panel shows. Hidden, the
/// inset is reset once and then left alone, so plans can set it over BRP.
fn follow_view_pane(
    settings: Res<EditorPanelSettings>,
    pane: Single<(&ComputedNode, &UiGlobalTransform), With<ViewPane>>,
    window: Single<&Window, With<PrimaryWindow>>,
    mut inset: ResMut<ViewportInset>,
    mut had_panel: Local<bool>,
) {
    if !settings.visible {
        if std::mem::take(&mut *had_panel) {
            inset.set_if_neq(ViewportInset::default());
        }
        return;
    }
    // The node's center and size are in physical pixels; the inset is in logical ones.
    let (node, transform) = *pane;
    let size = node.size();
    if size.x == 0.0 || size.y == 0.0 {
        return;
    }
    *had_panel = true;
    let center = transform.affine().translation;
    let min = center - size * 0.5;
    let max = center + size * 0.5;
    let bounds = window.physical_size().as_vec2();
    let scale = window.resolution.scale_factor();
    inset.set_if_neq(ViewportInset {
        left: min.x / scale,
        top: min.y / scale,
        right: (bounds.x - max.x) / scale,
        bottom: (bounds.y - max.y) / scale,
    });
}

#[derive(SystemParam)]
pub(crate) struct PanelBodyParams<'w> {
    mode: Option<Res<'w, State<EditorMode>>>,
}

impl ImmediateAttach<CapsUiFeathers> for PanelBody {
    type Params = PanelBodyParams<'static>;

    /// One child, keyed by the mode, so a mode switch replaces it; none while playing. The other
    /// modes have no panel yet.
    fn construct(ui: &mut Imm<CapsUiFeathers>, params: &mut PanelBodyParams) {
        match params.mode.as_deref().map(State::get) {
            Some(EditorMode::Realm) => {
                ui.ch_id("realm").on_spawn_apply_scene(realm_panel);
            }
            Some(EditorMode::Terrain) => {
                ui.ch_id("terrain").on_spawn_apply_scene(terrain_panel);
            }
            Some(EditorMode::Scenery) => {
                ui.ch_id("scenery").on_spawn_apply_scene(scenery_panel);
            }
            Some(EditorMode::Catalogs) | None => {}
        }
    }
}

#[derive(SystemParam)]
struct ShowMenuParams<'w, 's> {
    views: ResMut<'w, DebugViews>,
    commands: Commands<'w, 's>,
}

impl ImmediateAttach<CapsUiFeathers> for ShowMenu {
    type Params = ShowMenuParams<'static, 'static>;

    /// The Show menu, named `Show`: a checkbox per debug view, named by its caption. Checkboxes,
    /// not menu items: picking an item closes the menu, and a checkbox takes the focus without
    /// closing it, so several views can be toggled in one visit.
    fn construct(ui: &mut Imm<CapsUiFeathers>, params: &mut ShowMenuParams) {
        // Edit a copy: writing through `ResMut` every frame would mark the views changed every
        // frame.
        let mut views = params.views.clone();
        ui.ch_id("show_menu")
            // As tall as its button, which is taller than feathers' row height.
            .on_spawn_apply_scene(|| bsn! { @FeathersMenu Node { height: px(ROW_BUTTON) } })
            .add(|ui| {
                ui.ch().on_spawn_apply_scene(|| {
                    bsn! {
                        @FeathersMenuToolButton {
                            @caption: bsn! { @icon("editor/icons/menu.png") Node { height: px(20) } },
                            @arrow: false,
                        }
                        // Square, as tall as the mode selector's buttons.
                        Node { width: px(ROW_BUTTON), height: px(ROW_BUTTON) }
                        #Show
                    }
                });
                ui.ch()
                    .on_spawn_apply_scene(|| bsn! { @FeathersMenuPopup })
                    .add(|ui| {
                        ui.ch()
                            .on_spawn_insert(|| column(UiRect::axes(px(8), px(4))))
                            .add(|ui| {
                                checkbox(ui.ch_id("physics"), "Physics")
                                    .checked(&mut views.physics);
                                // Groups indented under Physics, which they filter.
                                ui.ch()
                                    .on_spawn_insert(|| column(UiRect::left(px(16))))
                                    .add(|ui| {
                                        let mut groups =
                                            Group::from_bits_truncate(views.physics_groups);
                                        for &(caption, group) in collision_groups::CAPTIONED {
                                            let mut shown = groups.contains(group);
                                            checkbox(ui.ch_id(caption), caption)
                                                .checked(&mut shown);
                                            groups.set(group, shown);
                                        }
                                        views.physics_groups = groups.bits();
                                    });
                            });
                    });
            });
        if params.views.set_if_neq(views) {
            params.commands.queue(SaveSettingsDeferred(SAVE_DELAY));
        }
    }
}

fn column(padding: UiRect) -> Node {
    Node {
        flex_direction: FlexDirection::Column,
        padding,
        row_gap: px(4),
        ..default()
    }
}

/// A feathers checkbox showing `caption`, named after it.
fn checkbox<'r, 'w, 's>(
    item: ImmEntity<'r, 'w, 's, CapsUiFeathers>,
    caption: &'static str,
) -> ImmEntity<'r, 'w, 's, CapsUiFeathers> {
    item.on_spawn_apply_scene(move || {
        bsn! { @FeathersCheckbox { @caption: bsn! { Text(caption) ThemedText } } }
    })
    .on_spawn_insert(move || Name::new(caption))
}
