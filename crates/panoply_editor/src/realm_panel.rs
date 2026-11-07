//! Realm mode's panel: stats about the map, then the nav points with Go, + and −. Go, a
//! double-click or Enter travels to the selected point; + names the current view as a new point,
//! through the text-entry dialog; − removes the selected point. + and − change the nav points in
//! memory, where the list shows them at once, and mark them unsaved for the save button.

use bevy::{
    ecs::system::SystemParam,
    feathers::{
        containers::{group, group_body},
        controls::FeathersButton,
        display::label_dim,
        theme::ThemedText,
    },
    platform::collections::HashSet,
    prelude::*,
    ui::InteractionDisabled,
    ui_widgets::Activate,
};
use bevy_immediate::{
    attach::{BevyImmediateAttachPlugin, ImmediateAttach},
    ui::{text::ImmUiText, CapsUiFeathers},
    Imm,
};
use panoply_core::{NavPoint, NavPointsAsset, NavPointsHandleResource, Realm, Viewpoint};

use crate::{
    mode_selector::EditorMode,
    nav_point_list::{nav_point_list, ActivateNavPoint, NavPointList, Travel},
    text_entry::{text_entry_dialog, TextEntered},
    unsaved::UnsavedAssets,
};

/// The panel's root.
#[derive(Component, Default, Clone)]
pub struct RealmPanel;

/// The root of the immediate-mode stats.
#[derive(Component, Default, Clone)]
struct RealmStats;

/// A button disabled while no nav point is selected: Go and −.
#[derive(Component, Default, Clone)]
struct NeedsSelection;

pub struct RealmPanelPlugin;

impl Plugin for RealmPanelPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(BevyImmediateAttachPlugin::<CapsUiFeathers, RealmStats>::new())
            .add_systems(Update, enable_buttons.run_if(in_state(EditorMode::Realm)));
    }
}

/// The panel, which `PanelBody` shows while in realm mode.
pub(crate) fn realm_panel() -> impl Scene {
    bsn! {
        #RealmPanel
        RealmPanel
        Node {
            flex_direction: FlexDirection::Column,
            row_gap: px(4),
            flex_grow: 1.0,
            min_height: px(0),
        }
        on(travel_to_activated)
        Children [
            @group() Children [
                @group_body()
                // Feathers pads a group's body by 6; the editor spaces by 4.
                Node { padding: px(4) }
                Children [
                    RealmStats
                    Node { flex_direction: FlexDirection::Column, row_gap: px(4) }
                ]
            ]
            --
            @nav_point_list(false)
            --
            Node { justify_content: JustifyContent::FlexEnd, column_gap: px(4) }
            Children [
                @FeathersButton { @caption: bsn! { Text("Go") ThemedText } }
                #Go
                NeedsSelection
                InteractionDisabled
                on(go)
                --
                @FeathersButton { @caption: bsn! { Text("+") ThemedText } }
                #AddNavPoint
                on(open_add_dialog)
                --
                @FeathersButton { @caption: bsn! { Text("−") ThemedText } }
                #RemoveNavPoint
                NeedsSelection
                InteractionDisabled
                on(remove_selected)
            ]
        ]
    }
}

#[derive(SystemParam)]
struct RealmStatsParams<'w, 's> {
    viewpoint: Res<'w, Viewpoint>,
    realms: Query<'w, 's, &'static Realm>,
}

impl ImmediateAttach<CapsUiFeathers> for RealmStats {
    type Params = RealmStatsParams<'static, 'static>;

    /// A row per stat: the viewpoint's realm, its size in parcels, and the view position.
    fn construct(ui: &mut Imm<CapsUiFeathers>, params: &mut RealmStatsParams) {
        let realm = params.viewpoint.realm.and_then(|e| params.realms.get(e).ok());
        let position = params.viewpoint.position;
        let rows = [
            ("Realm", realm.map_or("--".into(), |r| r.name.clone())),
            (
                "Size",
                realm.map_or("--".into(), |r| {
                    let size = r.parcel_bounds.size();
                    format!("{} × {} parcels", size.x, size.y)
                }),
            ),
            (
                "View",
                format!("{:.1}, {:.1}, {:.1}", position.x, position.y, position.z),
            ),
        ];
        for (label, value) in rows {
            ui.ch_id(label)
                .on_spawn_insert(|| Node {
                    column_gap: px(4),
                    ..default()
                })
                .add(|ui| {
                    // Feathers' labels style themselves: font and color propagate only through
                    // entities with `ThemedText`, and these rows have none.
                    ui.ch()
                        .on_spawn_apply_scene(move || label_dim(label))
                        .on_spawn_insert(|| Node {
                            width: px(48),
                            flex_shrink: 0.0,
                            ..default()
                        });
                    ui.ch()
                        .on_spawn_apply_scene(|| bevy::feathers::display::label(""))
                        .on_spawn_insert(move || Name::new(label))
                        .text(value);
                });
        }
    }
}

