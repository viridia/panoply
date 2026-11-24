//! Catalog entries (D90): typed records loaded and saved as exemplars are, with no aspects and no
//! prototype. A file holds entries of one kind, a JSON object from each entry's label to its
//! fields, and its extension names the kind (`floors.floor.json`). An entry is read through
//! reflection, onto its type's default, so a file leaves out what is at its default; it is
//! referred to by `file#Label`, and loaded by path through
//! [`CatalogServer`](crate::CatalogServer).

use std::{fmt, marker::PhantomData};

use bevy::{
    asset::{io::Reader, AssetLoader, LoadContext},
    platform::collections::HashSet,
    prelude::*,
    reflect::{
        serde::TypedReflectDeserializer, GetTypeRegistration, TypeRegistry, TypeRegistryArc, Typed,
    },
};
use panoply_saga::{FormulaQueue, SagaHost, SagaPlugin};
use serde::de::{self, DeserializeSeed, MapAccess, Visitor};
use serde_json::Value;
use thiserror::Error;

use crate::save::{merge, reflect_json};

/// A kind of catalog entry. Its caption in the editor is its type's `DisplayName`.
pub trait CatalogEntry: Asset + Reflect + Typed + GetTypeRegistration + Default + Clone {
    /// The extension of the files holding entries of this kind, as `floor.json`.
    const EXTENSION: &'static str;
}

/// A file of catalog entries of one kind. Its entries are its labeled assets, `file#Label`.
#[derive(Asset, TypePath)]
pub struct EntryCatalog<T: CatalogEntry> {
    /// Each entry's label, handle, and data as loaded. An edit changes the entry in
    /// `Assets<T>`, so this stays as the file says; saving compares the two to find what was
    /// edited.
    entries: Vec<(String, Handle<T>, T)>,
}

impl<T: CatalogEntry> EntryCatalog<T> {
    /// Each entry's label and handle, in the file's order.
    pub fn entries(&self) -> impl Iterator<Item = (&str, &Handle<T>)> {
        self.entries
            .iter()
            .map(|(label, handle, _)| (label.as_str(), handle))
    }

    /// Writes the catalog's edits into `file`, its file's JSON, as an exemplar catalog's are
    /// (D73). An entry whose data serializes as it did when loaded is left as the file has it. A
    /// field at its default is written as absent, as `None` is, so a field edited back to its
    /// default leaves the file; one the file gives at its default, unedited, stays.
    pub fn merge_into(
        &self,
        file: &mut Value,
        entries: &Assets<T>,
        registry: &TypeRegistry,
    ) -> Result<(), BevyError> {
        let defaults = reflect_json(T::default().as_partial_reflect(), registry)?;
        let object = file
            .as_object_mut()
            .ok_or("the catalog is not a JSON object")?;
        for (label, handle, loaded) in &self.entries {
            let Some(now) = entries.get(handle) else {
                continue;
            };
            let mut base = reflect_json(loaded.as_partial_reflect(), registry)?;
            let mut new = reflect_json(now.as_partial_reflect(), registry)?;
            for value in [&mut base, &mut new] {
                defaults_absent(value, &defaults);
            }
            match object.get_mut(label) {
                Some(original) => merge(original, &base, &new),
                None => {
                    object.insert(label.clone(), new);
                }
            }
        }
        Ok(())
    }
}

/// Writes each field of `value`, an entry's fields, that equals its default in `defaults` as
/// `null`, which a merge treats as absent.
fn defaults_absent(value: &mut Value, defaults: &Value) {
    let (Some(fields), Some(defaults)) = (value.as_object_mut(), defaults.as_object()) else {
        return;
    };
    for (key, field) in fields.iter_mut() {
        if defaults.get(key) == Some(field) {
            *field = Value::Null;
        }
    }
}

/// Reads the entries of `json`, a file of entries of kind `T`, in order, as the loader does.
/// Formulas in them are read into `formulas`, to be compiled once the whole file has been read.
pub fn read_entries<T: CatalogEntry>(
    json: &[u8],
    registry: &TypeRegistry,
    formulas: &mut FormulaQueue,
) -> Result<Vec<(String, T)>, serde_json::Error> {
    let reader = formulas.track_json(json);
    let mut deserializer = serde_json::Deserializer::from_reader(reader);
    let entries = EntriesSeed::<T> {
        registry,
        formulas,
        kind: PhantomData,
    }
    .deserialize(&mut deserializer)?;
    deserializer.end()?;
    Ok(entries)
}

