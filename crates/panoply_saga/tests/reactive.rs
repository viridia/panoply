//! Cells and memos, brought up to date by polling the world.

use std::cell::Cell;

use bevy::{
    prelude::*,
    reflect::{GetTypeRegistration, TypeRegistry, serde::TypedReflectDeserializer},
};
use panoply_saga::{
    ExprType, Formula, FormulaQueue, HostState, InvocationContext, ReactionReads, Reactions,
    VMError, natives::add_natives,
};
use serde::de::DeserializeSeed;

#[derive(Component, Reflect, Default)]
struct Lever {
    on: bool,
}

/// The entity a formula measures its distance to.
#[derive(Component, Reflect)]
struct Target {
    entity: Entity,
}

thread_local! {
    static EVALUATIONS: Cell<usize> = const { Cell::new(0) };
}

/// `count()`: true, counting the evaluations that call it.
fn count(ctx: &mut InvocationContext) -> Result<(), VMError> {
    EVALUATIONS.with(|count| count.set(count.get() + 1));
    ctx.ret(true)
}

fn evaluations() -> usize {
    EVALUATIONS.with(Cell::get)
}

fn host() -> HostState {
    let mut host = HostState::new();
    add_natives(&mut host);
    host.add_component::<Lever>("lever");
    host.add_component::<Target>("target");
    host.add_function("count", count, Vec::new(), ExprType::Boolean);
    host
}

#[derive(Reflect)]
struct Entry {
    when: Formula<bool>,
}

fn formula(source: &str, host: &HostState) -> Formula<bool> {
    let json = format!(r#"{{"when": {source:?}}}"#);
    let mut registry = TypeRegistry::new();
    registry.register::<Entry>();
    let registration = registry
        .get(Entry::get_type_registration().type_id())
        .unwrap();
    let mut formulas = FormulaQueue::default();
    let value = {
        let reader = formulas.track_json(json.as_bytes());
        let mut deserializer = serde_json::Deserializer::from_reader(reader);
        TypedReflectDeserializer::with_processor(registration, &registry, &mut formulas)
            .deserialize(&mut deserializer)
            .unwrap()
    };
    futures_lite::future::block_on(formulas.compile("test.json", host, json.as_bytes(), &[]));
    let when = Entry::from_reflect(&*value).unwrap().when;
    assert!(when.module().is_some(), "{:?}", when.error());
    when
}

/// A pass, then the tick advancing past it, as the scheduler advances it after a system.
fn pass(reactions: &mut Reactions, world: &mut World, host: &HostState) {
    reactions.refresh(world, host);
    world.increment_change_tick();
}

#[test]
fn a_cell_runs_again_only_when_what_it_read_changes() {
    let host = host();
    let mut world = World::new();
    let lever = world.spawn(Lever { on: false }).id();
    let other = world.spawn(Lever { on: false }).id();
    let mut reactions = Reactions::default();
    let cell = reactions.add(lever, formula("count() && self.lever.on", &host));
    assert_eq!(reactions.value(cell), None);

    pass(&mut reactions, &mut world, &host);
    assert_eq!((reactions.value(cell), evaluations()), (Some(false), 1));

    // Nothing it read changed.
    world.get_mut::<Lever>(other).unwrap().on = true;
    pass(&mut reactions, &mut world, &host);
    assert_eq!(evaluations(), 1);

    world.get_mut::<Lever>(lever).unwrap().on = true;
    pass(&mut reactions, &mut world, &host);
    assert_eq!((reactions.value(cell), evaluations()), (Some(true), 2));
}

#[test]
fn a_component_added_runs_the_cell_again() {
    let host = host();
    let mut world = World::new();
    let lever = world.spawn_empty().id();
    let mut reactions = Reactions::default();
    let cell = reactions.add(lever, formula("self.lever.on", &host));

    // Without the component the evaluation fails, and the cell has no value yet.
    pass(&mut reactions, &mut world, &host);
    assert_eq!(reactions.value(cell), None);

    world.entity_mut(lever).insert(Lever { on: true });
    pass(&mut reactions, &mut world, &host);
    assert_eq!(reactions.value(cell), Some(true));
}

#[test]
fn proximity_holds_between_enter_and_leave_without_running_its_cell() {
    let host = host();
    let mut world = World::new();
    let object = world.spawn(GlobalTransform::from_xyz(5., 0., 0.)).id();
    let subject = world
        .spawn((GlobalTransform::IDENTITY, Target { entity: object }))
        .id();
    let mut reactions = Reactions::default();
    let cell = reactions.add(
        subject,
        formula(
            "count() && proximity(self, self.target.entity, 3.0, 4.0)",
            &host,
        ),
    );
    let place = |world: &mut World, x: f32| {
        world
            .entity_mut(object)
            .insert(GlobalTransform::from_xyz(x, 0., 0.));
    };

    pass(&mut reactions, &mut world, &host);
    assert_eq!((reactions.value(cell), evaluations()), (Some(false), 1));

    place(&mut world, 3.5);
    pass(&mut reactions, &mut world, &host);
    assert_eq!((reactions.value(cell), evaluations()), (Some(false), 1));

    place(&mut world, 2.5);
    pass(&mut reactions, &mut world, &host);
    assert_eq!((reactions.value(cell), evaluations()), (Some(true), 2));

    // Moving within `leave`: the memo computes again, its value holds, and the cell does not run.
    place(&mut world, 3.5);
    pass(&mut reactions, &mut world, &host);
    assert_eq!((reactions.value(cell), evaluations()), (Some(true), 2));

    place(&mut world, 4.5);
    pass(&mut reactions, &mut world, &host);
    assert_eq!((reactions.value(cell), evaluations()), (Some(false), 3));
}

#[test]
fn a_memo_outside_a_cell_is_computed_afresh() {
    let host = host();
    let mut world = World::new();
    let object = world.spawn(GlobalTransform::from_xyz(2., 0., 0.)).id();
    let subject = world
        .spawn((GlobalTransform::IDENTITY, Target { entity: object }))
        .id();
    let near = formula("proximity(self, self.target.entity, 3.0, 4.0)", &host);
    assert_eq!(near.evaluate(&world, &host, subject), Some(true));
}

#[test]
fn reads_are_counted_with_their_repeats() {
    let host = host();
    let mut world = World::new();
    let lever = world.spawn(Lever { on: true }).id();
    let mut reactions = Reactions::default();
    reactions.add(lever, formula("self.lever.on && self.lever.on", &host));
    pass(&mut reactions, &mut world, &host);
    assert_eq!(
        reactions.reads(),
        ReactionReads {
            cells: 1,
            reads: 2,
            repeats: 1,
        }
    );
}
