# TODO

- Better picking on tiers - disable terrain picks
- Crash - inserting flora compute task on deleted entity.
- wall draw sometimes kicks into remove mode - missing exemplar ref?
- Quit confirm dialog
- Text input
- Use Arc<String> more in resources. Also make serialization mod.
- File unsaved indicator.
  - Track which things are unsaved.
- Undo / Redo
- Terrain contour list.
  - filter by group
  - empty tile shows nothing, should show black floor.
- think about converting realm name to Arc<String>. Either that or have a realm id.
- move prefs to own crate (and use it).
- World camera pan needs to use bubbled scroll events.
- Build an entity inspector
- Bugs:
  - Materials get loaded multiple times?
- Assets
  - Asset Reader that allows cache miss
  - Remove Box2d types, replace with 4-tuple, or perhaps 2-tuple of 2-tuple.
  - Preferences API.
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
  - random
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

# Unsaved:

- precincts
- contours
- terrain maps
- exemplars

# Editor folder organization

- scenery
  - mod
  - panel
  - tool_create_floor etc.
  - enter / exit / update
  - mutations / commands
  - drag states
  - overlays
