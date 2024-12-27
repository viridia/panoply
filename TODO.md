# Migration

- Scripting
- Biome data
- Camera movement
- Revive editor
- Preferences

# TODO

- Better picking on tiers - disable terrain picks
- Crash - inserting flora compute task on deleted entity.
- wall draw sometimes kicks into remove mode - missing exemplar ref?
- separate crate for realms?
- Quit confirm dialog
- Text input
- File unsaved indicator.
  - Track which things are unsaved.
- Terrain contour list.
  - filter by group
  - contours not appearing in the correct order in groups.
  - empty tile shows nothing, should show black floor.
- think about converting realm name to Arc<String>. Either that or have a realm id.
- World camera pan needs to use bubbled scroll events.
- Build an entity inspector
- Bugs:
  - Materials get loaded multiple times?
- Assets
  - Asset Reader that allows cache miss
- Wall Physics
  - implement
  - Use enum for collider type? Really depends on whether it's editable.
- Finish terrain shader
  - biomes (snow, desert, etc.)
- Portals
  - perspective projection
  - modified frustum
- Possibly move to common crate:
  - msgpack (no, we want to remove this actually, get rid of extension types)
  - inline_assets?
  - reflect_types
- Skybox experiments

* TODO: Wheel rotation should only work if mouse within viewport. We'll need to add a system
  to track which region we're in.
* Finish water motion.

Future:

- Floor surface texture rotation
- Unit tests for HexColor.
- Editor-only floor surfaces (for things like water currents / hints)

# Bevy Issues

