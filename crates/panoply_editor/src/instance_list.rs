//! Selection mode's panel. Its list holds the selected precinct's named instances, scenery and
//! actors, as `wooden-chest-2 (WoodenChest)`, as faery's `SceneryInstanceChooser` did; choosing one
//! selects it, and a click in the view moves the list's selection to the instance clicked. Below,
//! a pane names the selected instance's kind and exemplar, and shows its id, which Rename… changes.

use bevy::{
    app::PropagateOver,
    ecs::system::SystemParam,
    feathers::{
        constants::{fonts, size},
        containers::{pane, pane_body, pane_header},
        controls::FeathersButton,
        theme::{ThemeTextColor, ThemedText},
        tokens,
    },
    platform::collections::HashSet,
    prelude::*,
    text::{FontSourceTemplate, FontStyle},
};
use bevy_immediate::{
    attach::{BevyImmediateAttachPlugin, ImmediateAttach},
    ui::{activated::ImmUiActivated, text::ImmUiText},
    Imm,
};
use panoply_actors::{PlacedAs, PrecinctActors};
use panoply_exemplar::Exemplar;
use panoply_precinct::{PrecinctAsset, SceneryInstanceId};
use panoply_scenery::{precinct::Precinct, scenery_element::SceneryElement, WALL_TYPE};

use crate::{
    chooser::{chooser, Chooser},
    instance_selection::ScenerySelection,
    mode_selector::SceneryTool,
    precinct_edit::PrecinctEdits,
    properties::{caption_cell, CapsPanel, CAPTION_WIDTH},
    scenery_properties::header_text,
    text_entry::{text_entry_dialog_with_text, TextEntered},
};

/// The chooser of the selected precinct's named instances.
#[derive(Component, Default, Clone)]
pub struct InstanceChooser;

/// The immediate-mode root of the selected instance's pane.
#[derive(Component, Default, Clone)]
pub(crate) struct InstancePane;

/// The rename dialog, holding the instance it renames.
#[derive(Component, Clone)]
struct RenameInstance {
    precinct: Entity,
    iid: SceneryInstanceId,
}

pub struct InstanceListPlugin;

impl Plugin for InstanceListPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(BevyImmediateAttachPlugin::<CapsPanel, InstancePane>::new())
            .add_systems(
                Update,
                sync_instance_list.run_if(in_state(SceneryTool::SelectInstance)),
            );
    }
}

/// The list, empty until `sync_instance_list` fills it. Its filter box must not take the focus,
/// as the tools' lists' must not.
pub fn instance_chooser() -> impl Scene {
    bsn! {
        @chooser(false, Vec::new(), None)
        InstanceChooser
    }
}

/// The label of the exemplar at `path`, a `file#Label` path.
fn label(path: &str) -> &str {
    path.rsplit_once('#').map_or(path, |(_, label)| label)
}

/// The named instances of `asset`, each with its row in the list, sorted by row.
fn named_rows(asset: &PrecinctAsset) -> Vec<(String, SceneryInstanceId)> {
    let scenery = asset.scenery.iter().map(|instance| {
        let path = asset
            .scenery_types
            .get(instance.id)
            .map_or("", String::as_str);
        (&instance.iid, path)
    });
    let actors = asset
        .actors
        .iter()
        .map(|actor| (&actor.iid, actor.exemplar.as_str()));
    let mut rows: Vec<_> = scenery
        .chain(actors)
        .filter_map(|(iid, path)| match iid {
            SceneryInstanceId::External(name) => {
                Some((format!("{name} ({})", label(path)), iid.clone()))
            }
            _ => None,
        })
        .collect();
    rows.sort_by(|a, b| a.0.cmp(&b.0));
    rows
}

/// The scenery elements and actors of a precinct.
#[derive(SystemParam)]
pub(crate) struct PrecinctInstances<'w, 's> {
    precincts: Query<
        'w,
        's,
        (
            &'static Precinct,
            Option<&'static Children>,
            Option<&'static PrecinctActors>,
        ),
    >,
    elements: Query<'w, 's, &'static SceneryElement>,
    actors: Query<'w, 's, &'static PlacedAs>,
}

impl PrecinctInstances<'_, '_> {
    /// The instance of `precinct` whose id is `iid`.
    fn find(&self, precinct: Entity, iid: &SceneryInstanceId) -> Option<Entity> {
        let (_, children, placed) = self.precincts.get(precinct).ok()?;
        children
            .into_iter()
            .flatten()
            .copied()
            .chain(placed.into_iter().flat_map(|actors| actors.iter()))
            .find(|&entity| {
                self.elements
                    .get(entity)
                    .is_ok_and(|element| element.iid == *iid)
                    || self
                        .actors
                        .get(entity)
                        .is_ok_and(|actor| actor.iid() == iid)
            })
    }
}

