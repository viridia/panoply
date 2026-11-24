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

An aspect is known by its key (`Aspect::key`): a native aspect's short type name, a scripted one's
package path and name (D107). A native aspect implements `NativeAspect`, whose `Config` bundle
attaching inserts and detaching removes: usually the aspect itself, `()` when nothing queries it.
Its state, if it has any, `attach_state` inserts where the entity lacks it, and detaching leaves. A
scripted aspect, `ScriptedAspect`, is declared in a `.saga` module ([saga.md](saga.md), Aspects) and
behaves the same way: its config is a component that attaching inserts and detaching removes, and
its state one that attaching inserts where the entity lacks it.

## Config and state

An aspect's fields are config, what is authored, or state, what play changes (D88). Config is
authored at two stages: on the exemplar, as a kind is designed, and on the instance, as it is
placed. A chest has all three:

- its container type, which background the inventory screen shows when it opens, set on the
  exemplar;
- its key id, which key unlocks it, set on the instance;
- whether it is open, changed in play.

Fields set at different stages go in different aspects (D103): the container type in `Container`,
the key id in `Lock`, as `Portal` and `PortalTarget` already split a portal's shape from its target.
An override replaces a whole aspect, so an instance that overrides `Lock` leaves `Container`
inherited, and no field inherits on its own (X99).

An instance may add aspects its exemplar lacks, and never removes an inherited one (D104). So `Lock`
is a mixin any chest takes, and a lockable chest needs no exemplar of its own. An added aspect shows
in the property editor as an aspect, beside the inherited ones.

- Bevy's required components do not express this. `Container` cannot require `Lock`, since not every
  container is locked, and `Lock` cannot require `Container`, since doors take it too. What the
  editor wants is the "suggested components" idea floated for Bevy and not taken far: aspects known
  to work with another, which the add menu offers first. Which side declares the pairing is open;
  whichever it is, native and scripted aspects declare it the same way (D107).
- A formula on the exemplar may read an aspect only some instances add: `Proto.Container`'s "Open"
  is offered not while locked, reading `LockState`. The container's entries read a missing
  `LockState` as unlocked through SAGA's `Option` (`?.`, `??`; [saga.md](saga.md), Values), and D71
  checks such a read as optional, rather than composing entries from the aspects an instance has.
- A scripted aspect whose fields span both stages, as a sign's frame and text or a relay's tuning
  and roles, declares its instance's fields in an `instance` block, which is an aspect of its own
  (PAN104). A native aspect names its instance part the same way, as `Portal` names `PortalTarget`,
  and the editor shows both kinds of pair as one group with two override states (D107); a part with
  a field that has no default is one the instance must supply, checked as it spawns.

## Loading Exemplars

To load an exemplar, you'll need to register the exemplar asset loader.

Once an exemplar is loaded, you can attach it to an entity by issuing the `UpdateAspects`
custom command. This command will merge the aspects from the instance, the exemplar, and
any extension exemplars, eliminating duplicate aspects. If the entity already had aspects
attached, then the command will do a "diff" of the old and new state, adding and removing
aspects as needed, while preserving the state of aspects that didn't change.

The entity's `OwnedAspects` holds the definition of each aspect attached, shared with the asset
(`Arc<dyn Aspect>`), in the order they were resolved. An aspect whose key is no longer resolved is
detached through its old definition. Gathering like data from aspects of different kinds iterates
it, calling trait methods.

## Editing workflow

In most cases, the editor will not edit instances directly, but rather it will edit the assets
used to spawn those instances; the game engine will then update the instances (using asset
change detection) to reflect the new state. This avoids most of the problems of converting
runtime instance data back into a form which is serializable.

## Composing behavior

Open for goal trees; dialogue and remarks are settled (D86), and so is presence (D106). It decides
how X17's behavior reaches an actor. An instance may add aspects its exemplar lacks (D104), so
whatever a piece of behavior becomes, an instance can take one.

