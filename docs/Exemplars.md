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
custom command. This command will merge the aspects from the instance, the exemplar, and
any extension exemplars, eliminating duplicate aspects. If the entity already had aspects
attached, then the command will do a "diff" of the old and new state, adding and removing
aspects as needed, while preserving the state of aspects that didn't change.

This `diffing` is facilitated by the `OwnedAspects` component, which is a bookkeeping component
on the entity that stores the `Detach` trait object for each aspect that has been attached to the
entity. `OwnedAspect` is a newtype struct which contains a `HashMap<TypeId, &'static dyn Detach>`.
This allows any aspect to be removed from the entity simply by knowing it's type id.

## Editing workflow

In most cases, the editor will not edit instances directly, but rather it will edit the assets
used to spawn those instances; the game engine will then update the instances (using asset
change detection) to reflect the new state. This avoids most of the problems of converting
runtime instance data back into a form which is serializable.

## Composing behavior

Open. It decides how X17's behavior reaches an actor, and with it whether an instance may add
aspects its exemplar lacks (PAN28e).

An actor's behavior is goal trees, dialogue sets, remark sets and presence, and a character combines
pieces of it that vary independently: a town guard has the guards' patrol and dialogue, the town's
regional lines, and one guard also a quest's questions and remarks. faery's guardsman
(`packages/game/src/behaviors/demo.tsx`) lists three dialogue sets: generic, regional and quest.
faery wrote an aspect type per character, which Panoply cannot: `UpdateAspects` keys aspects by
`TypeId`, the nearest of each type winning, so two pieces of behavior cannot both be aspects of one
type.

Whatever the mechanism, these hold:

- A piece of behavior is an asset named by path, so a precinct's table can hold it while the
  precinct is live (X115).
- Behavior is authored, never added or removed at runtime. A higher-priority goal overrides an
  inherited one, and a `when` switches one off.
- Formulas cannot ask which pieces an actor has, since that would tie content to how its files are
  split. Role, faction and alliance are native aspects.
- Dialogue and remarks draw on the union of the actor's sets, by the rules in X17, so the order of
  the pieces matters: farthest ancestor first, the instance last. `UpdateAspects` walks the other
  way, instance first.

### Alternatives

**A list in one aspect.** One native aspect, `BehaviorPackages`, lists the actor's packages; a merge
annotation makes the list append along `extends` and the instance, dropping duplicates. Aspects stay
one per component, and a package is a plain asset reference. Against: a second merge rule, inside
one field, unlike every other field; packages are absent from the aspect list where the rest of an
actor's makeup is; the editor has to show inherited entries inside the field. A variation drops the
merge: when the aspect is added to an instance, the editor fills it with the inherited list.
Overrides stay uniform (the list replaces), the file shows the whole list, and removing or
reordering an inherited package is free. But the copy goes stale: when the guard package gains a set
or is split, every guard holding a copy must be edited.

**A dynamic component per package.** Each package registers a zero-sized component at load
(`register_component_with_descriptor`, `insert_by_id`), and the aspect union, keyed by
`ComponentId`, composes packages unchanged. A package is then an aspect in every sense, queryable by
Bevy. Against: component ids are per run, so files and saves still use names; the editor, the
serializer and `Aspect::id` assume Rust types. Its distinctive gains, querying an actor by package
and gating an action on one (D59), are excluded above.

**Keyed aspects.** An aspect type may name a key field, and the union keys on type and key. One
reflected type, `BehaviorPackage { path }`, keyed by `path`, adds its path to a runtime component
when attached. No new merge rule: packages appear in the aspect list, inherit along `extends`, and
show as inherited or overridden as other aspects do. Against: whatever iterates aspects by type
(D71's checks, the add menu, saving) must learn keys, and the grid shows a package as a struct with
a `path` field.

**Native and external aspects.** Keyed aspects in their best form: a native aspect is a Rust type
and a component; an external aspect is an asset, keyed by its path and collected into one component.
The list shows `tambry:poul` itself. Both share the aspect list, the union, adding and removing, and
the inherited display, so most of what an author touches is shared. faery's aspects split the same
way ([modding.md](modding.md), What mods need): its 15 per-character scripts had no fields. It is
two mechanisms presented as one, and the presentation shows at the seams:

- An external aspect has no fields until the property grid can edit types declared in data, the
  layer X64 needs, which Panoply would build itself; until then per-instance data goes in a native
  aspect or the variables aspect (X53).
- External aspects are not part of `self`'s interface: formulas cannot see them, and D59's actions
  need native ones.
- Order matters for external aspects only.
- Detaching one despawns its goal trees and drops its saved state, and its formulas need their own
  check, as X96's do.

Mods could author external aspects, which gives them aspects without declaring Rust types (X64).

**Aspects as definitions.** An aspect stops being a component: it is a definition in an exemplar or
instance, keyed by a Rust type's name or an asset path, and what it does when attached is its own. A
native aspect's `attach` inserts components (its state, as D74 already has `ContainerState`, and the
config its systems read); a package's spawns goal trees and registers its dialogue and remark sets.
The aspect list, the union and the editor work on definitions alone, so the native and external
split moves from what an aspect is to how one is implemented.

- Uniqueness moves to the components: no two aspects may attach the same component, checked as an
  exemplar loads and reported in the editor. Two packages never collide.
