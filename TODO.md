# TODO

- Bugs:
  - Surface changes when scene loads?
  - Generation error [REPORTED]
  - Materials get loaded multiple times?
  - CottageWallSkirtCorner turned wrong way.
- Facing and rotations: make consistent? I don't even know what they are now.
  Probably should be degrees.
- Models
  - [x] replace model_loader
- Wall Physics
  - implement
  - Use enum for collider type? Really depends on whether it's editable.
- Finish terrain shader
  - paths
  - biomes

* cursors
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
- [Error in PBR shader code](https://github.com/bevyengine/bevy/issues/12326)
- [Support archive assets](https://github.com/bevyengine/bevy/issues/12279)
- [Allow missing assets](https://github.com/bevyengine/bevy/issues/12210)
- [Gltf material substitutions](https://github.com/bevyengine/bevy/issues/12209)
- [Support hemisphere light](https://github.com/bevyengine/bevy/issues/12027)
- [Tool to create env maps](https://github.com/bevyengine/bevy/issues/12233)

# Aspects to define

- InventoryItem
  - [ ] inventory::Item - appearance, weight, stack size, price
  - [ ] inventory::Container - carrying capacity
  - [ ] inventory::Equippable - equip slot
  - [ ] inventory::Weapon - damage type, range
  - [ ] inventory::Document - link to text content, page style
  - [ ] inventory::QuestItem - quest id, stage
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
  - [ ] scenery::LightSource
  - [ ] scenery::SoundSource
  - [ ] scenery::WallSize - grid alignment options
  - [ ] mechanics::ToggleButton - click to interact
  - [ ] mechanics::PushButton - click to interact
  - [ ] mechanics::PressurePlate - senses being walked on
  - [ ] mechanics::ControlledOpenable - change state via remote signal
  - [ ] mechanics::AutoDoor
  - [ ] trigger::Circle - detects when player is within circle
  - [ ] trigger::Rect - detects when player is within rect
  - [ ] scenery::Waymark - used for NPC scripted events
- Sfx
  - [ ] sfx::Music
  - [ ] sfx::WaterFx
  - [ ] sfx::Particles
- Encounter
- Book
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
  - TranslucentLines
  - TranslucentSprites
  - TranslucentPoints
  - TranslucentMesh
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
  - Fixture
  - FixtureArchetype
  - FixtureModels
  - FixtureObstacles
  - FixturePhysics
  - Floor
  - FloorArchetype
  - FloorModels
  - FloorObstacles
  - FloorPhysics
  - PrecinctCache
  - Precinct
  - Tier
- skills
- terrain
  - [x] ParcelCache
  - [x] Parcel
  - [x] TerrainShape
  - TerrainFx
- view
  - Viewpoint
  - Portals
  - Cutaways
  - Nameplates
- world
  - Biome
  - Realm
  - World

# Convert PNG to premultiplied alpha:

convert quest.png -background black -alpha Remove quest.png -compose Copy_Opacity -composite quest.png
convert artwork/export/editor/building.png -background black -alpha Remove artwork/export/editor/building.png -compose Copy_Opacity -composite assets/editor/building.png

# Overlays

- Show
- For
- Overlay
- TranslucentShape
- TranslucentLines
- TranslucentPoints
- TranslucentSprites

Overlay structure:

Node
old reflex?
Children

We need local memoization

```rust
fn (ctx: Local<ViewContext>) {
  let color = ctx.create_selector(|world| world.get(...).resource.color);
  let state = ctx.create_memo(|world| { la la la });
  return ctx.render(||
    Group::new([
        FlatRect {
          color: color(),
          opacity: if state().selected { 1. } else { .5 },
          ..default(),
        }
    ])
  )
}

```

# Templating Take 2

```rust
impl Control for Button {
  // FIX: We need variable params like systems.
  fn render(state: &mut ResMut<Self>, ctx: &UiContext) -> Element {
    Element {
      events: (
        On::<Pointer<Over>>::run(button_pointer_over),
        On::<Pointer<Out>>::run(button_pointer_out),
        On::<Pointer<DragStart>>::run(button_drag_start),
        On::<Pointer<DragEnd>>::run(button_drag_end),
      ),
      stylesets: &[server.load("editor/ui/button.ui#button")],
      classes: &[
        ("hover", state.hover),
        ("pressed", state.pressed),
        ("disabled", state.disabled),
      ],
      children: &[
        // Background element
        Element {
          stylesets: &[server.load("editor/ui/button.ui#button-background")],
          children: &[
            Element {

            },
          ],
        },
        state.label,
      ],
      ..default()
    }
  }
}

struct Patch {
  rect: Rect,
}

struct NinePatch {
  image: Handle<Image>,
  patches: Vec<Patch>,
}

impl NinePatch {
  fn with_margins(rect: Rect) -> Self {
    patches.push(Patch {
      rect: Rect(0, 0, rect.x0, rect.y0),
    },
    Patch {
      rect: Rect(rect.x0, 0, rect.x1, rect.y0),
    },
    Patch {
      rect: Rect(rect.x1, 0, rect.x1, rect.y0),
    },
    )
  }
}

impl Control for NinePatch {
  fn render(state: &Res<Self>) -> Element {
    Element {
      style: {
        display: "grid",
      },
      children: 0..9.map(|n| {
        let x = n % 3;
        let y = n / 3;
        Element {
          style: {
            background_image: state.image,
            clip: CalculatedClip {

            }
          }
        }
      })
    }
  }
}
```
