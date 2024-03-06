# TODO

- Floors
  - Surface texture rotation
  - Hex color parsing / Srgba
  - Editor-only surfaces (for things like water / hints)
- Bugs:
  - Surface changes when scene loads?
  - Could not decode precinct
  - Generation error [REPORTED]
  - Failed to load asset
  - Materials get loaded multiple times?
- Models
  - replace model_loader

* cursors
* TODO: Wheel rotation should only work if mouse within viewport. We'll need to add a system
  to track which region we're in.
* Finish water motion.

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