An actor's behavior is goal trees, dialogue sets, remark sets and presence, and a character combines
pieces of it that vary independently: a town guard has the guards' patrol and dialogue, the town's
regional lines, and one guard also a quest's questions and remarks. faery's guardsman
(`packages/game/src/behaviors/demo.tsx`) lists three dialogue sets: generic, regional and quest.
faery wrote an aspect type per character, and so does Panoply: the union keys aspects by
`Aspect::key` (D107), so each piece of behavior is a scripted aspect with a key of its own, and two
pieces never displace each other as the nearest of one type would.

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

Two mods each adding lines to one guard override the same `Dialogues`, and the later wins; quest
roles would let both survive (X123).

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
and gating an interaction on one (D114), are excluded above.

**Keyed aspects.** An aspect type may name a key field, and the union keys on type and key. One
reflected type, `BehaviorPackage { path }`, keyed by `path`, adds its path to a runtime component
when attached. No new merge rule: packages appear in the aspect list, inherit along `extends`, and
show as inherited or overridden as other aspects do. Against: whatever iterates aspects by type
(D71's checks, the add menu, saving) must learn keys, and the grid shows a package as a struct with
a `path` field.

**Native and scripted aspects.** Keyed aspects in their best form: a native aspect is a Rust type
and a component; a scripted aspect is declared in a SAGA module, keyed by its module's path and its
name, its config and state each a component of its own (D107, D113). The list shows `tambry:poul`
itself. Both share the aspect list, the union, adding and removing, and the inherited display, so
most of what an author touches is shared. faery's aspects split the same way
([modding.md](modding.md), What mods need): its 15 per-character scripts had no fields. It is two
mechanisms presented as one, and the presentation shows at the seams:

- A scripted aspect's fields are types declared in data, which the property grid edits only through
  the layer X64 needs (PAN104b).
- A formula reads a scripted aspect through its accessor (D117); one without an accessor is not part
  of `self`'s interface.
- Detaching one despawns its goal trees and drops its saved state, and its formulas need their own
  check, as X96's do.

Mods author scripted aspects, which gives them aspects without declaring Rust types.

**Aspects as definitions.** An aspect stops being a component: it is a definition in an exemplar or
instance, keyed by a Rust type's name or an asset path, and what it does when attached is its own. A
native aspect's `attach` inserts components (its state, as D74 already has `ContainerState`, and the
config its systems read); a package's spawns goal trees. The aspect list, the union and the editor
work on definitions alone, so the native and scripted split moves from what an aspect is to how one
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
  `ContainerState` to `Container`; interactions register actions by aspect). The union
  (`UpdateAspects`, `OwnedAspects`), the editor and the files know an aspect by its key
  (`Aspect::key`), not its type, as keyed aspects and native and scripted aspects both need. Over
  native aspects this alternative and native and scripted aspects then do the same; they differ in
  what a package is called and where uniqueness is checked.
- D114's "an exemplar's aspects are the interface `self` has" becomes the components its aspects
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

**Quest-side composition.** The other alternatives put a piece of behavior on the actor that has it.
This one lets a quest name the actors it involves and carry what it gives each, as the Creation
Kit's quest aliases do: a role, bound to an actor by name, holding goals, lines and presence. A role
names its actor by key, which needs no precinct's path, and the actor stays a precinct actor: it
takes the role's bindings as it spawns ([instances.md](instances.md), Instances take their bindings
from quests), so the quest never needs it loaded. The binding is authored and static, so behavior is
still never added or removed at runtime; the quest's stage gates what a role gives through `when`,
as any goal is gated. What a role gives joins the actor's own in one union, competing by priority,
its dialogue sets after the actor's own. It is a relay for actors: PAN79's argument for keeping
wiring off its participants applies, and a quest's goals and lines travel as one bundle again, and
two mods that each give one guard a quest both survive, since neither edits the guard. Against:
inspecting an actor does not show what quests do to it, so the editor lists the quests that name an
actor, as PAN79 lists relays; and a role's target may be missing (D47).

