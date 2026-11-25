//! Interactions declared in an aspect: their fields, checked against the host's interaction type,
//! and the effects they perform.

use std::sync::Arc;

use bevy::{
    ecs::{entity::Entity, world::World},
    reflect::Reflect,
};
use futures_lite::future;
use panoply_saga::{
    CompilationError, ExprType, FieldKind, HostState, NoImports, ScriptModule, compile_module,
    scripted::{self, AspectDecl, InteractionDecl, ScriptedFields},
};

const SWITCH: &str = r#"
    #[accessor(toggle_switch)]
    aspect ToggleSwitch for Wall, Fixture {
        state on: bool = false,
        state presses: i32 = 0,

        interaction use {
            caption: "Use",
            perform: {
                on = !on;
                presses += 1;
                // A write is buffered: the field still reads as it was.
                if on { presses += 10; }
            },
        }
        interaction reset {
            caption: "Reset",
            when: self.toggle_switch.on || presses > 0,
            distance: 0.5,
            perform: { on = false; presses = 0 },
        }
    }
"#;

/// A host whose interactions have a caption and a perform, and may have a `when` and a distance.
fn host() -> HostState {
    let mut host = HostState::new();
    host.add_interaction_field("caption", FieldKind::Constant(ExprType::String), true);
    host.add_interaction_field("when", FieldKind::Formula(ExprType::Boolean), false);
    host.add_interaction_field("distance", FieldKind::Constant(ExprType::F32), false);
    host.add_interaction_field("perform", FieldKind::Effect, true);
    host
}

fn module(src: &str, host: &HostState) -> Result<Arc<ScriptModule>, CompilationError> {
    future::block_on(compile_module(
        "scripts/toggle_switch.saga",
        src,
        host,
        &NoImports,
    ))
    .map(ScriptModule::share)
}

fn spawn_switch(world: &mut World, decl: &Arc<AspectDecl>, on: bool, presses: i32) -> Entity {
    let state: Vec<Box<dyn Reflect>> = vec![Box::new(on), Box::new(presses)];
    let mut entity = world.spawn_empty();
    scripted::insert(
        &mut entity,
        ScriptedFields::new(decl.clone(), true, state).unwrap(),
    );
    entity.id()
}

fn state(world: &World, entity: Entity, decl: &AspectDecl) -> (bool, i32) {
    let (_, fields) = scripted::get(world, entity, &decl.key, true);
    let fields = fields.unwrap();
    (
        *fields.get("on").unwrap().downcast_ref::<bool>().unwrap(),
        *fields
            .get("presses")
            .unwrap()
            .downcast_ref::<i32>()
            .unwrap(),
    )
}

fn interaction<'a>(decl: &'a AspectDecl, name: &str) -> &'a InteractionDecl {
    decl.interactions.iter().find(|i| i.name == name).unwrap()
}

#[test]
fn an_aspect_declares_its_interactions() {
    let host = host();
    let module = module(SWITCH, &host).unwrap();
    let decl = &module.aspects()[0];
    let names: Vec<_> = decl.interactions.iter().map(|i| i.name.as_str()).collect();
    assert_eq!(names, ["use", "reset"]);
    let reset = interaction(decl, "reset");
    // Constants are values in the declaration; formulas and effects are functions.
    assert_eq!(reset.constant::<String>("caption").unwrap(), "Reset");
    assert_eq!(reset.constant::<f32>("distance"), Some(&0.5));
    let fields: Vec<_> = reset.fields.iter().map(|(name, _)| name.as_str()).collect();
    assert_eq!(fields, ["when", "perform"]);
    assert!(!interaction(decl, "use").sets("when"));
    // The aspect finds its module while the module is shared.
    assert!(Arc::ptr_eq(&decl.module().unwrap(), &module));
}

#[test]
fn formulas_read_the_aspect_and_effects_write_its_state() {
    let host = host();
    let module = module(SWITCH, &host).unwrap();
    let decl = module.aspects()[0].clone();
    let mut world = World::new();
    let switch = spawn_switch(&mut world, &decl, false, 0);

    let field = |interaction_name: &str, field: &str| {
        interaction(&decl, interaction_name).field(field).unwrap()
    };
    let when = |world: &World| -> bool {
        module
            .run_formula(world, &host, switch, field("reset", "when"))
            .unwrap()
    };
    assert!(!when(&world));

    // The effect's writes wait in its buffer, and read the values from before it ran.
    let effects = module
        .run_effect(&world, &host, switch, field("use", "perform"))
        .unwrap();
    assert_eq!(effects.len(), 2);
    assert_eq!(state(&world, switch, &decl), (false, 0));
    effects.apply(&mut world);
    assert_eq!(state(&world, switch, &decl), (true, 1));
    assert!(when(&world));

    // Now `on` reads true, so the third write is made, and the last write to a field wins.
    module
        .run_effect(&world, &host, switch, field("use", "perform"))
        .unwrap()
        .apply(&mut world);
    assert_eq!(state(&world, switch, &decl), (false, 11));

    module
        .run_effect(&world, &host, switch, field("reset", "perform"))
        .unwrap()
        .apply(&mut world);
    assert_eq!(state(&world, switch, &decl), (false, 0));
}

#[test]
fn a_write_to_an_entity_that_lost_its_aspect_is_skipped() {
    let host = host();
    let module = module(SWITCH, &host).unwrap();
    let decl = module.aspects()[0].clone();
    let mut world = World::new();
    let switch = spawn_switch(&mut world, &decl, false, 0);
    let perform = interaction(&decl, "use").field("perform").unwrap();
    let effects = module.run_effect(&world, &host, switch, perform).unwrap();
    scripted::remove(&mut world.entity_mut(switch), &decl.key, true);
    effects.apply(&mut world);
    assert!(scripted::get(&world, switch, &decl.key, true).1.is_none());
}

#[test]
fn bad_interactions_are_errors() {
    let host = host();
    let aspect = |body: &str| {
        format!("aspect A for Wall {{ label: String = \"a\", state on: bool, {body} }}")
    };
    let cases = [
        (
            "interaction use { caption: \"Use\", perform: { on = true }, captoin: \"x\" }",
            "Interactions have no field captoin",
        ),
        (
            "interaction use { caption: \"Use\" }",
            "Interaction use must set perform",
        ),
        (
            "interaction use { caption: \"Use\", caption: \"Again\", perform: {} }",
            "Name redefinition: caption",
        ),
        (
            "interaction use { caption: \"Use\", perform: {} } interaction use { caption: \"Use\", perform: {} }",
            "Name redefinition: use",
        ),
        (
            "interaction use { caption: 3, perform: {} }",
            "caption must be a literal of type String",
        ),
        (
            "interaction use { caption: label, perform: {} }",
            "caption must be a literal of type String",
        ),
        (
            "interaction use { caption: \"Use\", when: 1, perform: {} }",
            "Cannot assign type i32 to bool",
        ),
        (
            "interaction use { caption: \"Use\", when: { on = true; on }, perform: {} }",
            "Only an effect may write on",
        ),
        (
            "interaction use { caption: \"Use\", perform: { label = \"b\" } }",
            "label is config; only state fields can be written",
        ),
    ];
    for (body, message) in cases {
        let src = aspect(body);
        let err = module(&src, &host).unwrap_err();
        assert_eq!(err.to_string(), message, "{src}");
    }
}
