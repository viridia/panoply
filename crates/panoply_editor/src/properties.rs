//! The property editor's rows: a subpane per inspected value, and in it a row per field, named
//! after its field. Booleans get a checkbox, numbers a number input carrying its caption inside, a
//! `Vec3` three of them, colors a swatch that opens a picker, nested structs a heading over their
//! own rows, indented. A newtype shows its one field's row, an `Option` its value's row when it has
//! one and none when it is `None`. Every other field is a row that shows its value without editing
//! it. Annotations from `bevy::annotations` set a row's caption, a number's range and step, and
//! whether it can be edited.
//!
//! Forked from arbor_day's property editor (X48). The rows read values and never write them: a
//! widget edits a copy, and a changed copy becomes an edit (see [`edit`]).

pub mod edit;

use std::{ops::RangeInclusive, sync::Arc};

use bevy::{
    annotations::{DisplayName, ReadOnly, Step, ValueRange},
    app::PropagateOver,
    feathers::{
        constants::{fonts, size},
        containers::{subpane, subpane_body, subpane_header},
        controls::{
            ColorInputValue, FeathersCheckbox, FeathersColorInput, FeathersNumberInput, HardLimit,
            NumberInputStep,
        },
        display::label,
        theme::{ThemeTextColor, ThemeToken, ThemedText},
        tokens,
    },
    prelude::*,
    reflect::{attributes::CustomAttributes, ReflectRef, TypeInfo},
    text::FontSourceTemplate,
    ui_widgets::NumericRange,
};
use bevy_immediate::{
    impl_capability_set,
    ui::{
        activated, anchored, base, button_variant,
        checked::{self, ImmUiChecked},
        clicked, color_plane, color_swatch, disabled, interaction, layout_order, look,
        number_input::{self, ImmUiNumberInput, ImmUiNumericValue},
        selected, slider_base_color, slider_value, text, text_input,
        track_value_change_plugin::{NewValueChange, TrackValueChangePlugin},
        ImplCapsUiFeathers,
    },
    CapSet, Imm, ImmCapAccessRequests, ImmCapability, ImmEntity,
};
use panoply_core::HexColor;

use edit::{PropertyEdits, PropertyRoot};

/// The panel's capabilities: `bevy_immediate`'s for feathers, and binding a color input.
pub struct CapsPanel;

impl_capability_set!(
    CapsPanel,
    ImplCapsPanel > ImplCapsUiFeathers,
    (
        base::CapabilityUiBase,
        layout_order::CapabilityUiLayoutOrder,
        look::CapabilityUiLook,
        disabled::CapabilityUiDisabled,
        interaction::CapabilityUiInteraction,
        text::CapabilityUiText,
        text_input::CapabilityUiTextInput,
        selected::CapabilityUiSelectable,
        checked::CapabilityUiChecked,
        clicked::CapabilityUiClicked,
        activated::CapabilityUiActivated,
        anchored::CapabilityUiAnchored,
        slider_value::CapabilityUiSliderValue,
        number_input::CapabilityUiNumberInput,
        slider_base_color::CapabilityUiSliderBaseColor,
        button_variant::CapabilityUiFeathersButtonVariant,
        color_swatch::CapabilityUiColorSwatch,
        color_plane::CapabilityUiColorPlane,
        CapabilityColorInput,
    )
);

/// Lets the panel bind a feathers color input: read the color it shows, and take the color the
/// user last picked.
pub struct CapabilityColorInput;

impl ImmCapability for CapabilityColorInput {
    fn build<Caps: CapSet>(app: &mut App, cap_req: &mut ImmCapAccessRequests<Caps>) {
        cap_req.request_component_read::<ColorInputValue>(app.world_mut());
        cap_req.request_component_write::<NewValueChange<Color>>(app.world_mut());
        if !app.is_plugin_added::<TrackValueChangePlugin<Color>>() {
            app.add_plugins(TrackValueChangePlugin::<Color>::default());
        }
    }
}