A quest changes scenery too, and the choice is the same there: realm layers that appear at a stage
(X109), and stateful walls and fixtures, such as a castle's portcullis open only once a stage is
reached. Each is a reactive formula testing the quest. World-side, it is written on the thing, a
layer's `when` as X109 has it or a formula on the portcullis's open state, naming the quest, which
is global; the logic is spread across the world's data. Quest-side, the quest names the thing and
gives its condition, as a role does an actor: a layer by realm and name, a wall or fixture by key,
which the thing pulls as it spawns, as an actor does ([instances.md](instances.md)). A sketch, in a
`.saga` module beside scripted aspects:

```rust
quest UpInArms {
    state stage: Stage = Stage::Offered,

    role sergeant = actor("stone") {
        present: stage < Stage::Complete,
        lines: ["tambry.dialogue#Stone.UpInArms"],
    }

    role leader = actor("keep-leader") {
        goals {
            Attack { when: self.sees(player()), target: player(), priority: Mission }
        }
    }
}
```

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

A variation of native and scripted aspects, or of aspects as definitions: a package is a scripted
aspect (PAN104), declaring its fields, goal trees, interactions and presence together, with formulas
inline, in place of an asset of data. It composes as any other aspect does; what changes is how it
is written. Two forms:

- **Aspects as values.** A function or constant returns an aspect, and goal trees are SAGA values of
  types the native goals expose, written as literals, as a Python dict display is. Parameters are a
  package's fields, and the language builds trees (a patrol from a list of points) and reuses them.
  But a tree mixes two times: what the function computes once, as the package loads, and the
  conditions that must stay formulas, evaluated as the world changes, so SAGA has to tell them
  apart, and the editor sees a tree only by running code. Against.
- **Declarations.** A tree is declared, as a scripted aspect is, with formulas embedded, as BSN
  embeds Rust expressions. Its shape is static, read without evaluating anything, so the editor sees
  its fields and trees. Whether a field is a formula is the goal type's: a `Formula<bool>` such as
  `when` compiles its expression as a formula, re-evaluated as the world changes; any other field
  takes a constant or a parameter of its tree, fixed as the tree is instantiated, as BSN's props
  are. A `tree` declares its parameters with types, and a node that publishes a value to its subtree
  is bound by name (`as interaction`). Nodes nest as QML's objects do, children in `[ ... ]` as in
  BSN. faery's Ivy, sketched:

  ```rust
  aspect Ivy {
      goals {
          Converse { when: self.targeted_by_player, priority: Social }
          Sleep { when: hour_between(11, 7), priority: Health, target: fixture("ivy.bed") }
          Sequence { loop: true } [
              Random { norepeat: true } [
                  UseFixture { fixture: "inn.cookpot", distance: 0.4,
                               tool: StirringSpoon, clips: [KitchenStir, KitchenWork] }
                  UseFixture { fixture: "inn.worktable.0", distance: 0.6, ... }
                  UseBroom { area: "ivy.sweep" }
                  Contingent [ WalkTo { target: "ivy.counter" }  Wait { delay: 10 } ]
              ]
              Wait { delay: 2 }
          ]
      }
  }

  tree UseFixture(fixture: FixtureRef, mark: String = "use", distance: f32,
                  tool: ItemRef, clips: [Clip]) {
      SceneryInteraction { target: fixture, mark, distance } as interaction [
          Contingent [
              LookAt { target: interaction.position }
              Equip { item: tool, instant: true }
              Pose { delay: 4, clips, repeat: true }
              Unequip { instant: true }
          ]
      ]
  }
  ```

  It answers most of what [goal-format.md](goal-format.md)'s JSON leaves open: parameters are typed
  in the signature, scoped values are bindings, faery's four work trees are one, and it is about as
  long as faery's TSX.

Either way:

- faery's first goal editor drew trees and was abandoned: one diagram showing type, structure and
  execution at once is hard to read, and text keeps them apart. Its later trees were TSX, JSX
  without a compiler, and syntactically awkward; SAGA has a compiler, so its syntax can be designed
  for them.
