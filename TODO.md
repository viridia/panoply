# Migration

- Scripting
- Revive editor

# TODO

- Better picking on tiers - disable terrain picks
- wall draw sometimes kicks into remove mode - missing exemplar ref?
- Terrain contour list.
  - filter by group
  - contours not appearing in the correct order in groups.
  - empty tile shows nothing, should show black floor.
- Possibly move to common crate:
  - inline_assets?
  - reflect_types

Future:

- Floor surface texture rotation
- Unit tests for HexColor.
- Editor-only floor surfaces (for things like water currents / hints)

# Bevy Issues

- [Reflection type aliases](https://github.com/bevyengine/bevy/issues/12387)
- [Support archive assets](https://github.com/bevyengine/bevy/issues/12279)
- [Allow missing assets](https://github.com/bevyengine/bevy/issues/12210)

# Project hierarchy

- actors
  - Action
  - [x] Actor
  - ActorTemplate
  - ActorArchetype
  - ActorAffix
  - goals
    - components
      - Activate
      - ApplySkill
      - Attack
      - Contingent
      - Deactivate
      - Dialogue
      - Equip
      - FaceToward
      - LookAt
      - Park
      - Pose
      - Pursue
      - Random
      - Ranked
      - Remark
      - SceneryInteraction
      - Sequence
      - TargetEnemy
      - ThreatChange
      - Travel
      - Unequip
      - Wait
      - Wander
    - GoalRoot
    - PrioritizedGoalList
  - parking
  - SkinnedModel
  - ThreatMap
- assets
  - archetypes
    - Archetype
- audio
  - AudioFx
  - AudioFxTemplate
  - AudioFxSystem
- books
- dialogue
  - CutScene
  - DialogueSet
  - RemarkSet
- items
  - Inventory
  - InventoryItem
  - InventoryItemArchetype
- nav
  - NavigationMesh
  - NavigationMeshBuilder
  - NavController
  - NavTract
  - NavRouteRequest
  - NavRouteTask
- overlays
  - DebugPhysicsOverlay
  - TargetingCircle
  - PathVisualizer
  - [x] TranslucentLines
  - TranslucentSprites
  - TranslucentPoints
  - [x] TranslucentMesh
- quests
  - StageId
  - Quest
  - QuestMgr
- particles
  - MissileSystem
  - ParticleEffect
  - ParticleEmitter
  - ParticleAspect
- physics
- scenery
  - [x] Fixture
  - [x] FixtureArchetype
  - [x] FixtureModels
  - FixtureObstacles
  - [x] FixturePhysics
  - [x] Floor
  - [x] FloorArchetype
  - [x] FloorModels
  - FloorObstacles
  - [x] FloorPhysics
  - [x] PrecinctCache
  - [x] Precinct
  - [x] Tier
- skills
- terrain
  - [x] ParcelCache
  - [x] Parcel
  - [x] TerrainShape
  - [x] TerrainFx
- view
  - [x] Viewpoint
  - Portals
  - Cutaways
  - Nameplates
- world
  - [x] Biome
  - [x] Realm
  - [x] World

# Convert PNG to premultiplied alpha:

convert quest.png -background black -alpha Remove quest.png -compose Copy_Opacity -composite quest.png
convert artwork/export/editor/building.png -background black -alpha Remove artwork/export/editor/building.png -compose Copy_Opacity -composite assets/editor/building.png

# Editor folder organization

- scenery
  - mod
  - panel
  - tool_create_floor etc.
  - enter / exit / update
  - mutations / commands
  - drag states
  - overlays

# Scripting

Access to:

- player
- viewpoint

- register quest
- register dialog
- register mod
  - add character asset

- In old engine, scripts were part of archetypes.

```js
/** Defines a package of behaviors for an actor or object in the world. */
export interface IAspectType<
  SelfType extends Instance = Instance,
  Props extends {} = {},
  Config extends {} = {}
> {
  /** Type of instance that this behavior can apply to (Actor, fixture, etc.). */
  type: InstanceTypeMask;

  /** Qualified name of this aspect (filled in by loader). */
  qname?: string;

  /** Configuration parameters for this aspect. */
  config?: IPropertyDescriptors<Config>;

  /** Properties which are added to instances that attach this behavior. */
  properties?: IPropertyDescriptors<Props>;

  /** Formulas to be bound to the specified properties */
  formulas?: IFormulaGenerators<SelfType, Props, Config>;

  /** Ways in which the player can interact with this instance. */
  interactions?: IInteraction<SelfType, Props, Config>[];

  /** List of quest roles associated with this actor. */
  questRoles?: string[];

  /** A task which runs when instance is instantiated and is run in the instance scope.
      This scope will get destroyed when the instance is unloaded.
   */
  init?: (self: SelfType, props: Props, config: Config) => void;

  /** Generates list of goals to be associated with this behavior (actors only) */
  goals?: (self: SelfType, props: Props) => GoalChildren;

  /** Method invoked when the actor is not doing anything. Returns the type of animation
      that should be run when idling.
   */
  idle?: (self: SelfType, props: Props) => string | undefined;

  /** Predicate function for sensor instances. */
  canSense?: (self: SelfType, props: Props, target: PositionableInstance) => boolean;

  /** If defined, this actor is conditional - only appears in specific quest stages. */
  present?: (self: SelfType, props: Props) => boolean;
}
```

- effectively we want a script to be able to implement:
  - a Scenery exemplar
  - an Actor exemplar
  - an Item exempler
  - a Skill
  - a QuestStage

- argument to handlers:
  self
  self.here
  self.world

```lua
book(self) {
  self.add_interaction({
      id: 'read',
      cursor: 'read',
      icon: 'read',
      caption: 'Read',
      action: (self, props, config) => {
        if (props?.content) {
          const ui = getSystem(GAME_UI_STATE_KEY);
          ui.openBook = {
            content: props.content,
            frame: config.frame ?? 'codex',
          };
        }
      },
      distance: 1,
  })
}

combat(self) {
  self.add_interaction({
    id: 'attack',
    cursor: 'melee',
    icon: 'sword',
    caption: 'Attack',
    when: self => {
      return self.alive;
    },
    focus: true,
    combat: true,
  });

  self.add_interaction({
    id: 'loot',
    cursor: 'loot',
    icon: 'loot',
    caption: 'Loot',
    distance: 1,
    when: self => {
      return !self.alive && self.hasInventoryItems;
    },
    action: self => {
      const ui = getSystem(GAME_UI_STATE_KEY);
      ui.openContainerDialog(self, 'corpse');
    },
  });

  self.add_interaction({
    id: 'loot-empty',
    cursor: 'loot-empty',
    icon: 'lootEmpty',
    caption: 'Empty',
    distance: 1,
    when: self => {
      return !self.alive && !self.hasInventoryItems;
    },
    action: self => {
      const ui = getSystem(GAME_UI_STATE_KEY);
      ui.openContainerDialog(self, 'corpse');
    },
  }),
}

fn combat(self) {
  self.on_interact = combat_interact;
}

fn combat_interact() {
    if self.alive {
      return {
        id: 'attack',
        cursor: 'melee',
        icon: 'sword',
        caption: 'Attack',
        when: self => {
          return self.alive;
        },
        focus: true,
        combat: true,
      }
    } else if self.hasInventoryItems {
    } else {
    }
}
```

# Scripting data types:

- builtin types
  - i32
  - i64
  - f32
  - f64
  - bool
  - string
  - struct
  - array
  - tuple
  - union
  - function
  - closure
- native types
  - optional
  - vec2 / vec4 / vec4
  - ivec2 / ivec3 / ivec4
  - uvec2 / uvec3 / uvec4

# Scripting TODOs

- Records
- Global Structs
- Memory Structs
- Intrinsic Functions
  - How to return an error from a script?
- Imports
- String data model
- Streamline unit tests.
- Unit tests for assign
- Risk: Closures in Components
- Built-in coercions (intrinsic functions)
- Allow intrinsics to be shared?
- Assert
- If
- Loop / Break / Continue
- While
- For
- Switch
- Logical Operator Short-Circuiting
- Relational operators
- Global Let and Const
- Structs:
  - constructor
  - references
  - field access
- TupleStruct
  - type definition
  - constructor
  - references
  - field access
  - newtype / tuple structs
- EmptyStruct (Struct with no members)
- Enums:
  - type definition
  - constructor
  - variant access
