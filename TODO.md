# TODO

* Guise
  * Hot reload
  * Only recompute styles on change
  * Differential updates
  * Finish style attributes (grid, etc.)
  * Override background to be transparent
  * Presenters / Classes
  * Selectors
  * Vars
  * Parent Selectors
  * Background Image
  * Presenters
  * Templates calling templates
  * Eval expressions with params
  * Styles as assets
  * Color functions
  * Controllers
    * Picking
    * Splitters
  * Styles
    * Image Section prop
    * Selectors - eval
    * Vars - use
    * Inherited props
    * Styles passed into to call
    * Finish unit tests
    * JSON array as UiRect
  * Guise Bundle type
  * Template params
  * Conditional logic
  * Text styles
* cursors
* TODO: Wheel rotation should only work if mouse within viewport. We'll need to add a system
  to track which region we're in.
* Finish water motion.

# Project hierarchy

* actors
  * Action
  * Actor
  * ActorTemplate
  * ActorArchetype
  * ActorAffix
  * goals
    * components
      * Activate
      * ApplySkill
      * Attack
      * Contingent
      * Deactivate
      * Dialogue
      * Equip
      * FaceToward
      * LookAt
      * Park
      * Pose
      * Pursue
      * Random
      * Ranked
      * Remark
      * SceneryInteraction
      * Sequence
      * TargetEnemy
      * ThreatChange
      * Travel
      * Unequip
      * Wait
      * Wander
    * GoalRoot
    * PrioritizedGoalList
  * parking
  * SkinnedModel
  * ThreatMap
* assets
  * archetypes
    * Archetype
* audio
  * AudioFx
  * AudioFxTemplate
  * AudioFxSystem
* books
* dialogue
  * CutScene
  * DialogueSet
  * RemarkSet
* items
  * Inventory
  * InventoryItem
  * InventoryItemArchetype
* nav
  * NavigationMesh
  * NavigationMeshBuilder
  * NavController
  * NavTract
  * NavRouteRequest
  * NavRouteTask
* overlays
  * DebugPhysicsOverlay
  * TargetingCircle
  * PathVisualizer
  * TranslucentLines
  * TranslucentSprites
  * TranslucentPoints
  * TranslucentMesh
* quests
  * StageId
  * Quest
  * QuestMgr
* particles
  * MissileSystem
  * ParticleEffect
  * ParticleEmitter
  * ParticleAspect
* physics
* scenery
  * Fixture
  * FixtureArchetype
  * FixtureModels
  * FixtureObstacles
  * FixturePhysics
  * Floor
  * FloorArchetype
  * FloorModels
  * FloorObstacles
  * FloorPhysics
  * PrecinctCache
  * Precinct
  * Tier
* skills
* terrain
  * [x] ParcelCache
  * [x] Parcel
  * [x] TerrainShape
  * TerrainFx
* view
  * Viewpoint
  * Portals
  * Cutaways
  * Nameplates
* world
  * Biome
  * Realm
  * World

# Convert PNG to premultiplied alpha:

convert quest.png -background black -alpha Remove quest.png -compose Copy_Opacity -composite quest.png
convert artwork/export/editor/building.png -background black -alpha Remove artwork/export/editor/building.png -compose Copy_Opacity -composite assets/editor/building.png

# Overlays

* Show
* For
* Overlay
* TranslucentShape
* TranslucentLines
* TranslucentPoints
* TranslucentSprites

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

let style = Style::new()
  .gap(Size::Medium)
  .flex_direction(Flex::Horizontal);

fn ui_system(mut commands: Commands, ctx: Local<ViewContext>) {
  // 'Render' only calls 'counters' if it detects a change.
  ctx.create_root(counters);
}

fn counters(ctx: UiContext) -> UiComponent {
  return FlexBox::new(style, (
    Ui::new(counter, true),
    Ui::new(counter, false),
  ));
}

fn counter(ctx: UiContext, something: bool) -> UiComponent {
  let mut count = ctx.select_resource::<Counter>(|res| &res.count);
  return FlexBox::new(style, (
    Button::new(format!("count: {}", count.get())).on_click(|| *count = *count + 1),
    Button::new("reset", || *count = 0),
  ));
}

fn conditional(ctx: UiContext, something: bool) -> UiComponent {
  let mut selected = ctx.select_resource::<Counter>(|res| &res.selected);
  return If::new(
    condition,
    Button::new(format!("count: {}", count.get())).on_click(|| *count = *count + 1),
    Button::new("reset", || *count = 0),
  );
}

fn iterator(ctx: UiContext, something: bool) -> UiComponent {
  let mut selected = ctx.select_resource::<Counter>(|res| &res.selected);
  // The problem with this, is that it creates a new component each time.
  // We want to memoize the list and do a diff.
  return For::new(
    iterable,
    |elt| Button::new(format!("count: {}", elt)).on_click(|| *count = *count + 1),
  );
}

fn switch(ctx: UiContext, something: bool) -> UiComponent {
  let mut selected = ctx.select_resource::<Counter>(|res| &res.selected);
  return Switch::new((
    Case::new(condition, component),
    Case::new(condition, component),
    Case::new(condition, component),
  ));
}

fn router(ctx: UiContext, something: bool) -> UiComponent {
  let mut selected = ctx.select_resource::<Counter>(|res| &res.selected);
  return Router::new(route, (
    Route::new(path, component),
    Route::new(path, component),
    Route::new(path, component),
  ));
}

```

# Templating Take 2

```rust
struct Counter {
  count: i32,
}

impl Controller for Counter {
  fn render(elt: &mut ViewElement, template: &Box<TemplateNode>) -> UiComponent {
    let mut count = ctx.select_resource::<Counter>(|res| &res.count);
    return FlexBox::new(style, (
      Button::new(format!("count: {}", count.get())).on_click(|| *count = *count + 1),
      Button::new("reset", || *count = 0),
    ));

    return Element {
      style: {
        display: "flex",
        background_color: Expr::Var("button-fill"),
        vars: [
          ("button-fill", Expr::new(Color.RED)),
        ],
        selectors: [
          ("&.selected", {
            vars: [
              ("button-fill", Expr::new(Color.GREEN)),
            ],
          }),
          ("&.disabled", {
            "opacity": 0.5,
          }),
       ]
      },
      stylesets: &[server.load("editor/ui/button.ui#button")],
      children: &[
        Slider {

        },
        Element {

        },
        Text {

        }
      ],
    }
  }
}

struct UiContext {
  capture
  focus
  context vars.
}

struct Button {
  hover: bool,
  pressed: bool,
  disabled: bool,
  label: Element,
}

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
