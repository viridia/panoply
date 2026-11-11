use std::sync::Arc;

use bevy::{
    ecs::{component::Component, entity::Entity, world::World},
    math::Vec3,
    reflect::{Reflect, Typed},
};
use futures_lite::future;
use panoply_saga::{
    CompilationError, ExprType, HostParam, HostState, InvocationContext, ModuleResolver, NoImports,
    ScriptModule, ScriptVm, StackValue, TokenLocation, TrackingScope, VMError, compile_formula,
    compile_module, format_error,
};
use smol_str::SmolStr;

#[derive(Component)]
struct Health(f32);

#[derive(Component)]
struct Position(Vec3);

#[derive(Component, Reflect)]
struct Lid {
    open: bool,
}

fn entity_health(ctx: &mut InvocationContext, actor: StackValue) -> Result<StackValue, VMError> {
    let StackValue::Entity(entity) = actor else {
        return Err(VMError::InvalidType(ExprType::None));
    };
    match ctx.component::<Health>(entity) {
        Some(&Health(h)) => Ok(StackValue::F32(h)),
        None => Err(VMError::MissingComponent("Health")),
    }
}

fn entity_heal(ctx: &mut InvocationContext) -> Result<StackValue, VMError> {
    let entity: Entity = ctx.argument(0)?;
    let amount: f32 = ctx.argument(1)?;
    match ctx.component::<Health>(entity) {
        Some(&Health(h)) => Ok(StackValue::F32(h + amount)),
        None => Err(VMError::MissingComponent("Health")),
    }
}

fn entity_position(ctx: &mut InvocationContext, actor: StackValue) -> Result<StackValue, VMError> {
    let StackValue::Entity(entity) = actor else {
        return Err(VMError::InvalidType(ExprType::None));
    };
    match ctx.component::<Position>(entity) {
        Some(Position(p)) => Ok(ctx.create_world_ref(p.as_reflect())),
        None => Err(VMError::MissingComponent("Position")),
    }
}

fn vec3_new(ctx: &mut InvocationContext) -> Result<StackValue, VMError> {
    let x = ctx.argument::<f32>(0)?;
    let y = ctx.argument::<f32>(1)?;
    let z = ctx.argument::<f32>(2)?;
    ctx.create_heap_ref(Vec3 { x, y, z })
}

fn param(name: &'static str, ty: ExprType) -> HostParam {
    HostParam {
        name: SmolStr::new_static(name),
        ty,
    }
}

/// A host whose `Entity` has `health`, `position` and `heal(amount)`.
fn actor_host() -> HostState {
    let mut host = HostState::new();
    host.get_host_type_mut::<Entity>()
        .unwrap()
        .add_property("health", entity_health, ExprType::F32)
        .add_property(
            "position",
            entity_position,
            ExprType::Reflected(Vec3::type_info()),
        )
        .add_method(
            "heal",
            entity_heal,
            vec![param("amount", ExprType::F32)],
            ExprType::F32,
        );
    host
}

fn formula(src: &str, host: &HostState, ty: ExprType) -> Result<ScriptModule, CompilationError> {
    future::block_on(compile_formula("--str--", src, host, &NoImports, ty))
}

fn module(src: &str, host: &HostState) -> Result<ScriptModule, CompilationError> {
    future::block_on(compile_module("--str--", src, host, &NoImports))
}

fn run(
    module: &ScriptModule,
    function: &str,
    world: &World,
    host: &HostState,
    owner: Entity,
) -> Result<StackValue, VMError> {
    let mut tracking = TrackingScope::default();
    let mut vm = ScriptVm::new(world, host, &mut tracking);
    vm.owner = owner;
    vm.run(module, function)
}

/// Compiles and runs a formula with no world state.
fn eval(src: &str, ty: ExprType) -> Result<StackValue, VMError> {
    let host = HostState::new();
    let module = formula(src, &host, ty).unwrap_or_else(|e| panic!("{src}: {e}"));
    run(
        &module,
        ScriptModule::DEFAULT,
        &World::new(),
        &host,
        Entity::PLACEHOLDER,
    )
}

/// Compiles and runs a formula on an actor with health 22 at (1, 2, 3).
fn eval_actor(src: &str, ty: ExprType) -> Result<StackValue, VMError> {
    let mut world = World::new();
    let actor = world
        .spawn((Health(22.0), Position(Vec3::new(1.0, 2.0, 3.0))))
        .id();
    let host = actor_host();
    let module = formula(src, &host, ty).unwrap_or_else(|e| panic!("{src}: {e}"));
    run(&module, ScriptModule::DEFAULT, &world, &host, actor)
}