- A package is checked as it compiles, and find references covers what it names. The native goal
  runtime runs its trees, so scripts still hold no memory but the world's.
- Behavior then needs no BSN (X1), and no formulas embedded in assets.
- A field declared in SAGA is a type declared in data, so setting one per actor goes through the
  layer X64 needs and an `instance` block (PAN104b). Values that vary by placement are mostly
  references: a patrol path is a fixture, edited in the world, which a guard package's field names
  (naming across precincts is PAN79's).
- Each declaration fails alone, as a formula in an asset does: a `tree` that fails to compile
  disables the aspects that use it, not its module.
- Against: SAGA grows from formulas to declarations, with tooling to match, though scripted aspects
  (PAN104) already pay most of it.

### Worked example: three quests

The first three quests of one chain, beats told only as far as the engine sees them. **Asset** is
content, **formula** is SAGA, **native** is Panoply's Rust. Where the two leading forms differ,
**actor-side** is packages as scripted aspects on the actors (Packages written in SAGA, with D104)
and **quest-side** is roles (above).

The first quest, *Up in Arms*:

1. **Beforehand.** *Asset:* the quest, with its stages and its state. A sergeant placed in Tambry's
   inn; animals, a squad and its leader placed in an abandoned keep; a switch and a door, wired by a
   relay (PAN79); a ledger placed on a table; a letter in the leader's inventory. The animals and
   the squad attack on sight through a generic hostile package on their exemplars, the same either
   way.
2. **The sergeant offers it.** *Native:* the player targets him; his conversation goal draws on his
   dialogue sets (X17). *Asset:* the quest's lines. Actor-side, his instance lists them in
   `Dialogues` (D86); quest-side, his role does. *Formula, effect:* the accepting option starts the
   quest. *Formula, on demand:* until the player returns with the letter, an option asks him about
   the task, offered across every step between.
3. **The keep.** *Native:* combat, routing through the switched door (PAN16j), the relay (PAN79). No
   quest behavior; nothing differs.
4. **The leader.** *Asset:* her goal, on sight, to bark and attack. Actor-side, a package on her
   instance, as faery's `heroncreek:cwLeader` was; quest-side, her role.
5. **The ledger and the letter.** *Native:* taking an item; looting a corpse (PAN106). *Formula,
   reactive:* the stage advances when the player holds the ledger.
6. **Paid, and gone.** *Formula, on demand:* the sergeant's option is offered while the player holds
   the letter. *Formula, effect:* it takes the letter, pays, and advances the stage. *Asset:* his
   presence, ending once the next stage is accepted (D106); he stays where he is until then, and the
   latch keeps him until he is off the screen, so he never leaves in view and needs no goal to walk
   out. Actor-side, his instance holds it, and each later appearance is another instance with its
   own (R85); quest-side, each chapter's quest binds its own sergeant.

The second, at an inn in Marheim:

7. **The way in.** *Formula, on demand:* the innkeeper's option naming the sergeant is offered only
   once the first quest is done, reading its stage.
8. **The guards' own day.** *Asset:* a patrol package on the guards' exemplar: a route through the
   district, and a shift break at the inn by the hour (R89). Persistent, placed, the same either
   way. *Native:* routes through doors (PAN16j), fixtures named across precincts (PAN79).
9. **Waiting.** *Asset:* patrons' lines that change with the stage: entries in their sets with
   conditions on it, the same either way.
10. **The shakedown and the duel.** *Asset:* goals on three of those guards, only while the quest is
    at its evening stage: approach the player and start a conversation; for the leader, an unarmed
    duel; for his men, to watch. Actor-side, a package added on each of three instances, placed in
    whatever precinct their patrol starts in; quest-side, three roles in the quest. *Native:* the
    duel, and a yield the stage reads.
11. **After.** *Native:* the quest goals' `when` fails and the patrol package wins again. *Formula,
    reactive:* later quests read this one's stage, which is the grudge.

The third, at a boarded-up shop:

12. **Getting in.** *Asset:* the shop's door with a `Lock`, and a way to force it, as an interaction
    of its scripted `Lock` (D114) whose effect clears native lock state (D105).
13. **The evidence.** *Asset:* a notice on the door and records inside, documents the player takes
    (PAN78). *Formula, reactive:* the stage advances once the player holds both. A clue is an object
    the player carries, never a fact only read, so the player never knows what their character does
    not, and the quest reads it as possession.
14. **The official.** *Asset:* four options, one gated on an item the player found and shows.
    *Formula, effect:* each advances the stage and records the method chosen, which later quests
    read.
15. **Released.** *Asset:* the freed weaver placed in the town, present once released, and another
    in the camp, present until then (R85). Actor-side, each instance holds a `Presence`; quest-side,
    each is a role.

What the walk shows:

- **Behavior falls in two kinds.** A character's own (the patrol, the hostility, a character's
  habits) belongs on its exemplar, the same in both forms. What exists because of a quest (steps 2,
  4, 6, 10, 15) is where the forms differ, and it is most of what a quest adds. The same holds for
  scenery a quest changes (Quest-side composition, above), which these quests do not exercise.
