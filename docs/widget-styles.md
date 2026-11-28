# Widget styles

Panoply, an RPG built on Bevy, draws its game UI's widgets from artwork: nine-patched frames and
tinted text. Their behaviour comes from Bevy's headless widgets (`bevy_ui_widgets`). This doc is
about their look: a style sheet per kind of widget, shared by every instance. Nothing here is
implemented yet.

## Requirements

1. A widget's look follows its states: hovered, pressed, focused, checked, selected, expanded,
   disabled.
2. A sheet is loaded once and shared. An instance holds a handle to it and its current states,
   never a copy of its rules.
3. A sheet sets any of Bevy's visual components: `ImageNode` with a `TextureSlicer`,
   `BackgroundColor`, `BorderColor`, `BorderRadius`, `BoxShadow`, gradients, `TextColor`, and
   `UiTransform`, which moves, scales and turns a node after layout, and `MaterialNode`. It can also
   set `Visibility`, as for a checkbox's mark, and `EntityCursor`. It never sets layout (`Node`), so
   a change of state never moves the widgets around it.
4. A change of state can transition over time.
5. No cascade. What styles an entity is named at the widget, and a tool can list the rules that
   never take effect.
6. Sheets are assets, which a mod can replace.

## Model

- **Sheet**: an asset holding an ordered list of rules and the sheet's transitions. It may
  `extend` a base sheet, whose rules come first.
- **Rule**: a state mask, the parts it styles, and a patch: a scene of components, with no children.
  A rule that names no parts styles the root.
- **States**: one bit for each of `bevy_ui`'s interaction markers: `Hovered`, `Pressed`,
  `InteractionDisabled`, `Checked`, `Selected`, `Expanded`, and one for input focus. A mask matches
  when all its bits are set; the empty mask always matches. One component holds a widget's bits,
  kept current by observers on each marker's insertion and removal.
- **Variant**: a different sheet, not a state. A primary button and a plain one have a sheet each,
  and so does a checkbox that activates on press, which never shows its pressed look. A variant that
  differs only in colour extends the base sheet and redefines its variables (see Variables).
- **Part**: an entity in the widget's own template marked `StylePart("label")`, with one name per
  entity. A rule for a part reads the states of the widget's root. Parts never reach into another
  widget, even a child one.
- **Root**: the entity holding `WidgetStyle`, found by that component and never by name.
- **Instance**: `WidgetStyle(Handle<StyleSheet>)` on the widget's root, and its current states.

A part's name is not its `Name`. A `Name` is for debugging and for finding an entity in its scene,
and a caller can override the root's. Parts that look alike are listed together in the sheet's rule,
not given a shared name. An entity that needs two looks is split into a part and a child part.

A single name falls short when a sub-scene is used twice, as a number input's two arrows might be:
the scene would name the part "arrow" and each use add "increment" or "decrement", but a `StylePart`
set by the outer scene replaces the inner one. If a widget needs that, a marker component becomes a
second way to name a part, since BSN lets an entity hold several: `.Arrow`, by the type's short name
(`TypeRegistry::get_with_short_type_path`), beside `.arrow` for a `StylePart`. A short name two
registered types share is a load error naming both.

Order is the only precedence. A pressed button is also hovered, so its pressed rule comes after its
hovered one; disabled comes last, so it wins over both.

## Layers

A widget drawn in layers has a part per layer. Faery's dialog button
([DialogControls.css.ts](../../faery/packages/game-ui/src/components/DialogControls.css.ts)) had
five, each placed over the button:

1. A background colour, brighter when hovered or visibly focused.
2. An ornamental frame.
3. A glow: a 2 px border with a radial gradient, blurred.
4. A sharper 1 px border of the same kind.
5. A focus ring, ignoring picking, shown with focus.

In Panoply each is a child in the button's template, which places it; the sheet styles each by its
`StylePart`. Faery's glows were stacked blurred borders, the way CSS designers stack shadows for an
eased fall-off. In Panoply a glow is a sliced image with alpha, drawn in white and tinted by
`ImageNode::color`: the art holds the fall-off, and the sheet sets the tint per state and variant,
which transitions. It can sit inside the edge or outside it. `BoxShadow`, which holds a list of
shadows each with its own blur, also stacks an outer glow, but has no inset shadow. `bevy_ui` has no
saturation filter: a disabled look is a tint or its own art.

An animated layer, such as a slowly moving pattern behind a frame, is a `MaterialNode` whose shader
reads the time. A material's parameters live in its asset, shared by every node that uses it, so a
look that changes by state is a material asset per combination, and a change of state swaps the
handle. A handle can't transition; a cross-fade is two layers whose alphas do.

## Resolution