/// A number type the editor shows in a number input.
trait NumberField: ImmUiNumericValue + PartialReflect {
    fn range(range: &RangeInclusive<f64>) -> NumericRange;
}

impl NumberField for f32 {
    fn range(range: &RangeInclusive<f64>) -> NumericRange {
        NumericRange::F32(*range.start() as f32..=*range.end() as f32)
    }
}

impl NumberField for f64 {
    fn range(range: &RangeInclusive<f64>) -> NumericRange {
        NumericRange::F64(range.clone())
    }
}

impl NumberField for i32 {
    fn range(range: &RangeInclusive<f64>) -> NumericRange {
        NumericRange::I32(*range.start() as i32..=*range.end() as i32)
    }
}

impl NumberField for i64 {
    fn range(range: &RangeInclusive<f64>) -> NumericRange {
        NumericRange::I64(*range.start() as i64..=*range.end() as i64)
    }
}

/// The color of a pane's caption: feathers' own header text is too dim against the panel.
// TODO: a feathers token of its own for bright text.
pub const HEADER_TEXT: ThemeToken = tokens::LISTROW_TEXT;

/// A subpane captioned with `root`'s caption, holding the rows of `value`, `root`'s value. The
/// edits its widgets ask for go to `edits`.
pub fn root_pane(
    ui: &mut Imm<CapsPanel>,
    root: &Arc<dyn PropertyRoot>,
    value: &dyn PartialReflect,
    edits: &mut PropertyEdits,
) {
    let caption = root.caption();
    ui.ch_id(root.key())
        .on_spawn_apply_scene(subpane)
        .add(|ui| {
            ui.ch().on_spawn_apply_scene(subpane_header).add(|ui| {
                ui.ch().on_spawn_apply_scene(move || {
                    bsn! { Text({caption.clone()}) ThemeTextColor(HEADER_TEXT) }
                });
            });
            ui.ch()
                .on_spawn_apply_scene(subpane_body)
                .add(|ui| root_rows(ui, root, value, edits));
        });
}

/// The rows of `value`, `root`'s value, in whatever holds them: a struct's fields with no heading
/// of their own, or another value's one row. The edits its widgets ask for go to `edits`.
pub fn root_rows(
    ui: &mut Imm<CapsPanel>,
    root: &Arc<dyn PropertyRoot>,
    value: &dyn PartialReflect,
    edits: &mut PropertyEdits,
) {
    let mut rows = Rows { root, edits };
    match value.reflect_ref() {
        ReflectRef::Struct(_) => rows.fields(ui, value, ""),
        _ => rows.row(
            ui,
            short_type_name(value),
            &CustomAttributes::default(),
            value,
            String::new(),
        ),
    }
}

/// The short name of `value`'s type, as "ActorScale".
fn short_type_name(value: &dyn PartialReflect) -> &'static str {
    value
        .get_represented_type_info()
        .map_or("?", |info| info.type_path_table().short_path())
}

/// Builds the rows of one root's value, sending the edits they ask for to `edits`.
struct Rows<'a> {
    root: &'a Arc<dyn PropertyRoot>,
    edits: &'a mut PropertyEdits,
}

