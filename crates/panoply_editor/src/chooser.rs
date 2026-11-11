//! A list of named items, sorted by name, with a filter box below it (`search_filter`), and a modal
//! dialog holding one. A click selects an item; a double-click triggers [`ActivateChooserItem`],
//! which bubbles up from the list to whatever holds it. While the filter box has focus, Up and Down
//! move the selection and Enter activates the selected item, or the first shown if none is.

use bevy::{
    ecs::system::SystemParam,
    feathers::{
        containers::{group, group_body},
        controls::{
            ButtonVariant, FeathersButton, FeathersDialog, FeathersDialogBody, FeathersDialogClose,
            FeathersDialogFooter, FeathersDialogHeader, FeathersListRow, FeathersListView,
            FeathersTextInput, FeathersTextInputContainer,
        },
        theme::ThemedText,
    },
    input_focus::{AutoFocus, InputFocus},
    picking::{events::PointerClick, pointer::PointerButton},
    prelude::*,
    text::{EditableText, FontStyle, Placeholder},
    ui::{InteractionDisabled, ScrollIntoView, ScrollIntoViewContainer, Selected},
    ui_widgets::{
        listbox_update_selection, Activate, ListBox, RequestClose, SetSelected, ValueChange,
    },
};
use bevy_action_map::prelude::*;
use bevy_immediate::{
    attach::{BevyImmediateAttachPlugin, ImmediateAttach},
    ui::CapsUiFeathers,
    Imm, ImmMarker,
};

use crate::search_filter::SearchFilter;

/// The list's root. A selected item the filter hides is deselected.
#[derive(Component, Default, Clone)]
pub struct Chooser {
    pub items: Vec<String>,
    pub selected: Option<String>,
}

/// The immediate-mode root that holds the rows.
#[derive(Component, Default, Clone)]
pub(crate) struct ChooserRows;

/// The filter box.
#[derive(Component, Default, Clone)]
struct ChooserFilter;

/// A row, holding its item's name.
#[derive(Component, Default, Clone)]
struct ChooserRow(String);

/// A double-click on a row, or Enter in the filter box, triggered on the list's root; it bubbles
/// up to its ancestors.
#[derive(EntityEvent, Clone, Debug)]
#[entity_event(propagate, auto_propagate)]
pub struct ActivateChooserItem {
    #[event_target]
    pub list: Entity,
    /// The item's name.
    pub name: String,
}

/// The item a [`chooser_dialog`] settled on, triggered on the dialog as it closes.
#[derive(EntityEvent, Clone, Debug)]
pub struct ItemChosen {
    #[event_target]
    pub dialog: Entity,
    pub name: String,
}

/// The filter box's keys, above `widget_keys`' claim on a focused text input.
#[derive(InputContext)]
#[context(path = "editor.chooser_filter", tick = Render, priority = 1100)]
pub struct ChooserFilterKeys;

/// Select the row below the selected one.
#[derive(InputAction)]
#[action(path = "editor.chooser_filter.next", output = bool, intent = Button)]
pub struct SelectNextChooserItem;

/// Select the row above the selected one.
#[derive(InputAction)]
#[action(path = "editor.chooser_filter.previous", output = bool, intent = Button)]
pub struct SelectPreviousChooserItem;

/// Activate the selected row.
#[derive(InputAction)]
#[action(path = "editor.chooser_filter.choose", output = bool, intent = Button)]
pub struct ChooseChooserItem;

/// A [`chooser_dialog`]'s root.
#[derive(Component, Default, Clone)]
struct ChooserDialog;

/// A [`chooser_dialog`]'s confirm button, disabled while no item is selected.
#[derive(Component, Default, Clone)]
struct ChooserConfirm;

pub struct ChooserPlugin;

impl Plugin for ChooserPlugin {
    fn build(&self, app: &mut App) {
        app.add_context::<ChooserFilterKeys>(|context| {
            context.active_if(filter_focused);
            context.bind::<SelectNextChooserItem>(KeyCode::ArrowDown).consume();
            context.bind::<SelectPreviousChooserItem>(KeyCode::ArrowUp).consume();
            context.bind::<ChooseChooserItem>(KeyCode::Enter).consume();
            context.bind::<ChooseChooserItem>(KeyCode::NumpadEnter).consume();
        })
        .add_systems(Startup, |mut commands: Commands| {
            commands.spawn((Name::new("ChooserFilterKeys"), ChooserFilterKeys));
        })
        .add_plugins(BevyImmediateAttachPlugin::<CapsUiFeathers, ChooserRows>::new())
        .add_systems(Update, (filter_keys, enable_confirm))
        .add_observer(name_row)
        .add_observer(hint_filter);
    }
}

