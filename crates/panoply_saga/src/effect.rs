//! Effects: what an interaction performs (and later a hook). An effect reads the world as a
//! formula does, but may also write, and its writes are buffered: they apply once it has
//! returned, so a read within the effect sees the value from before it ran.

use std::sync::Arc;

use bevy::{
    ecs::{entity::Entity, world::World},
    log::error,
    reflect::PartialReflect,
};
use smol_str::SmolStr;

use crate::{FromSlots, HostState, ScriptModule, ScriptVm, TrackingScope, VMError, scripted};

/// One write an effect made.
enum EffectWrite {
    /// A state field of a scripted aspect.
    Scripted {
        entity: Entity,
        key: Arc<str>,
        field: SmolStr,
        value: Box<dyn PartialReflect>,
    },
    /// What a host command queued.
    Command(HostCommand),
}

/// A change a host command queued, as `damage` lowering health.
pub(crate) type HostCommand = Box<dyn FnOnce(&mut World) + Send>;

impl std::fmt::Debug for EffectWrite {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Scripted {
                entity, key, field, ..
            } => write!(f, "{entity} {key}.{field}"),
            Self::Command(_) => write!(f, "a host command"),
        }
    }
}

/// The writes an effect made, in order, applied together once it has returned.
#[derive(Debug, Default)]
pub struct EffectBuffer {
    writes: Vec<EffectWrite>,
}

impl EffectBuffer {
    /// Whether the effect wrote nothing.
    pub fn is_empty(&self) -> bool {
        self.writes.is_empty()
    }

    /// The number of writes.
    pub fn len(&self) -> usize {
        self.writes.len()
    }

    pub(crate) fn write_scripted(
        &mut self,
        entity: Entity,
        key: Arc<str>,
        field: SmolStr,
        value: Box<dyn PartialReflect>,
    ) {
        self.writes.push(EffectWrite::Scripted {
            entity,
            key,
            field,
            value,
        });
    }

    pub(crate) fn queue(&mut self, command: HostCommand) {
        self.writes.push(EffectWrite::Command(command));
    }

    /// Applies the writes in order. One that fails, as on an entity that lost its aspect, is
    /// logged, and the rest still apply.
    pub fn apply(self, world: &mut World) {
        for write in self.writes {
            match write {
                EffectWrite::Command(command) => command(world),
                EffectWrite::Scripted {
                    entity,
                    key,
                    field,
                    value,
                } => {
                    let Some(mut fields) = scripted::get_mut(world, entity, &key, true) else {
                        error!("{entity} has no {key} for an effect to write {field}");
                        continue;
                    };
                    let applied = fields
                        .get_mut(&field)
                        .map(|target| target.try_apply(value.as_ref()));
                    match applied {
                        Some(Ok(())) => {}
                        Some(Err(err)) => error!("{key}.{field}: an effect's write failed: {err}"),
                        None => error!("{key} has no field {field} for an effect to write"),
                    }
                }
            }
        }
    }
}

impl ScriptModule {
    /// Runs function `index`, a formula taking no arguments, on `owner`.
    pub fn run_formula<T>(
        &self,
        world: &World,
        host: &HostState,
        owner: Entity,
        index: usize,
    ) -> Result<T, VMError>
    where
        T: for<'a> FromSlots<'a>,
    {
        let mut tracking = TrackingScope::default();
        let mut vm = ScriptVm::new(world, host, &mut tracking);
        vm.set_owner(owner);
        vm.run_index_as(self, index)
    }

    /// Runs function `index`, an effect taking no arguments, on `owner`, returning its writes for
    /// the caller to apply.
    pub fn run_effect(
        &self,
        world: &World,
        host: &HostState,
        owner: Entity,
        index: usize,
    ) -> Result<EffectBuffer, VMError> {
        let mut tracking = TrackingScope::default();
        let mut effects = EffectBuffer::default();
        let mut vm = ScriptVm::new(world, host, &mut tracking);
        vm.set_owner(owner).set_effects(&mut effects);
        vm.run_index(self, index, &[])?;
        Ok(effects)
    }
}