#[test]
fn compile_literals_and_arithmetic() {
    assert_eq!(eval("20", ExprType::I32), Ok(StackValue::I32(20)));
    assert_eq!(eval("20 + 10", ExprType::I32), Ok(StackValue::I32(30)));
    assert_eq!(
        eval("20.0 + 10.0", ExprType::F32),
        Ok(StackValue::F32(30.0))
    );
    assert_eq!(eval("7 % 3 * 2 - 1", ExprType::I32), Ok(StackValue::I32(1)));
    assert_eq!(eval("{ 20 }", ExprType::I32), Ok(StackValue::I32(20)));
}

#[test]
fn compile_mismatched_types() {
    let host = HostState::new();
    let error = formula("20.0 + 10", &host, ExprType::F32).unwrap_err();
    assert!(matches!(error, CompilationError::MismatchedTypes(_, _, _)));
    assert_eq!(error.location().start(), 7);
    assert_eq!(error.location().end(), 9);
}

#[test]
fn compile_relational() {
    let cases = [
        ("20 == 10", false),
        ("20 != 10", true),
        ("20 < 10", false),
        ("20 <= 10", false),
        ("20 > 10", true),
        ("20 >= 10", true),
        ("true == false", false),
        ("true != false", true),
    ];
    for (src, expected) in cases {
        assert_eq!(
            eval(src, ExprType::Boolean),
            Ok(StackValue::Bool(expected)),
            "{src}"
        );
    }
}

#[test]
fn compile_shifts_and_bits() {
    assert_eq!(eval("1 << 3", ExprType::I32), Ok(StackValue::I32(8)));
    assert_eq!(
        eval("256i64 >> 4i64", ExprType::I64),
        Ok(StackValue::I64(16))
    );
    assert_eq!(
        eval("6 & 3 | 8 ^ 1", ExprType::I32),
        Ok(StackValue::I32(11))
    );
}

#[test]
fn compile_unary() {
    assert_eq!(eval("-5", ExprType::I32), Ok(StackValue::I32(-5)));
    assert_eq!(eval("-2.5", ExprType::F32), Ok(StackValue::F32(-2.5)));
    assert_eq!(
        eval("!true", ExprType::Boolean),
        Ok(StackValue::Bool(false))
    );
    assert_eq!(eval("~0", ExprType::I32), Ok(StackValue::I32(-1)));
}

#[test]
fn compile_casts() {
    assert_eq!(eval("5 as f32", ExprType::F32), Ok(StackValue::F32(5.0)));
    assert_eq!(eval("2.7 as i32", ExprType::I32), Ok(StackValue::I32(2)));
    assert_eq!(eval("true as i32", ExprType::I32), Ok(StackValue::I32(1)));
    assert_eq!(eval("5i64 as i32", ExprType::I32), Ok(StackValue::I32(5)));
    assert_eq!(
        eval("1.5f64 as f32", ExprType::F32),
        Ok(StackValue::F32(1.5))
    );
    assert_eq!(eval("3 as i32", ExprType::I32), Ok(StackValue::I32(3)));
    assert_eq!(
        eval("{ let x = 2; x as f64 }", ExprType::F64),
        Ok(StackValue::F64(2.0))
    );

    let host = HostState::new();
    let error = formula("1.0 as bool", &host, ExprType::Boolean).unwrap_err();
    assert!(
        matches!(error, CompilationError::InvalidCast(..)),
        "{error}"
    );
}

#[test]
fn compile_locals() {
    assert_eq!(
        eval("let x = 19; let y = x + 1; y", ExprType::I32),
        Ok(StackValue::I32(20))
    );
    assert_eq!(
        eval("let x = 1; x += 2; x = x * 2; x", ExprType::I32),
        Ok(StackValue::I32(6))
    );
    assert_eq!(
        eval("let s = \"test\"; s.len()", ExprType::I32),
        Ok(StackValue::I32(4))
    );

    let host = HostState::new();
    let error = formula("const x = 1; x = 2; x", &host, ExprType::I32).unwrap_err();
    assert!(matches!(
        error,
        CompilationError::InvalidAssignmentTarget(_)
    ));
}

