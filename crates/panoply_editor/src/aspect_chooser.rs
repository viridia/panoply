//! The dialog the exemplar pane's Aspects… opens: a checkbox per aspect type that can attach to
//! the exemplar's type, under a heading per crate (until mods have packages, X65), checked where
//! the exemplar has the aspect as its own. Save adds the newly checked aspects, each at its type's
//! default, and removes the unchecked ones, as one edit. Cancel, Esc or a click outside closes it.

use std::{collections::BTreeMap, sync::Arc};

use bevy::{
    feathers::{
        controls::{
            ButtonVariant, FeathersButton, FeathersCheckbox, FeathersDialog, FeathersDialogBody,
            FeathersDialogClose, FeathersDialogFooter, FeathersDialogHeader,
        },
        display::{caption, label},
        theme::ThemedText,
    },
    platform::collections::HashSet,
    prelude::*,
    reflect::std_traits::ReflectDefault,
    ui::Checked,
    ui_widgets::{checkbox_self_update, Activate, RequestClose},
};
use panoply_exemplar::{Aspect, Exemplar, ReflectAspect};

use crate::{
    properties::edit::{PropertyEdits, PropertyRoot},
    scenery_properties::exemplar_of,
};

/// A chooser's root: the exemplar root it edits, and the aspects the exemplar had as it opened.
#[derive(Component)]
struct AspectChooser {
    root: Arc<dyn PropertyRoot>,
    had: HashSet<String>,
}

/// An aspect type's checkbox, holding its short type path.
#[derive(Component, Default, Clone)]
struct AspectOption(String);

/// Opens a chooser of the aspects `root`, an exemplar's root, may have.
pub(crate) fn open_aspect_chooser(
    In(root): In<Arc<dyn PropertyRoot>>,
    registry: Res<AppTypeRegistry>,
    server: Res<AssetServer>,
    exemplars: Res<Assets<Exemplar>>,
    mut commands: Commands,
) {
    let Some(data) = exemplar_of(root.as_ref(), &server).and_then(|id| exemplars.get(id)) else {
        return;
    };
    let had: HashSet<String> = data
        .0
        .aspects
        .iter()
        .map(|aspect| aspect.reflect_short_type_path().to_string())
        .collect();
    let registry = registry.read();
    let mut groups: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for registration in registry.iter() {
        let (Some(reflect_aspect), Some(default)) = (
            registration.data::<ReflectAspect>(),
            registration.data::<ReflectDefault>(),
        ) else {
            continue;
        };
        let value = default.default();
        let Some(aspect) = reflect_aspect.get(value.as_ref()) else {
            continue;
        };
        if !aspect.can_attach(data.0.meta_type) {
            continue;
        }
        let table = registration.type_info().type_path_table();
        groups
            .entry(table.crate_name().unwrap_or("other"))
            .or_default()
            .push(table.short_path());
    }
    let rows: Vec<Box<dyn SceneList>> = groups
        .into_iter()
        .map(|(group, mut names)| {
            names.sort_unstable();
            group_rows(group, &names, &had)
        })
        .collect();
    let rows: Box<dyn SceneList> = Box::new(rows);
    commands
        .spawn_scene(chooser_dialog(rows))
        .insert((Name::new("AspectChooser"), AspectChooser { root, had }));
}

/// A group's heading, then a checkbox per aspect in it, checked where `had` holds it.
fn group_rows(group: &str, names: &[&str], had: &HashSet<String>) -> Box<dyn SceneList> {
    let group = group.to_string();
    let checkboxes: Vec<Box<dyn SceneList>> = names
        .iter()
        .map(|&name| checkbox(name, had.contains(name)))
        .collect();
    let checkboxes: Box<dyn SceneList> = Box::new(checkboxes);
    Box::new(bsn_list! {
        @label(group.clone())
        Node { margin: UiRect::top(px(6)) }
        --
        Node {
            flex_direction: FlexDirection::Column,
            row_gap: px(2),
            padding: UiRect::left(px(12)),
        }
        Children [ {checkboxes} ]
    })
}

