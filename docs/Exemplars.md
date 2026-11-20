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

## Config and state

Open; PAN99 settles it. D88 divides an aspect's fields into config, what is authored, and state,
what play changes. But config is authored at two stages: on the exemplar, as a kind is designed, and
on the instance, as it is placed. A chest has all three:

- its container type, which background the inventory screen shows when it opens, set on the
  exemplar;
- its key id, which key unlocks it, set on the instance;
- whether it is open, changed in play.

An override replaces a whole aspect, so if the container type and the key id are fields of one
aspect, a chest that the silver key opens restates its container type, in a new exemplar or in an
instance override, and no longer follows the exemplar's.

Does an aspect need three sets of fields with distinct life cycles, or will two do, and are two
possible only with per-field overrides?

- **Three sets.** An aspect declares exemplar config, instance config and state apart, and an
  instance sets only its own. Against: a field's stage is fixed by the aspect's author, so the rare
  instance that wants another container type needs an exemplar of its own.
- **Two, with per-field overrides (X99).** An instance sets any field, and the rest stay inherited.
  Against: X99's cost, `Inherit<T>` resolved at evaluation, on every field an instance may set, and
  a property editor that shows each field as inherited or set, which is hard to present.
- **Two, split by aspect.** Fields set at different stages go in different aspects, as `Portal` and
  `PortalTarget` already do: the key id in `Lock`, the container type in `Container`, so overriding
  `Lock` leaves `Container` inherited. Against: the split is the aspect author's to get right, the
  aspect list grows, and a scripted aspect whose tuning is set on the exemplar and whose roles on
  the instance, as PAN79's relay logic, splits one concept across two aspects.

Leaning: two, split by aspect, with instances adding aspects their exemplars lack. Without that, a
chest and a lockable chest are two exemplars; with it, `Lock` is a mixin any chest takes. Adding an
aspect is a smaller lift than per-field overrides, since an added aspect shows in the property
editor as an aspect, not as a field's state.

- Bevy's required components do not express this. `Container` cannot require `Lock`, since not every
  container is locked, and `Lock` cannot require `Container`, since doors take it too. What the
  editor wants is the "suggested components" idea floated for Bevy and not taken far: aspects known
  to work with another, which the add menu offers first. Which side declares the pairing is open.
- A formula on the exemplar may read an aspect only some instances add: `Proto.Container`'s "Open"
  is offered not while locked, reading `LockState`. D71 checks a formula against its exemplar's
  chain, so the formula needs to read an absent aspect as a default, or the check moves to the
  instance. Leaning: the container's entries know about locks, reading a missing `LockState` as
  unlocked through SAGA's `Option` (`?.`, `??`; [saga.md](saga.md), Values), so D71 checks such a
  read as optional rather than composing entries from the aspects an instance has, which would ask
  far more of overriding.

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

Open for goal trees and presence; dialogue and remarks are settled (D86). It decides how X17's
behavior reaches an actor, and with it whether an instance may add aspects its exemplar lacks
(PAN28e).

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

### Dialogue and remarks

Dialogue composes apart from goals: a goal puts an actor into conversation, and what it can say
comes from one native aspect, `Dialogues { inherit, sets }`, a list of dialogue set assets.
Resolving it walks from the instance up the chain to the first `Dialogues` whose `inherit` is false,
and takes the sets farthest first, the order X17's union needs. `inherit` is true by default, so a
guard given a quest's lines lists only the quest's set; `inherit: false` starts over, which is how
an inherited set is dropped. `Proto.NPC` carries an empty `Dialogues`, so a guard's quest lines
override an aspect it has rather than add one. Remarks compose the same way. The panel shows the
list and the flag, and while the flag is on, the inherited sets above them, read-only.

Two mods each adding lines to one guard override the same `Dialogues`, and the later wins; merging
them is X74's.

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
config its systems read); a package's spawns goal trees. The aspect list, the union and the editor
work on definitions alone, so the native and external split moves from what an aspect is to how one
is implemented.

- Uniqueness moves to the components: no two aspects may attach the same component, checked as an
  exemplar loads and reported in the editor. Two packages never collide.
- Aspect identity no longer needs a Rust type, which X64 needs.
- X17's reason for keeping goals out of aspects, several goals of one type with different
  parameters, no longer constrains aspects.
