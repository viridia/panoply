//! Every precinct file, loaded and saved, holds what it held, apart from what saving is meant to
//! drop or change: faery's `layers`, aspect types no plugin registers (D25), and fields at their
//! defaults or null. Keys and labels come back as they were. Floats compare at `f32`, which is what the
//! game reads them as.

use std::path::{Path, PathBuf};

use bevy::{asset::LoadState, prelude::*, reflect::TypeRegistry};
use panoply_precinct::{PrecinctAsset, PrecinctAssetLoader};
use serde_json::{Map, Value};

/// The precinct files under `dir`, relative to `assets`.
fn precinct_files(assets: &Path, dir: &Path, found: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            precinct_files(assets, &path, found);
        } else if path.extension().is_some_and(|ext| ext == "msgpack") {
            found.push(path.strip_prefix(assets).unwrap().to_path_buf());
        }
    }
}

/// A msgpack value read as JSON, its byte strings (instance keys) as arrays of numbers, since
/// `serde_json::Value` has no bytes of its own.
struct Packed(Value);

impl<'de> serde::Deserialize<'de> for Packed {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(PackedVisitor).map(Packed)
    }
}

struct PackedVisitor;

impl<'de> serde::de::Visitor<'de> for PackedVisitor {
    type Value = Value;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("any msgpack value")
    }

    fn visit_bool<E>(self, v: bool) -> Result<Value, E> {
        Ok(Value::Bool(v))
    }

    fn visit_i64<E>(self, v: i64) -> Result<Value, E> {
        Ok(Value::from(v))
    }

    fn visit_u64<E>(self, v: u64) -> Result<Value, E> {
        Ok(Value::from(v))
    }

    fn visit_f64<E>(self, v: f64) -> Result<Value, E> {
        Ok(Value::from(v))
    }

    fn visit_str<E>(self, v: &str) -> Result<Value, E> {
        Ok(Value::from(v))
    }

    fn visit_bytes<E>(self, v: &[u8]) -> Result<Value, E> {
        Ok(Value::Array(v.iter().map(|&b| Value::from(b)).collect()))
    }

    fn visit_unit<E>(self) -> Result<Value, E> {
        Ok(Value::Null)
    }

    fn visit_none<E>(self) -> Result<Value, E> {
        Ok(Value::Null)
    }

    fn visit_seq<A: serde::de::SeqAccess<'de>>(self, mut seq: A) -> Result<Value, A::Error> {
        let mut items = Vec::new();
        while let Some(Packed(item)) = seq.next_element()? {
            items.push(item);
        }
        Ok(Value::Array(items))
    }

    fn visit_map<A: serde::de::MapAccess<'de>>(self, mut map: A) -> Result<Value, A::Error> {
        let mut fields = Map::new();
        while let Some((key, Packed(value))) = map.next_entry::<String, Packed>()? {
            fields.insert(key, value);
        }
        Ok(Value::Object(fields))
    }
}

/// `value` as the comparison sees it.
fn normalized(value: &Value, registry: &TypeRegistry) -> Value {
    let mut precinct = value.as_object().unwrap().clone();
    precinct.remove("layers");
    precinct.retain(|_, field| !is_default(field));
    // faery's tiers hold bounds, always null, which the loader ignores.
    if let Some(Value::Array(tiers)) = precinct.get_mut("tiers") {
        for tier in tiers.iter_mut() {
            tier.as_object_mut()
                .unwrap()
                .retain(|_, field| !is_default(field));
        }
    }
    if let Some(Value::Array(scenery)) = precinct.get_mut("scenery") {
        for instance in scenery.iter_mut() {
            let tuple = instance.as_array_mut().unwrap();
            if let Some(Value::Object(aspects)) = tuple.get_mut(4) {
                registered_only(aspects, registry);
                if aspects.is_empty() {
                    tuple.truncate(4);
                }
            }
        }
    }
    if let Some(Value::Array(actors)) = precinct.get_mut("actors") {
        for actor in actors.iter_mut() {
            let actor = actor.as_object_mut().unwrap();
            if actor.get("facing").and_then(Value::as_f64) == Some(0.0) {
                actor.remove("facing");
            }
            if let Some(Value::Object(aspects)) = actor.get_mut("aspects") {
                registered_only(aspects, registry);
                if aspects.is_empty() {
                    actor.remove("aspects");
                }
            }
        }
    }
    floats_as_f32(&Value::Object(precinct))
}