#[test]
fn compile_control_flow() {
    assert_eq!(
        eval("if 1 > 0 { 2.0 } else { 3.0 }", ExprType::F32),
        Ok(StackValue::F32(2.0))
    );
    // The inner `if` is not laid out after its condition, so it branches on true, then
    // unconditionally to the else block.
    assert_eq!(
        eval(
            "if true { if false { 1 } else { 2 } } else { 3 }",
            ExprType::I32
        ),
        Ok(StackValue::I32(2))
    );
    assert_eq!(
        eval(
            "if false { 1 } else if false { 2 } else { 3 }",
            ExprType::I32
        ),
        Ok(StackValue::I32(3))
    );
    assert_eq!(
        eval("false || true", ExprType::Boolean),
        Ok(StackValue::Bool(true))
    );
    assert_eq!(
        eval("true && false", ExprType::Boolean),
        Ok(StackValue::Bool(false))
    );
    assert_eq!(
        eval("(false || false) || (true && true)", ExprType::Boolean),
        Ok(StackValue::Bool(true))
    );
}

#[test]
fn compile_runtime_errors() {
    assert_eq!(eval("1 / 0", ExprType::I32), Err(VMError::DivideByZero));
    assert_eq!(eval("1 % 0", ExprType::I32), Err(VMError::DivideByZero));
}

#[test]
fn compile_string_method() {
    assert_eq!(
        eval("\"test\".len()", ExprType::I32),
        Ok(StackValue::I32(4))
    );
}

#[test]
fn compile_entity_members() {
    assert_eq!(
        eval_actor("self.health", ExprType::F32),
        Ok(StackValue::F32(22.0))
    );
    assert_eq!(
        eval_actor("self.health > 0.0 && self.health < 1.0", ExprType::Boolean),
        Ok(StackValue::Bool(false))
    );
    assert_eq!(
        eval_actor("let e = self; e.health", ExprType::F32),
        Ok(StackValue::F32(22.0))
    );
    assert_eq!(
        eval_actor("self.heal(3.0)", ExprType::F32),
        Ok(StackValue::F32(25.0))
    );
    assert_eq!(
        eval_actor("self.position.x", ExprType::F32),
        Ok(StackValue::F32(1.0))
    );
}

#[test]
fn compile_records_component_reads() {
    let mut world = World::new();
    let actor = world.spawn(Health(22.0)).id();
    let host = actor_host();
    let module = formula("self.health", &host, ExprType::F32).unwrap();
    let mut tracking = TrackingScope::default();
    let mut vm = ScriptVm::new(&world, &host, &mut tracking);
    vm.owner = actor;
    vm.run(&module, ScriptModule::DEFAULT).unwrap();
    let health = world.component_id::<Health>().unwrap();
    assert_eq!(tracking.components(), &[(actor, health, true)]);
}

#[test]
fn self_is_the_owner() {
    let mut world = World::new();
    let actor = world.spawn(()).id();
    let host = HostState::new();
    let module = formula("self", &host, ExprType::Entity).unwrap();
    let result = run(&module, ScriptModule::DEFAULT, &world, &host, actor);
    assert_eq!(result, Ok(StackValue::Entity(actor)));
}

#[test]
fn component_accessors_read_fields() {
    let mut world = World::new();
    let shut = world.spawn(Lid { open: false }).id();
    let open = world.spawn(Lid { open: true }).id();
    let lidless = world.spawn(()).id();
    let mut host = HostState::new();
    host.add_component::<Lid>("lid");
    let module = formula("!self.lid.open", &host, ExprType::Boolean).unwrap();
    let eval = |owner| run(&module, ScriptModule::DEFAULT, &world, &host, owner);
    assert_eq!(eval(shut), Ok(StackValue::Bool(true)));
    assert_eq!(eval(open), Ok(StackValue::Bool(false)));
    assert_eq!(
        eval(lidless),
        Err(VMError::MissingComponent(core::any::type_name::<Lid>()))
    );

    let error = formula("self.lid.shut", &host, ExprType::Boolean).unwrap_err();
    assert!(
        matches!(error, CompilationError::UnknownField(..)),
        "{error}"
    );
}

#[test]
fn self_components_lists_reads_through_self() {
    let mut host = HostState::new();
    host.add_component::<Lid>("lid");
    let module = formula("self.lid.open || !self.lid.open", &host, ExprType::Boolean).unwrap();
    assert_eq!(module.self_components(), &[core::any::TypeId::of::<Lid>()]);
    let module = formula("let s = self; s.lid.open", &host, ExprType::Boolean).unwrap();
    assert!(module.self_components().is_empty());
}

