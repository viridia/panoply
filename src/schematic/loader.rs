use bevy::{
    asset::{io::Reader, AssetLoader, LoadContext},
    prelude::*,
    reflect::{TypeRegistry, TypeRegistryArc},
    utils::{hashbrown::HashMap, thiserror::Error, BoxedFuture},
};
use futures_lite::AsyncReadExt;
use serde::{
    de::{self, DeserializeSeed, Visitor},
    Deserialize, Deserializer,
};
use std::{fmt, sync::Arc};

use super::aspect_list::AspectListDeserializer;
use super::{InstanceType, Schematic, SchematicCatalog, SchematicData};

#[derive(Deserialize)]
#[serde(field_identifier, rename_all = "snake_case")]
enum Field {
    Type,
    DisplayName,
    Alias,
    Aspects,
    Extends,
}

struct SchematicVisitor<'a, 'b> {
    type_registry: &'a TypeRegistry,
    load_context: &'a mut LoadContext<'b>,
    schematic_name: &'a str,
}

impl<'de, 'a, 'b> Visitor<'de> for SchematicVisitor<'a, 'b> {
    type Value = SchematicData;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a schematic")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::MapAccess<'de>,
    {
        let mut result = SchematicData {
            meta_type: InstanceType::None,
            display_name: None,
            alias: Vec::new(),
            aspects: Vec::new(),
            extends: None,
        };
        while let Some(key) = map.next_key()? {
            match key {
                Field::Type => {
                    if result.meta_type != InstanceType::None {
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
                        parent_label: self.schematic_name,
                    })?;
                }
                Field::Extends => todo!(),
            }
        }
        Ok(result)
    }
}

struct SchematicDeserializer<'a, 'b> {
    type_registry: &'a TypeRegistry,
    load_context: &'a mut LoadContext<'b>,
    schematic_name: &'a str,
}

impl<'de, 'a, 'b> DeserializeSeed<'de> for SchematicDeserializer<'a, 'b> {
    type Value = SchematicData;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_map(SchematicVisitor {
            type_registry: self.type_registry,
            load_context: self.load_context,
            schematic_name: self.schematic_name,
        })
    }
}

struct CatalogVisitor<'a, 'b> {
    type_registry: &'a TypeRegistry,
    load_context: &'a mut LoadContext<'b>,
}

impl<'de, 'a, 'b> Visitor<'de> for CatalogVisitor<'a, 'b> {
    type Value = SchematicCatalog;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("schematic catalog")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: de::MapAccess<'de>,
    {
        let mut entries: HashMap<String, Handle<Schematic>> =
            HashMap::with_capacity(map.size_hint().unwrap_or(0));
        while let Some(key) = map.next_key::<String>()? {
            let mut lc = self.load_context.begin_labeled_asset();
            let sdata = map.next_value_seed(SchematicDeserializer {
                type_registry: self.type_registry,
                load_context: &mut lc,
                schematic_name: &key,
            })?;
            let aliases = sdata.alias.clone();
            let schematic = Arc::new(sdata);
            let handle = lc.finish(Schematic(schematic.clone()), None);
            let handle = self
                .load_context
                .add_loaded_labeled_asset(key.clone(), handle);
            for alias in &aliases {
                self.load_context
                    .add_labeled_asset(alias.clone(), Schematic(schematic.clone()));
            }

            entries.insert(key, handle);
        }

        Ok(SchematicCatalog { entries })
    }
}

struct CatalogDeserializer<'a, 'b> {
    type_registry: &'a TypeRegistry,
    load_context: &'a mut LoadContext<'b>,
}

impl<'de, 'a, 'b> DeserializeSeed<'de> for CatalogDeserializer<'a, 'b> {
    type Value = SchematicCatalog;

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

/// AssetLoader for Schematics.
pub struct SchematicLoader {
    type_registry: TypeRegistryArc,
}

#[non_exhaustive]
#[derive(Debug, Error)]
pub enum SchematicLoaderError {
    #[error("Could not load schematic: {0}")]
    Io(#[from] std::io::Error),
    #[error("Could not decode schematic: {0}")]
    Decode(#[from] serde_json::Error),
}

impl FromWorld for SchematicLoader {
    fn from_world(world: &mut World) -> Self {
        SchematicLoader {
            type_registry: world.resource::<AppTypeRegistry>().0.clone(),
        }
    }
}

impl AssetLoader for SchematicLoader {
    type Asset = SchematicCatalog;
    type Error = SchematicLoaderError;
    type Settings = ();

    fn load<'a>(
        &'a self,
        reader: &'a mut Reader,
        _settings: &'a Self::Settings,
        load_context: &'a mut LoadContext,
    ) -> BoxedFuture<'a, Result<Self::Asset, Self::Error>> {
        Box::pin(async move {
            let mut bytes = Vec::new();
            reader.read_to_end(&mut bytes).await?;
            let mut deserializer = serde_json::Deserializer::from_slice(&bytes);
            let schematic_deserializer = CatalogDeserializer {
                type_registry: &self.type_registry.read(),
                load_context,
            };
            let catalog: SchematicCatalog =
                schematic_deserializer.deserialize(&mut deserializer)?;
            Ok(catalog)
        })
    }

    fn extensions(&self) -> &[&str] {
        &["json"]
    }
}
