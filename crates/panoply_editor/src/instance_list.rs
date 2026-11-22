//! Selection mode's panel. Its list holds the selected precinct's instances, scenery and actors,
//! each by its label or else its exemplar and place, as faery's `SceneryInstanceChooser` listed its
//! named ones; choosing one selects it, and a click in the view moves the list's selection to the
//! instance clicked. A menu in the filter box narrows the list by kind and to the labelled. Below,
//! a pane names the selected instance's kind and exemplar, and shows its label, which Rename… sets
//! or clears.

use bevy::{
    app::PropagateOver,
    ecs::system::SystemParam,
    feathers::{
        constants::{fonts, size},
        containers::{pane, pane_body, pane_header},
        controls::{FeathersButton, FeathersCheckbox, FeathersMenu, FeathersMenuDivider},
        theme::{ThemeTextColor, ThemedText},
        tokens,
    },
    platform::collections::{HashMap, HashSet},
    prelude::*,
    text::{FontSourceTemplate, FontStyle},
};
use bevy_immediate::{
    attach::{BevyImmediateAttachPlugin, ImmediateAttach},
    ui::{activated::ImmUiActivated, checked::ImmUiChecked, text::ImmUiText, CapsUiFeathers},
    Imm, ImmEntity,
};
use panoply_actors::{PlacedAs, PrecinctActors};
use panoply_exemplar::Exemplar;
use panoply_precinct::{PrecinctAsset, SceneryInstanceId};
use panoply_scenery::{precinct::Precinct, scenery_element::SceneryElement, WALL_TYPE};

use crate::{
    adornment::adornment_menu_button,
    chooser::{chooser, Chooser},
    instance_index::{InstanceIndex, PrecinctPlace},
    instance_selection::ScenerySelection,
    mode_selector::SceneryTool,
    precinct_edit::PrecinctEdits,
    properties::{caption_cell, right_aligned_popup, CapsPanel, CAPTION_WIDTH},
    scenery_properties::header_text,
    text_entry::{clearable_text_entry_dialog, TextEntered},
    wall_tool::wall_types,
};

/// The chooser of the selected precinct's instances.
#[derive(Component, Default, Clone)]
pub struct InstanceChooser;

/// The immediate-mode root of the filter menu, in the chooser's filter box.
#[derive(Component, Default, Clone)]
pub(crate) struct InstanceFilterMenu;

/// Which instances the list holds: those of the kinds checked, and with `labelled`, only the
/// labelled among them.
#[derive(Resource, Clone, PartialEq, Eq, Debug)]
pub struct InstanceFilter {
    pub actor: bool,
    pub wall: bool,
    pub fixture: bool,
    pub labelled: bool,
}

impl Default for InstanceFilter {
    fn default() -> Self {
        Self {
            actor: true,
            wall: true,
            fixture: true,
            labelled: true,
        }
    }
}

/// What an instance is, as the pane names it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum InstanceKind {
    Actor,
    Wall,
    Fixture,
}

impl InstanceFilter {
    /// Whether the list holds an instance of `kind`, labelled or not.
    fn keeps(&self, kind: InstanceKind, labelled: bool) -> bool {
        let kind_kept = match kind {
            InstanceKind::Actor => self.actor,
            InstanceKind::Wall => self.wall,
            InstanceKind::Fixture => self.fixture,
        };
        kind_kept && (labelled || !self.labelled)
    }
}

/// The immediate-mode root of the selected instance's pane.
#[derive(Component, Default, Clone)]
pub(crate) struct InstancePane;

/// The label dialog, holding the instance it labels.
#[derive(Component, Clone)]
struct LabelInstance {
    precinct: Entity,
    iid: SceneryInstanceId,
}

pub struct InstanceListPlugin;

impl Plugin for InstanceListPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<InstanceFilter>()
            .add_plugins((
                BevyImmediateAttachPlugin::<CapsPanel, InstancePane>::new(),
                BevyImmediateAttachPlugin::<CapsUiFeathers, InstanceFilterMenu>::new(),
            ))
            .add_systems(
                Update,
                sync_instance_list.run_if(in_state(SceneryTool::SelectInstance)),
            );
    }
}