- **Actor-side scatters a quest.** Step 10 edits three instances, wherever their patrol starts, and
  a quest's goals, lines and presence spread over each instance it touches; a second quest for the
  same guard edits him again. Quest-side keeps each quest in one place.
- **Each side references the other's things.** Actor-side, an actor's package names quests, which
  are global already, by name. Quest-side, a quest names actors, by key, and each takes what its
  roles give as it spawns, so the actors stay in their precincts ([instances.md](instances.md),
  Instances take their bindings from quests).
- **A quest's state is its stage alone.** Objectives that complete in any order are subquests, and
  the method chosen in step 14 is a branch, its own span of stages ([instances.md](instances.md),
  Quest state).
- **What a quest gives an actor spans its states** (step 2's question about the task). It is listed
  once, for the quest's whole life, and each entry conditions itself by a formula over the quest's
  state, as a dialogue set's entries already do (X17); by default a quest gives it while it is
  offered or in progress, so few entries repeat that.
- **Clues are possessions** (steps 6, 13), so quests read inventory through host functions and need
  no record of what the player has read.
- **A quest's roles apply from the start of play,** before it is offered: the sergeant offers it.
  Its `when`s and presence keep them inert until a stage asks.
- **A missing target** (D47): a role whose instance is gone stays unbound and is logged, as a
  missing fixture is; actor-side, a package whose module is missing waits on X20.

### Dialogue text

Under any alternative, dialogue is an asset type of its own, and its format is a question apart.
faery's was first written as JSON, a large form of quotes and commas, then in the property editor,
which was better but no help in following transitions. Ink and Yarn write dialogue as text made for
prose: a line is the line, a node a heading, an option a marked line, a condition a formula, a jump
a name. Yarn Spinner shows the transitions in a graph generated from the text, each node linking to
its lines, so the graph shows flow alone. Stages, topics, flags and other sets would resolve as the
text compiles, and lines need stable ids, mostly generated, so translations survive edits.

### Leaning

Settled by D107: native and scripted aspects, packages among the scripted ones, with no package
asset type of their own, and composition kept in the aspect list, where an instance can add them
(D104). Aspects as values are against.

After the worked example, both sides of composition, by kind: a character's own behavior is a
package on its exemplar, and what exists because of a quest is a role in that quest, declared in
SAGA as packages are, joining the actor's union as the actor spawns ([instances.md](instances.md)).
Instances then rarely take packages; D104 still lets them.

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

impl NativeAspect for PortalTarget {
    /// What attaching inserts and detaching removes.
    type Config = Self;

    fn config(&self) -> Self {
        self.clone()
    }

    /// Which kinds of instance the aspect can be attached to, as the aspect chooser offers it.
    fn can_attach(&self, kind: InstanceKind) -> bool {
        matches!(kind, InstanceKind::Wall | InstanceKind::Fixture)
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
