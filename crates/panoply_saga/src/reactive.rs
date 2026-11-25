//! Cells, memos and the reaction pass (see the crate's Terms).
//!
//! The world tells no one when it changes, so cells poll. Each pass, a cell first brings its
//! memos up to date, each recomputing only if something it read has changed. The cell then runs
//! its formula again only if a memo's value changed or something the formula read did, and
//! compares the result with its last value.
//!
//! A pass is an exclusive system. It records the world's change tick, which the scheduler
//! advances after the pass: a write before the pass carries a tick no newer, one after it a newer
//! tick, so comparing ticks with the pass's misses nothing.

use std::any::Any;

use bevy::{
    ecs::{change_detection::Tick, schedule::ScheduleLabel},
    platform::collections::HashMap,
    prelude::*,
};

use crate::{
    Formula, FromSlots, HostState, InvocationContext, SagaHost, SlotStore, StackValue, ToSlots,
    TrackingScope, VMError,
};

/// A native that keeps state between evaluations, registered with
/// [`HostState::add_memo`]. Its value is recomputed only when something it read has changed or
/// its arguments have.
pub trait NativeMemo: Default + Send + Sync + 'static {
    /// Its arguments, read from primitive and entity slots only, since they are kept between
    /// evaluations, after the strings and values other slots refer to are gone.
    type Args: for<'a> FromSlots<'a>;

    /// Its value, compared with the last to decide whether it changed.
    type Value: ToSlots + PartialEq + Clone + Send + Sync;

    /// The value now, tracking what it reads. `self` keeps whatever the memo remembers, such as
    /// its last value.
    fn compute(
        &mut self,
        args: Self::Args,
        world: &World,
        tracking: &mut TrackingScope,
    ) -> Result<Self::Value, VMError>;
}

/// A [`NativeMemo`] at one call site: its state, arguments, value and reads.
struct MemoCell<M: NativeMemo> {
    memo: M,
    args: Vec<StackValue>,
    value: M::Value,
    tracking: TrackingScope,
}

impl<M: NativeMemo> MemoCell<M> {
    fn new(args: Vec<StackValue>, world: &World) -> Result<Self, VMError> {
        let mut memo = M::default();
        let mut tracking = TrackingScope::default();
        let value = memo.compute(read_args::<M>(&args)?, world, &mut tracking)?;
        Ok(Self {
            memo,
            args,
            value,
            tracking,
        })
    }

    /// Computes the value again, saying whether it changed.
    fn recompute(&mut self, world: &World) -> Result<bool, VMError> {
        self.tracking.clear();
        let value = self
            .memo
            .compute(read_args::<M>(&self.args)?, world, &mut self.tracking)?;
        let changed = value != self.value;
        self.value = value;
        Ok(changed)
    }
}

fn read_args<M: NativeMemo>(args: &[StackValue]) -> Result<M::Args, VMError> {
    M::Args::from_slots(args, &SlotStore::default())
}

/// A [`MemoCell`] of any memo, as its cell holds it.
trait AnyMemo: Send + Sync {
    /// Recomputes the memo if something it read has changed, saying whether its value did.
    fn refresh(&mut self, world: &World, last_run: Tick, this_run: Tick) -> Result<bool, VMError>;

    fn tracking(&self) -> &TrackingScope;

    fn as_any_mut(&mut self) -> &mut dyn Any;
}

impl<M: NativeMemo> AnyMemo for MemoCell<M> {
    fn refresh(&mut self, world: &World, last_run: Tick, this_run: Tick) -> Result<bool, VMError> {
        if !self.tracking.changed_since(world, last_run, this_run) {
            return Ok(false);
        }
        self.recompute(world)
    }