/// The list, empty until `sync_instance_list` fills it, with the filter menu in its filter box.
/// Its filter box must not take the focus, as the tools' lists' must not.
pub fn instance_chooser() -> impl Scene {
    bsn! {
        @chooser(false, Vec::new(), None, bsn_list! { InstanceFilterMenu Node })
        InstanceChooser
    }
}

/// The label of the exemplar at `path`, a `file#Label` path.
fn label(path: &str) -> &str {
    path.rsplit_once('#').map_or(path, |(_, label)| label)
}

/// The instances of `asset`, the precinct at `place`, that `filter` keeps, each with its row in the
/// list, sorted by row. `walls` says which of its scenery types are walls. A row is the instance's
/// label, or else its exemplar and place, as `WoodenChest, holm-overland (10, 16)`; a second
/// instance of an exemplar is `WoodenChest 2, holm-overland (10, 16)`, numbered in the precinct's
/// order among those with no label, shown or not, so that each row names one instance and keeps its
/// name as the filter changes.
fn instance_rows(
    asset: &PrecinctAsset,
    place: Option<&PrecinctPlace>,
    walls: &[bool],
    filter: &InstanceFilter,
) -> Vec<(String, SceneryInstanceId)> {
    let scenery = asset.scenery.iter().map(|instance| {
        let path = asset
            .scenery_types
            .get(instance.id)
            .map_or("", String::as_str);
        let kind = match walls.get(instance.id) {
            Some(true) => InstanceKind::Wall,
            _ => InstanceKind::Fixture,
        };
        (&instance.iid, instance.label.as_deref(), path, kind)
    });
    let actors = asset.actors.iter().map(|actor| {
        let path = actor.exemplar.as_str();
        (&actor.iid, actor.label.as_deref(), path, InstanceKind::Actor)
    });
    let mut seen = HashMap::<&str, usize>::new();
    let mut rows: Vec<_> = scenery
        .chain(actors)
        .filter_map(|(iid, instance_label, path, kind)| {
            // Counted whether kept or not, so a row's number does not depend on the filter.
            let count = instance_label.is_none().then(|| {
                let count = seen.entry(label(path)).or_default();
                *count += 1;
                *count
            });
            if !filter.keeps(kind, instance_label.is_some()) {
                return None;
            }
            let row = match (instance_label, count) {
                (Some(instance_label), _) => instance_label.to_string(),
                (None, count) => {
                    let mut row = label(path).to_string();
                    if let Some(count) = count.filter(|&count| count > 1) {
                        row += &format!(" {count}");
                    }
                    if let Some(place) = place {
                        row += &format!(", {place}");
                    }
                    row
                }
            };
            Some((row, iid.clone()))
        })
        .collect();
    rows.sort_by(|a, b| a.0.cmp(&b.0));
    rows
}

