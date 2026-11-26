//! An actor's health: its base max, from its exemplar, which play never changes; the effective
//! max; and the current health, which `damage` lowers. At zero the actor is dead.

use bevy::{annotations::DisplayName, prelude::*};
use panoply_exemplar::{annotations::AspectIcon, InstanceKind, NativeAspect, ReflectAspect};
use panoply_saga::{ExprType, HostParam, HostState, InvocationContext, VMError};
use serde::{Deserialize, Serialize};

/// An actor that can be hurt, and its base max health.
#[derive(Component, Debug, Reflect, Clone, Serialize, Deserialize)]
#[reflect(Aspect, Default, Serialize, Deserialize)]
#[reflect(@AspectIcon("editor/icons/lucide/heart.png"))]
#[serde(default)]
pub struct Health {
    /// The max health before modifiers.
    #[reflect(@DisplayName("Max"))]
    pub max: f32,
}

impl Default for Health {
    /// Enough for a few blows: an actor given health by a requirement, at its defaults, starts
    /// alive.
    fn default() -> Self {
        Self { max: 20.0 }
    }
}

impl NativeAspect for Health {
    type Config = (Self, HealthMax);

    /// The effective max is the base until X53's modifiers make it a cell over it.
    fn config(&self) -> Self::Config {
        (self.clone(), HealthMax { value: self.max })
    }

    fn can_attach(&self, kind: InstanceKind) -> bool {
        kind == InstanceKind::Actor
    }

    fn attach_state(&self, entity: &mut EntityWorldMut) {
        entity.insert_if_new(HealthState { current: self.max });
    }

    /// The current health.
    fn has_state(&self) -> bool {
        true
    }
}

/// An actor's effective max health, apart from [`HealthState`] so that a change to one is not a
/// change to the other.
#[derive(Component, Debug, Reflect, Clone, Default)]
#[reflect(Component, Default)]
pub struct HealthMax {
    pub value: f32,
}

/// An actor's current health, kept apart from [`Health`] so that reapplying aspects leaves it as
/// it was. Play state, which a save holds.
#[derive(Component, Debug, Reflect, Clone, Default, Serialize, Deserialize)]
#[reflect(Component, Default, Serialize, Deserialize)]
pub struct HealthState {
    /// Zero when dead.
    pub current: f32,
}

impl HealthState {
    pub fn dead(&self) -> bool {
        self.current <= 0.0
    }
}

/// Adds `damage(target, amount)` to the host, for effects to call.
pub(crate) fn add_damage_command(host: &mut HostState) {
    let param = |name: &str, ty| HostParam {
        name: name.into(),
        ty,
    };
    host.add_command(
        "damage",
        damage,
        vec![
            param("target", ExprType::Entity),
            param("amount", ExprType::F32),
        ],
    );
}

/// `damage(target, amount)`, an effect's host command.
fn damage(ctx: &mut InvocationContext) -> Result<(), VMError> {
    let target: Entity = ctx.arg()?;
    let amount: f32 = ctx.arg()?;
    ctx.queue(Damage { target, amount })
}

/// Lowers the target's current health by `amount`, to no less than zero. A target without health
/// is logged. A value rather than a closure, for a combat pipeline to rewrite before it applies
/// (X149).
#[derive(Debug, Clone, Copy)]
pub struct Damage {
    pub target: Entity,
    pub amount: f32,
}

impl Command for Damage {
    type Out = ();

    fn apply(self, world: &mut World) {
        match world
            .get_entity_mut(self.target)
            .ok()
            .and_then(|entity| entity.into_mut::<HealthState>())
        {
            Some(mut health) => health.current = (health.current - self.amount).max(0.0),
            None => error!("damage: {} has no health", self.target),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use futures_lite::future::block_on;
    use panoply_exemplar::{add_interaction_fields, annotations::add_editor_attributes};
    use panoply_saga::{compile_module, format_error, NoImports};
    use panoply_scenery::LockState;

    use super::*;

    /// A host with the interaction type, the editor's attributes, and the components and
    /// commands scripts use here and in scenery.
    fn host() -> HostState {
        let mut host = HostState::new();
        add_interaction_fields(&mut host);
        add_editor_attributes(&mut host);
        host.add_component::<LockState>("lock");
        host.add_component::<HealthState>("health");
        host.add_component::<HealthMax>("health_max");
        add_damage_command(&mut host);
        host
    }

    fn scripts() -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/scripts")
    }

    /// Every module in `assets/scripts/` compiles.
    #[test]
    fn every_script_compiles() {
        let host = host();
        let mut failures = Vec::new();
        let mut count = 0;
        for entry in std::fs::read_dir(scripts()).unwrap() {
            let path = entry.unwrap().path();
            if path.extension().is_none_or(|ext| ext != "saga") {
                continue;
            }
            count += 1;
            let name = format!("scripts/{}", path.file_name().unwrap().to_string_lossy());
            let src = std::fs::read_to_string(&path).unwrap();
            if let Err(err) = block_on(compile_module(&name, &src, &host, &NoImports)) {
                failures.push(format_error(&name, &src, &err));
            }
        }
        assert!(count > 0, "no modules under {}", scripts().display());
        assert!(failures.is_empty(), "{}", failures.join("\n"));
    }

    /// `combatant.saga`'s Attack is offered while the actor is alive, and each blow lowers its
    /// health, to zero and no further.
    #[test]
    fn attacks_lower_health_until_dead() {
        let host = host();
        let src = std::fs::read_to_string(scripts().join("combatant.saga")).unwrap();
        let module = block_on(compile_module(
            "scripts/combatant.saga",
            &src,
            &host,
            &NoImports,
        ))
        .unwrap();
        let attack = &module.aspects()[0].interactions[0];
        let (when, perform) = (
            attack.field("when").unwrap(),
            attack.field("perform").unwrap(),
        );

        let mut world = World::new();
        let actor = world.spawn(HealthState { current: 12.0 }).id();
        let mut healths = Vec::new();
        while module
            .run_formula::<bool>(&world, &host, actor, when)
            .unwrap()
        {
            module
                .run_effect(&world, &host, actor, perform)
                .unwrap()
                .apply(&mut world);
            healths.push(world.get::<HealthState>(actor).unwrap().current);
        }
        assert_eq!(healths, [7.0, 2.0, 0.0]);
        assert!(world.get::<HealthState>(actor).unwrap().dead());
    }

    /// A target with no health is logged and left alone.
    #[test]
    fn damage_to_what_has_no_health_does_nothing() {
        let mut world = World::new();
        let rock = world.spawn_empty().id();
        Damage {
            target: rock,
            amount: 5.0,
        }
        .apply(&mut world);
        assert!(world.get::<HealthState>(rock).is_none());
    }
}
