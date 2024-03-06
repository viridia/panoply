use bevy::{
    asset::{io::Reader, AssetLoader, LoadContext},
    prelude::*,
    reflect::{serde::TypedReflectDeserializer, TypeRegistration, TypeRegistry, TypeRegistryArc},
    utils::{hashbrown::HashMap, thiserror::Error, BoxedFuture},
};
use futures_lite::AsyncReadExt;
use serde::{
    de::{self, DeserializeSeed, Visitor},
    Deserialize, Deserializer,
};
use std::{fmt, sync::Arc};

use crate::schematic::aspect::ReflectAspect;

use super::aspect::Aspect;
use super::{InstanceType, Schematic, SchematicCatalog, SchematicData};

struct AspectDeserializer<'a> {
    type_registration: &'a TypeRegistration,
    type_registry: &'a TypeRegistry,
}

impl<'a, 'de> DeserializeSeed<'de> for AspectDeserializer<'a> {
    type Value = Box<dyn Aspect>;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        let reflect_deserializer =
            TypedReflectDeserializer::new(self.type_registration, self.type_registry);
        let deserialized_value: Box<dyn Reflect> =
            reflect_deserializer.deserialize(deserializer).unwrap();
        let rd = self.type_registration.data::<ReflectDefault>().unwrap();
        let mut value = rd.default();
        value.apply(&*deserialized_value);
        let reflect_aspect = self
            .type_registry
            .get_type_data::<ReflectAspect>(self.type_registration.type_id())
            .unwrap();
        let aspect = reflect_aspect.get_boxed(value).unwrap();
        Ok(aspect)
    }
}

struct AspectMapVisitor<'a, 'b> {
    type_registry: &'a TypeRegistry,
    load_context: &'a mut LoadContext<'b>,
    schematic_name: &'a str,
}

impl<'de, 'a, 'b> Visitor<'de> for AspectMapVisitor<'a, 'b> {
    type Value = Vec<Box<dyn Aspect>>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("an aspect map")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::MapAccess<'de>,
    {
        let mut result: Vec<Box<dyn Aspect>> = Vec::with_capacity(map.size_hint().unwrap_or(0));
        while let Some(key) = map.next_key::<String>()? {
            let type_registration = self
                .type_registry
                .get_with_short_type_path(&key)
                .ok_or_else(|| de::Error::custom(format!("Unknown aspect type: {}", key)))?;
            let mut aspect = map.next_value_seed(AspectDeserializer {
                type_registration,
                type_registry: self.type_registry,
            })?;
            aspect.load_dependencies(self.schematic_name, self.load_context);
            result.push(aspect);
        }
        Ok(result)
    }
}

struct AspectListDeserializer<'a, 'b> {
    type_registry: &'a TypeRegistry,
    load_context: &'a mut LoadContext<'b>,
    schematic_name: &'a str,
}

impl<'de, 'a, 'b> DeserializeSeed<'de> for AspectListDeserializer<'a, 'b> {
    type Value = Vec<Box<dyn Aspect>>;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_map(AspectMapVisitor {
            type_registry: self.type_registry,
            load_context: self.load_context,
            schematic_name: self.schematic_name,
        })
    }
}

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
                    // if secs.is_some() {
                    //     return Err(de::Error::duplicate_field("secs"));
                    // }
                    // secs = Some(map.next_value()?);
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
                        schematic_name: self.schematic_name,
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
        let mut entries: HashMap<String, Arc<SchematicData>> =
            HashMap::with_capacity(map.size_hint().unwrap_or(0));
        while let Some(key) = map.next_key::<String>()? {
            let schematic = map.next_value_seed(SchematicDeserializer {
                type_registry: self.type_registry,
                load_context: self.load_context,
                schematic_name: &key,
            })?;
            entries.insert(key, Arc::new(schematic));
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
            println!("Loading schematics: {:#?}", load_context.asset_path());

            let mut deserializer = serde_json::Deserializer::from_slice(&bytes);
            let schematic_deserializer = CatalogDeserializer {
                type_registry: &self.type_registry.read(),
                load_context,
            };
            let mut catalog: SchematicCatalog =
                schematic_deserializer.deserialize(&mut deserializer)?;

            let catalog_handle = load_context.load(load_context.asset_path().clone());
            for (key, schematic) in catalog.entries.iter_mut() {
                load_context.add_labeled_asset(
                    key.clone(),
                    Schematic {
                        key: key.clone(),
                        inner: schematic.clone(),
                        catalog: catalog_handle.clone(),
                    },
                );
                for alias in &schematic.alias {
                    load_context.add_labeled_asset(
                        alias.clone(),
                        Schematic {
                            key: key.clone(),
                            inner: schematic.clone(),
                            catalog: catalog_handle.clone(),
                        },
                    );
                }
            }
            Ok(catalog)
        })
    }

    fn extensions(&self) -> &[&str] {
        &["json"]
    }
}