- [Invalid Generation Error](https://github.com/bevyengine/bevy/issues/12345)
- [Reflection type aliases](https://github.com/bevyengine/bevy/issues/12387)
- [Upstream bevy_mod_picking](https://github.com/bevyengine/bevy/issues/12365)
- [Multi-pass materials](https://github.com/bevyengine/bevy/issues/12208)
- [Support archive assets](https://github.com/bevyengine/bevy/issues/12279)
- [Allow missing assets](https://github.com/bevyengine/bevy/issues/12210)
- [Support hemisphere light](https://github.com/bevyengine/bevy/issues/12027)

# Aspects to define

- Floor
  - [x] floor::StdSurface - texture asset
  - [x] floor::NoiseSurface - procedural texture
  - [x] floor::Geometry - floor mesh options
  - [x] floor::Nav - pathfinding effects
- SceneryElement
  - [x] scenery::Models - list of glb models to display
  - [x] scenery::Colliders - physics colliders
  - [x] scenery::Marks - interaction marks
  - [ ] scenery::Container - open / close / lock behaviors
  - [ ] scenery::Door - open / close / lock
  - [ ] scenery::Stairs - allows click-to-climb
  - [ ] scenery::Ladder - allows click-to-climb
  - [ ] scenery::Sign - click to read
  - [ ] scenery::PortalAperture - portal dimensions
  - [ ] scenery::PortalTarget - portal target location
  - [ ] scenery::LightSource - point light source location
  - [ ] scenery::SoundSource - ambient sound emitter
  - [ ] scenery::WallSize - grid alignment options
  - [ ] mechanics::PushButton - click to interact
  - [ ] mechanics::ToggleButton - click to interact
  - [ ] mechanics::PressurePlate - senses being walked on
  - [ ] mechanics::ControlledOpenable - change state via remote signal
  - [ ] mechanics::AutoDoor
  - [ ] trigger::Circle - detects when player is within circle
  - [ ] trigger::Rect - detects when player is within rect
  - [ ] trigger::Encounter - increases chance of enemy spawn based on proximity
  - [ ] scenery::Waymark - used for NPC scripted events
- Sfx
  - [ ] sfx::Music
  - [ ] sfx::WaterFx
  - [ ] sfx::Particles
- Actors
  - [ ] actor::Model
  - [ ] actor::ColorSlots
  - [ ] actor::Colors
  - [ ] actor::FeatureSlots
  - [ ] actor::Features
  - [ ] actor::EquippedSlots
  - [ ] actor::Equipped
  - [ ] actor::Skills
  - [ ] actor::Physics
  - [ ] actor::Gender
  - [ ] actor::Ally
  - [ ] actor::Portrait
  - [ ] actor::GoalsXXX\* (can be multiple)
- InventoryItem
  - [ ] inventory::Item - appearance, weight, stack size, price
  - [ ] inventory::Container - carrying capacity
  - [ ] inventory::Equippable - equip slot
  - [ ] inventory::Weapon - damage type, range
  - [ ] inventory::Document - link to text content, page style
  - [ ] inventory::QuestItem - quest id, stage
- Book

# Project hierarchy

- actors
  - Action
  - Actor
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
  - FixtureModels
  - FixtureObstacles
  - FixturePhysics
  - Floor
  - [x] FloorArchetype
  - [x] FloorModels
  - FloorObstacles
  - FloorPhysics
  - [x] PrecinctCache
  - [x] Precinct
  - [x] Tier
- skills
- terrain
  - [x] ParcelCache
  - [x] Parcel
  - [x] TerrainShape
  - TerrainFx
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

# New Ground Shader

- texture slots
  - dirt
  - cobbles
- biome slots
  - texture index
  - blend weights for corners (xyzw)
  - rotation
  - scale
  - blend params
    - noise factor

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

# Scripted assets

- We should make floors an asset rather than an aspect.

2024-12-27T05:19:41.898533Z ERROR bevy_asset::server: Path not found: /Users/talin/Projects/games/panoply/assets/scenery/floors/ice-cavefloor.ron
2024-12-27T05:19:41.964606Z ERROR bevy_asset::server: Path not found: /Users/talin/Projects/games/panoply/assets/scenery/floors/wood-dark-walnut.ron
2024-12-27T05:19:41.964628Z ERROR bevy_asset::server: Path not found: /Users/talin/Projects/games/panoply/assets/scenery/floors/wood-walnut-planks.ron
2024-12-27T05:19:41.964633Z ERROR bevy_asset::server: Path not found: /Users/talin/Projects/games/panoply/assets/scenery/floors/wood-dark-walnut-planks.ron
2024-12-27T05:19:41.964664Z ERROR bevy_asset::server: Path not found: /Users/talin/Projects/games/panoply/assets/scenery/floors/dark-green-cobbles.ron
2024-12-27T05:19:41.964670Z ERROR bevy_asset::server: Path not found: /Users/talin/Projects/games/panoply/assets/scenery/floors/dirty-stone.ron
2024-12-27T05:19:41.964677Z ERROR bevy_asset::server: Path not found: /Users/talin/Projects/games/panoply/assets/scenery/floors/mossy-pavers-bordered.ron
2024-12-27T05:19:41.964709Z ERROR bevy_asset::server: Path not found: /Users/talin/Projects/games/panoply/assets/scenery/floors/cavefloor.ron
2024-12-27T05:19:41.964714Z ERROR bevy_asset::server: Path not found: /Users/talin/Projects/games/panoply/assets/scenery/floors/diamond-parquet.ron
2024-12-27T05:19:41.964719Z ERROR bevy_asset::server: Path not found: /Users/talin/Projects/games/panoply/assets/scenery/floors/carpet-square-red.ron
2024-12-27T05:19:41.964722Z ERROR bevy_asset::server: Path not found: /Users/talin/Projects/games/panoply/assets/scenery/floors/carpet-square-black.ron
2024-12-27T05:19:41.964729Z ERROR bevy_asset::server: Path not found: /Users/talin/Projects/games/panoply/assets/scenery/floors/carpet-square-gold.ron
2024-12-27T05:19:41.964733Z ERROR bevy_asset::server: Path not found: /Users/talin/Projects/games/panoply/assets/scenery/floors/grassy-stone.ron