    fn tracking(&self) -> &TrackingScope {
        &self.tracking
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

struct MemoEntry {
    memo: Box<dyn AnyMemo>,
    /// Whether the cell's evaluation running, or the last one, called it.
    used: bool,
}

/// A cell's memos, by call site.
#[derive(Default)]
pub(crate) struct MemoTable(HashMap<usize, MemoEntry>);

impl MemoTable {
    /// Brings every memo up to date, saying whether any value changed. A memo whose computing
    /// fails is dropped, so that the call computes it again and its evaluation reports the
    /// failure.
    fn refresh(&mut self, world: &World, last_run: Tick, this_run: Tick) -> bool {
        let mut changed = false;
        self.0.retain(
            |_, entry| match entry.memo.refresh(world, last_run, this_run) {
                Ok(memo_changed) => {
                    changed |= memo_changed;
                    true
                }
                Err(_) => {
                    changed = true;
                    false
                }
            },
        );
        changed
    }

    /// Before an evaluation, which marks the memos it calls.
    fn begin(&mut self) {
        for entry in self.0.values_mut() {
            entry.used = false;
        }
    }

    /// After an evaluation: memos it did not call, as on a branch not taken, are dropped.
    fn end(&mut self) {
        self.0.retain(|_, entry| entry.used);
    }

    fn reads(&self) -> impl Iterator<Item = &TrackingScope> {
        self.0.values().map(|entry| entry.memo.tracking())
    }
}

/// The host function a memo is registered as: the memo at this call site, if its arguments are
/// the last call's, else computed anew. Outside a cell, as for an interaction's `when`, it is
/// computed afresh and forgotten.
pub(crate) fn call_memo<M: NativeMemo>(ctx: &mut InvocationContext) -> Result<(), VMError> {
    let args = ctx.arg_slots();
    let primitive = |slot: &StackValue| {
        matches!(
            slot,
            StackValue::Bool(_)
                | StackValue::I32(_)
                | StackValue::I64(_)
                | StackValue::F32(_)
                | StackValue::F64(_)
                | StackValue::Entity(_)
        )
    };
    if !args.iter().all(primitive) {
        return Err(VMError::MemoArguments);
    }
    let args = args.to_vec();
    let world = ctx.world();
    let site = ctx.site;
    let Some(table) = ctx.memos.as_deref_mut() else {
        let value = MemoCell::<M>::new(args, world)?.value;
        return ctx.ret(value);
    };
    let current = table
        .0
        .get_mut(&site)
        .and_then(|entry| {
            entry.used = true;
            entry.memo.as_any_mut().downcast_mut::<MemoCell<M>>()
        })
        .filter(|memo| memo.args == args)
        .map(|memo| memo.value.clone());
    let value = match current {
        Some(value) => value,
        None => {
            let memo = MemoCell::<M>::new(args, world)?;
            let value = memo.value.clone();
            table.0.insert(
                site,
                MemoEntry {
                    memo: Box::new(memo),
                    used: true,
                },
            );
            value
        }
    };
    ctx.ret(value)
}

/// A formula kept alive on its owner: its last value, what it read, and its memos.
struct ReactiveCell {
    owner: Entity,
    formula: Formula<bool>,
    /// `None` until an evaluation succeeds. A failed evaluation keeps the last value.
    value: Option<bool>,
    /// The pass it was last brought up to date in; `None` before its first.
    ran: Option<Tick>,
    tracking: TrackingScope,
    memos: MemoTable,
}

impl ReactiveCell {
    fn refresh(&mut self, world: &World, host: &HostState, this_run: Tick) {
        let stale = match self.ran {
            None => true,
            Some(last_run) => {
                // Memos first, even when the formula will run anyway: the calls it makes return
                // their memos' values.
                let memo_changed = self.memos.refresh(world, last_run, this_run);
                memo_changed || self.tracking.changed_since(world, last_run, this_run)
            }
        };
        self.ran = Some(this_run);
        if !stale {
            return;
        }
        self.tracking.clear();
        self.memos.begin();
        if let Some(value) =
            self.formula
                .evaluate_in(world, host, self.owner, &mut self.tracking, &mut self.memos)
        {
            self.value = Some(value);
        }
        self.memos.end();
    }
}

/// Names a cell in [`Reactions`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CellId(u64);

/// Every cell, brought up to date by the reaction pass. A resource rather than components on the
/// owners, because a cell's formula reads the world while the pass writes the cell. Whatever adds
/// a cell removes it, when the component holding its [`CellId`] is replaced or removed.
#[derive(Resource, Default)]
pub struct Reactions {
    cells: HashMap<CellId, ReactiveCell>,
    next: u64,
}

/// How much the cells read, for measuring the pass: a resource each pass updates, which
/// `plans/perf.py` reads.
#[derive(Resource, Reflect, Debug, Default, PartialEq)]
#[reflect(Resource)]
pub struct ReactionReads {
    pub cells: usize,
    /// Reads recorded by the cells' last evaluations and their memos', repeats included.
    pub reads: u32,
    /// Of those, the reads that repeated one already recorded.
    pub repeats: u32,
}

impl Reactions {
    /// Adds a cell running `formula` on `owner`, first evaluated in the next pass.
    pub fn add(&mut self, owner: Entity, formula: Formula<bool>) -> CellId {
        let id = CellId(self.next);
        self.next += 1;
        self.cells.insert(
            id,
            ReactiveCell {
                owner,
                formula,
                value: None,
                ran: None,
                tracking: TrackingScope::default(),
                memos: MemoTable::default(),
            },
        );
        id
    }

    pub fn remove(&mut self, id: CellId) {
        self.cells.remove(&id);
    }

    /// The cell's value, or `None` before its first evaluation succeeds or once it is removed.
    pub fn value(&self, id: CellId) -> Option<bool> {
        self.cells.get(&id)?.value
    }

    /// Brings every cell up to date, at the world's current change tick.
    pub fn refresh(&mut self, world: &World, host: &HostState) {
        let this_run = world.read_change_tick();
        for cell in self.cells.values_mut() {
            cell.refresh(world, host, this_run);
        }
    }

    /// What the cells read in their last evaluations.
    pub fn reads(&self) -> ReactionReads {
        let mut reads = ReactionReads {
            cells: self.cells.len(),
            ..default()
        };
        for cell in self.cells.values() {
            for scope in std::iter::once(&cell.tracking).chain(cell.memos.reads()) {
                reads.reads += scope.reads();
                reads.repeats += scope.repeats();
            }
        }
        reads
    }
}

/// The reaction pass, in [`ReactionPass`]. Systems in `Update` read the values of the frame
/// before.
pub fn run_reactions(world: &mut World) {
    let host = world.resource::<SagaHost>().clone();
    world.resource_scope(|world, mut reactions: Mut<Reactions>| {
        reactions.refresh(world, host.get());
        world.insert_resource(reactions.reads());
    });
}

/// The schedule the reaction pass runs in, right after `Update`, where gameplay has run. A schedule
/// of its own, because the pass is an exclusive system: within `Update` it would stop every system
/// there from running beside it.
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ReactionPass;
