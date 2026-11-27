//! The dialog the root pane's Aspects… opens, for an exemplar or an instance: a checkbox per aspect
//! that can attach to the root's type and that it does not inherit, native or declared by a module
//! in `scripts/` (D107), though an instance is offered native ones only, checked where the root has
//! the aspect as its own. Native aspects are under a heading per crate (until mods have packages,
//! X65), scripted ones under their module's path. Save adds the newly checked
//! aspects, each at its default, and removes the unchecked ones, as one edit. Cancel, Esc or a
//! click outside closes it.

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
use panoply_exemplar::{new_aspect, Aspect, InstanceKind, ReflectAspect};
use panoply_saga::{scripted::AspectDecl, ScriptModuleAsset};

use crate::{
    properties::edit::{aspect_name, PropertyEdits, PropertyRoot},
    scenery_properties::held_aspects,
};

/// A chooser's root: the root it edits, and the aspects the root had as its own as it opened.
#[derive(Component)]
struct AspectChooser {
    root: Arc<dyn PropertyRoot>,
    had: HashSet<String>,
}

/// An aspect's checkbox, holding its key.
#[derive(Component, Default, Clone)]
struct AspectOption(String);

/// Opens a chooser of the aspects `root`, an exemplar's or an instance's root, may have as its
/// own. Those it inherits are left out, since their pencils override them; an instance is offered
/// native aspects only, since a precinct does not load scripted ones.
// TODO(PAN135): scripted aspects on instances.
pub(crate) fn open_aspect_chooser(
    In(root): In<Arc<dyn PropertyRoot>>,
    world: &World,
    mut commands: Commands,
) {
    let Some(held) = held_aspects(root.as_ref(), world) else {
        return;
    };
    let modules = world.resource::<Assets<ScriptModuleAsset>>();
    let had: HashSet<String> = held.own.iter().cloned().collect();
    let registry = world.resource::<AppTypeRegistry>().read();
    let mut groups: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let offered = |key: &str| !held.inherited.iter().any(|inherited| inherited == key);
    for decl in scripted_aspects(modules) {
        if !held.instance
            && offered(&decl.key)
            && held.kind.is_some_and(|kind| can_attach(decl, kind))
        {
            let (module, _) = decl.key.rsplit_once("::").unwrap_or_default();
            let group = groups.entry(module.to_string()).or_default();
            group.push(decl.key.to_string());
        }
    }
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
        if !offered(aspect.key()) || !held.kind.is_some_and(|kind| aspect.can_attach(kind)) {
            continue;
        }
        let group = registration.type_info().type_path_table().crate_name();
        groups
            .entry(group.unwrap_or("other").to_string())
            .or_default()
            .push(aspect.key().to_string());
    }
    let rows: Vec<Box<dyn SceneList>> = groups
        .into_iter()
        .map(|(group, mut keys)| {
            keys.sort_unstable_by(|a, b| aspect_name(a).cmp(aspect_name(b)));
            keys.dedup();
            group_rows(&group, &keys, &had)
        })
        .collect();
    let rows: Box<dyn SceneList> = Box::new(rows);
    commands
        .spawn_scene(chooser_dialog(rows))
        .insert((Name::new("AspectChooser"), AspectChooser { root, had }));
}

/// The aspects the modules declare.
fn scripted_aspects(modules: &Assets<ScriptModuleAsset>) -> impl Iterator<Item = &Arc<AspectDecl>> {
    modules.iter().flat_map(|(_, asset)| asset.module.aspects())
}

/// Whether the aspect `decl` declares attaches to `kind`, as [`panoply_exemplar::ScriptedAspect`]'s `can_attach`
/// has it.
fn can_attach(decl: &AspectDecl, kind: InstanceKind) -> bool {
    decl.kinds.iter().any(|name| name.parse() == Ok(kind))
}

/// A group's heading, then a checkbox per aspect in it by key, checked where `had` holds it.
fn group_rows(group: &str, keys: &[String], had: &HashSet<String>) -> Box<dyn SceneList> {
    let group = group.to_string();
    let checkboxes: Vec<Box<dyn SceneList>> = keys
        .iter()
        .map(|key| checkbox(key, had.contains(key)))
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

/// The checkbox for the aspect `key`, captioned and named with the aspect's name.
fn checkbox(key: &str, checked: bool) -> Box<dyn SceneList> {
    let name = aspect_name(key).to_string();
    let key = key.to_string();
    if checked {
        bsn! {
            @FeathersCheckbox { @caption: bsn! { @caption(name.clone()) } }
            Checked
            AspectOption({key.clone()})
            Name({name.clone()})
            on(checkbox_self_update)
        }
        .into()
    } else {
        bsn! {
            @FeathersCheckbox { @caption: bsn! { @caption(name.clone()) } }
            AspectOption({key.clone()})
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
/// defaults, and the unchecked ones it had to be removed, then closes the chooser.
fn save_aspects(
    activate: On<Activate>,
    choosers: Query<&AspectChooser>,
    parents: Query<&ChildOf>,
    children: Query<&Children>,
    options: Query<(&AspectOption, Has<Checked>)>,
    registry: Res<AppTypeRegistry>,
    modules: Res<Assets<ScriptModuleAsset>>,
    mut edits: ResMut<PropertyEdits>,
    mut commands: Commands,
) {
    let Some((dialog, chooser)) = parents
        .iter_ancestors(activate.event_target())
        .find_map(|e| choosers.get(e).ok().map(|chooser| (e, chooser)))
    else {
        return;
    };
    let listed: Vec<(&AspectOption, bool)> = children
        .iter_descendants(dialog)
        .filter_map(|e| options.get(e).ok())
        .collect();
    let checked: HashSet<String> = listed
        .iter()
        .filter(|(_, checked)| *checked)
        .map(|(option, _)| option.0.clone())
        .collect();
    let registry = registry.read();
    let mut add: Vec<Arc<dyn Aspect>> = Vec::new();
    for key in checked.difference(&chooser.had) {
        match new_aspect(&registry, scripted_aspects(&modules), key) {
            Ok(aspect) => add.push(aspect),
            Err(err) => error!("Cannot make a {key}: {err}"),
        }
    }
    add.sort_by(|a, b| a.key().cmp(b.key()));
    // Only an aspect the dialog listed can be unchecked: one it did not, such as one from a
    // module that failed to load, stays.
    let mut remove: Vec<String> = listed
        .iter()
        .filter(|(option, checked)| !checked && chooser.had.contains(&option.0))
        .map(|(option, _)| option.0.clone())
        .collect();
    remove.sort();
    edits.change_aspects(&chooser.root, add, remove);
    commands.entity(dialog).despawn();
}
