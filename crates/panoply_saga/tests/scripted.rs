//! Aspects declared in SAGA, and formulas reading them.

use std::sync::Arc;

use bevy::{
    ecs::{entity::Entity, world::World},
    math::Vec3,
    reflect::{Reflect, Typed},
};
use futures_lite::future;
use panoply_saga::{
    CompilationError, ExprType, HostState, NoImports, ScriptModule, ScriptVm, TrackingScope,
    VMError, compile_formula, compile_module,
    scripted::{self, AspectDecl, ScriptedComponents, ScriptedFields},
};

const SWITCH: &str = r#"
    #[accessor(toggle_switch)]
    aspect ToggleSwitch for Wall, Fixture {
        caption: String = "Use",
        reach: f32 = 2,
        offset: Vec3,
        state on: bool = false,
    }

    aspect Quiet for Wall {
        state count: i32 = -3,
    }
"#;

fn host() -> HostState {
    let mut host = HostState::new();
    host.add_host_type::<Vec3>("Vec3");
    host
}

fn module(src: &str, host: &HostState) -> Result<ScriptModule, CompilationError> {
    future::block_on(compile_module(
        "scripts/toggle_switch.saga",
        src,
        host,
        &NoImports,
    ))
}

fn formula(
    src: &str,
    host: &HostState,
    scripted: &[Arc<AspectDecl>],
) -> Result<ScriptModule, CompilationError> {
    future::block_on(compile_formula(
        "test.json",
        src,
        host,
        &NoImports,
        ExprType::Boolean,
        scripted,
    ))
}

/// An entity with the switch's config at its declared values (`offset` at (0, 1, 0)), and its
/// state `on`.
fn spawn_switch(world: &mut World, decl: &Arc<AspectDecl>, on: bool) -> Entity {
    let config: Vec<Box<dyn Reflect>> = vec![
        Box::new("Use".to_string()),
        Box::new(2.0f32),
        Box::new(Vec3::Y),
    ];
    let mut entity = world.spawn_empty();
    scripted::insert(
        &mut entity,
        ScriptedFields::new(decl.clone(), false, config).unwrap(),
    );
    scripted::insert(
        &mut entity,
        ScriptedFields::new(decl.clone(), true, vec![Box::new(on)]).unwrap(),
    );
    entity.id()
}

fn evaluate(
    world: &World,
    host: &HostState,
    module: &ScriptModule,
    owner: Entity,
) -> (Result<bool, VMError>, TrackingScope) {
    let mut tracking = TrackingScope::default();
    let mut vm = ScriptVm::new(world, host, &mut tracking);
    vm.set_owner(owner);
    let result = vm.run_as::<bool>(module, ScriptModule::DEFAULT);
    (result, tracking)
}

#[test]
fn a_module_declares_its_aspects() {
    let host = host();
    let module = module(SWITCH, &host).unwrap();
    let [switch, quiet] = module.aspects() else {
        panic!("{:?}", module.aspects())
    };
    assert_eq!(&*switch.key, "scripts::toggle_switch::ToggleSwitch");
    assert_eq!(switch.accessor.as_deref(), Some("toggle_switch"));
    assert_eq!(switch.kinds, ["Wall", "Fixture"]);
    let fields: Vec<_> = switch
        .fields
        .iter()
        .map(|f| (f.name.as_str(), f.is_state, f.default.is_some()))
        .collect();
    assert_eq!(
        fields,
        [
            ("caption", false, true),
            ("reach", false, true),
            ("offset", false, false),
            ("on", true, true),
        ]
    );
    // An integer literal is a float field's default.
    let reach = switch.field("reach").unwrap();
    assert_eq!(
        reach.default.as_ref().unwrap().downcast_ref::<f32>(),
        Some(&2.0)
    );
    assert!(quiet.accessor.is_none());
    let count = quiet.field("count").unwrap();
    assert_eq!(
        count.default.as_ref().unwrap().downcast_ref::<i32>(),
        Some(&-3)
    );
}

#[test]
fn bad_declarations_are_errors() {
    let host = host();
    let cases = [
        (
            "#[icon(lock)] aspect A for Wall { }",
            "Unknown attribute: #[icon]",
        ),
        (
            "aspect A for Wall { on: bool = 1 }",
            "The default of on must be a literal of type bool",
        ),
        (
            "aspect A for Wall { on: bool = !true }",
            "The default of on must be a literal of type bool",
        ),
        ("aspect A for Wall { on: Lever }", "Unknown type: Lever"),
        (
            "aspect A for Wall { a: f32, a: f32 }",
            "Name redefinition: a",
        ),
        (
            "aspect A for Wall { } aspect A for Fixture { }",
            "Name redefinition: A",
        ),
    ];
    for (src, message) in cases {
        let err = module(src, &host).unwrap_err();
        assert_eq!(err.to_string(), message, "{src}");
    }
}

#[derive(Reflect, Debug, PartialEq)]
struct Range(f64, f64);

#[derive(Reflect, Debug, PartialEq)]
struct Fixed;

#[derive(Reflect, Debug, PartialEq)]
struct Icon(String);

