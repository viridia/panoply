//! Realm mode's panel: stats about the map, then the nav points with Go, + and −. Go, a
//! double-click or Enter travels to the selected point; + names the current view as a new point,
//! through the text-entry dialog; − removes the selected point. + and − change the nav points in
//! memory, where the list shows them at once, mark them unsaved for the save button, and can be
//! undone.

use std::borrow::Cow;

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
    undo::{UndoAction, UndoStack},
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
    mut edits: NavPointEdits,
    mut list: PanelList,
) {
    let Some(realm) = viewpoint.realm.and_then(|e| realms.get(e).ok()) else {
        warn!("No nav point added: the view is in no realm");
        return;
    };
    edits.make(NavPointChange::Insert {
        index: usize::MAX,
        point: NavPoint {
            name: entered.text.clone(),
            realm: realm.name.clone(),
            pos: viewpoint.position,
            caption: None,
            icon: None,
        },
    });
    list.select(Some(entered.text.clone()));
}

/// Observer on −: removes the selected point.
fn remove_selected(_activate: On<Activate>, mut edits: NavPointEdits, mut list: PanelList) {
    let Some(name) = list.selected() else {
        return;
    };
    edits.make(NavPointChange::Remove { name });
    list.select(None);
}

/// What + and − need to change the nav points.
#[derive(SystemParam)]
struct NavPointEdits<'w> {
    handle: Res<'w, NavPointsHandleResource>,
    assets: ResMut<'w, Assets<NavPointsAsset>>,
    unsaved: ResMut<'w, UnsavedAssets>,
    history: ResMut<'w, UndoStack>,
    mode: Res<'w, State<EditorMode>>,
}

impl NavPointEdits<'_> {
    /// Makes `change`, marks the points unsaved, and records how to reverse it.
    fn make(&mut self, change: NavPointChange) {
        let Some(mut points) = self.assets.get_mut(&self.handle.0) else {
            return;
        };
        let Some(inverse) = change.make(&mut points.0) else {
            return;
        };
        self.unsaved.mark(&self.handle.0);
        self.history.push(
            *self.mode.get(),
            NavPointUndo {
                asset: self.handle.0.id(),
                change: inverse,
            },
        );
    }
}

/// One edit to the list of nav points.
enum NavPointChange {
    /// Adds a point at `index`, or last if the list is shorter.
    Insert { index: usize, point: NavPoint },
    /// Removes the point called `name`.
    Remove { name: String },
}

impl NavPointChange {
    /// Makes the change and returns its inverse, or `None` if the point to remove is not there.
    /// Reinserting at the index removed from keeps the file's order.
    fn make(&self, points: &mut Vec<NavPoint>) -> Option<NavPointChange> {
        match self {
            NavPointChange::Insert { index, point } => {
                points.insert((*index).min(points.len()), point.clone());
                Some(NavPointChange::Remove {
                    name: point.name.clone(),
                })
            }
            NavPointChange::Remove { name } => {
                let index = points.iter().position(|p| &p.name == name)?;
                Some(NavPointChange::Insert {
                    index,
                    point: points.remove(index),
                })
            }
        }
    }
}

/// Makes a [`NavPointChange`] from the history.
struct NavPointUndo {
    asset: AssetId<NavPointsAsset>,
    change: NavPointChange,
}

impl UndoAction for NavPointUndo {
    fn apply(self: Box<Self>, world: &mut World) -> Box<dyn UndoAction> {
        let inverse = world
            .resource_mut::<Assets<NavPointsAsset>>()
            .get_mut(self.asset)
            .and_then(|mut points| self.change.make(&mut points.0));
        let Some(change) = inverse else {
            warn!("{} did nothing: the nav point is gone", self.label());
            return self;
        };
        world.resource_mut::<UnsavedAssets>().mark(self.asset);
        Box::new(NavPointUndo {
            asset: self.asset,
            change,
        })
    }

    fn label(&self) -> Cow<'static, str> {
        match &self.change {
            NavPointChange::Insert { point, .. } => format!("Add nav point {}", point.name),
            NavPointChange::Remove { name } => format!("Remove nav point {name}"),
        }
        .into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn point(name: &str) -> NavPoint {
        NavPoint {
            name: name.into(),
            realm: "playground".into(),
            pos: Vec3::ZERO,
            caption: None,
            icon: None,
        }
    }

    fn names(points: &[NavPoint]) -> Vec<&str> {
        points.iter().map(|p| p.name.as_str()).collect()
    }

    #[test]
    fn undoing_a_removal_restores_the_order() {
        let mut points = vec![point("a"), point("b"), point("c")];
        let remove = NavPointChange::Remove { name: "b".into() };
        let insert = remove.make(&mut points).unwrap();
        assert_eq!(names(&points), ["a", "c"]);
        insert.make(&mut points).unwrap();
        assert_eq!(names(&points), ["a", "b", "c"]);
    }

    #[test]
    fn an_index_past_the_end_appends() {
        let mut points = vec![point("a")];
        let insert = NavPointChange::Insert {
            index: usize::MAX,
            point: point("b"),
        };
        insert.make(&mut points).unwrap();
        assert_eq!(names(&points), ["a", "b"]);
    }

    #[test]
    fn removing_a_missing_point_has_no_inverse() {
        let mut points = vec![point("a")];
        let remove = NavPointChange::Remove { name: "z".into() };
        assert!(remove.make(&mut points).is_none());
    }
}
