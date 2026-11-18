//! What a scenery tool places, in a [`Chooser`]: the exemplars of its type, abstract ones too, in
//! italics, so they can be edited, though they cannot be placed; or the catalog entries of its
//! kind (`catalog_entries`). Every file under `exemplars/` and `catalog/` loads when scenery mode
//! is first entered, since the game loads them only as precincts name them. Each tool's choice is
//! saved, so returning to the tool, or restarting, keeps it.

use std::any::TypeId;

use bevy::{
    asset::LoadedFolder,
    platform::collections::HashMap,
    prelude::*,
    settings::{ReflectSettingsGroup, SaveSettingsDeferred, SettingsGroup},
};
use panoply_actors::ACTOR_TYPE;
use panoply_exemplar::{Exemplar, InstanceType};
use panoply_scenery::{FloorSurface, TerrainEffect, FIXTURE_TYPE, WALL_TYPE};

use crate::{
    chooser::{chooser, Chooser},
    mode_selector::{EditorMode, SceneryTool},
    shell::SAVE_DELAY,
};

/// The wall list's first row, which places no wall: the wall tool erases with it.
pub const ERASE: &str = "(erase)";

/// The folders of exemplar catalogs and of catalog entries, loaded and held while editing.
#[derive(Resource, Default)]
pub(crate) struct CatalogFolders {
    exemplars: Option<Handle<LoadedFolder>>,
    entries: Option<Handle<LoadedFolder>>,
}

impl CatalogFolders {
    /// Whether the folder of catalog entries has loaded, with everything in it.
    pub(crate) fn entries_loaded(&self, server: &AssetServer) -> bool {
        self.entries
            .as_ref()
            .is_some_and(|folder| server.is_loaded_with_dependencies(folder))
    }
}

/// What is chosen in each tool's list, by its name there, saved between runs.
#[derive(Resource, SettingsGroup, Reflect, Clone, PartialEq, Debug, Default)]
#[reflect(Resource, SettingsGroup, Default)]
#[settings_group(group = "scenery_choices")]
pub struct SceneryChoices {
    pub floor: Option<String>,
    pub wall: Option<String>,
    pub fixture: Option<String>,
    pub actor: Option<String>,
    pub terrain_fx: Option<String>,
}

impl SceneryChoices {
    /// The choice for `tool`.
    pub fn choice(&self, tool: SceneryTool) -> Option<String> {
        self.clone().choice_mut(tool).cloned().flatten()
    }

    /// The choice for `tool`, if it has a list.
    fn choice_mut(&mut self, tool: SceneryTool) -> Option<&mut Option<String>> {
        match tool {
            SceneryTool::PlaceFloor => Some(&mut self.floor),
            SceneryTool::PlaceWall => Some(&mut self.wall),
            SceneryTool::PlaceFixture => Some(&mut self.fixture),
            SceneryTool::PlaceActor => Some(&mut self.actor),
            SceneryTool::DrawTerrainFx => Some(&mut self.terrain_fx),
            _ => None,
        }
    }
}

/// A chooser listing what `tool` places.
#[derive(Component, Default, Clone)]
pub struct ToolChooser {
    pub tool: SceneryTool,
}

/// The instance type `tool` places, if it places exemplars.
pub fn placed_type(tool: SceneryTool) -> Option<InstanceType> {
    match tool {
        SceneryTool::PlaceWall => Some(WALL_TYPE),
        SceneryTool::PlaceFixture => Some(FIXTURE_TYPE),
        SceneryTool::PlaceActor => Some(ACTOR_TYPE),
        SceneryTool::PlaceFloor
        | SceneryTool::DrawTerrainFx
        | SceneryTool::SelectInstance
        | SceneryTool::Layers
        | SceneryTool::RectSelect => None,
    }
}

/// The kind of catalog entry `tool` places, if it places entries.
pub fn placed_entry(tool: SceneryTool) -> Option<TypeId> {
    match tool {
        SceneryTool::PlaceFloor => Some(TypeId::of::<FloorSurface>()),
        SceneryTool::DrawTerrainFx => Some(TypeId::of::<TerrainEffect>()),
        _ => None,
    }
}

/// Whether `tool` has a list of what it places.
pub fn has_list(tool: SceneryTool) -> bool {
    placed_type(tool).is_some() || placed_entry(tool).is_some()
}

pub struct ExemplarListPlugin;

impl Plugin for ExemplarListPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CatalogFolders>()
            .add_systems(OnEnter(EditorMode::Scenery), load_catalogs)
            .add_systems(Update, (list_exemplars, save_choice));
    }
}

/// A chooser of what `tool` places, with the saved choice selected. Its filter box must not take
/// the focus: in the panel, the keys that move the view would type into it.
pub fn tool_chooser(tool: SceneryTool, selected: Option<String>) -> impl Scene {
    bsn! {
        @chooser(false, Vec::new(), selected.clone())
        ToolChooser { tool: {tool} }
    }
}

fn load_catalogs(mut folders: ResMut<CatalogFolders>, server: Res<AssetServer>) {
    if folders.exemplars.is_none() {
        folders.exemplars = Some(server.load_folder("exemplars"));
    }
    if folders.entries.is_none() {
        folders.entries = Some(server.load_folder("catalog"));
    }
}