#[test]
fn compile_static_method() {
    let mut host = HostState::new();
    host.add_host_type::<Vec3>("Vec3").add_static_method(
        "new",
        vec3_new,
        vec![
            param("x", ExprType::F32),
            param("y", ExprType::F32),
            param("z", ExprType::F32),
        ],
        ExprType::Reflected(Vec3::type_info()),
    );
    let module = module("fn test() -> f32 { Vec3::new(1.0, 2.0, 3.0).y }", &host).unwrap();
    let result = run(&module, "test", &World::new(), &host, Entity::PLACEHOLDER);
    assert_eq!(result, Ok(StackValue::F32(2.0)));
}

#[test]
fn compile_module_functions() {
    let host = HostState::new();
    let cases = [
        ("fn test() -> i32 { 20 }", 20),
        ("fn test2() -> i32 { 20 } fn test() -> i32 { test2() }", 20),
        (
            "fn test2(i: i32) -> i32 { 20 + i } fn test() -> i32 { test2(5) }",
            25,
        ),
        ("fn test2(i: i32) {} fn test() -> i32 { test2(0); 5 }", 5),
        ("fn test() -> i32 { let x = 19; let y = 1; x + y }", 20),
        (
            "fn twice(x: f32) -> f32 { x * 2.0 } fn test() -> i32 { twice(2.5) as i32 }",
            5,
        ),
    ];
    for (src, expected) in cases {
        let module = module(src, &host).unwrap_or_else(|e| panic!("{src}: {e}"));
        let result = run(&module, "test", &World::new(), &host, Entity::PLACEHOLDER);
        assert_eq!(result, Ok(StackValue::I32(expected)), "{src}");
    }
}

#[test]
fn compile_module_errors() {
    let host = HostState::new();
    let error = module("fn f() -> i32 { 1 } fn f() -> i32 { 2 }", &host).unwrap_err();
    assert!(matches!(error, CompilationError::FunctionRedefinition(..)));
    let error = module("fn f(a: i32, a: i32) -> i32 { a }", &host).unwrap_err();
    assert!(matches!(error, CompilationError::NameRedefinition(..)));
}

/// A resolver with one module, to fix the shape of the API imports will use.
struct OneModule(Arc<ScriptModule>);

impl ModuleResolver for OneModule {
    async fn resolve(
        &self,
        location: TokenLocation,
        path: &[&str],
    ) -> Result<Arc<ScriptModule>, CompilationError> {
        match path {
            ["package", "util"] => Ok(self.0.clone()),
            _ => Err(CompilationError::UnresolvedImport(
                location,
                path.join("::"),
            )),
        }
    }
}

#[test]
fn compile_with_resolver() {
    let host = HostState::new();
    let util = Arc::new(module("pub fn one() -> i32 { 1 }", &host).unwrap());
    let resolver = OneModule(util);
    let compiled = future::block_on(compile_module(
        "--str--",
        "fn test() -> i32 { 2 }",
        &host,
        &resolver,
    ))
    .unwrap();
    let result = run(&compiled, "test", &World::new(), &host, Entity::PLACEHOLDER);
    assert_eq!(result, Ok(StackValue::I32(2)));

    let error = future::block_on(compile_module(
        "--str--",
        "import package::util::one;",
        &host,
        &resolver,
    ))
    .unwrap_err();
    assert!(matches!(error, CompilationError::Unsupported(..)));
}

#[test]
fn disassembles() {
    let host = HostState::new();
    let module = formula(
        "let x = 2; if x > 1 { x * 3 } else { 0 }",
        &host,
        ExprType::I32,
    )
    .unwrap();
    let expected = [
        "fn 0:",
        "   0: const i32 2",
        "   5: store local 0",
        "   8: load local 0",
        "  11: const i32 1",
        "  16: gt.i32",
        "  17: branch_if_false 36 [+14]",
        "  22: load local 0",
        "  25: const i32 3",
        "  30: mul.i32",
        "  31: branch 41 [+5]",
        "  36: const i32 0",
        "  41: ret",
        "",
    ];
    assert_eq!(module.disassemble(), expected.join("\n"));
}

#[test]
fn formats_errors_with_the_source_line() {
    let host = HostState::new();
    let src = "let speed = 2.0;\nspeed * hast";
    let error = formula(src, &host, ExprType::F32).unwrap_err();
    let expected = [
        "chest.saga:2:9 Can't find the name 'hast' in this scope",
        "speed * hast",
        "        ^^^^",
    ];
    assert_eq!(format_error("chest.saga", src, &error), expected.join("\n"));

    // A parse error at the end of the input points just past it.
    let src = "1 +";
    let error = formula(src, &host, ExprType::I32).unwrap_err();
    let expected = ["chest.saga:1:4 Expression expected", "1 +", "   ^"];
    assert_eq!(format_error("chest.saga", src, &error), expected.join("\n"));
}
