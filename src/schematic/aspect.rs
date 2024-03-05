use bevy::{asset::LoadContext, prelude::*, utils::HashMap};
use std::any::TypeId;

use super::InstanceType;

/// Object which can remove an aspect from an entity.
pub trait DetachAspect: Send + Sync {
    /// Get the [`TypeId`] for this aspect.
    fn type_id(&self) -> TypeId;

    /// Remove the aspect from the entity.
    fn detach_aspect(&self, entity: &mut EntityWorldMut);
}

/// An `DetachAspect` that removes a specific component from an entity.
pub struct SimpleDetachAspect<T: Component> {
    _marker: std::marker::PhantomData<T>,
}

impl<T: Component> SimpleDetachAspect<T> {
    pub const fn new() -> Self {
        SimpleDetachAspect {
            _marker: std::marker::PhantomData,
        }
    }
}

impl<T: Component> DetachAspect for SimpleDetachAspect<T> {
    fn type_id(&self) -> TypeId {
        TypeId::of::<T>()
    }

    fn detach_aspect(&self, entity: &mut EntityWorldMut) {
        entity.remove::<T>();
    }
}

/// An Aspect is like an ECS component for a prototype.
#[reflect_trait]
pub trait Aspect: Reflect
where
    Self: 'static + Sync + Send,
{
    /// Type name of this aspect
    fn name(&self) -> &str;

    /// Whether this aspect can be attached to an instance of the given type.
    fn can_attach(&self, meta_type: InstanceType) -> bool;

    /// Get the [`TypeId`] for this aspect.
    fn id(&self) -> TypeId;

    /// Load any dependencies required by this aspect.
    #[allow(unused_variables)]
    fn load_dependencies(&mut self, label: &str, load_context: &mut LoadContext) {}

    /// Attach or apply this aspect to the given entity.
    fn apply(&self, entity: &mut EntityWorldMut) -> &'static dyn DetachAspect;

    //   /** Configuration parameters for this aspect. */
    //   config?: IPropertyDescriptors<Config>;

    //   /** Properties which are added to instances that attach this behavior. */
    //   properties?: IPropertyDescriptors<Props>;

    //   /** Formulas to be bound to the specified properties */
    //   formulas?: IFormulaGenerators<SelfType, Props, Config>;

    //   /** Ways in which the player can interact with this instance. */
    //   interactions?: IInteraction<SelfType, Props, Config>[];

    //   /** List of quest roles associated with this actor. */
    //   questRoles?: string[];

    //   /** A task which runs when instance is instantiated and is run in the instance scope.
    //       This scope will get destroyed when the instance is unloaded.
    //    */
    //   init?: (self: SelfType, props: Props, config: Config) => void;

    //   /** Generates list of goals to be associated with this behavior (actors only) */
    //   goals?: (self: SelfType, props: Props) => GoalChildren;

    //   /** Method invoked when the actor is not doing anything. Returns the type of animation
    //       that should be run when idling.
    //    */
    //   idle?: (self: SelfType, props: Props) => string | undefined;

    //   /** Predicate function for sensor instances. */
    //   canSense?: (self: SelfType, props: Props, target: PositionableInstance) => boolean;

    //   /** If defined, this actor is conditional - only appears in specific quest stages. */
    //   present?: (self: SelfType, props: Props) => boolean;
}

#[derive(Component)]
pub struct OwnedAspects(pub(crate) HashMap<TypeId, &'static dyn DetachAspect>);