/// Fills the list from the selected precinct's asset, selects in the scenery what is chosen in the
/// list, and chooses in the list what is selected in the scenery. The row last chosen or shown is
/// kept, so a row the filter hides, which the chooser clears, is not chosen again.
fn sync_instance_list(
    mut selection: ResMut<ScenerySelection>,
    instances: PrecinctInstances,
    assets: Res<Assets<PrecinctAsset>>,
    mut choosers: Query<&mut Chooser, With<InstanceChooser>>,
    mut shown: Local<Option<String>>,
) {
    let Ok(mut chooser) = choosers.single_mut() else {
        return;
    };
    let precinct = selection.precinct;
    let rows = precinct
        .and_then(|precinct| instances.precincts.get(precinct).ok())
        .and_then(|(record, ..)| assets.get(&record.asset))
        .map(named_rows)
        .unwrap_or_default();
    let items: Vec<String> = rows.iter().map(|(row, _)| row.clone()).collect();
    if chooser.items != items {
        chooser.items = items;
    }
    let picked = chooser
        .selected
        .clone()
        .filter(|row| Some(row) != shown.as_ref());
    if let Some(row) = picked {
        let found = precinct.zip(rows.iter().find(|(r, _)| *r == row));
        if let Some((precinct, (_, iid))) = found {
            if let Some(entity) = instances.find(precinct, iid) {
                selection.set_if_neq(ScenerySelection::instance(precinct, entity, iid.clone()));
            }
        }
        *shown = Some(row);
        return;
    }
    let desired = selection
        .instance
        .and_then(|_| rows.iter().find(|(_, iid)| *iid == selection.iid))
        .map(|(row, _)| row.clone());
    if desired != *shown {
        chooser.selected = desired.clone();
        *shown = desired;
    }
}

#[derive(SystemParam)]
pub(crate) struct InstancePaneParams<'w, 's> {
    selection: Res<'w, ScenerySelection>,
    tool: Option<Res<'w, State<SceneryTool>>>,
    elements: Query<'w, 's, &'static SceneryElement>,
    actors: Query<'w, 's, &'static PlacedAs>,
    precincts: Query<'w, 's, &'static Precinct>,
    exemplars: Res<'w, Assets<Exemplar>>,
    assets: Res<'w, Assets<PrecinctAsset>>,
    server: Res<'w, AssetServer>,
}

impl InstancePaneParams<'_, '_> {
    /// The selected instance's kind and exemplar's label.
    fn described(&self) -> Option<(&'static str, String)> {
        let entity = self.selection.instance?;
        if let Ok(element) = self.elements.get(entity) {
            let wall = self
                .exemplars
                .get(&element.exemplar)
                .is_some_and(|exemplar| exemplar.0.meta_type == WALL_TYPE);
            let path = self.server.get_path(&element.exemplar)?;
            let label = path.label().unwrap_or_default().to_string();
            return Some((if wall { "Wall" } else { "Fixture" }, label));
        }
        let actor = self.actors.get(entity).ok()?;
        Some(("Actor", label(actor.exemplar()).to_string()))
    }
}

impl ImmediateAttach<CapsPanel> for InstancePane {
    type Params = InstancePaneParams<'static, 'static>;