/// Lists, in each [`ToolChooser`] whose tool places exemplars, those exemplars, once every catalog
/// and the catalogs they extend have loaded, so that the list is whole when it first shows: rows
/// arriving later would sort in above a selected row that had scrolled into view, and push it out.
/// After that, lists again as a catalog reloads.
fn list_exemplars(
    mut events: MessageReader<AssetEvent<Exemplar>>,
    added: Query<(), Added<ToolChooser>>,
    folders: Res<CatalogFolders>,
    mut was_loaded: Local<bool>,
    server: Res<AssetServer>,
    exemplars: Res<Assets<Exemplar>>,
    mut choosers: Query<(&mut Chooser, &ToolChooser)>,
) {
    let changed = events.read().count() > 0;
    let loaded = folders
        .exemplars
        .as_ref()
        .is_some_and(|folder| server.is_loaded_with_dependencies(folder));
    let just_loaded = loaded && !*was_loaded;
    *was_loaded = loaded;
    if !loaded || !(just_loaded || changed || !added.is_empty()) {
        return;
    }
    for (mut chooser, list) in &mut choosers {
        let Some(instance_type) = placed_type(list.tool) else {
            continue;
        };
        let named = named_exemplars(&server, &exemplars, instance_type);
        let mut items: Vec<String> = named.iter().map(|(name, _)| name.clone()).collect();
        let italic: Vec<String> = named
            .iter()
            .filter(|(_, id)| exemplars.get(*id).is_some_and(|ex| ex.0.is_abstract))
            .map(|(name, _)| name.clone())
            .collect();
        if list.tool == SceneryTool::PlaceWall {
            items.insert(0, ERASE.to_string());
        }
        if chooser.items != items || chooser.italic != italic {
            chooser.items = items;
            chooser.italic = italic;
        }
    }
}

/// The exemplar named `name` in `tool`'s list.
pub fn chosen_exemplar(
    server: &AssetServer,
    exemplars: &Assets<Exemplar>,
    tool: SceneryTool,
    name: &str,
) -> Option<AssetId<Exemplar>> {
    named_exemplars(server, exemplars, placed_type(tool)?)
        .into_iter()
        .find_map(|(found, id)| (found == name).then_some(id))
}

/// What `tool` would place: its chosen exemplar, or `None` for `(erase)`. `Err` while nothing
/// placeable is chosen, with why, if the reason is worth telling.
pub(crate) fn chosen_placeable(
    choices: &SceneryChoices,
    tool: SceneryTool,
    server: &AssetServer,
    exemplars: &Assets<Exemplar>,
) -> Result<Option<AssetId<Exemplar>>, Option<String>> {
    let Some(name) = choices.choice(tool) else {
        return Err(None);
    };
    if name == ERASE {
        return Ok(None);
    }
    let id = chosen_exemplar(server, exemplars, tool, &name).ok_or(None)?;
    match exemplars.get(id) {
        Some(exemplar) if exemplar.0.is_abstract => {
            Err(Some(format!("{name} is abstract, so it cannot be placed")))
        }
        Some(_) => Ok(Some(id)),
        None => Err(None),
    }
}

/// The exemplars of `instance_type`, each with its name in the list: its label, or, where two
/// catalogs use the same label, the label and its catalog's name. The old names an exemplar keeps
/// as aliases are left out.
pub(crate) fn named_exemplars(
    server: &AssetServer,
    exemplars: &Assets<Exemplar>,
    instance_type: InstanceType,
) -> Vec<(String, AssetId<Exemplar>)> {
    let entries: Vec<(String, String, AssetId<Exemplar>)> = exemplars
        .iter()
        .filter(|(_, exemplar)| exemplar.0.meta_type == instance_type)
        .filter_map(|(id, exemplar)| {
            let path = server.get_path(id)?;
            let label = path.label()?.to_string();
            if exemplar.0.alias.contains(&label) {
                return None;
            }
            let file = path.path().file_name()?.to_string_lossy();
            let catalog = file.strip_suffix(".exem.json").unwrap_or(&file).to_string();
            Some((label, catalog, id))
        })
        .collect();
    let mut counts = HashMap::<&str, usize>::default();
    for (label, ..) in &entries {
        *counts.entry(label.as_str()).or_default() += 1;
    }
    entries
        .iter()
        .map(|(label, catalog, id)| match counts[label.as_str()] {
            1 => (label.clone(), *id),
            _ => (format!("{label} ({catalog})"), *id),
        })
        .collect()
}

/// Saves each chooser's selection as its tool's choice. A selection the filter hides is cleared in
/// the chooser but kept as the choice.
fn save_choice(
    choosers: Query<(&Chooser, &ToolChooser), Changed<Chooser>>,
    mut choices: ResMut<SceneryChoices>,
    mut commands: Commands,
) {
    for (chooser, list) in &choosers {
        let Some(selected) = &chooser.selected else {
            continue;
        };
        let mut next = choices.clone();
        if let Some(choice) = next.choice_mut(list.tool) {
            *choice = Some(selected.clone());
        }
        if choices.set_if_neq(next) {
            commands.queue(SaveSettingsDeferred(SAVE_DELAY));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Each tool that has a list has a choice saved for it.
    #[test]
    fn every_placing_tool_has_a_choice() {
        let tools = [
            SceneryTool::PlaceFloor,
            SceneryTool::PlaceWall,
            SceneryTool::PlaceFixture,
            SceneryTool::PlaceActor,
            SceneryTool::DrawTerrainFx,
            SceneryTool::SelectInstance,
            SceneryTool::Layers,
            SceneryTool::RectSelect,
        ];
        let mut choices = SceneryChoices::default();
        for tool in tools {
            assert_eq!(has_list(tool), choices.choice_mut(tool).is_some());
        }
    }
}