At load, the sheet resolves every combination of states into one scene per part: the matching rules,
in order, resolved into a single `ResolvedScene`. It resolves only the bits its rules use: a
button's three give 8 combinations, and all seven give 128, computed once per sheet. When a widget's
states change, its combination's scenes are applied to its root and parts
(`ResolvedSceneRoot::apply`).

Applying a scene replaces each of its components whole, so the layering happens at load, never on
the entity. Two load errors follow from it:

- A component a rule sets must also be set by a rule with the empty mask. Otherwise leaving the
  hovered state would leave the hovered colour behind.
- A rule's patch has no children. Applying it would spawn them, on every change of state.

A component the sheet sets belongs to the sheet: the widget's template leaves it out.

## Transitions

The sheet names the fields that transition, each with a duration and an easing. A field is a
component and an accessor, with get and set, typed by its value. Only types that interpolate can
transition: colours, radii, shadows, transforms. An image or atlas index switches at once, so an
`ImageNode` transitions by its `color` field while its image switches. Bevy's `animated_field!` has
the same shape, but its `Blendable` covers no UI type (`Color`, `Val`, `BorderColor`,
`ShadowStyle`), so sheets keep their own trait.

A transition holds its start, its target, the time since it began and its duration. A change of
state starts a new one from the current value, even mid-way, to the new combination's value: a sheet
has many combinations, not two ends to run between. When the new target is the previous start, as
when hover ends part-way through its fade, the duration is scaled by how far the transition had got,
as in CSS.

The sheet knows at load which fields transition, so it builds their type-erased steps once. A widget
holds a transition component only while a field is moving, and loses it when the last one finishes.
The driver runs in `PostUpdate`, before `UiSystems::Prepare`.

Two load errors guard interpolation, since the sheet sees every combination's values:

- A transitioning `Val` must keep its unit across combinations: `Px` can't interpolate to `Percent`.
- Shadows interpolate in pairs, so a transitioning `BoxShadow` keeps its count. A state that adds a
  shadow gives the resting rule a transparent one in its place.

Picking hits a node where its transform puts it, so a root that grows on hover can lose the hover
at its new edge, shrink back, and flicker. Grow a part that ignores picking instead.

## Dead rules

A component in a rule is dead when no combination's final value comes from it. With seven bits at
most, the lint tries every combination. It sees what the format lets it see: whole components for a
BSN sheet, since a resolved template is opaque, and single fields for a parsed one.

## Formats

The same sheet in each format: a framed button whose label brightens on hover and dims when
disabled.

### BSN

Until BSN assets land, a sheet is a `bsn!` scene in Rust, never spawned; then the same scene
moves into a file. Each child is a rule:

```rust
fn button_sheet() -> impl Scene {
    bsn! {
        StyleSheet
        StyleTransitions {
            text_color: Some(StyleTransition { secs: 0.12, ease: EaseFunction::QuadraticOut }),
        }
        Children [
            StyleRule
            ImageNode {
                image: "ui/button.png",
                image_mode: NodeImageMode::Sliced(TextureSlicer {
                    border: BorderRect::all(12.0),
                }),
            }
            --
            StyleRule { parts: ["label"] }
            TextColor(Color::srgb(0.91, 0.85, 0.69))
            --
            StyleRule { when: StyleState::HOVERED }
            ImageNode { image: "ui/button_hover.png" }
            --
            StyleRule { when: StyleState::HOVERED, parts: ["label"] }
            TextColor(Color::WHITE)
            --
            StyleRule { when: StyleState::PRESSED }
            ImageNode { image: "ui/button_pressed.png" }
            --
            StyleRule { when: StyleState::DISABLED }
            ImageNode { image: "ui/button_disabled.png" }
            --
            StyleRule { when: StyleState::DISABLED, parts: ["label"] }
            TextColor(Color::srgb(0.54, 0.50, 0.42))
        ]
    }
}
```

The hovered rule's `ImageNode` patches only `image`; it keeps the slicer, because the empty-mask
rule comes before it in every combination.

### Parsed text

A small language of its own, with a fixed set of property names, each mapped to a component's
field:

```text
extends "ui/base.style"

transition text-color 120ms quadratic-out

image: "ui/button.png" sliced(12)
:hovered  { image: "ui/button_hover.png" }
:pressed  { image: "ui/button_pressed.png" }
:disabled { image: "ui/button_disabled.png" }

.label {
    text-color: #e8d9b0
    :hovered  { text-color: #ffffff }
    :disabled { text-color: #8a7f6a }
}
```

Properties outside any block style the root. `.label` names a part, as in CSS; a block can style
several parts that look alike: `.icon, .badge { … }`. Rules keep the order they're written in;
`:hovered:pressed` would need both. There is no `#name` selector: a sheet that matched a `Name`
would break when an entity was renamed for debugging.

### Choosing

