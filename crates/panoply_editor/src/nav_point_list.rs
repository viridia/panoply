//! A list of the nav points, sorted by name, with a filter box below it (`search_filter`). A click
//! selects a point; a double-click triggers [`ActivateNavPoint`], which bubbles up from the list
//! to whatever holds it. While the filter box has focus, Up and Down move the selection and Enter
//! activates the selected point, or the first shown if none is.

use bevy::{
    ecs::system::SystemParam,
    feathers::{
        containers::{group, group_body},
        controls::{
            FeathersListRow, FeathersListView, FeathersTextInput, FeathersTextInputContainer,
        },
        theme::ThemedText,
    },
    input_focus::{AutoFocus, InputFocus},
    picking::{events::PointerClick, pointer::PointerButton},
    prelude::*,
    text::EditableText,
    ui::{ScrollIntoView, ScrollIntoViewContainer, Selected},
    ui_widgets::{listbox_update_selection, ListBox, SetSelected, ValueChange},
};
use bevy_action_map::prelude::*;
use bevy_immediate::{
    attach::{BevyImmediateAttachPlugin, ImmediateAttach},
    ui::CapsUiFeathers,
    Imm, ImmMarker,
};
use panoply_core::{GotoLocation, NavPoint, NavPointsAsset, NavPointsHandleResource};

use crate::search_filter::SearchFilter;

/// The list's root. A selected point the filter hides is deselected.
#[derive(Component, Default, Clone)]
pub struct NavPointList {
    pub selected: Option<String>,
}

/// The immediate-mode root that holds the rows.
#[derive(Component, Default, Clone)]
pub(crate) struct NavPointRows;

/// The filter box.
#[derive(Component, Default, Clone)]
struct NavPointFilter;

/// A row, holding its point's name.
#[derive(Component, Default, Clone)]
struct NavPointRow(String);

/// A double-click on a row, or Enter in the filter box, triggered on the list's root; it bubbles
/// up to its ancestors.
#[derive(EntityEvent, Clone, Debug)]
#[entity_event(propagate, auto_propagate)]
pub struct ActivateNavPoint {
    #[event_target]
    pub list: Entity,
    /// The point's name.
    pub name: String,
}

/// The filter box's keys, above `widget_keys`' claim on a focused text input.
#[derive(InputContext)]
#[context(path = "editor.nav_point_filter", tick = Render, priority = 1100)]
pub struct NavPointFilterKeys;

/// Select the row below the selected one.
#[derive(InputAction)]
#[action(path = "editor.nav_point_filter.next", output = bool, intent = Button)]
pub struct SelectNextNavPoint;

/// Select the row above the selected one.
#[derive(InputAction)]
#[action(path = "editor.nav_point_filter.previous", output = bool, intent = Button)]
pub struct SelectPreviousNavPoint;

/// Activate the selected row.
#[derive(InputAction)]
#[action(path = "editor.nav_point_filter.choose", output = bool, intent = Button)]
pub struct ChooseNavPoint;

pub struct NavPointListPlugin;

impl Plugin for NavPointListPlugin {
    fn build(&self, app: &mut App) {
        app.add_context::<NavPointFilterKeys>(|context| {
            context.active_if(filter_focused);
            context.bind::<SelectNextNavPoint>(KeyCode::ArrowDown).consume();
            context.bind::<SelectPreviousNavPoint>(KeyCode::ArrowUp).consume();
            context.bind::<ChooseNavPoint>(KeyCode::Enter).consume();
            context.bind::<ChooseNavPoint>(KeyCode::NumpadEnter).consume();
        })
        .add_systems(Startup, |mut commands: Commands| {
            commands.spawn((Name::new("NavPointFilterKeys"), NavPointFilterKeys));
        })
        .add_plugins(BevyImmediateAttachPlugin::<CapsUiFeathers, NavPointRows>::new())
        .add_systems(Update, filter_keys)
        .add_observer(name_row);
    }
}