/// The panel's nav point list.
#[derive(SystemParam)]
struct PanelList<'w, 's> {
    panels: Query<'w, 's, Entity, With<RealmPanel>>,
    children: Query<'w, 's, &'static Children>,
    lists: Query<'w, 's, &'static mut NavPointList>,
}

impl PanelList<'_, '_> {
    fn entity(&self) -> Option<Entity> {
        let panel = self.panels.iter().next()?;
        self.children
            .iter_descendants(panel)
            .find(|&e| self.lists.contains(e))
    }

    fn selected(&self) -> Option<String> {
        self.lists.get(self.entity()?).ok()?.selected.clone()
    }

    fn select(&mut self, name: Option<String>) {
        if let Some(list) = self.entity() {
            self.lists.get_mut(list).unwrap().selected = name;
        }
    }
}

/// Enables Go and − while a point is selected.
fn enable_buttons(
    list: PanelList,
    panels: Query<Entity, With<RealmPanel>>,
    children: Query<&Children>,
    buttons: Query<Has<InteractionDisabled>, With<NeedsSelection>>,
    mut commands: Commands,
) {
    let Some(panel) = panels.iter().next() else {
        return;
    };
    let selected = list.selected().is_some();
    for button in children.iter_descendants(panel) {
        match (selected, buttons.get(button)) {
            (true, Ok(true)) => {
                commands.entity(button).remove::<InteractionDisabled>();
            }
            (false, Ok(false)) => {
                commands.entity(button).insert(InteractionDisabled);
            }
            _ => {}
        }
    }
}

/// Observer on Go.
fn go(_activate: On<Activate>, list: PanelList, mut travel: Travel) {
    if let Some(name) = list.selected() {
        travel.to(&name);
    }
}

/// Observer on the panel: a double-click or Enter in its list.
fn travel_to_activated(activate: On<ActivateNavPoint>, mut travel: Travel) {
    travel.to(&activate.name);
}

/// Observer on +: asks for the new point's name, which must not be taken.
fn open_add_dialog(
    _activate: On<Activate>,
    handle: Res<NavPointsHandleResource>,
    assets: Res<Assets<NavPointsAsset>>,
    mut commands: Commands,
) {
    let taken: HashSet<String> = assets
        .get(&handle.0)
        .map(|points| points.0.iter().map(|p| p.name.clone()).collect())
        .unwrap_or_default();
    commands
        .spawn_scene(text_entry_dialog("New Nav Point", "Name", taken))
        .observe(add_point);
}

/// Observer on the dialog: adds a point named as entered, at the current view, and selects it.
fn add_point(
    entered: On<TextEntered>,
    viewpoint: Res<Viewpoint>,
    realms: Query<&Realm>,
    handle: Res<NavPointsHandleResource>,
    mut assets: ResMut<Assets<NavPointsAsset>>,
    mut unsaved: ResMut<UnsavedAssets>,
    mut list: PanelList,
) {
    let Some(realm) = viewpoint.realm.and_then(|e| realms.get(e).ok()) else {
        warn!("No nav point added: the view is in no realm");
        return;
    };
    let Some(mut points) = assets.get_mut(&handle.0) else {
        return;
    };
    points.0.push(NavPoint {
        name: entered.text.clone(),
        realm: realm.name.clone(),
        pos: viewpoint.position,
        caption: None,
        icon: None,
    });
    unsaved.mark(&handle.0);
    list.select(Some(entered.text.clone()));
}

/// Observer on −: removes the selected point.
fn remove_selected(
    _activate: On<Activate>,
    handle: Res<NavPointsHandleResource>,
    mut assets: ResMut<Assets<NavPointsAsset>>,
    mut unsaved: ResMut<UnsavedAssets>,
    mut list: PanelList,
) {
    let Some(name) = list.selected() else {
        return;
    };
    let Some(mut points) = assets.get_mut(&handle.0) else {
        return;
    };
    points.0.retain(|p| p.name != name);
    unsaved.mark(&handle.0);
    list.select(None);
}