/// Whether a precinct's field holds nothing the loader would not default to.
fn is_default(field: &Value) -> bool {
    match field {
        Value::Null => true,
        Value::Array(items) => items.is_empty(),
        _ => false,
    }
}

fn registered_only(aspects: &mut Map<String, Value>, registry: &TypeRegistry) {
    aspects.retain(|name, _| registry.get_with_short_type_path(name).is_some());
}

/// Every number as the `f32` nearest it, so an integer and a float of the same value compare
/// equal, and a float written as `f64` compares with the same value read and written as `f32`.
fn floats_as_f32(value: &Value) -> Value {
    match value {
        Value::Number(number) => Value::from(number.as_f64().unwrap() as f32 as f64),
        Value::Array(items) => Value::Array(items.iter().map(floats_as_f32).collect()),
        Value::Object(fields) => Value::Object(
            fields
                .iter()
                .map(|(key, value)| (key.clone(), floats_as_f32(value)))
                .collect(),
        ),
        other => other.clone(),
    }
}

/// The first place `a` and `b` differ, as a path into them.
fn first_difference(a: &Value, b: &Value, path: String) -> Option<String> {
    match (a, b) {
        (Value::Object(a), Value::Object(b)) => {
            for key in a.keys().chain(b.keys()) {
                match (a.get(key), b.get(key)) {
                    (Some(x), Some(y)) => {
                        if let Some(found) = first_difference(x, y, format!("{path}.{key}")) {
                            return Some(found);
                        }
                    }
                    (x, y) => return Some(format!("{path}.{key}: {x:?} vs {y:?}")),
                }
            }
            None
        }
        (Value::Array(a), Value::Array(b)) if a.len() == b.len() => a
            .iter()
            .zip(b)
            .enumerate()
            .find_map(|(i, (x, y))| first_difference(x, y, format!("{path}[{i}]"))),
        _ if a == b => None,
        _ => Some(format!("{path}: {a} vs {b}")),
    }
}

#[test]
fn every_precinct_saves_as_it_loads() {
    let assets = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets");
    let mut files = Vec::new();
    precinct_files(&assets, &assets.join("scenery/precincts"), &mut files);
    assert!(
        !files.is_empty(),
        "no precinct files under {}",
        assets.display()
    );

    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin {
            file_path: "../../assets".into(),
            ..default()
        },
        panoply_saga::SagaPlugin,
    ))
    .init_asset::<PrecinctAsset>()
    .init_asset_loader::<PrecinctAssetLoader>()
    // The instance aspect types the game registers. `Group` and `Combatant` are faery's, which no
    // plugin registers yet.
    .register_type::<panoply_portals::PortalTarget>();
    app.finish();
    app.cleanup();

    let handles: Vec<Handle<PrecinctAsset>> = {
        let server = app.world().resource::<AssetServer>();
        files.iter().map(|file| server.load(file.clone())).collect()
    };
    for _ in 0..10_000 {
        app.update();
        let server = app.world().resource::<AssetServer>();
        if handles.iter().all(|handle| {
            matches!(
                server.load_state(handle),
                LoadState::Loaded | LoadState::Failed(_)
            )
        }) {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(1));
    }

    let registry = app.world().resource::<AppTypeRegistry>().read();
    let precincts = app.world().resource::<Assets<PrecinctAsset>>();
    let mut failures = Vec::new();
    for (file, handle) in files.iter().zip(&handles) {
        let Some(precinct) = precincts.get(handle) else {
            failures.push(format!("{}: did not load", file.display()));
            continue;
        };
        let Packed(original) = rmp_serde::from_slice(&std::fs::read(assets.join(file)).unwrap())
            .expect("the file decodes");
        let Packed(saved) = rmp_serde::from_slice(&precinct.to_bytes(&registry).unwrap())
            .expect("the saved file decodes");
        let (original, saved) = (
            normalized(&original, &registry),
            normalized(&saved, &registry),
        );
        if let Some(difference) = first_difference(&original, &saved, String::new()) {
            failures.push(format!("{}: {difference}", file.display()));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