| | BSN | Parsed text |
|---|---|---|
| Components | any, by reflection | the fixed property set |
| Dead-rule lint | whole components | single fields |
| Errors | the scene parser's | its own |
| Maintenance | Bevy's | a parser and a property table |
| Shared with widget templates | the same language | a second one |
| Available | `bsn!` now, files once BSN assets land | now |

## Variables

A sheet can name a value instead of writing it: `$accent`, as in CSS variables. A sheet defines its
variables, and a sheet that extends it can redefine them. Faery's button had five colour variants,
each setting four variables and no rules; in Panoply each is a sheet of a few lines that extends the
base button. A name is looked up in the sheet, then in its bases.

A variable is typed by the field it fills, so a colour can't fill a length:

- **Parsed text**: the property table knows `text-color` takes a colour, so `$accent` must be
  defined as one.
- **BSN**: the field still takes a `Color`, so the variable is a template that makes one:
  `TextColor(var::color("accent"))`.

A name no sheet in the chain defines, or defines with another type, is a load error.

## Loading

Each format has its own `AssetLoader`, and both make the same `StyleSheet` asset, one sheet per
file. A `bsn!` sheet in Rust becomes one through `ScenePatch::load`, which loads the paths its scene
names.

A widget is styled from the frame it spawns:

- A template that names its sheet, as in `WidgetStyle("ui/button.style")`, isn't spawned until the
  sheet is loaded with its dependencies: the sheets it extends and the images its rules name.
- The sheet resolves into the asset itself, in a system on `LoadedWithDependencies` ordered before
  scenes spawn, as `resolve_scene_patches` does for scenes.
- An observer on the insertion of `WidgetStyle` applies the widget's combination.
- A `WidgetStyle` inserted later, by code rather than a template, isn't waited for. The widget keeps
  its current look until the new sheet resolves.

A changed sheet file resolves again, and every widget holding it has its current combination
applied, so a look can be tuned while the game runs.

## Feathers

Feathers, Bevy's editor widgets, could style its widgets with sheets too. Each of its controls
styles itself by hand, with a pair of systems:

- One runs on `Changed<Hovered>`, since `Hovered` is a component holding a bool, or on `Added<T>`
  for the markers: `Pressed`, `Checked`, `InteractionDisabled`, `Selected`, `Expanded`.
- The other reads a `RemovedComponents<T>` for each marker, since a removal can't be a query filter.
  It looks each entity up again and repeats the first system's work.

Both find the parts by searching the descendants for a marker component, and both call a function
that chooses each part's colour token through nested `if`s, one branch per combination of states.
Value components (`Changed<SliderValue>`, `Changed<ColorSwatchValue>`) and theme switches
(`Changed<ThemeContext>`) each have their own systems.

A sheet replaces the state handling: the state bits and their observers in place of the system
pairs, `StylePart` in place of the descendant search, and ordered rules in place of the branches.
`ActivateOnPress` and `ButtonVariant` become variants.

Feathers' theming would need the theming opportunity below. Its `Theme*Color` components hold a
token, a string resolved through the current theme. The theme holds only colours, so a token can't
become anything else, but its name is unchecked: a missing one logs a warning and draws fuchsia, at
runtime, on the first frame that uses it. Variables are checked when the sheet loads.

It replaces little of feathers' size. Of its 13,000 lines, the number input, tabs and colour input
make up 6,000, and nearly all of it is behaviour and layout. The styling is a few hundred lines in
each simple control. A sheet would make the styling regular, not small.

## Opportunities

Not requirements: each is taken up if a good design turns up, and left otherwise.

- **Theming.** Panoply has fixed variants and no themes. A palette, an asset of typed variables
  beneath every sheet's own, would let a theme swap one asset, and a theme switch resolve each sheet
  again.
- **Surface levels.** Feathers resolves a token differently by the surface a widget sits on
  (`semantic_overrides`), with the level copied down the tree to every descendant
  (`Propagate<ThemeContext>`). The level could be state bits, so a sheet resolves each level once
  and a widget moving between surfaces changes state.
- **Libraries.** Several sheets in one file as labeled sub-assets (`"ui/game.style#button"`), the
  root holding a handle to each, as glTF does. A base sheet sits beside its variants, and one handle
  preloads them all; a mod then replaces the whole file to change one sheet. Moving to libraries
  changes only the paths in templates.

## Open questions

- `Scene::resolve` consumes the scene, and a rule takes part in many combinations. In Rust, each
  rule can be a function that makes its scene; whether a `ScenePatch` asset can be resolved more
  than once is to be found out.
- Reading a rule's `StyleRule` back out of its resolved scene, through the downcastable template.
- Whether a sheet can be a `ScenePatch` and resolve through it, or needs an asset of its own that
  borrows its dependency loading.
- Whether a `bsn!` field can take a template that resolves when the sheet loads, as a variable
  needs.
- Focus as a state: any focus, or only visible focus (`InputFocusVisible`).