//! Catalog entries (D90) in scenery mode: the entries of the kind a tool places, listed as its
//! exemplars are, and the chosen one's fields in the property editor. A kind is added with
//! [`EditCatalogEntryAppExt::edit_catalog_entry`]; nothing else here names a kind.

use std::{any::TypeId, marker::PhantomData, sync::Arc};

use bevy::{
    annotations::DisplayName, asset::AssetPath, platform::collections::HashMap, prelude::*,
    reflect::TypeInfo,
};
use panoply_exemplar::{CatalogEntry, EntryCatalog};
use panoply_scenery::FloorSurface;

use crate::{
    chooser::Chooser,
    exemplar_list::{placed_entry, CatalogFolders, SceneryChoices, ToolChooser},
    mode_selector::{EditorModeSettings, SceneryTool},
    properties::edit::{apply_property_edits, read_inspected_values, InspectedRoots, PropertyRoot},
    unsaved::{RegisterSaveable, UnsavedAssets},
};

/// The kinds of catalog entry the editor edits.
pub struct CatalogEntriesPlugin;

impl Plugin for CatalogEntriesPlugin {
    fn build(&self, app: &mut App) {
        app.edit_catalog_entry::<FloorSurface>();
    }
}

/// Adds a kind of catalog entry to the editor.
pub trait EditCatalogEntryAppExt {
    /// Lists `T`'s entries for the tool that places them (`placed_entry`), shows the chosen one
    /// in the property editor, and saves its files.
    fn edit_catalog_entry<T: CatalogEntry>(&mut self) -> &mut Self;
}

impl EditCatalogEntryAppExt for App {
    fn edit_catalog_entry<T: CatalogEntry>(&mut self) -> &mut Self {
        self.register_type::<EntryRoot<T>>()
            .register_saveable::<EntryCatalog<T>>()
            .add_systems(Update, list_entries::<T>)
            .add_systems(
                PostUpdate,
                inspect_chosen_entry::<T>
                    .after(apply_property_edits)
                    .before(read_inspected_values),
            )
    }
}

/// A catalog entry, the property editor's pane, captioned with its kind and label.
#[derive(Reflect)]
pub(crate) struct EntryRoot<T: CatalogEntry> {
    entry: AssetPath<'static>,
    name: String,
    #[reflect(ignore)]
    kind: PhantomData<fn() -> T>,
}

impl<T: CatalogEntry> EntryRoot<T> {
    fn id(&self, server: &AssetServer) -> Option<AssetId<T>> {
        server.get_path_id(&self.entry)?.try_typed::<T>().ok()
    }
}

impl<T: CatalogEntry> PropertyRoot for EntryRoot<T> {
    fn caption(&self) -> String {
        self.name.clone()
    }

    fn kind(&self) -> String {
        let info = T::type_info();
        let attributes = match info {
            TypeInfo::Struct(info) => Some(info.custom_attributes()),
            TypeInfo::TupleStruct(info) => Some(info.custom_attributes()),
            TypeInfo::Enum(info) => Some(info.custom_attributes()),
            _ => None,
        };
        attributes
            .and_then(|attributes| attributes.get::<DisplayName>())
            .map_or(info.type_path_table().short_path(), |name| name.0)
            .to_string()
    }

    fn key(&self) -> String {
        self.entry.to_string()
    }

    fn read(&self, world: &World) -> Option<Box<dyn PartialReflect>> {
        let id = self.id(world.resource::<AssetServer>())?;
        let entry = world.resource::<Assets<T>>().get(id)?;
        Some(Box::new(entry.clone()))
    }

    fn modify(
        &self,
        world: &mut World,
        change: &mut dyn FnMut(&mut dyn PartialReflect) -> Result<(), BevyError>,
    ) -> Result<(), BevyError> {
        let server = world.resource::<AssetServer>().clone();
        let id = self
            .id(&server)
            .ok_or_else(|| format!("{} is not loaded", self.entry))?;
        let catalog = server
            .get_path_id(self.entry.without_label())
            .ok_or_else(|| format!("{} has no catalog", self.entry))?;
        let mut entries = world.resource_mut::<Assets<T>>();
        let mut entry = entries
            .get(id)
            .ok_or_else(|| format!("{} is not loaded", self.entry))?
            .clone();
        change(entry.as_partial_reflect_mut())?;
        // Through `get_mut`, so the change is an `AssetEvent::Modified`, which what is built from
        // the entry is rebuilt on.
        if let Some(mut slot) = entries.get_mut(id) {
            *slot = entry;
        }
        world.resource_mut::<UnsavedAssets>().mark(catalog);
        Ok(())
    }
}

