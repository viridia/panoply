use bevy::{ecs::component::ComponentId, prelude::*};
use std::any::Any;

use super::InstanceType;

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

    /// Returns this aspect as an 'any'.
    fn as_any(&self) -> &dyn Any;

    /// Get the ECS component id of this aspect.
    fn component_id(&self, world: &mut World) -> ComponentId;

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