/// The host, with `range(min, max)` and `fixed` for fields and `icon(path)` for aspects.
fn attributed_host() -> HostState {
    let mut host = host();
    host.add_field_attribute(
        "range",
        vec![ExprType::F64, ExprType::F64],
        |builder, args| {
            let bound = |index: usize| *args[index].downcast_ref::<f64>().unwrap();
            builder.attribute(Range(bound(0), bound(1)))
        },
    );
    host.add_field_attribute("fixed", Vec::new(), |builder, _| builder.attribute(Fixed));
    host.add_aspect_attribute("icon", vec![ExprType::String], |builder, args| {
        builder.attribute(Icon(args[0].downcast_ref::<String>().unwrap().clone()))
    });
    host
}

#[test]
fn attributes_come_from_the_hosts_table() {
    let host = attributed_host();
    let src = r#"
        #[icon("lock.png")] #[accessor(lever)]
        aspect Lever for Wall {
            #[range(0, -2.5)] #[fixed]
            reach: f32,
            plain: bool,
        }
    "#;
    let lever_module = module(src, &host).unwrap();
    let lever = &lever_module.aspects()[0];
    assert_eq!(lever.accessor.as_deref(), Some("lever"));
    assert_eq!(
        lever.attributes.get::<Icon>(),
        Some(&Icon("lock.png".into()))
    );
    let reach = &lever.field("reach").unwrap().attributes;
    assert_eq!(reach.get::<Range>(), Some(&Range(0.0, -2.5)));
    assert!(reach.contains::<Fixed>());
    assert!(lever.field("plain").unwrap().attributes.is_empty());

    let cases = [
        (
            "aspect A for Wall { #[rnage(0, 1)] a: f32 }",
            "Unknown attribute: #[rnage]",
        ),
        // An aspect's attributes are not a field's.
        (
            "#[fixed] aspect A for Wall { }",
            "Unknown attribute: #[fixed]",
        ),
        (
            "aspect A for Wall { #[icon(\"a.png\")] a: f32 }",
            "Unknown attribute: #[icon]",
        ),
        (
            "aspect A for Wall { #[range(0)] a: f32 }",
            "Expected #[range(f64, f64)]",
        ),
        (
            "aspect A for Wall { #[range(0, x)] a: f32 }",
            "Expected #[range(f64, f64)]",
        ),
        (
            "aspect A for Wall { #[fixed(true)] a: f32 }",
            "Expected #[fixed]",
        ),
        (
            "aspect A for Wall { #[fixed] #[fixed] a: f32 }",
            "Name redefinition: fixed",
        ),
        (
            "#[accessor(a)] #[accessor(b)] aspect A for Wall { }",
            "Name redefinition: accessor",
        ),
    ];
    for (src, message) in cases {
        let err = module(src, &host).unwrap_err();
        assert_eq!(err.to_string(), message, "{src}");
    }
}

#[test]
fn a_formula_reads_config_and_state() {
    let host = host();
    let decls = module(SWITCH, &host).unwrap().aspects().to_vec();
    let switch = &decls[0];
    let mut world = World::new();
    let off = spawn_switch(&mut world, switch, false);
    let on = spawn_switch(&mut world, switch, true);

    let when = formula("!self.toggle_switch.on", &host, &decls).unwrap();
    let read: Vec<&str> = when.self_aspects().iter().map(|a| &*a.key).collect();
    assert_eq!(read, [&*switch.key]);
    assert_eq!(evaluate(&world, &host, &when, off).0, Ok(true));
    assert_eq!(evaluate(&world, &host, &when, on).0, Ok(false));

    // The read is tracked as the state component's.
    let ids = world
        .resource::<ScriptedComponents>()
        .ids(&switch.key)
        .unwrap();
    let (_, tracking) = evaluate(&world, &host, &when, off);
    assert_eq!(tracking.components(), [(off, ids.state, true)]);

    // Config fields, and fields of a struct field, in one load.
    let config = formula(
        "self.toggle_switch.reach > 1.5 && self.toggle_switch.offset.y == 1.0",
        &host,
        &decls,
    )
    .unwrap();
    assert_eq!(evaluate(&world, &host, &config, off).0, Ok(true));
    let text = config.disassemble();
    assert!(text.contains("load scripted[0].2 1"), "{text}");

    // An entity without the aspect fails the evaluation.
    let bare = world.spawn_empty().id();
    assert_eq!(
        evaluate(&world, &host, &when, bare).0,
        Err(VMError::MissingAspect(switch.key.clone()))
    );
}

#[test]
fn bad_reads_are_errors() {
    let host = host();
    let decls = module(SWITCH, &host).unwrap().aspects().to_vec();
    let cases = [
        (
            "self.toggle_switch.nope",
            "Unknown field: ToggleSwitch.nope",
        ),
        (
            "self.toggle_switch",
            "Not supported yet: a scripted aspect as a value; read one of its fields",
        ),
        (
            "self.quiet.count == 0",
            "Unknown field: Entity.quiet; Quiet has no #[accessor]",
        ),
    ];
    for (src, message) in cases {
        let err = formula(src, &host, &decls).unwrap_err();
        assert_eq!(err.to_string(), message, "{src}");
    }
    // Out of scope, the accessor is unknown.
    let err = formula("self.toggle_switch.on", &host, &[]).unwrap_err();
    assert_eq!(err.to_string(), "Unknown field: Entity.toggle_switch");
}

#[test]
fn field_types_are_reflected() {
    // `bool::type_info()` and the field's info are the same type.
    let host = host();
    let decls = module(SWITCH, &host).unwrap().aspects().to_vec();
    assert_eq!(
        decls[0].field("on").unwrap().info.type_id(),
        bool::type_info().type_id()
    );
}