impl Rows<'_> {
    /// Asks for `value` to be written at `path`.
    fn set(&mut self, path: &str, value: impl PartialReflect) {
        self.edits.set(self.root, path.to_string(), value);
    }

    /// The rows of the fields of `value`, a struct at `path`.
    fn fields(&mut self, ui: &mut Imm<CapsPanel>, value: &dyn PartialReflect, path: &str) {
        let (Some(TypeInfo::Struct(info)), ReflectRef::Struct(fields)) =
            (value.get_represented_type_info(), value.reflect_ref())
        else {
            return;
        };
        for (index, field_info) in info.iter().enumerate() {
            if let Some(field) = fields.field_at(index) {
                let name = field_info.name();
                self.row(
                    ui,
                    name,
                    field_info.custom_attributes(),
                    field,
                    format!("{path}.{name}"),
                );
            }
        }
    }

    /// The row of `value`, at `path`, captioned `caption` unless its attributes `attrs` name it.
    fn row(
        &mut self,
        ui: &mut Imm<CapsPanel>,
        caption: &'static str,
        attrs: &CustomAttributes,
        value: &dyn PartialReflect,
        path: String,
    ) {
        let caption = attrs.get::<DisplayName>().map_or(caption, |name| name.0);
        // An `Option` is its value's row, or no row while it has none.
        if is_option(value) {
            if let ReflectRef::Enum(option) = value.reflect_ref() {
                if let Some(inner) = option.field_at(0) {
                    self.row(ui, caption, attrs, inner, format!("{path}.0"));
                }
            }
            return;
        }
        // A newtype is its field's row.
        if let Some(inner) = newtype_field(value) {
            if value.try_downcast_ref::<HexColor>().is_none() {
                self.row(ui, caption, attrs, inner, format!("{path}.0"));
                return;
            }
        }
        // Rows are keyed by path, so they keep their entities from frame to frame.
        let row = ui.ch_id(path.clone());
        if attrs.get::<ReadOnly>().is_some() {
            read_only_row(row, caption, value);
            return;
        }
        if let Some(&flag) = value.try_downcast_ref::<bool>() {
            let mut edited = flag;
            row.on_spawn_apply_scene(move || {
                bsn! { @FeathersCheckbox { @caption: bsn! { Text(caption) ThemedText } } }
            })
            .on_spawn_insert(move || Name::new(caption))
            .checked(&mut edited);
            if edited != flag {
                self.set(&path, edited);
            }
        } else if let Some(&number) = value.try_downcast_ref::<f32>() {
            self.number_row(row, caption, attrs, number, &path);
        } else if let Some(&number) = value.try_downcast_ref::<f64>() {
            self.number_row(row, caption, attrs, number, &path);
        } else if let Some(&number) = value.try_downcast_ref::<i32>() {
            self.number_row(row, caption, attrs, number, &path);
        } else if let Some(&number) = value.try_downcast_ref::<i64>() {
            self.number_row(row, caption, attrs, number, &path);
        } else if let Some(&vector) = value.try_downcast_ref::<Vec3>() {
            self.vec3_row(row, caption, attrs, vector, &path);
        } else if let Some(&color) = value.try_downcast_ref::<HexColor>() {
            let mut edited = Color::Srgba(color.0);
            color_row(row, caption, &mut edited);
            if edited != Color::Srgba(color.0) {
                self.set(&path, HexColor(edited.to_srgba()));
            }
        } else if let ReflectRef::Struct(_) = value.reflect_ref() {
            row.on_spawn_insert(move || (Name::new(caption), heading_node()))
                .add(|ui| {
                    ui.ch()
                        .on_spawn_apply_scene(move || heading_text(caption, tokens::TEXT_MAIN));
                    ui.ch().on_spawn_insert(indented_node).add(|ui| {
                        self.fields(ui, value, &path);
                    });
                });
        } else {
            read_only_row(row, caption, value);
        }
    }

    /// A number input captioned inside, named `input` so plans can read its value.
    fn number_row<T: NumberField>(
        &mut self,
        row: ImmEntity<CapsPanel>,
        caption: &'static str,
        attrs: &CustomAttributes,
        value: T,
        path: &str,
    ) {
        let look = NumberLook::from_attributes::<T>(attrs);
        let mut edited = value;
        row.on_spawn_insert(move || row_node(caption)).add(|ui| {
            number_input(
                ui.ch(),
                "input",
                Some(caption),
                look,
                percent(100),
                &mut edited,
            );
        });
        if edited != value {
            self.set(path, edited);
        }
    }

    /// Three number inputs on one line, captioned x, y and z inside, after the row's caption.
    fn vec3_row(
        &mut self,
        row: ImmEntity<CapsPanel>,
        caption: &'static str,
        attrs: &CustomAttributes,
        value: Vec3,
        path: &str,
    ) {
        let look = NumberLook::from_attributes::<f32>(attrs);
        let mut edited = value;
        row.on_spawn_insert(move || row_node(caption)).add(|ui| {
            ui.ch().on_spawn_apply_scene(move || label(caption));
            ui.ch().on_spawn_insert(widget_line).add(|ui| {
                let axes = [
                    ("x", &mut edited.x),
                    ("y", &mut edited.y),
                    ("z", &mut edited.z),
                ];
                for (axis, component) in axes {
                    number_input(
                        ui.ch(),
                        axis,
                        Some(axis),
                        look.clone(),
                        Val::Auto,
                        component,
                    );
                }
            });
        });
        for (axis, old, new) in [
            ("x", value.x, edited.x),
            ("y", value.y, edited.y),
            ("z", value.z, edited.z),
        ] {
            if new != old {
                self.set(&format!("{path}.{axis}"), new);
            }
        }
    }
}