struct EntriesSeed<'a, T> {
    registry: &'a TypeRegistry,
    formulas: &'a mut FormulaQueue,
    kind: PhantomData<fn() -> T>,
}

impl<'de, T: CatalogEntry> DeserializeSeed<'de> for EntriesSeed<'_, T> {
    type Value = Vec<(String, T)>;

    fn deserialize<D: de::Deserializer<'de>>(
        self,
        deserializer: D,
    ) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_map(self)
    }
}

impl<'de, T: CatalogEntry> Visitor<'de> for EntriesSeed<'_, T> {
    type Value = Vec<(String, T)>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "a map of {} entries", T::type_path())
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        let registration = self
            .registry
            .get(std::any::TypeId::of::<T>())
            .ok_or_else(|| de::Error::custom(format!("{} is not registered", T::type_path())))?;
        let mut entries = Vec::with_capacity(map.size_hint().unwrap_or(0));
        let mut labels = HashSet::new();
        while let Some(label) = map.next_key::<String>()? {
            if !labels.insert(label.clone()) {
                return Err(de::Error::custom(format!("{label} is in the file twice")));
            }
            let fields = map.next_value_seed(TypedReflectDeserializer::with_processor(
                registration,
                self.registry,
                &mut *self.formulas,
            ))?;
            // Onto the default, so a field the file leaves out is at its default.
            let mut entry = T::default();
            entry
                .try_apply(&*fields)
                .map_err(|err| de::Error::custom(format!("{label}: {err}")))?;
            entries.push((label, entry));
        }
        Ok(entries)
    }
}

/// Loads a file of catalog entries of kind `T`.
#[derive(TypePath)]
pub struct EntryCatalogLoader<T: CatalogEntry> {
    type_registry: TypeRegistryArc,
    host: SagaHost,
    kind: PhantomData<fn() -> T>,
}

impl<T: CatalogEntry> FromWorld for EntryCatalogLoader<T> {
    fn from_world(world: &mut World) -> Self {
        Self {
            type_registry: world.resource::<AppTypeRegistry>().0.clone(),
            host: world.resource::<SagaHost>().clone(),
            kind: PhantomData,
        }
    }
}

/// Why a file of catalog entries did not load.
#[non_exhaustive]
#[derive(Debug, Error)]
pub enum EntryCatalogLoaderError {
    /// The file could not be read.
    #[error("Could not load catalog entries: {0}")]
    Io(#[from] std::io::Error),
    /// The file is not a map of entries of its kind.
    #[error("Could not decode catalog entries: {0}")]
    Decode(#[from] serde_json::Error),
}

impl<T: CatalogEntry> AssetLoader for EntryCatalogLoader<T> {
    type Asset = EntryCatalog<T>;
    type Settings = ();
    type Error = EntryCatalogLoaderError;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &Self::Settings,
        load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        let mut formulas = FormulaQueue::default();
        let read = read_entries::<T>(&bytes, &self.type_registry.read(), &mut formulas)?;
        let entries = read
            .into_iter()
            .map(|(label, entry)| {
                let handle = load_context.add_labeled_asset(label.clone(), entry.clone());
                (label, handle, entry)
            })
            .collect();
        let path = load_context.path().to_string();
        formulas.compile(&path, self.host.get(), &bytes, &[]).await;
        Ok(EntryCatalog { entries })
    }

    fn extensions(&self) -> &[&str] {
        std::slice::from_ref(&T::EXTENSION)
    }
}

/// Adds a kind of catalog entry to an app.
pub trait CatalogEntryAppExt {
    /// Registers `T`, its asset, and the loader of its files. Adds [`SagaPlugin`], which the
    /// loader compiles formulas with, if it is not added yet.
    fn register_catalog_entry<T: CatalogEntry>(&mut self) -> &mut Self;
}

impl CatalogEntryAppExt for App {
    fn register_catalog_entry<T: CatalogEntry>(&mut self) -> &mut Self {
        if !self.is_plugin_added::<SagaPlugin>() {
            self.add_plugins(SagaPlugin);
        }
        self.register_type::<T>()
            .init_asset::<T>()
            .init_asset::<EntryCatalog<T>>()
            .init_asset_loader::<EntryCatalogLoader<T>>()
    }
}

#[cfg(test)]
mod tests {
    use bevy::reflect::TypeRegistry;
    use panoply_saga::{Formula, HostState};
    use serde_json::json;

