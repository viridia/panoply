# TODO

* Guise
  * Controllers
    * Picking
    * Splitters
  * Styles
    * Finish unit tests
  * Guise Bundle type
  * Template invocation
  * Template params
  * Conditional logic
  * Text styles
  * Whitespace trimming
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

.with_styles((style, cond(style, cond)));

stylex(style, style, if const { style } else Style::Empty)

type StyleAttributes = HashMap<key, StyleValue>;

enum Style {
  layer: usize,
  base: StyleAttributes,
  selectors: Map<String, StyleAttributes>,
  variants: Map<String, StyleAttributes>,
}
```

Vanilla-extract:

- styles
- recipes
- sprinkles
- css vars
- theme vars
- descendants?

Properties
* display: ['flex', 'grid', 'block', 'inline'],
* alignItems: ['stretch', 'center', 'flex-start', 'flex-end'],
* alignSelf: ['stretch', 'center', 'flex-start', 'flex-end'],
* alignContent: ['stretch', 'center', 'flex-start', 'flex-end'],
* justifySelf
* justifyContent
* justifyItems
* gap: Space
* flexDirection
* flexWrap
* flexGrow: [0, 1, 2, 3],
* flexShrink: [0, 1, 2, 3],
* flexBasis:
* minHeight: Size,
* minWidth: Size,
* maxHeight: Size,
* maxWidth: Size,
* height: Size,
* width: Size,

* marginLeft: Space,
* marginRight: Space,
* marginTop: Space,
* marginBottom: Space,

* paddingLeft: Space,
* paddingRight: Space,
* paddingTop: Space,
* paddingBottom: Space,

* overflow: ['auto'],
* overflowX: ['auto'],
* overflowY: ['auto'],

  shorthands: {
    w: ['width'],
    h: ['height'],
    ml: ['marginLeft'],
    mr: ['marginRight'],
    mt: ['marginTop'],
    mb: ['marginBottom'],
    m: ['marginLeft', 'marginRight', 'marginTop', 'marginBottom'],
    pl: ['paddingLeft'],
    pr: ['paddingRight'],
    pt: ['paddingTop'],
    pb: ['paddingBottom'],
    p: ['paddingLeft', 'paddingRight', 'paddingTop', 'paddingBottom'],
  },

* aquifer
* sequin
* fluid
* guise

## CSS attribute syntax:

```
  declaration
    : property ':' S* expr prio?
    ;
  prio
    : IMPORTANT_SYM S*
    ;
  expr
    : term [ operator? term ]*
    ;
  term
    : unary_operator?
      [ NUMBER S* | PERCENTAGE S* | LENGTH S* | EMS S* | EXS S* | ANGLE S* |
        TIME S* | FREQ S* ]
    | STRING S* | IDENT S* | URI S* | hexcolor | function
    ;
  function
    : FUNCTION S* expr ')' S*
    ;
```