/// The entries of kind `T`, each with its name in the list: its label, or, where two files use
/// the same label, the label and its file's name.
pub(crate) fn named_entries<T: CatalogEntry>(
    server: &AssetServer,
    catalogs: &Assets<EntryCatalog<T>>,
) -> Vec<(String, AssetId<T>)> {
    let suffix = format!(".{}", T::EXTENSION);
    let entries: Vec<(String, String, AssetId<T>)> = catalogs
        .iter()
        .filter_map(|(id, catalog)| {
            let path = server.get_path(id)?;
            let file = path.path().file_name()?.to_string_lossy().into_owned();
            let file = file.strip_suffix(&suffix).unwrap_or(&file).to_string();
            Some(
                catalog
                    .entries()
                    .map(|(label, handle)| (label.to_string(), file.clone(), handle.id()))
                    .collect::<Vec<_>>(),
            )
        })
        .flatten()
        .collect();
    let mut counts = HashMap::<&str, usize>::default();
    for (label, ..) in &entries {
        *counts.entry(label.as_str()).or_default() += 1;
    }
    entries
        .iter()
        .map(|(label, file, id)| match counts[label.as_str()] {
            1 => (label.clone(), *id),
            _ => (format!("{label} ({file})"), *id),
        })
        .collect()
}

/// Whether `tool` places entries of kind `T`.
fn places<T: CatalogEntry>(tool: SceneryTool) -> bool {
    placed_entry(tool) == Some(TypeId::of::<T>())
}

/// Lists, in each [`ToolChooser`] whose tool places entries of kind `T`, those entries, once the
/// catalog folder has loaded, and again as a file of them reloads.
fn list_entries<T: CatalogEntry>(
    mut events: MessageReader<AssetEvent<EntryCatalog<T>>>,
    added: Query<(), Added<ToolChooser>>,
    folders: Res<CatalogFolders>,
    mut was_loaded: Local<bool>,
    server: Res<AssetServer>,
    catalogs: Res<Assets<EntryCatalog<T>>>,
    mut choosers: Query<(&mut Chooser, &ToolChooser)>,
) {
    let changed = events.read().count() > 0;
    let loaded = folders.entries_loaded(&server);
    let just_loaded = loaded && !*was_loaded;
    *was_loaded = loaded;
    if !loaded || !(just_loaded || changed || !added.is_empty()) {
        return;
    }
    for (mut chooser, list) in &mut choosers {
        if !places::<T>(list.tool) {
            continue;
        }
        let items: Vec<String> = named_entries(&server, &catalogs)
            .into_iter()
            .map(|(name, _)| name)
            .collect();
        if chooser.items != items {
            chooser.items = items;
        }
    }
}

/// Inspects the entry chosen in the current tool's list, if the tool places entries of kind `T`.
fn inspect_chosen_entry<T: CatalogEntry>(
    settings: Res<EditorModeSettings>,
    choices: Res<SceneryChoices>,
    server: Res<AssetServer>,
    catalogs: Res<Assets<EntryCatalog<T>>>,
    mut inspected: ResMut<InspectedRoots>,
) {
    let tool = settings.scenery_tool;
    if !places::<T>(tool) {
        return;
    }
    let chosen = choices.choice(tool).and_then(|name| {
        let (_, id) = named_entries(&server, &catalogs)
            .into_iter()
            .find(|(found, _)| *found == name)?;
        Some((name, server.get_path(id)?.into_owned()))
    });
    let Some((name, entry)) = chosen else {
        if !inspected.roots.is_empty() {
            inspected.roots.clear();
        }
        return;
    };
    let root: Arc<dyn PropertyRoot> = Arc::new(EntryRoot::<T> {
        entry,
        name,
        kind: PhantomData,
    });
    let same = inspected.roots.len() == 1
        && inspected.roots[0].key() == root.key()
        && inspected.roots[0].caption() == root.caption();
    if !same {
        inspected.roots = vec![root];
    }
}