    use super::*;

    /// A kind of entry for the tests, with a formula, an optional field and a defaulted one.
    #[derive(Asset, Reflect, Clone, Default, Debug)]
    #[reflect(Default)]
    struct Sign {
        text: String,
        size: Option<f32>,
        lit: bool,
        when: Option<Formula<bool>>,
    }

    impl CatalogEntry for Sign {
        const EXTENSION: &'static str = "sign.json";
    }

    fn registry() -> TypeRegistry {
        let mut registry = TypeRegistry::new();
        registry.register::<Sign>();
        registry
    }

    fn read(json: &str, host: &HostState) -> Result<Vec<(String, Sign)>, serde_json::Error> {
        let mut formulas = FormulaQueue::default();
        let entries = read_entries::<Sign>(json.as_bytes(), &registry(), &mut formulas)?;
        futures_lite::future::block_on(formulas.compile(
            "test.sign.json",
            host,
            json.as_bytes(),
            &[],
        ));
        Ok(entries)
    }

    #[test]
    fn entries_load_by_label_in_order_with_defaults() {
        let entries = read(
            r#"{ "Inn": { "text": "The Inn", "size": 2 }, "Gate": { "text": "Gate", "lit": true } }"#,
            &HostState::new(),
        )
        .unwrap();
        let labels: Vec<&str> = entries.iter().map(|(label, _)| label.as_str()).collect();
        assert_eq!(labels, ["Inn", "Gate"]);
        assert_eq!(entries[0].1.size, Some(2.));
        assert!(!entries[0].1.lit);
        assert_eq!(entries[1].1.size, None);
        assert!(entries[1].1.lit);
    }

    #[test]
    fn a_label_twice_is_an_error() {
        let result = read(
            r#"{ "Inn": { "text": "a" }, "Inn": { "text": "b" } }"#,
            &HostState::new(),
        );
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Inn is in the file twice"));
    }

    #[test]
    fn a_formula_compiles_and_a_bad_one_names_its_line() {
        let entries = read(
            "{\n  \"Inn\": { \"text\": \"a\", \"when\": \"1 < 2\" },\n  \"Gate\": { \"text\": \"b\", \"when\": \"missing\" }\n}",
            &HostState::new(),
        )
        .unwrap();
        let good = entries[0].1.when.as_ref().unwrap();
        assert!(good.module().is_some(), "{:?}", good.error());
        let error = entries[1].1.when.as_ref().unwrap().error().unwrap();
        assert!(error.starts_with("test.sign.json:3:"), "{error}");
    }

    #[test]
    fn a_save_writes_only_the_edit() {
        let mut registry = registry();
        registry.register::<Formula<bool>>();
        let mut assets = Assets::<Sign>::default();
        let inn = Sign {
            text: "The Inn".into(),
            size: Some(2.),
            ..default()
        };
        let gate = Sign {
            text: "Gate".into(),
            lit: true,
            ..default()
        };
        let inn_handle = assets.add(inn.clone());
        let gate_handle = assets.add(gate.clone());
        let catalog = EntryCatalog {
            entries: vec![
                ("Inn".into(), inn_handle.clone(), inn),
                ("Gate".into(), gate_handle.clone(), gate),
            ],
        };
        assets.get_mut(&inn_handle).unwrap().size = None;
        // Edited back to its default, `lit` leaves the file; `size`, given at its default and
        // not edited, stays.
        assets.get_mut(&gate_handle).unwrap().lit = false;
        let mut file = json!({
            "Inn": { "text": "The Inn", "size": 2, "note": "kept" },
            "Gate": { "text": "Gate", "lit": true, "size": null }
        });
        catalog.merge_into(&mut file, &assets, &registry).unwrap();
        assert_eq!(
            file,
            json!({
                "Inn": { "text": "The Inn", "note": "kept" },
                "Gate": { "text": "Gate", "size": null }
            })
        );
    }
}