    /// In selection mode, a pane for the selected instance: "Fixture: WoodenChest" in its header,
    /// then its id, or "none" for an instance with no name, and Rename….
    fn construct(ui: &mut Imm<CapsPanel>, params: &mut InstancePaneParams) {
        let selecting = params
            .tool
            .as_ref()
            .is_some_and(|tool| *tool.get() == SceneryTool::SelectInstance);
        let Some((kind, name)) = selecting.then(|| params.described()).flatten() else {
            return;
        };
        let kind = format!("{kind}:");
        let current = current_name(&params.selection.iid);
        let named = current.is_some();
        let id = current.unwrap_or_else(|| "none".to_string());
        ui.ch_id("pane")
            .on_spawn_apply_scene(|| bsn! { @pane() Node { flex_grow: 1.0, min_height: px(0) } })
            .add(|ui| {
                ui.ch()
                    .on_spawn_apply_scene(pane_header)
                    .on_spawn_insert(|| Name::new("InstancePaneHeader"))
                    .add(|ui| {
                        ui.ch()
                            .on_spawn_insert(|| Node {
                                column_gap: px(4),
                                align_items: AlignItems::Baseline,
                                ..default()
                            })
                            .add(|ui| {
                                ui.ch()
                                    .on_spawn_apply_scene(|| {
                                        header_text(fonts::REGULAR, FontWeight::NORMAL)
                                    })
                                    .on_hash_change_text_fn(&kind, || kind.clone());
                                ui.ch()
                                    .on_spawn_apply_scene(|| {
                                        header_text(fonts::BOLD, FontWeight::BOLD)
                                    })
                                    .on_hash_change_text_fn(&name, || name.clone());
                            });
                    });
                ui.ch().on_spawn_apply_scene(pane_body).add(|ui| {
                    // As a property row: the caption in its column, then the value.
                    ui.ch()
                        .on_spawn_insert(|| Node {
                            align_items: AlignItems::Center,
                            ..default()
                        })
                        .add(|ui| {
                            caption_cell(ui, "Id".to_string(), CAPTION_WIDTH);
                            // An instance with no name shows "none", dim and italic, as a value
                            // that is not there.
                            let (key, style, color) = match named {
                                true => ("id", FontStyle::Normal, tokens::TEXT_MAIN),
                                false => ("none", FontStyle::Italic, tokens::TEXT_DIM),
                            };
                            ui.ch_id(key)
                                .on_spawn_apply_scene(move || {
                                    bsn! {
                                        Text("")
                                        TextFont {
                                            font: FontSourceTemplate::Families(fonts::SANS),
                                            font_size: size::MEDIUM_FONT,
                                            style: style,
                                        }
                                        PropagateOver<TextFont>
                                        TextLayout { linebreak: LineBreak::NoWrap }
                                        ThemeTextColor(color)
                                        Node { min_width: px(0), overflow: Overflow::clip_x() }
                                    }
                                })
                                .on_spawn_insert(|| Name::new("InstanceId"))
                                .on_hash_change_text_fn(&id, || id.clone());
                            ui.ch().on_spawn_insert(|| Node {
                                flex_grow: 1.0,
                                ..default()
                            });
                            let rename = ui
                                .ch_id("rename")
                                .on_spawn_apply_scene(|| {
                                    bsn! {
                                        @FeathersButton {
                                            @caption: bsn! { Text("Rename…") ThemedText },
                                        }
                                    }
                                })
                                .on_spawn_insert(|| Name::new("RenameInstance"))
                                .activated();
                            if let Some((taken, target)) =
                                rename.then(|| rename_target(params)).flatten()
                            {
                                let current = current_name(&target.iid).unwrap_or_default();
                                ui.commands_mut()
                                    .spawn_scene(text_entry_dialog_with_text(
                                        "Rename", "Id", taken, current,
                                    ))
                                    .insert(target)
                                    .observe(rename_instance);
                            }
                        });
                });
            });
    }
}

/// The name an instance's id gives it, if it has one.
fn current_name(iid: &SceneryInstanceId) -> Option<String> {
    match iid {
        SceneryInstanceId::External(id) => Some(id.to_string()),
        _ => None,
    }
}

/// What a rename of the selected instance refuses, the names its precinct's other instances use,
/// and the instance.
fn rename_target(params: &InstancePaneParams) -> Option<(HashSet<String>, RenameInstance)> {
    let precinct = params.selection.precinct?;
    let record = params.precincts.get(precinct).ok()?;
    let mut taken = params
        .assets
        .get(&record.asset)
        .map(PrecinctAsset::external_ids)
        .unwrap_or_default();
    if let Some(current) = current_name(&params.selection.iid) {
        taken.remove(&current);
    }
    let target = RenameInstance {
        precinct,
        iid: params.selection.iid.clone(),
    };
    Some((taken, target))
}

/// Observer on the rename dialog: gives the instance the name entered, as an edit of its precinct,
/// and keeps it selected under that name.
fn rename_instance(
    entered: On<TextEntered>,
    dialogs: Query<&RenameInstance>,
    precincts: Query<&Precinct>,
    mut edits: PrecinctEdits,
    mut selection: ResMut<ScenerySelection>,
) {
    let Ok(rename) = dialogs.get(entered.dialog) else {
        return;
    };
    let Ok(record) = precincts.get(rename.precinct) else {
        return;
    };
    let name = entered.text.clone();
    let old = match &rename.iid {
        SceneryInstanceId::External(id) => id.to_string(),
        _ => "instance".to_string(),
    };
    let mut renamed = false;
    edits.edit(&record.asset, |asset| {
        renamed = asset.rename_instance(&rename.iid, &name);
        renamed.then(|| format!("Rename {old} to {name}").into())
    });
    if renamed && selection.iid == rename.iid {
        selection.iid = SceneryInstanceId::External(name.into());
    }
}

#[cfg(test)]
mod tests {
    use bevy::math::Vec3;

    use super::*;

    /// The list names scenery and actors alike, sorted, and leaves out instances with no name.
    #[test]
    fn the_list_holds_the_named_instances() {
        let mut asset = PrecinctAsset {
            scenery_types: vec!["exemplars/furnishings.exem.json#WoodenChest".into()],
            ..default()
        };
        asset.add_scenery_element(0, 0., Vec3::ZERO, None);
        asset.add_scenery_element(
            0,
            0.,
            Vec3::X,
            Some(SceneryInstanceId::External(
                "wooden-chest-1".to_string().into(),
            )),
        );
        asset.add_actor("exemplars/characters.exem.json#Guard".into(), 0., Vec3::Z);
        let rows: Vec<String> = named_rows(&asset).into_iter().map(|(row, _)| row).collect();
        assert_eq!(rows, ["guard-1 (Guard)", "wooden-chest-1 (WoodenChest)"]);
    }
}