/// The list; with `autofocus`, its filter box takes the focus as it spawns. A list in the panel
/// must not, or the keys that move the view would type into it.
pub fn chooser(autofocus: bool, items: Vec<String>) -> impl Scene {
    bsn! {
        Chooser { items: {items.clone()} }
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
                            ChooserRows
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
            // Below the list, not above: empty, the box shows every item, so the list is the
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
        Box::new(bsn! { @FeathersTextInput #Filter ChooserFilter AutoFocus })
    } else {
        Box::new(bsn! { @FeathersTextInput #Filter ChooserFilter })
    }
}

/// Gives the filter box its hint, dim and italic, from an observer since `bsn!` needs a `Default`
/// that `Placeholder` lacks.
// TODO(X85): spawned in `bsn!`, in feathers' own placeholder style.
fn hint_filter(add: On<Add<ChooserFilter>>, mut commands: Commands) {
    commands.entity(add.entity).insert(Placeholder {
        font_style: Some(FontStyle::Italic),
        ..Placeholder::new("Filter…")
    });
}

/// A modal dialog choosing one of `items`. Esc, Cancel or a click outside closes it; the confirm
/// button, a double-click or Enter triggers [`ItemChosen`] on it and closes it. The opener observes
/// the event on the entity it spawned.
pub fn chooser_dialog(title: &str, confirm: &str, items: Vec<String>) -> impl Scene {
    let (title, confirm) = (title.to_string(), confirm.to_string());
    bsn! {
        @FeathersDialog {
            @width: px(320),
            @contents: {
                Box::new(dialog_contents(title.clone(), confirm.clone(), items.clone()))
                    as Box<dyn SceneList>
            },
        }
        ChooserDialog
        on(|close: On<RequestClose>, mut commands: Commands| {
            commands.entity(close.event_target()).despawn();
        })
        on(choose_activated)
    }
}

/// The contents, in a function of their own: a `bsn_list!` written inside the dialog's `bsn!` gave
/// its `#Cancel` the same entity as the root, which made the barrier and the Cancel button one
/// entity.
fn dialog_contents(title: String, confirm: String, items: Vec<String>) -> impl SceneList {
    bsn_list! {
        @FeathersDialogHeader Children [
            Text({title.clone()}) ThemedText
            --
            @FeathersDialogClose
        ]
        --
        @FeathersDialogBody
        Node { height: vh(60), min_height: px(0) }
        Children [ @chooser(true, items.clone()) ]
        --
        @FeathersDialogFooter Children [
            @FeathersButton { @caption: bsn! { Text("Cancel") ThemedText } }
            #Cancel
            on(|activate: On<Activate>, mut commands: Commands| {
                commands.trigger(RequestClose { source: activate.event_target() });
            })
            --
            @FeathersButton {
                @caption: bsn! { Text({confirm.clone()}) ThemedText },
                @variant: ButtonVariant::Primary,
            }
            #Go
            ChooserConfirm
            InteractionDisabled
            on(choose_selected)
        ]
    }
}

/// Observer on the dialog: a double-click or Enter in its list.
fn choose_activated(activate: On<ActivateChooserItem>, mut commands: Commands) {
    let dialog = activate.event_target();
    commands.trigger(ItemChosen {
        dialog,
        name: activate.name.clone(),
    });
    commands.entity(dialog).despawn();
}

/// Observer on the confirm button.
fn choose_selected(
    activate: On<Activate>,
    dialogs: Query<(), With<ChooserDialog>>,
    parents: Query<&ChildOf>,
    children: Query<&Children>,
    lists: Query<&Chooser>,
    mut commands: Commands,
) {
    let Some(dialog) = parents
        .iter_ancestors(activate.event_target())
        .find(|&e| dialogs.contains(e))
    else {
        return;
    };
    let selected = children
        .iter_descendants(dialog)
        .find_map(|e| lists.get(e).ok())
        .and_then(|list| list.selected.clone());
    if let Some(name) = selected {
        commands.trigger(ItemChosen { dialog, name });
        commands.entity(dialog).despawn();
    }
}

/// Disables each dialog's confirm button while its list has nothing selected.
fn enable_confirm(
    dialogs: Query<Entity, With<ChooserDialog>>,
    children: Query<&Children>,
    lists: Query<&Chooser>,
    buttons: Query<Has<InteractionDisabled>, With<ChooserConfirm>>,
    mut commands: Commands,
) {
    for dialog in &dialogs {
        let Some(list) = children
            .iter_descendants(dialog)
            .find_map(|e| lists.get(e).ok())
        else {
            continue;
        };
        for button in children.iter_descendants(dialog) {
            match (list.selected.is_some(), buttons.get(button)) {
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
}

/// The items that `filter` keeps, sorted by name.
pub fn filtered_items<'a>(items: &'a [String], filter: &str) -> Vec<&'a String> {
    let filter = SearchFilter::new(filter);
    let mut kept: Vec<&String> = items.iter().filter(|item| filter.matches(item)).collect();
    kept.sort_by_key(|item| item.to_lowercase());
    kept
}

#[derive(SystemParam)]
pub(crate) struct ChooserRowsParams<'w, 's> {
    lists: Query<'w, 's, &'static mut Chooser>,
    // The immediate context can write text on the entities it manages, and the filter is not one.
    filters: Query<
        'w,
        's,
        &'static EditableText,
        (With<ChooserFilter>, Without<ImmMarker<CapsUiFeathers>>),
    >,
    parents: Query<'w, 's, &'static ChildOf>,
    children: Query<'w, 's, &'static Children>,
}

impl ImmediateAttach<CapsUiFeathers> for ChooserRows {
    type Params = ChooserRowsParams<'static, 'static>;

    /// A row per item the filter shows, keyed by name, so a row keeps its selection while it
    /// stays shown.
    fn construct(ui: &mut Imm<CapsUiFeathers>, params: &mut ChooserRowsParams) {
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
        let state = params.lists.get(list).unwrap();
        let shown = filtered_items(&state.items, &text);
        for item in &shown {
            let name = (*item).clone();
            ui.ch_id(item.as_str()).on_spawn_apply_scene(move || row(name));
        }
        let hidden = state
            .selected
            .as_ref()
            .is_some_and(|selected| !shown.contains(&selected));
        if hidden {
            params.lists.get_mut(list).unwrap().selected = None;
        }
    }
}

fn row(name: String) -> impl Scene {
    let label = name.clone();
    bsn! {
        @FeathersListRow
        ChooserRow({name.clone()})
        Children [ Text({label.clone()}) ThemedText ]
    }
}

/// Names each row after its item, so a plan can find it.
fn name_row(add: On<Add<ChooserRow>>, rows: Query<&ChooserRow>, mut commands: Commands) {
    if let Ok(row) = rows.get(add.entity) {
        commands.entity(add.entity).insert(Name::new(row.0.clone()));
    }
}

/// The list root above `entity`.
fn list_of(
    entity: Entity,
    parents: &Query<&ChildOf>,
    lists: &Query<&mut Chooser>,
) -> Option<Entity> {
    parents.iter_ancestors(entity).find(|&e| lists.contains(e))
}

/// Observer on the list view: records the selected row's item.
fn select_row(
    change: On<ValueChange<Entity>>,
    rows: Query<&ChooserRow>,
    parents: Query<&ChildOf>,
    mut lists: Query<&mut Chooser>,
) {
    let Ok(row) = rows.get(change.value) else {
        return;
    };
    if let Some(list) = list_of(change.source, &parents, &lists) {
        lists.get_mut(list).unwrap().selected = Some(row.0.clone());
    }
}

/// Observer on the list view: a double-click on a row activates its item.
fn activate_row(
    click: On<PointerClick>,
    rows: Query<&ChooserRow>,
    parents: Query<&ChildOf>,
    lists: Query<&mut Chooser>,
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
        commands.trigger(ActivateChooserItem {
            list,
            name: row.0.clone(),
        });
    }
}

fn filter_focused(focus: Res<InputFocus>, filters: Query<(), With<ChooserFilter>>) -> bool {
    focus.get().is_some_and(|focused| filters.contains(focused))
}

/// Up and Down in the filter box move the selection, and Enter activates it.
fn filter_keys(
    input: ContextActions<ChooserFilterKeys>,
    focus: Res<InputFocus>,
    parents: Query<&ChildOf>,
    children: Query<&Children>,
    lists: Query<(), With<Chooser>>,
    listboxes: Query<(), With<ListBox>>,
    rows: Query<(&ChooserRow, Has<Selected>)>,
    mut commands: Commands,
) {
    let step: isize = if input.fired::<SelectNextChooserItem>() {
        1
    } else if input.fired::<SelectPreviousChooserItem>() {
        -1
    } else {
        0
    };
    let choose = input.fired::<ChooseChooserItem>();
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
    let shown: Vec<(Entity, &ChooserRow, bool)> = children
        .iter_descendants(listbox)
        .filter_map(|e| rows.get(e).ok().map(|(row, selected)| (e, row, selected)))
        .collect();
    if shown.is_empty() {
        return;
    }
    let current = shown.iter().position(|&(_, _, selected)| selected);
    if choose {
        let (_, row, _) = shown[current.unwrap_or(0)];
        commands.trigger(ActivateChooserItem {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filtered_and_sorted_by_name() {
        let items = ["warehouse", "Stick-village", "start", "demo"].map(String::from);
        let names = |filter| -> Vec<&str> {
            filtered_items(&items, filter)
                .iter()
                .map(|item| item.as_str())
                .collect()
        };
        assert_eq!(names(""), ["demo", "start", "Stick-village", "warehouse"]);
        assert_eq!(names("st"), ["start", "Stick-village"]);
        assert_eq!(names("stvi"), ["Stick-village"]);
    }
}
