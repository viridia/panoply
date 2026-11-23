use bevy::{asset::LoadContext, log::warn, reflect::TypeRegistry};
use panoply_saga::{scripted::AspectDecl, FormulaQueue};
use serde::{
    de::{self, DeserializeSeed, Visitor},
    Deserializer,
};
use std::{fmt, sync::Arc};

/// A list of aspects.
pub type AspectList = Vec<Arc<dyn Aspect>>;

use super::{aspect::Aspect, AspectDeserializer, ScriptedAspect};

struct AspectListVisitor<'a, 'b> {
    type_registry: &'a TypeRegistry,
    load_context: &'a mut LoadContext<'b>,
    formulas: &'a mut FormulaQueue,
    label_prefix: &'a str,
    scripted: &'a [Arc<AspectDecl>],
}

impl<'de> Visitor<'de> for AspectListVisitor<'_, '_> {
    type Value = AspectList;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("an aspect map")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::MapAccess<'de>,
    {
        let mut result: AspectList = Vec::with_capacity(map.size_hint().unwrap_or(0));
        while let Some(key) = map.next_key::<String>()? {
            // A scripted aspect's key names its module, which `::` sets apart from a type name.
            if key.contains("::") {
                let value: serde_json::Value = map.next_value()?;
                let path = self.load_context.path().to_string();
                let Some(decl) = self.scripted.iter().find(|decl| *decl.key == *key) else {
                    warn!("Skipping scripted aspect {key} in {path}: its module did not load");
                    continue;
                };
                match ScriptedAspect::from_json(decl, &value, self.type_registry, &path) {
                    Ok(aspect) => result.push(Arc::new(aspect)),
                    Err(err) => warn!("Skipping scripted aspect {key}: {err}"),
                }
                continue;
            }
            // An aspect type no plugin has registered is skipped rather than failing the whole
            // asset, so modules can be restored one at a time. Saving the asset drops it.
            let Some(type_registration) = self.type_registry.get_with_short_type_path(&key) else {
                warn!(
                    "Skipping unknown aspect type {key} in {}",
                    self.load_context.path()
                );
                map.next_value::<de::IgnoredAny>()?;
                continue;
            };
            let mut aspect = map.next_value_seed(AspectDeserializer {
                type_registration,
                type_registry: self.type_registry,
                formulas: self.formulas,
            })?;
            aspect.load_dependencies(self.label_prefix, self.load_context);
            result.push(Arc::from(aspect));
        }
        Ok(result)
    }
}

/// A deserializer for a vector of boxed aspects.
pub struct AspectListDeserializer<'a, 'b> {
    /// Reference to the type registry.
    pub type_registry: &'a TypeRegistry,

    /// Reference to the asset loader context.
    pub load_context: &'a mut LoadContext<'b>,

    /// Collects the aspects' formulas, for the loader to compile.
    pub formulas: &'a mut FormulaQueue,

    /// Prefix for created materials
    pub label_prefix: &'a str,

    /// The scripted aspects the list may hold, from the modules its file names. One it holds
    /// that is not among them is skipped.
    pub scripted: &'a [Arc<AspectDecl>],
}

impl<'de> DeserializeSeed<'de> for AspectListDeserializer<'_, '_> {
    type Value = AspectList;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_map(AspectListVisitor {
            type_registry: self.type_registry,
            load_context: self.load_context,
            formulas: self.formulas,
            label_prefix: self.label_prefix,
            scripted: self.scripted,
        })
    }
}