/// The list; with `autofocus`, its filter box takes the focus as it spawns. A list in the panel
/// must not, or the keys that move the view would type into it.
pub fn nav_point_list(autofocus: bool) -> impl Scene {
    bsn! {
        NavPointList
        Node {
            flex_direction: FlexDirection::Column,
            row_gap: px(4),
            flex_grow: 1.0,
            min_height: px(0),
        }
        Children [
            @group()
            Node { flex_grow: 1.0, min_height: px(0) }
            Children [
                @group_body()
                // Feathers pads a group's body by 6; the editor spaces by 4.
                Node { flex_grow: 1.0, min_height: px(0), padding: px(4) }
                Children [
                    @FeathersListView {
                        @rows: bsn_list! {
                            NavPointRows
                            Node {
                                flex_direction: FlexDirection::Column,
                                align_items: AlignItems::Stretch,
                            }
                        }
                    }
                    #Rows
                    Node { flex_grow: 1.0, min_height: px(0) }
                    on(listbox_update_selection)
                    on(select_row)
                    on(activate_row)
                ]
            ]
            --
            // Below the list, not above: empty, the box shows every point, so the list is the
            // content and the box only narrows it. A search whose empty box shows nothing would
            // put the box first.
            @FeathersTextInputContainer
            Node { flex_grow: 0.0, flex_shrink: 0.0 }
            Children [ @filter_input(autofocus) ]
        ]
    }
}

