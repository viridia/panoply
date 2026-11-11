//! Formulas embedded in a JSON asset, compiled through a `FormulaQueue`.

use bevy::reflect::{
    FromReflect, GetTypeRegistration, Reflect, TypeRegistry, serde::TypedReflectDeserializer,
};
use panoply_saga::{Formula, FormulaQueue, HostState};
use serde::de::DeserializeSeed;

#[derive(Reflect, Debug)]
struct Entry {
    id: String,
    when: Option<Formula<bool>>,
}

fn load(json: &str) -> Vec<Entry> {
    let mut registry = TypeRegistry::new();
    registry.register::<Vec<Entry>>();
    let registration = registry
        .get(Vec::<Entry>::get_type_registration().type_id())
        .unwrap();
    let mut formulas = FormulaQueue::default();
    let value = {
        let reader = formulas.track_json(json.as_bytes());
        let mut deserializer = serde_json::Deserializer::from_reader(reader);
        TypedReflectDeserializer::with_processor(registration, &registry, &mut formulas)
            .deserialize(&mut deserializer)
            .unwrap()
    };
    futures_lite::future::block_on(formulas.compile(
        "test.json",
        &HostState::new(),
        json.as_bytes(),
    ));
    Vec::<Entry>::from_reflect(&*value).unwrap()
}

#[test]
fn formulas_compile_as_their_asset_loads() {
    let entries = load(
        r#"[
  {"id": "good", "when": "1 < 2"},
  {"id": "none", "when": null}
]"#,
    );
    let good = entries[0].when.as_ref().unwrap();
    assert_eq!(good.source(), "1 < 2");
    assert!(good.module().is_some());
    assert!(good.error().is_none());
    assert!(entries[1].when.is_none());
}

#[test]
fn a_compile_error_names_its_line_and_column_in_the_file() {
    let entries = load(
        r#"[
  {"id": "good", "when": "1 < 2"},
  {"id": "bad", "when": "true &&\tmissing"}
]"#,
    );
    assert!(entries[0].when.as_ref().unwrap().module().is_some());
    let bad = entries[1].when.as_ref().unwrap();
    assert!(bad.module().is_none());
    let error = bad.error().unwrap();
    // `missing` follows a `\t` escape, two bytes in the file and one in the formula.
    assert!(
        error.starts_with("test.json:3:35 Can't find the name 'missing'"),
        "{error}"
    );
    assert!(
        error.ends_with(&format!("\n{}^^^^^^^", " ".repeat(34))),
        "{error}"
    );
}