/// Whether `value` is an `Option`.
fn is_option(value: &dyn PartialReflect) -> bool {
    value
        .get_represented_type_info()
        .is_some_and(|info| info.type_path_table().module_path() == Some("core::option"))
}

/// The one field of `value`, if it is a tuple struct with one field.
fn newtype_field(value: &dyn PartialReflect) -> Option<&dyn PartialReflect> {
    match value.reflect_ref() {
        ReflectRef::TupleStruct(tuple) if tuple.field_len() == 1 => tuple.field(0),
        _ => None,
    }
}

/// A row giving `caption` and `value` as text, for a value the panel cannot edit.
fn read_only_row(row: ImmEntity<CapsPanel>, caption: &'static str, value: &dyn PartialReflect) {
    let text = format!("{caption}: {}", summary(value));
    row.on_spawn_apply_scene(move || label(text.clone()))
        .on_spawn_insert(move || Name::new(caption));
}

/// `value` in a few words: a variant's name, a collection's length, a string or a number as
/// itself, and otherwise its type.
fn summary(value: &dyn PartialReflect) -> String {
    if let Some(text) = value.try_downcast_ref::<String>() {
        return format!("\"{text}\"");
    }
    match value.reflect_ref() {
        ReflectRef::Enum(variant) => variant.variant_name().to_string(),
        ReflectRef::List(list) => format!("{} items", list.len()),
        ReflectRef::Array(array) => format!("{} items", array.len()),
        ReflectRef::Map(map) => format!("{} items", map.len()),
        ReflectRef::Set(set) => format!("{} items", set.len()),
        ReflectRef::Opaque(_) => {
            let text = format!("{value:?}");
            // An opaque type without `Debug` prints as its type.
            match text.starts_with("Reflect(") {
                true => short_type_name(value).to_string(),
                false => text,
            }
        }
        _ => short_type_name(value).to_string(),
    }
}

/// How a number input limits and steps its number, read from a field's annotations.
#[derive(Clone)]
struct NumberLook {
    range: Option<NumericRange>,
    step: Option<f64>,
}

impl NumberLook {
    fn from_attributes<T: NumberField>(attrs: &CustomAttributes) -> Self {
        Self {
            range: attrs.get::<ValueRange>().map(|range| T::range(&range.0)),
            step: attrs.get::<Step>().map(|step| step.0),
        }
    }
}

/// A row node named after its field, holding widgets that carry their caption inside.
fn row_node(name: &'static str) -> (Name, Node) {
    (
        Name::new(name),
        Node {
            flex_direction: FlexDirection::Column,
            ..default()
        },
    )
}