/// The place of the precinct whose asset is `handle`.
fn place_of(server: &AssetServer, handle: &Handle<PrecinctAsset>) -> Option<PrecinctPlace> {
    PrecinctPlace::of_path(server.get_path(handle)?.path())
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
    exemplars: Res<Assets<Exemplar>>,
    server: Res<AssetServer>,
    filter: Res<InstanceFilter>,
    mut choosers: Query<&mut Chooser, With<InstanceChooser>>,
    mut shown: Local<Option<String>>,
) {
    let Ok(mut chooser) = choosers.single_mut() else {
        return;
    };
    let precinct = selection.precinct;
    let rows = precinct
        .and_then(|precinct| instances.precincts.get(precinct).ok())
        .and_then(|(record, ..)| {
            let place = place_of(&server, &record.asset);
            let asset = assets.get(&record.asset)?;
            let walls = wall_types(&server, &exemplars, asset);
            Some(instance_rows(asset, place.as_ref(), &walls, &filter))
        })
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

impl ImmediateAttach<CapsUiFeathers> for InstanceFilterMenu {
    type Params = ResMut<'static, InstanceFilter>;

    /// A funnel opening a checkbox per kind, then, below a divider, Labeled, each named by its
    /// caption. Checkboxes keep the menu open, as the Show menu's do.
    fn construct(ui: &mut Imm<CapsUiFeathers>, filter: &mut ResMut<InstanceFilter>) {
        // Edit a copy, so the filter is marked changed only when a toggle changes it.
        let mut edited = (**filter).clone();
        ui.ch_id("menu")
            .on_spawn_apply_scene(|| bsn! { @FeathersMenu })
            .add(|ui| {
                ui.ch().on_spawn_apply_scene(|| {
                    bsn! {
                        @adornment_menu_button("editor/icons/lucide/filter.png")
                        #InstanceFilter
                    }
                });
                ui.ch()
                    .on_spawn_apply_scene(right_aligned_popup)
                    .add(|ui| {
                        let section = || Node {
                            flex_direction: FlexDirection::Column,
                            padding: UiRect::axes(px(8), px(4)),
                            row_gap: px(4),
                            ..default()
                        };
                        ui.ch().on_spawn_insert(section).add(|ui| {
                            checkbox(ui.ch_id("actor"), "Actor").checked(&mut edited.actor);
                            checkbox(ui.ch_id("wall"), "Wall").checked(&mut edited.wall);
                            checkbox(ui.ch_id("fixture"), "Fixture")
                                .checked(&mut edited.fixture);
                        });
                        ui.ch().on_spawn_apply_scene(|| bsn! { @FeathersMenuDivider });
                        ui.ch().on_spawn_insert(section).add(|ui| {
                            checkbox(ui.ch_id("labelled"), "Labeled")
                                .checked(&mut edited.labelled);
                        });
                    });
            });
        filter.set_if_neq(edited);
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
    index: Res<'w, InstanceIndex>,
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

    /// The selected instance's label, if it has one.
    fn current_label(&self) -> Option<String> {
        let record = self.precincts.get(self.selection.precinct?).ok()?;
        let asset = self.assets.get(&record.asset)?;
        asset.label(&self.selection.iid).map(str::to_string)
    }
}

impl ImmediateAttach<CapsPanel> for InstancePane {
    type Params = InstancePaneParams<'static, 'static>;

    /// In selection mode, a pane for the selected instance: "Fixture: WoodenChest" in its header,
    /// then its label, or "none" for an instance with no label, and Rename….
    fn construct(ui: &mut Imm<CapsPanel>, params: &mut InstancePaneParams) {
        let selecting = params
            .tool
            .as_ref()
            .is_some_and(|tool| *tool.get() == SceneryTool::SelectInstance);
        let Some((kind, name)) = selecting.then(|| params.described()).flatten() else {
            return;
        };
        let kind = format!("{kind}:");
        let current = params.current_label();
        let labelled = current.is_some();
        let shown = current.unwrap_or_else(|| "none".to_string());
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
                            caption_cell(ui, "Label".to_string(), CAPTION_WIDTH);
                            // An instance with no label shows "none", dim and italic, as a value
                            // that is not there.
                            let (key, style, color) = match labelled {
                                true => ("label", FontStyle::Normal, tokens::TEXT_MAIN),
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
                                .on_spawn_insert(|| Name::new("InstanceLabel"))
                                .on_hash_change_text_fn(&shown, || shown.clone());
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
                                rename.then(|| label_target(params)).flatten()
                            {
                                let current = params.current_label().unwrap_or_default();
                                ui.commands_mut()
                                    .spawn_scene(clearable_text_entry_dialog(
                                        "Rename", "Label", taken, current,
                                    ))
                                    .insert(target)
                                    .observe(label_instance);
                            }
                        });
                });
            });
    }
}