/// The checkbox for the aspect type `name`, named after it.
fn checkbox(name: &str, checked: bool) -> Box<dyn SceneList> {
    let name = name.to_string();
    if checked {
        bsn! {
            @FeathersCheckbox { @caption: bsn! { @caption(name.clone()) } }
            Checked
            AspectOption({name.clone()})
            Name({name.clone()})
            on(checkbox_self_update)
        }
        .into()
    } else {
        bsn! {
            @FeathersCheckbox { @caption: bsn! { @caption(name.clone()) } }
            AspectOption({name.clone()})
            Name({name.clone()})
            on(checkbox_self_update)
        }
        .into()
    }
}

/// The dialog, holding `rows`.
fn chooser_dialog(rows: Box<dyn SceneList>) -> impl Scene {
    bsn! {
        @FeathersDialog {
            @width: px(320),
            @contents: { Box::new(dialog_contents(rows)) as Box<dyn SceneList> },
        }
        on(|close: On<RequestClose>, mut commands: Commands| {
            commands.entity(close.event_target()).despawn();
        })
    }
}

/// The contents, in a function of their own, as `model_picker`'s are.
fn dialog_contents(rows: Box<dyn SceneList>) -> impl SceneList {
    bsn_list! {
        @FeathersDialogHeader Children [
            Text("Add or remove aspects") ThemedText
            --
            @FeathersDialogClose
        ]
        --
        @FeathersDialogBody
        Node {
            flex_direction: FlexDirection::Column,
            max_height: vh(60),
            overflow: Overflow::scroll_y(),
        }
        Children [ {rows} ]
        --
        @FeathersDialogFooter Children [
            @FeathersButton { @caption: bsn! { Text("Cancel") ThemedText } }
            #Cancel
            on(|activate: On<Activate>, mut commands: Commands| {
                commands.trigger(RequestClose { source: activate.event_target() });
            })
            --
            @FeathersButton {
                @caption: bsn! { Text("Save") ThemedText },
                @variant: ButtonVariant::Primary,
            }
            #Save
            on(save_aspects)
        ]
    }
}

/// Observer on Save: asks for the checked aspects the exemplar lacked to be added, at their
/// types' defaults, and the unchecked ones it had to be removed, then closes the chooser.
fn save_aspects(
    activate: On<Activate>,
    choosers: Query<&AspectChooser>,
    parents: Query<&ChildOf>,
    children: Query<&Children>,
    options: Query<(&AspectOption, Has<Checked>)>,
    registry: Res<AppTypeRegistry>,
    mut edits: ResMut<PropertyEdits>,
    mut commands: Commands,
) {
    let Some((dialog, chooser)) = parents
        .iter_ancestors(activate.event_target())
        .find_map(|e| choosers.get(e).ok().map(|chooser| (e, chooser)))
    else {
        return;
    };
    let checked: HashSet<String> = children
        .iter_descendants(dialog)
        .filter_map(|e| options.get(e).ok())
        .filter(|(_, checked)| *checked)
        .map(|(option, _)| option.0.clone())
        .collect();
    let registry = registry.read();
    let mut add: Vec<Box<dyn Aspect>> = Vec::new();
    for name in checked.difference(&chooser.had) {
        match new_aspect(&registry, name) {
            Some(aspect) => add.push(aspect),
            None => error!("Cannot make a {name}"),
        }
    }
    add.sort_by(|a, b| a.reflect_short_type_path().cmp(b.reflect_short_type_path()));
    let mut remove: Vec<String> = chooser.had.difference(&checked).cloned().collect();
    remove.sort();
    edits.change_aspects(&chooser.root, add, remove);
    commands.entity(dialog).despawn();
}

/// A new aspect of the type whose short type path is `name`, at its default.
fn new_aspect(registry: &bevy::reflect::TypeRegistry, name: &str) -> Option<Box<dyn Aspect>> {
    let registration = registry.get_with_short_type_path(name)?;
    let default = registration.data::<ReflectDefault>()?.default();
    registration.data::<ReflectAspect>()?.get_boxed(default).ok()
}
