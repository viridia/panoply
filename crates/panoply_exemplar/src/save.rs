//! Writing an edited catalog back to its file. The exemplars in memory are not the whole file: the
//! loader skips aspect types no plugin registers, serde ignores fields no type has, and the file
//! leaves out fields at their defaults and names `extends` relative to itself. So a save does not
//! write the exemplars out; it writes each edit into the file's own JSON. Both the exemplar as
//! loaded and as it is now are serialized, and only where they differ does the file change
//! ([`merge`]). What the exemplars never held is never in either, and stays as the file has it.

use std::sync::Arc;

use bevy::{
    prelude::*,
    reflect::{serde::TypedReflectSerializer, TypeRegistry},
};
use serde_json::{Map, Value};

use crate::{exemplar::ExemplarData, Exemplar, ExemplarCatalog};

/// `exemplar` as a catalog entry, with each field the file would omit left out.
fn exemplar_json(
    exemplar: &ExemplarData,
    registry: &TypeRegistry,
    server: &AssetServer,
) -> Result<Value, BevyError> {
    let mut entry = Map::new();
    entry.insert("type".into(), serde_json::to_value(exemplar.meta_type)?);
    if let Some(name) = &exemplar.display_name {
        entry.insert("display_name".into(), name.clone().into());
    }
    if !exemplar.alias.is_empty() {
        entry.insert("alias".into(), serde_json::to_value(&exemplar.alias)?);
    }
    if exemplar.is_abstract {
        entry.insert("abstract".into(), true.into());
    }
    if let Some(extends) = &exemplar.extends {
        let path = server
            .get_path(extends)
            .ok_or("the exemplar it extends has no path")?;
        entry.insert("extends".into(), path.to_string().into());
    }
    let mut aspects = Map::new();
    for aspect in &exemplar.aspects {
        let serializer = TypedReflectSerializer::new(aspect.as_partial_reflect(), registry);
        // Through text: a `Value` holds an `f32` widened to `f64`, which writes 1.2 as
        // 1.2000000476837158, where text writes an `f32` as its shortest decimal.
        let text = serde_json::to_string(&serializer)?;
        aspects.insert(
            aspect.reflect_short_type_path().to_string(),
            serde_json::from_str(&text)?,
        );
    }
    entry.insert("aspects".into(), aspects.into());
    Ok(entry.into())
}

/// Writes into `original` the changes from `base` to `new`. An object's key that `new` dropped is
/// removed, one it added is inserted, and one whose value changed is merged in turn, so the keys
/// `original` holds that neither knows are kept. Arrays of one length merge item by item, for the
/// same reason; anything else that changed is replaced.
pub fn merge(original: &mut Value, base: &Value, new: &Value) {
    if base == new {
        return;
    }
    match (original, base, new) {
        (Value::Object(original), Value::Object(base), Value::Object(new)) => {
            for key in base.keys() {
                if !new.contains_key(key) {
                    original.shift_remove(key);
                }
            }
            for (key, value) in new {
                match (base.get(key), original.get_mut(key)) {
                    (Some(was), _) if was == value => {}
                    (Some(was), Some(written)) => merge(written, was, value),
                    _ => {
                        original.insert(key.clone(), value.clone());
                    }
                }
            }
        }
        (Value::Array(original), Value::Array(base), Value::Array(new))
            if original.len() == base.len() && base.len() == new.len() =>
        {
            for ((written, was), value) in original.iter_mut().zip(base).zip(new) {
                merge(written, was, value);
            }
        }
        (original, _, new) => *original = new.clone(),
    }
}

impl ExemplarCatalog {
    /// Writes the catalog's edits into `file`, its file's JSON. An exemplar whose data is still as
    /// loaded is not looked at.
    pub fn merge_into(
        &self,
        file: &mut Value,
        exemplars: &Assets<Exemplar>,
        registry: &TypeRegistry,
        server: &AssetServer,
    ) -> Result<(), BevyError> {
        let entries = file.as_object_mut().ok_or("the catalog is not a JSON object")?;
        for (name, loaded) in &self.loaded {
            let Some(now) = self
                .entries
                .get(name)
                .and_then(|handle| exemplars.get(handle))
            else {
                continue;
            };
            if Arc::ptr_eq(&now.0, loaded) {
                continue;
            }
            let base = exemplar_json(loaded, registry, server)?;
            let new = exemplar_json(&now.0, registry, server)?;
            match entries.get_mut(name) {
                Some(original) => merge(original, &base, &new),
                None => {
                    entries.insert(name.clone(), new);
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// `original` after merging the change from `base` to `new`.
    fn merged(mut original: Value, base: Value, new: Value) -> Value {
        merge(&mut original, &base, &new);
        original
    }

    #[test]
    fn nothing_changed_leaves_the_file_as_it_is() {
        let original = json!({ "b": 1, "a": [1, 2], "unknown": true });
        let base = json!({ "a": [1, 2], "b": 1, "c": null });
        assert_eq!(merged(original.clone(), base.clone(), base), original);
    }

    #[test]
    fn an_edit_changes_only_its_key_and_keeps_unknown_ones() {
        let original = json!({ "shape": "box", "size": [1, 2, 3], "margin": 0.2 });
        let base = json!({ "shape": "box", "size": [1, 2, 3], "offset": null });
        let new = json!({ "shape": "box", "size": [1, 5, 3], "offset": null });
        assert_eq!(
            merged(original, base, new),
            json!({ "shape": "box", "size": [1, 5, 3], "margin": 0.2 })
        );
    }

    #[test]
    fn an_edited_field_the_file_left_out_is_written() {
        let original = json!({ "shape": "box" });
        let base = json!({ "shape": "box", "offset": null });
        let new = json!({ "shape": "box", "offset": [0, 1, 0] });
        assert_eq!(
            merged(original, base, new),
            json!({ "shape": "box", "offset": [0, 1, 0] })
        );
    }

    #[test]
    fn a_removed_aspect_leaves_the_file_and_an_unknown_one_stays() {
        let original = json!({ "Portal": { "to": "x" }, "Lock": {}, "Models": [] });
        let base = json!({ "Lock": {}, "Models": [] });
        let new = json!({ "Models": [] });
        assert_eq!(
            merged(original, base, new),
            json!({ "Portal": { "to": "x" }, "Models": [] })
        );
    }

    #[test]
    fn array_items_keep_their_unknown_keys() {
        let original = json!([{ "size": 1, "margin": 0.2 }, { "size": 2 }]);
        let base = json!([{ "size": 1 }, { "size": 2 }]);
        let new = json!([{ "size": 3 }, { "size": 2 }]);
        assert_eq!(
            merged(original, base, new),
            json!([{ "size": 3, "margin": 0.2 }, { "size": 2 }])
        );
    }

    #[test]
    fn a_resized_array_is_replaced() {
        let original = json!({ "use": [[0, 1, 0]] });
        let base = json!({ "use": [[0, 1, 0]] });
        let new = json!({ "use": [[0, 1, 0], [1, 1, 0]] });
        assert_eq!(merged(original, base, new.clone()), new);
    }

    #[test]
    fn key_order_is_kept() {
        let original = json!({ "z": 1, "a": 2, "m": 3 });
        let base = json!({ "a": 2, "m": 3, "z": 1 });
        let new = json!({ "a": 5, "m": 3, "z": 1 });
        let keys: Vec<String> = merged(original, base, new)
            .as_object()
            .unwrap()
            .keys()
            .cloned()
            .collect();
        assert_eq!(keys, ["z", "a", "m"]);
    }
}
