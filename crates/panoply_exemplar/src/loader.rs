use bevy::platform::collections::HashMap;
use bevy::{
    asset::{io::Reader, AssetLoader, LoadContext},
    prelude::*,
    reflect::{TypeRegistry, TypeRegistryArc},
};
use panoply_saga::{scripted::AspectDecl, FormulaQueue, SagaHost, ScriptModuleAsset};
use serde::{
    de::{self, DeserializeSeed, Visitor},
    Deserialize, Deserializer,
};
use std::{fmt, sync::Arc};
use thiserror::Error;

use crate::exemplar::ExemplarData;

use super::aspect_list::AspectListDeserializer;
use super::{Exemplar, ExemplarCatalog};

#[derive(Deserialize)]
#[serde(field_identifier, rename_all = "snake_case")]
enum Field {
    Kind,
    DisplayName,
    Alias,
    Aspects,
    Extends,
    Abstract,
}

struct ExemplarVisitor<'a, 'b> {
    type_registry: &'a TypeRegistry,
    load_context: &'a mut LoadContext<'b>,
    formulas: &'a mut FormulaQueue,
    exemplar_name: &'a str,
    scripted: &'a [Arc<AspectDecl>],
}

impl<'de> Visitor<'de> for ExemplarVisitor<'_, '_> {
    type Value = ExemplarData;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("an exemplar")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::MapAccess<'de>,
    {
        let mut result = ExemplarData {
            kind: None,
            display_name: None,
            alias: Vec::new(),
            aspects: Vec::new(),
            extends: None,
            is_abstract: false,
            formulas: Vec::new(),
        };
        while let Some(key) = map.next_key()? {
            match key {
                Field::Kind => {
                    if result.kind.is_some() {
                        return Err(de::Error::duplicate_field("kind"));
                    }
                    result.kind = Some(map.next_value()?);
                }
                Field::DisplayName => {
                    if result.display_name.is_some() {
                        return Err(de::Error::duplicate_field("nanos"));
                    }
                    result.display_name = Some(map.next_value()?);
                }
                Field::Alias => {
                    if !result.alias.is_empty() {
                        return Err(de::Error::duplicate_field("alias"));
                    }
                    result.alias = map.next_value()?;
                }
                Field::Aspects => {
                    if !result.aspects.is_empty() {
                        return Err(de::Error::duplicate_field("aspects"));
                    }
                    let mark = self.formulas.mark();
                    result.aspects = map.next_value_seed(AspectListDeserializer {
                        type_registry: self.type_registry,
                        load_context: self.load_context,
                        formulas: self.formulas,
                        label_prefix: self.exemplar_name,
                        scripted: self.scripted,
                    })?;
                    result.formulas = self.formulas.read_since(mark);
                }
                Field::Extends => {
                    if result.extends.is_some() {
                        return Err(de::Error::duplicate_field("extends"));
                    }
                    let extends = map.next_value::<String>()?;
                    let extends_path = self
                        .load_context
                        .path()
                        .resolve_embed_str(&extends)
                        .map_err(de::Error::custom)?;
                    // An exemplar in this catalog: loading it by path would start another load
                    // of this file (Bevy issue #10549).
                    result.extends = Some(match extends_path.label() {
                        Some(label)
                            if extends_path.without_label() == *self.load_context.path() =>
                        {
                            self.load_context
                                .get_label_handle::<Exemplar>(label.to_owned())
                        }
                        _ => self.load_context.load::<Exemplar>(extends_path),
                    });
                }
                Field::Abstract => {
                    result.is_abstract = map.next_value()?;
                }
            }
        }
        Ok(result)
    }
}

struct ExemplarDeserializer<'a, 'b> {
    type_registry: &'a TypeRegistry,
    load_context: &'a mut LoadContext<'b>,
    formulas: &'a mut FormulaQueue,
    exemplar_name: &'a str,
    scripted: &'a [Arc<AspectDecl>],
}

impl<'de> DeserializeSeed<'de> for ExemplarDeserializer<'_, '_> {
    type Value = ExemplarData;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_map(ExemplarVisitor {
            type_registry: self.type_registry,
            load_context: self.load_context,
            formulas: self.formulas,
            exemplar_name: self.exemplar_name,
            scripted: self.scripted,
        })
    }
}

struct CatalogVisitor<'a, 'b> {
    type_registry: &'a TypeRegistry,
    load_context: &'a mut LoadContext<'b>,
    formulas: &'a mut FormulaQueue,
    scripted: &'a [Arc<AspectDecl>],
}