/// A heading's text, bold, in `color`.
fn heading_text(caption: &'static str, color: ThemeToken) -> impl Scene {
    bsn! {
        Text(caption)
        TextFont {
            font: FontSourceTemplate::Handle(fonts::BOLD),
            font_size: size::SMALL_FONT,
            weight: FontWeight::BOLD,
        }
        PropagateOver<TextFont>
        ThemeTextColor({color})
    }
}

/// A nested struct's heading and rows, spaced as the subpane's own rows are.
fn heading_node() -> Node {
    Node {
        flex_direction: FlexDirection::Column,
        row_gap: px(4),
        ..default()
    }
}

/// A nested struct's rows, set in a little from its heading. The step is small and the same at
/// every depth, so nesting leaves the values most of the panel's width.
fn indented_node() -> Node {
    Node {
        flex_direction: FlexDirection::Column,
        row_gap: px(4),
        padding: UiRect::left(px(8)),
        ..default()
    }
}

/// Widgets laid out left to right.
fn widget_line() -> Node {
    Node {
        flex_direction: FlexDirection::Row,
        align_items: AlignItems::Center,
        // As wide as the gap between rows.
        column_gap: px(4),
        ..default()
    }
}

/// A caption at the left end of a row and a widget at the right.
fn caption_line() -> Node {
    Node {
        justify_content: JustifyContent::SpaceBetween,
        ..widget_line()
    }
}

/// A number input named `name`, its caption, if any, inside it, `width` wide, or taking a widget
/// line's spare width if `Val::Auto`.
fn number_input<T: NumberField>(
    input: ImmEntity<CapsPanel>,
    name: &'static str,
    caption: Option<&'static str>,
    look: NumberLook,
    width: Val,
    value: &mut T,
) {
    let grow: f32 = if width == Val::Auto { 1.0 } else { 0.0 };
    let initial = value.to_number_input_value();
    // The input writes its text from the components present when its text field spawns, so the
    // value, limits and step go in before the scene. Binding on the same frame would write the
    // text a second time while the first write is still queued, and both would show ("11" for
    // 1); so the binding starts the frame after.
    let input = input
        .on_spawn_apply_commands(move |commands| {
            commands.insert((Name::new(name), initial));
            if let Some(range) = look.range {
                commands.insert(HardLimit(range));
            }
            if let Some(step) = look.step {
                commands.insert(NumberInputStep(step));
            }
        })
        .on_spawn_apply_scene(move || {
            bsn! {
                @FeathersNumberInput { @interior_label_text: {caption} }
                Node { flex_grow: {grow}, width: {width} }
            }
        });
    if !input.will_be_spawned() {
        input.number_input(value);
    }
}

/// A caption with a feathers color input, a swatch that opens a picker, at the row's right end.
/// The input is named `input`.
fn color_row(row: ImmEntity<CapsPanel>, caption: &'static str, color: &mut Color) {
    row.on_spawn_insert(move || (Name::new(caption), caption_line()))
        .add(|ui| {
            ui.ch().on_spawn_apply_scene(move || label(caption));
            let initial = *color;
            let mut input = ui
                .ch()
                .on_spawn_apply_scene(|| bsn! { #input @FeathersColorInput })
                .on_spawn_insert(move || {
                    (ColorInputValue(initial), NewValueChange::<Color>::default())
                });
            if input.will_be_spawned() {
                return;
            }
            if let Ok(Some(mut picked)) = input.cap_get_component_mut::<NewValueChange<Color>>() {
                if let Some(value) = NewValueChange::take(&mut picked) {
                    *color = value;
                }
            }
            // Shows a color changed elsewhere, or just picked: the input does not update itself.
            let shown = input
                .cap_get_component::<ColorInputValue>()
                .ok()
                .flatten()
                .map(|value| value.0);
            if shown != Some(*color) {
                input.entity_commands().insert(ColorInputValue(*color));
            }
        });
}
