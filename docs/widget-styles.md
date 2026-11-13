# Widget styles

Panoply, an RPG built on Bevy, draws its game UI's widgets from artwork: nine-patched frames and
tinted text. Their behaviour comes from Bevy's headless widgets (`bevy_ui_widgets`). This doc is
about their look: a style sheet per kind of widget, shared by every instance. Nothing here is
implemented yet.

## Requirements

1. A widget's look follows its states: hovered, pressed, focused, checked, disabled.
2. A sheet is loaded once and shared. An instance holds a handle to it and its current states,
   never a copy of its rules.
3. A sheet sets any of Bevy's visual components: `ImageNode` with a `TextureSlicer`,
   `BackgroundColor`, `BorderColor`, `BorderRadius`, `BoxShadow`, gradients, `TextColor`, and
   `UiTransform`, which moves, scales and turns a node after layout. It never sets layout (`Node`),
   so a change of state never moves the widgets around it.
4. A change of state can transition over time.
5. No cascade. What styles an entity is named at the widget, and a tool can list the rules that
   never take effect.
6. Sheets are assets, which a mod can replace.

## Model

- **Sheet**: an asset holding an ordered list of rules and the sheet's transitions. It may
  `extend` a base sheet, whose rules come first.
- **Rule**: a state mask, an optional part, and a patch: a scene of components, with no children.
- **States**: one bit for each of the headless widgets' markers: `Hovered`, `Pressed`,
  `InteractionDisabled`, `Checked`, and one for input focus. A mask matches when all its bits are
  set; the empty mask always matches.
- **Part**: an entity in the widget's own template, by its `#name`. A rule for a part reads the
  states of the widget's root. Parts never reach into another widget, even a child one.
- **Instance**: `WidgetStyle(Handle<StyleSheet>)` on the widget's root, and its current states.

Order is the only precedence. A pressed button is also hovered, so its pressed rule comes after its
hovered one; disabled comes last, so it wins over both.

## Resolution

At load, the sheet resolves every combination of states into one scene per part: the matching rules,
in order, resolved into a single `ResolvedScene`. Five bits give 32 combinations, computed once per
sheet. When a widget's states change, its combination's scenes are applied to its root and parts
(`ResolvedSceneRoot::apply`).

Applying a scene replaces each of its components whole, so the layering happens at load, never on
the entity. Two load errors follow from it:

- A component a rule sets must also be set by a rule with the empty mask. Otherwise leaving the
  hovered state would leave the hovered colour behind.
- A rule's patch has no children. Applying it would spawn them, on every change of state.

A component the sheet sets belongs to the sheet: the widget's template leaves it out.

## Transitions

The sheet names the components that transition, each with a duration and an easing. When a change
of state would change one, the widget holds the old value, the new one and the time since the
change, and writes the value between them each frame until the time runs out. Only types that
interpolate can transition: colours, radii, shadows, transforms. An image or atlas index switches
at once.

Picking hits a node where its transform puts it, so a root that grows on hover can lose the hover
at its new edge, shrink back, and flicker. Grow a part that ignores picking instead.

## Dead rules

A part of a rule is dead when no combination's final value comes from it. With five bits the lint
tries every combination. It sees what the format lets it see: whole components for a BSN sheet,
since a resolved template is opaque, and single fields for a parsed one.

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
            StyleRule { part: "label" }
            TextColor(Color::srgb(0.91, 0.85, 0.69))
            --
            StyleRule { when: StyleState::HOVERED }
            ImageNode { image: "ui/button_hover.png" }
            --
            StyleRule { when: StyleState::HOVERED, part: "label" }
            TextColor(Color::WHITE)
            --
            StyleRule { when: StyleState::PRESSED }
            ImageNode { image: "ui/button_pressed.png" }
            --
            StyleRule { when: StyleState::DISABLED }
            ImageNode { image: "ui/button_disabled.png" }
            --
            StyleRule { when: StyleState::DISABLED, part: "label" }
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

root {
    image: "ui/button.png" sliced(12)
    :hovered  { image: "ui/button_hover.png" }
    :pressed  { image: "ui/button_pressed.png" }
    :disabled { image: "ui/button_disabled.png" }
}

label {
    text-color: #e8d9b0
    :hovered  { text-color: #ffffff }
    :disabled { text-color: #8a7f6a }
}
```

Rules in a part's block keep the order they're written in; `:hovered:pressed` would need both.

### Choosing

| | BSN | Parsed text |
|---|---|---|
| Components | any, by reflection | the fixed property set |
| Dead-rule lint | whole components | single fields |
| Errors | the scene parser's | its own |
| Maintenance | Bevy's | a parser and a property table |
| Shared with widget templates | the same language | a second one |
| Available | `bsn!` now, files once BSN assets land | now |

## Open questions

- `Scene::resolve` consumes the scene, and a rule takes part in many combinations. In Rust, each
  rule can be a function that makes its scene; whether a `ScenePatch` asset can be resolved more
  than once is to be found out.
- Reading a rule's `StyleRule` back out of its resolved scene, through the downcastable template.
- Resolving a sheet that is never spawned: what waiting for its dependencies takes.
- Colours as named tokens of a palette, so a theme swaps one asset.
- Focus as a state: any focus, or only visible focus (`InputFocusVisible`).