- For native aspects the contract changes more than the code. Each of the 21 inserts a clone of
  itself, and most would keep doing so:
  - 11 are built on later by a system: `SceneryModels`, `SceneryColliders`, `SceneryAnimations`,
    `Skin`, `Armature`, `ClipNames`, `CollisionSize`, `ActorScale`, `Colors`, `Features` and
    `WieldedItems`. The work cannot happen in `attach`: a model loads after the aspects are applied,
    colliders are rebuilt in Rapier's world when an element moves and bodies when a model reloads,
    all through rebuild markers (`SceneryElementRebuildColliders`, `ActorRebuildModels`). The system
    then needs the definition per entity, as the instance resolved it over its chain, and the
    component is that resolved view; `Exemplar::find_aspect` reads the chain without the instance.
    So `attach` inserts a component holding the aspect's own data, as now.
  - `Interactions` is queried every frame, by the probe and picking; `Portal`, when it lands, by the
    engine. Both stay components.
  - `Container` and `Lock` supply play state (`ContainerState`, `LockState`), which D74 already
    keeps apart from the aspect.
  - `ItemModel` and `EquipSlot` are read from the exemplar, since an item in an inventory is not an
    entity. They are definitions already, and the components they insert go unread.
  - `WallSize`, `SceneryMarks` and `LightSource` are not read yet.

  So reading a native aspect from its definition by Rust type, along the chain, has to keep working.
  The registries already tell a component from the aspect that supplies it (`ScriptComponents` maps
  `ContainerState` to `Container`; interactions register actions by aspect). What keys on an
  aspect's type is the union (`TypeId`, in `UpdateAspects` and `OwnedAspects`), the editor (the
  short type path, in adding, removing, undo and each aspect's property root), and the files
  (`aspects` keyed by short type name). An aspect key replaces those three; keyed aspects and native
  and external aspects need the same change. Over native aspects this alternative and native and
  external aspects then do the same; they differ in what a package is called and where uniqueness is
  checked.
- D59's "an exemplar's aspects are the interface `self` has" becomes the components its aspects
  attach, which is what formulas read.
- A BSN entity is one component per type, so definitions, several of one kind among them, do not map
  onto it directly. They compile to it: an exemplar to a patch, `extends` to an `@` mixin, and
  aspects that can repeat to related entities, as X17's goal trees already are. Since native aspects
  stay components in practice, they map onto BSN as they do now; only packages need more.

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

**Bundles or lists per kind.** Settled for dialogue and remarks, which compose on their own, as
faery's `Converse` and `Remark` lists did. A bundle of goals, dialogue, remarks and presence would
have given a quest one name to travel and page by; a quest's goals and its lines are now edited in
two places.

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

### Packages written in SAGA

A variation of native and external aspects, or of aspects as definitions: an external aspect is a
SAGA source file, declaring its fields, goal trees, interactions and presence together, with
formulas inline, in place of an asset of data. It composes as any external aspect does; what changes
is how it is written. Two forms, unspecified:

- **Aspects as values.** A function or constant returns an aspect, and goal trees are SAGA values of
  types the native goals expose. Parameters are a package's fields, so its signature is the schema
  X64 needs, and the language builds trees (a patrol from a list of points) and reuses them. But a
  tree mixes two times: what the function computes once, as the package loads, and the conditions
  that must stay formulas, evaluated as the world changes, so SAGA has to tell them apart.
- **A declarative format.** A syntax outside functions, with SAGA embedded for conditions and
  effects, as BSN embeds Rust expressions. This extends SAGA less than it defines a format of its
  own, the bespoke format for goals that is the other way past X1. It is read without evaluating
  anything, so the editor sees its fields and trees, but it abstracts only as far as its syntax
  does.

Either way:

- faery's first goal editor drew trees and was abandoned: one diagram showing type, structure and
  execution at once is hard to read, and text keeps them apart. Its later trees were TSX, JSX
  without a compiler, and syntactically awkward; SAGA has a compiler, so its syntax can be designed
  for them.
- A package is checked as it compiles, and find references covers what it names. The native goal
  runtime runs its trees, so scripts still hold no memory but the world's.
- Behavior then needs no BSN (X1), and no formulas embedded in assets.
- A field declared in SAGA is a type declared in data, so setting one per actor waits on the layer
  X64 needs, as an external aspect's fields do. Values that vary by placement are mostly references:
  a patrol path is a fixture, edited in the world, which a guard package's field names (naming
  across precincts is PAN79's).
- Against: SAGA grows from formulas to declarations, with tooling to match, and a file that fails to
  compile loses its character's behavior whole, where a bad field loses one value.

### Dialogue text

Under any alternative, dialogue is an asset type of its own, and its format is a question apart.
faery's was first written as JSON, a large form of quotes and commas, then in the property editor,
which was better but no help in following transitions. Ink and Yarn write dialogue as text made for
prose: a line is the line, a node a heading, an option a marked line, a condition a formula, a jump
a name. Yarn Spinner shows the transitions in a graph generated from the text, each node linking to
its lines, so the graph shows flow alone. Stages, topics, flags and other sets would resolve as the
text compiles, and lines need stable ids, mostly generated, so translations survive edits.

### Leaning

Aspects as definitions, or native and external aspects, over a list in one aspect: composition stays
in the aspect list with one merge. Both put external pieces in the aspect list, so the rule on
instances adding aspects is settled with this, not after. Packages written in SAGA are against: they
disrupt much for an uncertain return.

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