- Aspect identity no longer needs a Rust type, which X64 needs.
- X17's reason for keeping goals out of aspects, several goals of one type with different
  parameters, no longer constrains aspects.
- For native aspects little code changes. Each of the 21 inserts a clone of itself (`Lock` and
  `Container` their state as well), and keeps doing so: the engine still queries for a `Portal`,
  inserted by the portal aspect. Of the 21, the engine queries 12 as components; `Container` and
  `Lock` only through their state; `TerrainEffect`, `TerrainHole`, `ItemModel` and `EquipSlot` it
  reads from the exemplar (`Exemplar::find_aspect`), since terrain vertices and an item in an
  inventory are not entities; three it does not read yet. So reading a native aspect from its
  definition by Rust type, along the chain, has to keep working. The registries already tell a
  component from the aspect that supplies it (`ScriptComponents` maps `Lid` to `Lidded`;
  interactions register actions by aspect). What keys on an aspect's type is the union (`TypeId`, in
  `UpdateAspects` and `OwnedAspects`), the editor (the short type path, in adding, removing, undo
  and each aspect's property root), and the files (`aspects` keyed by short type name). An aspect
  key replaces those three; keyed aspects and native and external aspects need the same change.
- D59's "an exemplar's aspects are the interface `self` has" becomes the components its aspects
  attach, which is what formulas read.
- A BSN entity is one component per type, so definitions, several of one kind among them, do not map
  onto it directly. They compile to it: an exemplar to a patch, `extends` to an `@` mixin, and
  aspects that can repeat to related entities, as X17's goal trees already are.

**Mixins.** An exemplar extends several others (`extends: [guard, lunch-quest]`), and a package is
an exemplar without a model. This is BSN's own composition: what it calls inheritance is a mixin, an
`@` patch, applied any number of times, in order. One concept covers kinds and behavior, and reuse
along independent axes is native. But patches merge by field, and a list is one field, so two mixins
each listing goals still leave one list: repeatable pieces need keys, a list merge or related
entities anyway. And it puts things that cannot be placed in catalogs of things that can. It is how
the others would compile to BSN rather than a rival to them.

**Single `extends`.** All behavior inline in a character's own exemplar, reused by a chain (village
guard, Tambry guard, lunch guard). The simplest, and faery's guardsman fits it. But a chain is one
axis: a Tambry innkeeper and a Tambry guard both want Tambry's regional lines from different role
branches, which means duplication or a chain per combination. And behavior inline in a catalog stays
resident with it.

**Bundles or lists per kind.** For any of these, the piece can be a bundle of goals, dialogue,
remarks and presence, or each kind can compose on its own, closer to faery's `Converse` and `Remark`
lists. A bundle gives a behavior one name, so a quest's goals, lines and remarks travel and page
together; a bundle holding only dialogue gives the finer reuse anyway. Either way a bundle
references dialogue set assets, since dialogue is an asset type of its own.

### BSN

Stock BSN composes by mixins, which merge by field, and holds one component per type. Panoply's
overrides replace a whole aspect, nearest first, so moving to BSN changes what an override means
whichever alternative is chosen. Each alternative beyond that adds a list merge, a key, or an aspect
that is not a component. Bevy's editor AST does not exist yet (X1), so Panoply owns the in-memory
form of the BSN it uses, and that form can hold whichever model is chosen, compiling it to patches.
The alternatives are judged on that basis.

What owning the form costs does not depend on the model. D73's merge works on `serde_json`'s value
tree, comparing the exemplars serialized as loaded and as now, and knows nothing of aspects. BSN has
no such tree, so owning the form means one for the subset Panoply uses, with a parser and a printer
that keeps a file's formatting, and a serializer from reflected values into it; the merge carries
over.

### Leaning

Aspects as definitions, or native and external aspects, over a list in one aspect: composition stays
in the aspect list with one merge. Both put external pieces in the aspect list, so the rule on
instances adding aspects is settled with this, not after.

## Aspect example

Each `Aspect` has a Rust class which is constructable via reflection. The `PortalTarget` aspect
gives an example of a basic aspect:

```rust
/// Defines the remote location of a portal.
#[derive(Component, Debug, Reflect, Clone, Default)]
#[reflect(Aspect, Default)]
pub struct PortalTarget {
    pub(crate) realm: String,
    pub(crate) pos: Vec3,
}

impl Aspect for PortalTarget {
    /// The name to display in the editor.
    fn name(&self) -> &str {
        "PortalTarget"
    }

    /// Predicate function which defines what kind of instances the aspect can be attached to.
    /// This is used in the interactive aspect chooser UI when editing an instance or exemplar.
    fn can_attach(&self, meta_type: InstanceType) -> bool {
        meta_type == WALL_TYPE || meta_type == FIXTURE_TYPE
    }

    /// Attach an aspect to an instance. This must return a "detach" object, which is responsible
    /// for "undoing" the attach operation when the aspect is removed.
    fn attach(&self, entity: &mut EntityWorldMut) -> &'static dyn DetachAspect {
        static DETACH: RemoveComponent<PortalTarget> = RemoveComponent::<PortalTarget>::new();
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
- `actor::Colors` - allows recoloring of an actor
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
