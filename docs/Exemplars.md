# Exemplar Architecture

An `Exemplar` is a blueprint or template for creating an entity. The concept is not new,
and is called by various names such as "schematic" in other frameworks.

The name `Exemplar` comes from _Sim City 4_ (and other Maxis "Sim" games). It was introduced
to me by the late Paul Pedriana, the lead engineer on the project. In particular, it reflects a
design philosophy in which game levels are not loaded as monolithic "scenes", but rather are
composed at runtime out of many individual parts, where each part is authored independently.
In _Sim City 4_, each type of building was defined by an exemplar; so was each editing tool
(for example, there was an exemplar for the "flatten terrain" sculpting mode).

Like ECS entities, exemplars have (almost) no properties by themselves. Instead, exemplars
contain a list of _apsects_, which are component-like objects.

Exemplars have the following features:

- **Serializable**: Exemplars support serde serialization and deserialization, and can be
  encoded in formats such as JSON or MsgPack.
- **Composable**: A given entity can only have a single exemplar, but the exemplar can
  combine multiple aspects. Different aspects represent different kinds of appearance or
  behavior. For example, a sword might be combination of an `Item` aspect (which allows
  it to be placed in the character's inventory), an `Equippable` aspect (which allows it
  to be held in the character's hand), and a `Weapon` aspect (which allows it to do damage).
- **Inheritable**: An exemplar can "extend" another exemplar. This is a form of _prototype
  inheritance_, a type of inheritance where an object inherits all the property values
  from its prototype. In the case of exemplars, what is inherited are all of the prototype's
  aspects.
- **Overrides**: An exemplar which extends another exemplar can also override specific aspects,
  or add new ones.
- **Editing**: Exemplars can be edited interactively, and changes will be immediately reflected
  in the game state. This means that all game entities which use an exemplar will be updated
  whenever any of the following happens:
  - An aspect is added or removed from an exemplar.
  - The properties of an aspect are modified.
- **Reflection**: All aspects support reflection via the `bevy_reflect` crate. This allows their
  properties to be edited interactively in an editor via a property grid.
- **Validation**: The JSON files that define an exemplar have comprehensive JSON-Schema files.
  This allows interactive editing in editors like Visual Studio Code, which permits both
  validation ("red squiggles") and autocompletion.

Note that the design of exemplars is still evolving. The design of exemplars reflects the needs
of Panoply, which is to have large game worlds with many interactive objects, many of whom share
a common prototype.

## Aspects

An exemplar is made up of a collection of `Aspects`.

`Aspects` can also be attached directly to a serialized entity without an exemplar. Many entities
will use a combimation of aspects which are inherited from the exemplar and ones that are directly
owned.

For example, portals are scenery elements that contain both a `Portal` and `PortalTarget` aspect.
The `Portal` aspect defines the shape and appearance of the portal aperture, and is generally
attached to the exemplar since there will be many instances of a given portal type. However,
each portal will reference a different target location, so the `PortalTarget` aspect is separate,
and is generally attached directly to the instance rather than to the exemplar.

Most aspects are also ECS `Components`: when the aspect is attached to an instance, a clone
of the aspect is inserted into the entity. However, this is not always the case: the `attach()`
method can insert multiple components or perform other operations on the entity. The only
requirement is that the changes be undoable, via the `DetachAspect` trait which is produced
during attachment.

## Loading Exemplars

To load an exemplar, you'll need to register the exemplar asset loader.

Once an exemplar is loaded, you can attach it to an entity by issuing the `UpdateAspects`
custom command. This command will merge the aspects from the instance, the schematic, and
any extension schematics, eliminating duplicate aspects. If the entity already had aspects
attached, then the command will do a "diff" of the old and new state, adding and removing
aspects as needed, while preserving the state of aspects that didn't change.

## Editing workflow

In most cases, the editor will not edit instances directly, but rather it will edit the assets
used to spawn those instances; the game engine will then update the instances (using asset
change detection) to reflect the new state. This avoids most of the problems of converting
runtime instance data back into a form which is serializable.

## Aspect example

Each `Aspect` has a Rust class which is constructable via reflection. The `PortalTarget` aspect
gives an example of a basic aspect:

```rust
/// Defines the remote location of a portal.
#[derive(Component, Debug, Reflect, Clone, Default)]
#[reflect(Aspect, Default)]
pub struct PortalTarget {
    pub(crate) realm: String,
    pub(crate) pos: Vector3,
}

impl Aspect for PortalTarget {
    /// The name to display in the editor.
    fn name(&self) -> &str {
        "PortalTarget"
    }

    /// Unique id for this aspect type, used to track the set of additions and removals.
    fn id(&self) -> TypeId {
        std::any::TypeId::of::<Self>()
    }

    /// Predicate function which defines what kind of instances the aspect can be attached to.
    /// This is used in the interactive aspect chooser UI when editing an instance or exemplar.
    fn can_attach(&self, meta_type: InstanceType) -> bool {
        meta_type == InstanceType::Wall || meta_type == InstanceType::Fixture
    }

    /// Attach an aspect to an instance. This must return a "detach" object, which is responsible
    /// for "undoing" the attach operation when the aspect is removed.
    fn attach(&self, entity: &mut EntityWorldMut) -> &'static dyn DetachAspect {
        static DETACH: SimpleDetachAspect<PortalTarget> = SimpleDetachAspect::<PortalTarget>::new();
        entity.insert(self.clone());
        &DETACH
    }

    /// Method to clone an aspect, used during instance deserialization.
    fn clone_boxed(&self) -> Box<dyn Aspect> {
        Box::new(self.clone())
    }
}
```

## Planned aspects

A list of aspects that are planned to be implement in panoply:

- `floor::StdSurface` - floor texture asset
- `floor::NoiseSurface` - floor procedural texture
- `floor::Geometry` - floor mesh options
- `floor::Nav` - pathfinding effects such as footpaths
- `scenery::Models` - list of glb models to display
- `scenery::Colliders` - physics colliders
- `scenery::Marks` - interaction marks
- `scenery::Container` - open / close / lock behaviors
- `scenery::Door` - open / close / lock
- `scenery::Stairs` - allows click-to-climb
- `scenery::Ladder` - allows click-to-climb
- `scenery::Sign` - click to read
- `scenery::PortalAperture` - portal dimensions
- `scenery::PortalTarget` - portal target location
- `scenery::LightSource` - point light source location
- `scenery::SoundSource` - ambient sound emitter
- `scenery::WallSize` - grid alignment options
- `mechanics::PushButton` - click to interact
- `mechanics::ToggleButton` - click to interact
- `mechanics::PressurePlate` - senses being walked on
- `mechanics::ControlledOpenable` - change state via remote signal
- `mechanics::AutoDoor`
- `trigger::Circle` - detects when player is within circle
- `trigger::Rect` - detects when player is within rect
- `trigger::Encounter` - increases chance of enemy spawn based on proximity
- `scenery::Waymark` - used for NPC scripted events
- `sfx::Music`
- `sfx::WaterFx`
- `sfx::Particles`
- `actor::Model`
- `actor::ColorSlots`
- `actor::Colors` - allows recoloring of an actor
- `actor::FeatureSlots`
- `actor::Features` - allows optional features (hat, beard, hair style)
- `actor::EquippedSlots`
- `actor::Equipped`
- `actor::Skills`
- `actor::Physics`
- `actor::Gender`
- `actor::Ally`
- `actor::Portrait`
- `actor::Goals`
- `inventory::Item` - appearance, weight, stack size, price
- `inventory::Container` - carrying capacity
- `inventory::Equippable` - equip slot
- `inventory::Weapon` - damage type, range
- `inventory::Document` - link to text content, page style
- `inventory::QuestItem` - quest id, stage