/// The labels a label for the selected instance may not take, those of every other instance in the
/// game, and the instance. The precincts loaded give their labels as they are now, unsaved edits
/// and all, and the index gives the rest.
fn label_target(params: &InstancePaneParams) -> Option<(HashSet<String>, LabelInstance)> {
    let precinct = params.selection.precinct?;
    params.precincts.get(precinct).ok()?;
    let mut loaded = HashSet::new();
    let mut taken = HashSet::new();
    for (id, asset) in params.assets.iter() {
        let place = params
            .server
            .get_path(id)
            .and_then(|path| PrecinctPlace::of_path(path.path()));
        loaded.extend(place);
        taken.extend(asset.labels().map(str::to_string));
    }
    taken.extend(params.index.labels_outside(&loaded).map(str::to_string));
    if let Some(current) = params.current_label() {
        taken.remove(&current);
    }
    let target = LabelInstance {
        precinct,
        iid: params.selection.iid.clone(),
    };
    Some((taken, target))
}

/// Observer on the label dialog: gives the instance the label entered, or clears its label if the
/// text is empty, as an edit of its precinct.
fn label_instance(
    entered: On<TextEntered>,
    dialogs: Query<&LabelInstance>,
    precincts: Query<&Precinct>,
    mut edits: PrecinctEdits,
) {
    let Ok(target) = dialogs.get(entered.dialog) else {
        return;
    };
    let Ok(record) = precincts.get(target.precinct) else {
        return;
    };
    let label = Some(entered.text.as_str()).filter(|text| !text.is_empty());
    edits.edit(&record.asset, |asset| {
        let old = asset.label(&target.iid).map(str::to_string);
        if old.as_deref() == label {
            return None;
        }
        asset.set_label(&target.iid, label)?;
        Some(match (old, label) {
            (Some(old), Some(label)) => format!("Rename {old} to {label}").into(),
            (None, Some(label)) => format!("Label {label}").into(),
            (Some(old), None) => format!("Clear label {old}").into(),
            (None, None) => unreachable!(),
        })
    });
}

#[cfg(test)]
mod tests {
    use bevy::math::{IVec2, Vec3};

    use super::*;

    /// The list holds the instances the filter keeps, scenery and actors alike, sorted: by label,
    /// or else by exemplar and place, numbered from the second of an exemplar on, whether or not
    /// the filter shows the others.
    #[test]
    fn the_list_holds_the_instances_the_filter_keeps() {
        let mut asset = PrecinctAsset {
            scenery_types: vec![
                "exemplars/furnishings.exem.json#WoodenChest".into(),
                "exemplars/walls.exem.json#BrickWall".into(),
            ],
            ..default()
        };
        for position in [Vec3::ZERO, Vec3::X, Vec3::Y] {
            asset.add_scenery_element(0, 0., position, None);
        }
        asset.add_scenery_element(1, 0., Vec3::Z, None);
        let second = asset.scenery[1].iid.clone();
        asset.set_label(&second, Some("strongbox"));
        asset.add_actor("exemplars/characters.exem.json#Guard".into(), 0., Vec3::Z);
        let marcus = asset.add_actor("exemplars/characters.exem.json#Guard".into(), 0., Vec3::X);
        asset.set_label(&marcus, Some("sgt.stone"));
        let place = PrecinctPlace {
            realm: "holm-overland".into(),
            coords: IVec2::new(10, 16),
        };
        let rows = |filter: InstanceFilter| -> Vec<String> {
            instance_rows(&asset, Some(&place), &[false, true], &filter)
                .into_iter()
                .map(|(row, _)| row)
                .collect()
        };
        assert_eq!(rows(InstanceFilter::default()), ["sgt.stone", "strongbox"]);
        let every = InstanceFilter {
            labelled: false,
            ..default()
        };
        assert_eq!(
            rows(every.clone()),
            [
                "BrickWall, holm-overland (10, 16)",
                "Guard, holm-overland (10, 16)",
                "WoodenChest 2, holm-overland (10, 16)",
                "WoodenChest, holm-overland (10, 16)",
                "sgt.stone",
                "strongbox",
            ]
        );
        let walls_and_actors = InstanceFilter {
            fixture: false,
            ..every
        };
        assert_eq!(
            rows(walls_and_actors),
            [
                "BrickWall, holm-overland (10, 16)",
                "Guard, holm-overland (10, 16)",
                "sgt.stone",
            ]
        );
    }
}
