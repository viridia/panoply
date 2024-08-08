use bevy::{
    asset::{io::Reader, AssetLoader, LoadContext},
    prelude::*,
    reflect::{TypeRegistry, TypeRegistryArc},
    utils::hashbrown::HashMap,
};
use futures_lite::AsyncReadExt;
use serde::{
    de::{self, DeserializeSeed, Visitor},
    Deserialize, Deserializer,
};
use std::{fmt, sync::Arc};
use thiserror::Error;

use crate::exemplar::ExemplarData;

use super::aspect_list::AspectListDeserializer;
use super::{Exemplar, ExemplarCatalog, InstanceType};

#[derive(Deserialize)]
#[serde(field_identifier, rename_all = "snake_case")]
enum Field {
    Type,
    DisplayName,
    Alias,
    Aspects,
    Extends,
}

struct ExemplarVisitor<'a, 'b> {
    type_registry: &'a TypeRegistry,
    load_context: &'a mut LoadContext<'b>,
    exemplar_name: &'a str,
}

impl<'de, 'a, 'b> Visitor<'de> for ExemplarVisitor<'a, 'b> {
    type Value = ExemplarData;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("an exemplar")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::MapAccess<'de>,
    {
        let mut result = ExemplarData {
            meta_type: InstanceType::NONE,
            display_name: None,
            alias: Vec::new(),
            aspects: Vec::new(),
            extends: None,
        };
        while let Some(key) = map.next_key()? {
            match key {
                Field::Type => {
                    if result.meta_type != InstanceType::NONE {
                        return Err(de::Error::duplicate_field("type"));
                    }
                    // TODO: Implement type deserialization.
                    let _meta_type: String = map.next_value()?;
                    // println!("meta_type: {}", meta_type);
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
                    result.aspects = map.next_value_seed(AspectListDeserializer {
                        type_registry: self.type_registry,
                        load_context: self.load_context,
                        label_prefix: self.exemplar_name,
                    })?;
                }
                Field::Extends => {
                    if result.extends.is_some() {
                        return Err(de::Error::duplicate_field("extends"));
                    }
                    let extends = map.next_value::<String>()?;
                    let extends_path = self
                        .load_context
                        .asset_path()
                        .resolve_embed(&extends)
                        .unwrap();
                    result.extends = Some(self.load_context.load::<Exemplar>(extends_path));
                }
            }
        }
        Ok(result)
    }
}

struct ExemplarDeserializer<'a, 'b> {
    type_registry: &'a TypeRegistry,
    load_context: &'a mut LoadContext<'b>,
    exemplar_name: &'a str,
}

impl<'de, 'a, 'b> DeserializeSeed<'de> for ExemplarDeserializer<'a, 'b> {
    type Value = ExemplarData;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_map(ExemplarVisitor {
            type_registry: self.type_registry,
            load_context: self.load_context,
            exemplar_name: self.exemplar_name,
        })
    }
}

struct CatalogVisitor<'a, 'b> {
    type_registry: &'a TypeRegistry,
    load_context: &'a mut LoadContext<'b>,
}

impl<'de, 'a, 'b> Visitor<'de> for CatalogVisitor<'a, 'b> {
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
        while let Some(key) = map.next_key::<String>()? {
            let mut lc = self.load_context.begin_labeled_asset();
            let sdata = map.next_value_seed(ExemplarDeserializer {
                type_registry: self.type_registry,
                load_context: &mut lc,
                exemplar_name: &key,
            })?;
            let aliases = sdata.alias.clone();
            let exemplar = Arc::new(sdata);
            let handle = lc.finish(Exemplar(exemplar.clone()), None);
            let handle = self
                .load_context
                .add_loaded_labeled_asset(key.clone(), handle);
            for alias in &aliases {
                self.load_context
                    .add_labeled_asset(alias.clone(), Exemplar(exemplar.clone()));
            }

            entries.insert(key, handle);
        }

        Ok(ExemplarCatalog { entries })
    }
}

struct CatalogDeserializer<'a, 'b> {
    type_registry: &'a TypeRegistry,
    load_context: &'a mut LoadContext<'b>,
}

impl<'de, 'a, 'b> DeserializeSeed<'de> for CatalogDeserializer<'a, 'b> {
    type Value = ExemplarCatalog;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_map(CatalogVisitor {
            type_registry: self.type_registry,
            load_context: self.load_context,
        })
    }
}

/// AssetLoader for Exemplars.
pub struct ExemplarLoader {
    type_registry: TypeRegistryArc,
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
        }
    }
}

impl AssetLoader for ExemplarLoader {
    type Asset = ExemplarCatalog;
    type Error = ExemplarLoaderError;
    type Settings = ();

    async fn load<'a>(
        &'a self,
        reader: &'a mut Reader<'_>,
        _settings: &'a Self::Settings,
        load_context: &'a mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        let mut deserializer = serde_json::Deserializer::from_slice(&bytes);
        let exemplar_deserializer = CatalogDeserializer {
            type_registry: &self.type_registry.read(),
            load_context,
        };
        let catalog: ExemplarCatalog = exemplar_deserializer.deserialize(&mut deserializer)?;
        Ok(catalog)
    }

    fn extensions(&self) -> &[&str] {
        &["json"]
    }
}
