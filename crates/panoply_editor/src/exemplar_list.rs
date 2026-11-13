//! The exemplars of the type a scenery tool places, in a [`Chooser`]: abstract ones too, in
//! italics, so they can be edited, though they cannot be placed. Every catalog under `exemplars/`
//! loads when scenery mode is first entered, since the game loads exemplars only as precincts name
//! them. Each tool's choice is saved, so returning to the tool, or restarting, keeps it.
// TODO(PAN5): the placing tools refuse an abstract exemplar.

use bevy::{
    asset::LoadedFolder,
    platform::collections::HashMap,
    prelude::*,
    settings::{ReflectSettingsGroup, SaveSettingsDeferred, SettingsGroup},
};
use panoply_actors::ACTOR_TYPE;
use panoply_exemplar::{Exemplar, InstanceType};
use panoply_scenery::{FIXTURE_TYPE, TERRAIN_FX_TYPE, WALL_TYPE};

use crate::{
    chooser::{chooser, Chooser},
    mode_selector::{EditorMode, SceneryTool},
    shell::SAVE_DELAY,
};

/// The wall list's first row, which places no wall: the wall tool erases with it.
pub const ERASE: &str = "(erase)";

/// The folder of exemplar catalogs, loaded and held while editing.
#[derive(Resource, Default)]
struct ExemplarFolder(Option<Handle<LoadedFolder>>);

/// The exemplar chosen in each tool's list, by its name there, saved between runs.
#[derive(Resource, SettingsGroup, Reflect, Clone, PartialEq, Debug, Default)]
#[reflect(Resource, SettingsGroup, Default)]
#[settings_group(group = "scenery_choices")]
pub struct SceneryChoices {
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

    /// The choice for `tool`, if it places exemplars.
    fn choice_mut(&mut self, tool: SceneryTool) -> Option<&mut Option<String>> {
        match tool {
            SceneryTool::PlaceWall => Some(&mut self.wall),
            SceneryTool::PlaceFixture => Some(&mut self.fixture),
            SceneryTool::PlaceActor => Some(&mut self.actor),
            SceneryTool::DrawTerrainFx => Some(&mut self.terrain_fx),
            _ => None,
        }
    }
}

/// A chooser listing the exemplars that `tool` places.
#[derive(Component, Default, Clone)]
pub struct ExemplarChooser {
    pub tool: SceneryTool,
}

/// The instance type `tool` places, if it places exemplars.
pub fn placed_type(tool: SceneryTool) -> Option<InstanceType> {
    match tool {
        SceneryTool::PlaceWall => Some(WALL_TYPE),
        SceneryTool::PlaceFixture => Some(FIXTURE_TYPE),
        SceneryTool::PlaceActor => Some(ACTOR_TYPE),
        SceneryTool::DrawTerrainFx => Some(TERRAIN_FX_TYPE),
        // TODO(PAN74): the floor types, which are not exemplars.
        SceneryTool::PlaceFloor
        | SceneryTool::Interact
        | SceneryTool::Layers
        | SceneryTool::RectSelect => None,
    }
}

pub struct ExemplarListPlugin;

impl Plugin for ExemplarListPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ExemplarFolder>()
            .add_systems(OnEnter(EditorMode::Scenery), load_exemplars)
            .add_systems(Update, (list_exemplars, save_choice));
    }
}

/// A chooser of the exemplars `tool` places, with the saved choice selected. Its filter box must
/// not take the focus: in the panel, the keys that move the view would type into it.
pub fn exemplar_chooser(tool: SceneryTool, selected: Option<String>) -> impl Scene {
    bsn! {
        @chooser(false, Vec::new(), selected.clone())
        ExemplarChooser { tool: {tool} }
    }
}

fn load_exemplars(mut folder: ResMut<ExemplarFolder>, server: Res<AssetServer>) {
    if folder.0.is_none() {
        folder.0 = Some(server.load_folder("exemplars"));
    }
}

/// Lists, in each [`ExemplarChooser`], the exemplars its tool places, once every catalog and the
/// catalogs they extend have loaded, so that the list is whole when it first shows: rows arriving
/// later would sort in above a selected row that had scrolled into view, and push it out. After
/// that, lists again as a catalog reloads.
fn list_exemplars(
    mut events: MessageReader<AssetEvent<Exemplar>>,
    added: Query<(), Added<ExemplarChooser>>,
    folder: Res<ExemplarFolder>,
    mut was_loaded: Local<bool>,
    server: Res<AssetServer>,
    exemplars: Res<Assets<Exemplar>>,
    mut choosers: Query<(&mut Chooser, &ExemplarChooser)>,
) {
    let changed = events.read().count() > 0;
    let loaded = folder
        .0
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
    choosers: Query<(&Chooser, &ExemplarChooser), Changed<Chooser>>,
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

    /// Each tool that places exemplars has a choice saved for it.
    #[test]
    fn every_placing_tool_has_a_choice() {
        let tools = [
            SceneryTool::PlaceFloor,
            SceneryTool::PlaceWall,
            SceneryTool::PlaceFixture,
            SceneryTool::PlaceActor,
            SceneryTool::DrawTerrainFx,
            SceneryTool::Interact,
            SceneryTool::Layers,
            SceneryTool::RectSelect,
        ];
        let mut choices = SceneryChoices::default();
        for tool in tools {
            assert_eq!(
                placed_type(tool).is_some(),
                choices.choice_mut(tool).is_some()
            );
        }
    }
}