impl<'de> Visitor<'de> for CatalogVisitor<'_, '_> {
    type Value = ExemplarCatalog;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("exemplar catalog")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: de::MapAccess<'de>,
    {
        let mut entries: HashMap<String, Handle<Exemplar>> =
            HashMap::with_capacity(map.size_hint().unwrap_or(0));
        let mut local_extends: Vec<(String, String)> = Vec::new();
        let mut loaded = Vec::with_capacity(map.size_hint().unwrap_or(0));
        while let Some(key) = map.next_key::<String>()? {
            let mut lc = self.load_context.begin_labeled_asset();
            let sdata = map.next_value_seed(ExemplarDeserializer {
                type_registry: self.type_registry,
                load_context: &mut lc,
                formulas: self.formulas,
                exemplar_name: &key,
                scripted: self.scripted,
            })?;
            if let Some(label) = sdata
                .extends
                .as_ref()
                .and_then(|h| h.path())
                .filter(|p| p.without_label() == *self.load_context.path())
                .and_then(|p| p.label())
            {
                local_extends.push((key.clone(), label.to_owned()));
            }
            let aliases = sdata.alias.clone();
            let exemplar = Arc::new(sdata);
            loaded.push((key.clone(), exemplar.clone()));
            let handle = lc.finish(Exemplar(exemplar.clone()));
            let handle = self
                .load_context
                .add_loaded_labeled_asset(key.clone(), handle);
            // The catalog holds every handle, so labels stay loaded while it is.
            for alias in aliases {
                let alias_handle = self
                    .load_context
                    .add_labeled_asset(alias.clone(), Exemplar(exemplar.clone()));
                entries.insert(alias, alias_handle);
            }

            entries.insert(key, handle);
        }

        // An `extends` naming a label this catalog lacks never loads; the chain stops there.
        for (key, label) in local_extends {
            if !entries.contains_key(&label) {
                warn!(
                    "Exemplar {}#{key} extends #{label}, which is not in the catalog",
                    self.load_context.path()
                );
            }
        }

        Ok(ExemplarCatalog { entries, loaded })
    }
}

struct CatalogDeserializer<'a, 'b> {
    type_registry: &'a TypeRegistry,
    load_context: &'a mut LoadContext<'b>,
    formulas: &'a mut FormulaQueue,
    scripted: &'a [Arc<AspectDecl>],
}

impl<'de> DeserializeSeed<'de> for CatalogDeserializer<'_, '_> {
    type Value = ExemplarCatalog;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_map(CatalogVisitor {
            type_registry: self.type_registry,
            load_context: self.load_context,
            formulas: self.formulas,
            scripted: self.scripted,
        })
    }
}

/// The module path a scripted aspect's key names: `scripts::toggle_switch::ToggleSwitch` is
/// declared in `scripts/toggle_switch.saga`.
fn module_path(key: &str) -> Option<String> {
    let (module, _) = key.rsplit_once("::")?;
    Some(format!("{}.saga", module.replace("::", "/")))
}

/// The aspects declared by the modules that a catalog's scripted aspects name, which are loaded
/// as dependencies of the catalog's loader, so that editing one reloads the catalog. A module that
/// fails to load is logged, and its aspects are skipped as the catalog loads.
async fn load_scripted_aspects(
    bytes: &[u8],
    load_context: &mut LoadContext<'_>,
) -> Vec<Arc<AspectDecl>> {
    // A file that does not parse fails in the deserializer, which says where.
    let Ok(serde_json::Value::Object(catalog)) = serde_json::from_slice(bytes) else {
        return Vec::new();
    };
    let mut modules: Vec<String> = catalog
        .values()
        .filter_map(|exemplar| exemplar.get("aspects")?.as_object())
        .flat_map(|aspects| aspects.keys())
        .filter_map(|key| module_path(key))
        .collect();
    modules.sort();
    modules.dedup();
    let mut decls = Vec::new();
    for module in modules {
        match load_context
            .load_builder()
            .load_value::<ScriptModuleAsset>(module.clone())
            .await
        {
            Ok(loaded) => {
                for decl in loaded.get().module.aspects() {
                    if decl.kinds.iter().any(|kind| kind.len() != 4) {
                        warn!(
                            "{module}: {} names a kind that is not a four-letter type",
                            decl.name
                        );
                    }
                    decls.push(decl.clone());
                }
            }
            Err(err) => warn!("{}: cannot load {module}: {err}", load_context.path()),
        }
    }
    decls
}

/// AssetLoader for Exemplars.
#[derive(TypePath)]
pub struct ExemplarLoader {
    type_registry: TypeRegistryArc,
    host: SagaHost,
}

#[non_exhaustive]
#[derive(Debug, Error)]
pub enum ExemplarLoaderError {
    #[error("Could not load exemplar: {0}")]
    Io(#[from] std::io::Error),
    #[error("Could not decode exemplar: {0}")]
    Decode(#[from] serde_json::Error),
}

impl FromWorld for ExemplarLoader {
    fn from_world(world: &mut World) -> Self {
        ExemplarLoader {
            type_registry: world.resource::<AppTypeRegistry>().0.clone(),
            host: world.resource::<SagaHost>().clone(),
        }
    }
}

impl AssetLoader for ExemplarLoader {
    type Asset = ExemplarCatalog;
    type Error = ExemplarLoaderError;
    type Settings = ();

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &Self::Settings,
        load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        let scripted = load_scripted_aspects(&bytes, load_context).await;
        let mut formulas = FormulaQueue::default();
        let catalog = {
            let mut deserializer =
                serde_json::Deserializer::from_reader(formulas.track_json(&bytes));
            CatalogDeserializer {
                type_registry: &self.type_registry.read(),
                load_context: &mut *load_context,
                formulas: &mut formulas,
                scripted: &scripted,
            }
            .deserialize(&mut deserializer)?
        };
        let path = load_context.path().to_string();
        formulas
            .compile(&path, self.host.get(), &bytes, &scripted)
            .await;
        Ok(catalog)
    }

    fn extensions(&self) -> &[&str] {
        &["json"]
    }
}