fn filter_input(autofocus: bool) -> Box<dyn Scene> {
    if autofocus {
        Box::new(bsn! { @FeathersTextInput #Filter NavPointFilter AutoFocus })
    } else {
        Box::new(bsn! { @FeathersTextInput #Filter NavPointFilter })
    }
}

/// The nav points that `filter` keeps, sorted by name.
pub fn filtered_points<'a>(points: &'a [NavPoint], filter: &str) -> Vec<&'a NavPoint> {
    let filter = SearchFilter::new(filter);
    let mut kept: Vec<&NavPoint> = points.iter().filter(|p| filter.matches(&p.name)).collect();
    kept.sort_by_key(|p| p.name.to_lowercase());
    kept
}

#[derive(SystemParam)]
pub(crate) struct NavPointRowsParams<'w, 's> {
    lists: Query<'w, 's, &'static mut NavPointList>,
    // The immediate context can write text on the entities it manages, and the filter is not one.
    filters: Query<
        'w,
        's,
        &'static EditableText,
        (With<NavPointFilter>, Without<ImmMarker<CapsUiFeathers>>),
    >,
    parents: Query<'w, 's, &'static ChildOf>,
    children: Query<'w, 's, &'static Children>,
    handle: Res<'w, NavPointsHandleResource>,
    assets: Res<'w, Assets<NavPointsAsset>>,
}

impl ImmediateAttach<CapsUiFeathers> for NavPointRows {
    type Params = NavPointRowsParams<'static, 'static>;

    /// A row per point the filter shows, keyed by name, so a row keeps its selection while it
    /// stays shown.
    fn construct(ui: &mut Imm<CapsUiFeathers>, params: &mut NavPointRowsParams) {
        let Some(points) = params.assets.get(&params.handle.0) else {
            return;
        };
        let Some(list) = ui.current_entity().and_then(|rows| {
            params
                .parents
                .iter_ancestors(rows)
                .find(|&e| params.lists.contains(e))
        }) else {
            return;
        };
        let Some(text) = params
            .children
            .iter_descendants(list)
            .find_map(|e| params.filters.get(e).ok())
        else {
            return;
        };
        let text = text.value();
        let shown = filtered_points(&points.0, &text);
        for point in &shown {
            let name = point.name.clone();
            ui.ch_id(&point.name).on_spawn_apply_scene(move || row(name));
        }
        let mut state = params.lists.get_mut(list).unwrap();
        let hidden = |selected: &String| !shown.iter().any(|p| p.name == *selected);
        if state.selected.as_ref().is_some_and(hidden) {
            state.selected = None;
        }
    }
}

fn row(name: String) -> impl Scene {
    let label = name.clone();
    bsn! {
        @FeathersListRow
        NavPointRow({name.clone()})
        Children [ Text({label.clone()}) ThemedText ]
    }
}

/// Names each row after its point, so a plan can find it.
fn name_row(add: On<Add<NavPointRow>>, rows: Query<&NavPointRow>, mut commands: Commands) {
    if let Ok(row) = rows.get(add.entity) {
        commands.entity(add.entity).insert(Name::new(row.0.clone()));
    }
}

/// The list root above `entity`.
fn list_of(
    entity: Entity,
    parents: &Query<&ChildOf>,
    lists: &Query<&mut NavPointList>,
) -> Option<Entity> {
    parents.iter_ancestors(entity).find(|&e| lists.contains(e))
}

/// Observer on the list view: records the selected row's point.
fn select_row(
    change: On<ValueChange<Entity>>,
    rows: Query<&NavPointRow>,
    parents: Query<&ChildOf>,
    mut lists: Query<&mut NavPointList>,
) {
    let Ok(row) = rows.get(change.value) else {
        return;
    };
    if let Some(list) = list_of(change.source, &parents, &lists) {
        lists.get_mut(list).unwrap().selected = Some(row.0.clone());
    }
}

/// Observer on the list view: a double-click on a row activates its point.
fn activate_row(
    click: On<PointerClick>,
    rows: Query<&NavPointRow>,
    parents: Query<&ChildOf>,
    lists: Query<&mut NavPointList>,
    mut commands: Commands,
) {
    if click.button != PointerButton::Primary || click.count < 2 {
        return;
    }
    let target = click.original_event_target();
    let Some(row) = std::iter::once(target)
        .chain(parents.iter_ancestors(target))
        .find_map(|e| rows.get(e).ok())
    else {
        return;
    };
    if let Some(list) = list_of(click.event_target(), &parents, &lists) {
        commands.trigger(ActivateNavPoint {
            list,
            name: row.0.clone(),
        });
    }
}

fn filter_focused(focus: Res<InputFocus>, filters: Query<(), With<NavPointFilter>>) -> bool {
    focus.get().is_some_and(|focused| filters.contains(focused))
}

/// Up and Down in the filter box move the selection, and Enter activates it.
fn filter_keys(
    input: ContextActions<NavPointFilterKeys>,
    focus: Res<InputFocus>,
    parents: Query<&ChildOf>,
    children: Query<&Children>,
    lists: Query<(), With<NavPointList>>,
    listboxes: Query<(), With<ListBox>>,
    rows: Query<(&NavPointRow, Has<Selected>)>,
    mut commands: Commands,
) {
    let step: isize = if input.fired::<SelectNextNavPoint>() {
        1
    } else if input.fired::<SelectPreviousNavPoint>() {
        -1
    } else {
        0
    };
    let choose = input.fired::<ChooseNavPoint>();
    if step == 0 && !choose {
        return;
    }
    let Some(filter) = focus.get() else {
        return;
    };
    let Some(list) = parents.iter_ancestors(filter).find(|&e| lists.contains(e)) else {
        return;
    };
    let Some(listbox) = children
        .iter_descendants(list)
        .find(|&e| listboxes.contains(e))
    else {
        return;
    };
    let shown: Vec<(Entity, &NavPointRow, bool)> = children
        .iter_descendants(listbox)
        .filter_map(|e| rows.get(e).ok().map(|(row, selected)| (e, row, selected)))
        .collect();
    if shown.is_empty() {
        return;
    }
    let current = shown.iter().position(|&(_, _, selected)| selected);
    if choose {
        let (_, row, _) = shown[current.unwrap_or(0)];
        commands.trigger(ActivateNavPoint {
            list,
            name: row.0.clone(),
        });
        return;
    }
    let last = shown.len() as isize - 1;
    let next = match current {
        Some(i) => (i as isize + step).clamp(0, last),
        None if step > 0 => 0,
        None => last,
    } as usize;
    let row = shown[next].0;
    commands.trigger(SetSelected {
        entity: listbox,
        row,
    });
    commands.trigger(ScrollIntoView {
        entity: row,
        container: ScrollIntoViewContainer::Nearest,
    });
}

/// Moves the viewpoint to a nav point, by name.
#[derive(bevy::ecs::system::SystemParam)]
pub(crate) struct Travel<'w> {
    handle: Res<'w, NavPointsHandleResource>,
    assets: Res<'w, Assets<NavPointsAsset>>,
    gotos: MessageWriter<'w, GotoLocation>,
}

impl Travel<'_> {
    /// Moves the viewpoint to the point named `name`, if there is one.
    pub(crate) fn to(&mut self, name: &str) {
        let point = self
            .assets
            .get(&self.handle.0)
            .and_then(|points| points.0.iter().find(|p| p.name == name));
        if let Some(point) = point {
            self.gotos.write(GotoLocation {
                realm: point.realm.clone(),
                position: point.pos,
                ..default()
            });
        }
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

    #[test]
    fn filtered_and_sorted_by_name() {
        let points = [
            point("warehouse"),
            point("Stick-village"),
            point("start"),
            point("demo"),
        ];
        let names = |filter| -> Vec<&str> {
            filtered_points(&points, filter)
                .iter()
                .map(|p| p.name.as_str())
                .collect()
        };
        assert_eq!(names(""), ["demo", "start", "Stick-village", "warehouse"]);
        assert_eq!(names("st"), ["start", "Stick-village"]);
        assert_eq!(names("stvi"), ["Stick-village"]);
    }
}
