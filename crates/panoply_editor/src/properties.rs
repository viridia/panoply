//! The property editor's rows: a subpane per inspected value, and in it a row per field, named
//! after its field. Booleans get a checkbox, numbers a number input carrying its caption inside, a
//! `Vec3` three of them, colors a swatch that opens a picker, strings a text input committed on
//! Enter or on leaving it, nested structs a heading over their own rows, indented. A newtype shows
//! its one field's row. An `Option` is its value's row, with a × that removes the value, or no row
//! while it is `None`: a struct's heading has a menu of its absent fields, which adds one with its
//! type's default. Every other field is a row that shows its value without editing it. Annotations
//! from `bevy::annotations` set a row's caption, a number's range and step, and whether it can be
//! edited; on an `Option`, they apply to its value.
//!
//! Forked from arbor_day's property editor (X48). The rows read values and never write them: a
//! widget edits a copy, and a changed copy becomes an edit (see [`edit`]).

pub mod edit;

use std::{ops::RangeInclusive, sync::Arc};

use bevy::{
    annotations::{DisplayName, ReadOnly, Step, ValueRange},
    app::PropagateOver,
    feathers::{
        constants::{fonts, icons, size},
        containers::{subpane, subpane_body, subpane_header},
        controls::{
            ButtonVariant, ColorInputValue, FeathersCheckbox, FeathersColorInput,
            FeathersListRow, FeathersMenu, FeathersMenuItem, FeathersMenuPopup,
            FeathersMenuToolButton, FeathersNumberInput, FeathersSelect, FeathersTextInput,
            FeathersTextInputContainer, FeathersToolButton, HardLimit, NumberInputSnapToStep,
            NumberInputStep, OptionIndex,
        },
        display::{caption as option_caption, icon, label},
        theme::{ThemeBorderColor, ThemeTextColor, ThemeToken, ThemedText},
        tokens,
    },
    input_focus::InputFocus,
    prelude::*,
    reflect::{
        attributes::CustomAttributes,
        enums::{DynamicEnum, DynamicVariant, Enum, VariantInfo, VariantType},
        std_traits::ReflectDefault,
        structs::DynamicStruct,
        tuple::DynamicTuple,
        ReflectFromReflect, ReflectRef, TypeInfo, TypeRegistry,
    },
    text::FontSourceTemplate,
    ui::Selected,
    ui_widgets::{
        popover::{Popover, PopoverAlign, PopoverPlacement, PopoverSide},
        NumericRange, SetSelected, ValueChange,
    },
};
use bevy_immediate::{
    impl_capability_set,
    ui::{
        activated, anchored, base, button_variant,
        checked::{self, ImmUiChecked},
        activated::ImmUiActivated,
        clicked, color_plane, color_swatch, disabled, interaction, layout_order, look,
        number_input::{self, ImmUiNumberInput, ImmUiNumericValue},
        selected, slider_base_color, slider_value, text,
        text_input::{self, ImmUiTextInput},
        track_value_change_plugin::{NewValueChange, TrackValueChangePlugin},
        ImplCapsUiFeathers,
    },
    CapSet, Imm, ImmCapAccessRequests, ImmCapability, ImmEntity,
};
use panoply_core::HexColor;

use edit::{EditKind, PropertyEdits, PropertyRoot};

/// The panel's capabilities: `bevy_immediate`'s for feathers, and binding a color input and a
/// text input's commits.
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
        CapabilityTextCommit,
        CapabilityEnumSelect,
    )
);

/// Lets the panel read and clear an enum select's [`PickedVariant`].
pub struct CapabilityEnumSelect;

impl ImmCapability for CapabilityEnumSelect {
    fn build<Caps: CapSet>(app: &mut App, cap_req: &mut ImmCapAccessRequests<Caps>) {
        cap_req.request_component_write::<PickedVariant>(app.world_mut());
    }
}

/// On an enum's select: the variant the user last picked, until the panel takes it, and the
/// variant the select shows.
#[derive(Component)]
pub struct PickedVariant {
    picked: Option<usize>,
    shown: usize,
}

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

/// Lets the panel take the text a text input last committed.
pub struct CapabilityTextCommit;

impl ImmCapability for CapabilityTextCommit {
    fn build<Caps: CapSet>(app: &mut App, cap_req: &mut ImmCapAccessRequests<Caps>) {
        cap_req.request_component_write::<CommittedText>(app.world_mut());
    }
}

/// The text a text input last committed, until the panel takes it.
#[derive(Component, Default)]
pub struct CommittedText(Option<String>);

/// Observer on a text input: keeps the text it commits for the panel.
fn keep_committed_text(change: On<ValueChange<String>>, mut texts: Query<&mut CommittedText>) {
    if let Ok(mut committed) = texts.get_mut(change.source) {
        committed.0 = Some(change.value.clone());
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

/// A subpane captioned with `root`'s caption, holding the rows of `value`, `root`'s value; a
/// struct's absent fields are in a menu at the caption's right. The edits its widgets ask for go
/// to `edits`; new values are built from the types in `registry`.
pub fn root_pane(
    ui: &mut Imm<CapsPanel>,
    root: &Arc<dyn PropertyRoot>,
    value: &dyn PartialReflect,
    registry: &TypeRegistry,
    edits: &mut PropertyEdits,
) {
    let caption = root.caption();
    let mut rows = Rows {
        root,
        registry,
        edits,
    };
    let name = caption.clone();
    ui.ch_id(root.key())
        .on_spawn_apply_scene(subpane)
        .on_spawn_insert(move || Name::new(name.clone()))
        .add(|ui| {
            ui.ch().on_spawn_apply_scene(subpane_header).add(|ui| {
                ui.ch().on_spawn_apply_scene(move || {
                    bsn! { Text({caption.clone()}) ThemeTextColor(HEADER_TEXT) }
                });
                rows.add_menu(ui, value, "");
                let (inner, path) = through_newtypes(value, String::new());
                rows.add_item_button(ui, inner, &path);
            });
            ui.ch()
                .on_spawn_apply_scene(subpane_body)
                .add(|ui| rows.root(ui, value));
        });
}

/// The rows of `value`, `root`'s value, in whatever holds them: a struct's fields with no heading
/// of their own, or another value's one row. The edits its widgets ask for go to `edits`; new
/// values are built from the types in `registry`.
pub fn root_rows(
    ui: &mut Imm<CapsPanel>,
    root: &Arc<dyn PropertyRoot>,
    value: &dyn PartialReflect,
    registry: &TypeRegistry,
    edits: &mut PropertyEdits,
) {
    Rows {
        root,
        registry,
        edits,
    }
    .root(ui, value);
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
    registry: &'a TypeRegistry,
    edits: &'a mut PropertyEdits,
}

impl Rows<'_> {
    /// Asks for `value` to be written at `path`.
    fn set(&mut self, path: &str, value: impl PartialReflect) {
        self.edits.set(self.root, path.to_string(), value);
    }

    /// Asks for the `Option` of type `option` at `path` to be set to `Some` holding its value
    /// type's default (`add`), or to `None`.
    fn add_or_remove(&mut self, path: &str, option: &TypeInfo, add: bool) {
        let kind = if add { EditKind::Add } else { EditKind::Remove };
        match option_value(self.registry, option, add) {
            Some(value) => self.edits.push(self.root, path.to_string(), kind, Some(value)),
            None => error!("Cannot make a {} value", option.type_path()),
        }
    }

    /// The rows of the root's value: a struct's fields, a list's items (through newtypes), or
    /// another value's one row.
    fn root(&mut self, ui: &mut Imm<CapsPanel>, value: &dyn PartialReflect) {
        let (inner, path) = through_newtypes(value, String::new());
        match (value.reflect_ref(), inner.reflect_ref()) {
            (ReflectRef::Struct(_), _) => self.fields(ui, value, ""),
            (_, ReflectRef::List(_)) => self.items(ui, inner, &path),
            _ => self.row(
                ui,
                short_type_name(value),
                &CustomAttributes::default(),
                value,
                String::new(),
                None,
            ),
        }
    }

    /// A boxed-plus menu of the absent fields of `value`, a struct at `path`: its `Option`
    /// fields that are `None` and can be edited. Picking one adds it. Nothing if none are absent.
    fn add_menu(&mut self, ui: &mut Imm<CapsPanel>, value: &dyn PartialReflect, path: &str) {
        let (Some(TypeInfo::Struct(info)), ReflectRef::Struct(fields)) =
            (value.get_represented_type_info(), value.reflect_ref())
        else {
            return;
        };
        let absent: Vec<_> = info
            .iter()
            .enumerate()
            .filter_map(|(index, field_info)| {
                let field = fields.field_at(index)?;
                let attrs = field_info.custom_attributes();
                let ReflectRef::Enum(option) = field.reflect_ref() else {
                    return None;
                };
                if !is_option(field) || option.variant_name() != "None" {
                    return None;
                }
                if attrs.get::<ReadOnly>().is_some() {
                    return None;
                }
                let caption = attrs
                    .get::<DisplayName>()
                    .map_or(field_info.name(), |name| name.0);
                Some((field_info.name(), caption, field.get_represented_type_info()?))
            })
            .collect();
        if absent.is_empty() {
            return;
        }
        let mut picked = None;
        ui.ch_id("add")
            .on_spawn_apply_scene(|| bsn! { @FeathersMenu #add })
            .add(|ui| {
                ui.ch().on_spawn_apply_scene(|| {
                    bsn! {
                        @FeathersMenuToolButton {
                            @caption: bsn! {
                                @icon(ADD_ICON) Node { height: px(18) }
                            },
                            @arrow: false,
                        }
                    }
                });
                ui.ch().on_spawn_apply_scene(right_aligned_popup).add(|ui| {
                    for &(name, caption, option) in &absent {
                        if menu_item(ui.ch_id(name), caption, format!("add_{name}")) {
                            picked = Some((name, option));
                        }
                    }
                });
            });
        if let Some((name, option)) = picked {
            self.add_or_remove(&format!("{path}.{name}"), option, true);
        }
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
                    None,
                );
            }
        }
    }

    /// The row of `value`, at `path`, captioned `caption` unless its attributes `attrs` name it.
    /// With `remove`, the row ends its widgets' line with a × that sets it.
    fn row(
        &mut self,
        ui: &mut Imm<CapsPanel>,
        caption: &'static str,
        attrs: &CustomAttributes,
        value: &dyn PartialReflect,
        path: String,
        remove: Option<&mut bool>,
    ) {
        let caption = attrs.get::<DisplayName>().map_or(caption, |name| name.0);
        // An `Option` is its value's row, ending in a × that removes the value, or no row while
        // it has none.
        if is_option(value) {
            let (ReflectRef::Enum(option), Some(info)) =
                (value.reflect_ref(), value.get_represented_type_info())
            else {
                return;
            };
            let Some(inner) = option.field_at(0) else {
                return;
            };
            let inner_path = format!("{path}.0");
            if attrs.get::<ReadOnly>().is_some() {
                self.row(ui, caption, attrs, inner, inner_path, remove);
                return;
            }
            let mut removed = false;
            self.row(ui, caption, attrs, inner, inner_path, Some(&mut removed));
            if removed {
                self.add_or_remove(&path, info, false);
            }
            return;
        }
        // A newtype is its field's row.
        if let Some(inner) = newtype_field(value) {
            if value.try_downcast_ref::<HexColor>().is_none() {
                self.row(ui, caption, attrs, inner, format!("{path}.0"), remove);
                return;
            }
        }
        // Rows are keyed by path, so they keep their entities from frame to frame.
        let row = ui.ch_id(path.clone());
        if attrs.get::<ReadOnly>().is_some() {
            read_only_row(row, caption, value, remove);
            return;
        }
        if let Some(&flag) = value.try_downcast_ref::<bool>() {
            let mut edited = flag;
            row.on_spawn_insert(move || (Name::new(caption), caption_line()))
                .add(|ui| {
                    ui.ch()
                        .on_spawn_apply_scene(move || {
                            bsn! {
                                @FeathersCheckbox { @caption: bsn! { Text(caption) ThemedText } }
                            }
                        })
                        .checked(&mut edited);
                    remove_button(ui, remove);
                });
            if edited != flag {
                self.set(&path, edited);
            }
        } else if let Some(&number) = value.try_downcast_ref::<f32>() {
            let shown = shown_f32(number);
            if let Some(edited) = self.number_row(row, caption, attrs, shown, remove) {
                self.set(&path, edited as f32);
            }
        } else if let Some(&number) = value.try_downcast_ref::<f64>() {
            if let Some(edited) = self.number_row(row, caption, attrs, number, remove) {
                self.set(&path, edited);
            }
        } else if let Some(&number) = value.try_downcast_ref::<i32>() {
            if let Some(edited) = self.number_row(row, caption, attrs, number, remove) {
                self.set(&path, edited);
            }
        } else if let Some(&number) = value.try_downcast_ref::<i64>() {
            if let Some(edited) = self.number_row(row, caption, attrs, number, remove) {
                self.set(&path, edited);
            }
        } else if let Some(&vector) = value.try_downcast_ref::<Vec3>() {
            self.vec3_row(row, caption, attrs, vector, &path, remove);
        } else if let Some(&color) = value.try_downcast_ref::<HexColor>() {
            let mut edited = Color::Srgba(color.0);
            color_row(row, caption, &mut edited, remove);
            if edited != Color::Srgba(color.0) {
                self.set(&path, HexColor(edited.to_srgba()));
            }
        } else if let Some(text) = value.try_downcast_ref::<String>() {
            if let Some(edited) = text_row(row, caption, text, remove) {
                self.set(&path, edited);
            }
        } else if let ReflectRef::Enum(_) = value.reflect_ref() {
            self.enum_row(row, caption, value, &path, remove);
        } else if let ReflectRef::List(_) = value.reflect_ref() {
            row.on_spawn_insert(move || (Name::new(caption), heading_node()))
                .add(|ui| {
                    ui.ch().on_spawn_insert(caption_line).add(|ui| {
                        ui.ch().on_spawn_apply_scene(move || {
                            heading_text(caption, tokens::TEXT_MAIN)
                        });
                        ui.ch().on_spawn_insert(widget_line).add(|ui| {
                            self.add_item_button(ui, value, &path);
                            remove_button(ui, remove);
                        });
                    });
                    ui.ch().on_spawn_apply_scene(indented_node).add(|ui| {
                        self.items(ui, value, &path);
                    });
                });
        } else if let ReflectRef::Struct(_) = value.reflect_ref() {
            row.on_spawn_insert(move || (Name::new(caption), heading_node()))
                .add(|ui| {
                    ui.ch().on_spawn_insert(caption_line).add(|ui| {
                        ui.ch().on_spawn_apply_scene(move || {
                            heading_text(caption, tokens::TEXT_MAIN)
                        });
                        ui.ch().on_spawn_insert(widget_line).add(|ui| {
                            self.add_menu(ui, value, &path);
                            remove_button(ui, remove);
                        });
                    });
                    ui.ch().on_spawn_apply_scene(indented_node).add(|ui| {
                        self.fields(ui, value, &path);
                    });
                });
        } else {
            read_only_row(row, caption, value, remove);
        }
    }

    /// A number input captioned inside, named `input` so plans can read its value. Returns the
    /// number edited in it, if it changed.
    fn number_row<T: NumberField>(
        &mut self,
        row: ImmEntity<CapsPanel>,
        caption: &'static str,
        attrs: &CustomAttributes,
        value: T,
        remove: Option<&mut bool>,
    ) -> Option<T> {
        let look = NumberLook::from_attributes::<T>(attrs);
        let mut edited = value;
        row.on_spawn_insert(move || (Name::new(caption), widget_line()))
            .add(|ui| {
                number_input(
                    ui.ch(),
                    "input",
                    NumberCaption::Inside(caption),
                    look,
                    Val::Auto,
                    &mut edited,
                );
                remove_button(ui, remove);
            });
        (edited != value).then_some(edited)
    }

    /// Three number inputs on one line, labeled X, Y and Z beside their axes' colored sigils,
    /// under the row's caption.
    fn vec3_row(
        &mut self,
        row: ImmEntity<CapsPanel>,
        caption: &'static str,
        attrs: &CustomAttributes,
        value: Vec3,
        path: &str,
        remove: Option<&mut bool>,
    ) {
        let look = NumberLook::from_attributes::<f64>(attrs);
        let shown = value.to_array().map(shown_f32);
        let mut edited = shown;
        row.on_spawn_insert(move || row_node(caption)).add(|ui| {
            ui.ch().on_spawn_apply_scene(move || label(caption));
            ui.ch().on_spawn_insert(widget_line).add(|ui| {
                let [x, y, z] = &mut edited;
                let axes = [
                    ("x", "X", tokens::TEXT_INPUT_X_AXIS, x),
                    ("y", "Y", tokens::TEXT_INPUT_Y_AXIS, y),
                    ("z", "Z", tokens::TEXT_INPUT_Z_AXIS, z),
                ];
                for (name, label, sigil, component) in axes {
                    number_input(
                        ui.ch(),
                        name,
                        NumberCaption::Sigil(label, sigil),
                        look.clone(),
                        Val::Auto,
                        component,
                    );
                }
                remove_button(ui, remove);
            });
        });
        for ((axis, old), new) in ["x", "y", "z"].into_iter().zip(shown).zip(edited) {
            if new != old {
                self.set(&format!("{path}.{axis}"), new as f32);
            }
        }
    }

    /// The rows of the items of `value`, a list at `path`, captioned "Item 1" and on, each with a
    /// × that removes it.
    fn items(&mut self, ui: &mut Imm<CapsPanel>, value: &dyn PartialReflect, path: &str) {
        let ReflectRef::List(list) = value.reflect_ref() else {
            return;
        };
        let mut removed = None;
        for (index, item) in list.iter().enumerate() {
            let mut remove = false;
            self.row(
                ui,
                item_caption(index),
                &CustomAttributes::default(),
                item,
                format!("{path}[{index}]"),
                Some(&mut remove),
            );
            if remove {
                removed = Some(index);
            }
        }
        if let Some(index) = removed {
            let kind = EditKind::RemoveItem(index);
            self.edits.push(self.root, path.to_string(), kind, None);
        }
    }

    /// If `value`, at `path`, is a list, a boxed plus, named `add_item`, that adds an item at its
    /// end, its type's default.
    fn add_item_button(&mut self, ui: &mut Imm<CapsPanel>, value: &dyn PartialReflect, path: &str) {
        let (ReflectRef::List(list), Some(TypeInfo::List(info))) =
            (value.reflect_ref(), value.get_represented_type_info())
        else {
            return;
        };
        if !icon_button(ui, "add_item", ADD_ICON, px(18), tokens::TEXT_MAIN) {
            return;
        }
        let item = self
            .registry
            .get_type_data::<ReflectDefault>(info.item_ty().id())
            .map(|default| default.default().into_partial_reflect());
        match item {
            Some(item) => {
                let kind = EditKind::AddItem(list.len());
                self.edits.push(self.root, path.to_string(), kind, Some(item));
            }
            None => error!("Cannot make a {} item", info.item_ty().path()),
        }
    }

    /// A caption with a select of the variants of `value`, an enum at `path`; below it, the
    /// fields of the variant chosen, set in. Picking another variant asks for it, carrying over
    /// the fields of the same name and type.
    fn enum_row(
        &mut self,
        row: ImmEntity<CapsPanel>,
        caption: &'static str,
        value: &dyn PartialReflect,
        path: &str,
        remove: Option<&mut bool>,
    ) {
        let (ReflectRef::Enum(current), Some(TypeInfo::Enum(info))) =
            (value.reflect_ref(), value.get_represented_type_info())
        else {
            return;
        };
        let index = current.variant_index();
        let mut picked = None;
        row.on_spawn_insert(move || (Name::new(caption), heading_node()))
            .add(|ui| {
                ui.ch().on_spawn_insert(caption_line).add(|ui| {
                    ui.ch().on_spawn_apply_scene(move || label(caption));
                    ui.ch().on_spawn_insert(widget_line).add(|ui| {
                        picked = variant_select(ui, info.variant_names(), index);
                        remove_button(ui, remove);
                    });
                });
                if current.field_len() > 0 {
                    // Keyed by variant, so another variant's fields get rows of their own.
                    ui.ch_id(current.variant_name())
                        .on_spawn_apply_scene(indented_node)
                        .add(|ui| self.variant_fields(ui, current, info, path));
                }
            });
        let Some(picked) = picked.filter(|&picked| picked != index) else {
            return;
        };
        let name = info.variant_names()[picked];
        match variant_value(self.registry, current, name) {
            Some(value) => {
                self.edits
                    .push(self.root, path.to_string(), EditKind::Set, Some(value))
            }
            None => error!("Cannot make a {}::{name}", info.type_path()),
        }
    }

    /// The rows of the fields of `value`'s variant, `value` an enum of type `info` at `path`.
    fn variant_fields(
        &mut self,
        ui: &mut Imm<CapsPanel>,
        value: &dyn Enum,
        info: &bevy::reflect::enums::EnumInfo,
        path: &str,
    ) {
        let Some(variant) = info.variant(value.variant_name()) else {
            return;
        };
        for index in 0..value.field_len() {
            let Some(field) = value.field_at(index) else {
                continue;
            };
            let (caption, attrs, field_path) = match variant {
                VariantInfo::Struct(variant) => {
                    let Some(field_info) = variant.field_at(index) else {
                        continue;
                    };
                    let name = field_info.name();
                    (name, field_info.custom_attributes(), format!("{path}.{name}"))
                }
                VariantInfo::Tuple(variant) => {
                    let Some(field_info) = variant.field_at(index) else {
                        continue;
                    };
                    let caption = short_type_name(field);
                    (caption, field_info.custom_attributes(), format!("{path}.{index}"))
                }
                VariantInfo::Unit(_) => return,
            };
            self.row(ui, caption, attrs, field, field_path, None);
        }
    }
}

/// An `f32` as the number input is given it: the `f64` of its shortest decimal, 0.35 rather than
/// 0.3499999940395355, so the input's chevrons step off it.
// TODO(I39): give the input the `f32` once feathers' chevrons allow for an `f32`'s error.
fn shown_f32(value: f32) -> f64 {
    value.to_string().parse().unwrap_or(value.into())
}

/// The boxed plus that adds a field or an item.
const ADD_ICON: &str = "editor/icons/add_box.png";

/// "Item 1", "Item 2" and on: a list item's caption, for its `index`. Each is made once and kept,
/// since rows take their captions for good.
fn item_caption(index: usize) -> &'static str {
    static CAPTIONS: std::sync::Mutex<Vec<&'static str>> = std::sync::Mutex::new(Vec::new());
    let mut captions = CAPTIONS.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    while captions.len() <= index {
        let caption = format!("Item {}", captions.len() + 1);
        captions.push(Box::leak(caption.into_boxed_str()));
    }
    captions[index]
}

/// `value` and its `path` within the root, or, while it is a newtype other than a color, its one
/// field and that field's path.
fn through_newtypes(value: &dyn PartialReflect, path: String) -> (&dyn PartialReflect, String) {
    match newtype_field(value) {
        Some(inner) if value.try_downcast_ref::<HexColor>().is_none() => {
            through_newtypes(inner, format!("{path}.0"))
        }
        _ => (value, path),
    }
}

/// The value of `current`'s enum type as its variant `name`: its fields carried over from
/// `current` where it has one of the same name (or, in a tuple variant, place) and type, the rest
/// their types' defaults. A real value of the type, built through `registry`.
pub(crate) fn variant_value(
    registry: &TypeRegistry,
    current: &dyn Enum,
    name: &str,
) -> Option<Box<dyn PartialReflect>> {
    let info = current.get_represented_type_info()?;
    let variant = info.as_enum().ok()?.variant(name)?;
    let field = |old: Option<&dyn PartialReflect>, type_id| -> Option<Box<dyn PartialReflect>> {
        let same_type = |old: &&dyn PartialReflect| {
            old.get_represented_type_info()
                .is_some_and(|info| info.type_id() == type_id)
        };
        match old.filter(same_type) {
            Some(old) => Some(old.reflect_clone().ok()?.into_partial_reflect()),
            None => Some(
                registry
                    .get_type_data::<ReflectDefault>(type_id)?
                    .default()
                    .into_partial_reflect(),
            ),
        }
    };
    let dynamic = match variant {
        VariantInfo::Unit(_) => DynamicVariant::Unit,
        VariantInfo::Struct(variant) => {
            let mut fields = DynamicStruct::default();
            for info in variant.iter() {
                let old = current.field(info.name());
                fields.insert_boxed(info.name(), field(old, info.type_id())?);
            }
            DynamicVariant::Struct(fields)
        }
        VariantInfo::Tuple(variant) => {
            let mut fields = DynamicTuple::default();
            for (index, info) in variant.iter().enumerate() {
                let old = (current.variant_type() == VariantType::Tuple)
                    .then(|| current.field_at(index))
                    .flatten();
                fields.insert_boxed(field(old, info.type_id())?);
            }
            DynamicVariant::Tuple(fields)
        }
    };
    let value = registry
        .get(info.type_id())?
        .data::<ReflectFromReflect>()?
        .from_reflect(&DynamicEnum::new(name, dynamic))?;
    Some(value.into_partial_reflect())
}

/// A select, named `select`, of the variants `names`, showing the one at `index`, each option
/// named after its variant so plans can pick one. Returns the index of the variant picked.
fn variant_select(
    ui: &mut Imm<CapsPanel>,
    names: &'static [&'static str],
    index: usize,
) -> Option<usize> {
    let mut select = ui
        .ch_id("select")
        .on_spawn_insert(move || PickedVariant {
            picked: None,
            shown: index,
        })
        .on_spawn_observe(pick_variant)
        .on_spawn_apply_scene(move || {
            let rows: Vec<Box<dyn SceneList>> = names
                .iter()
                .enumerate()
                .map(|(option, &name)| -> Box<dyn SceneList> {
                    if option == index {
                        bsn! { @FeathersListRow Selected OptionIndex(option) Name(name) Children [ @option_caption(name) ] }.into()
                    } else {
                        bsn! { @FeathersListRow OptionIndex(option) Name(name) Children [ @option_caption(name) ] }.into()
                    }
                })
                .collect();
            let rows: Box<dyn SceneList> = Box::new(rows);
            bsn! { @FeathersSelect { @options: {rows} } #select Node { min_width: px(96) } }
        });
    let Ok(Some(mut state)) = select.cap_get_component_mut::<PickedVariant>() else {
        return None;
    };
    let picked = state.picked.take();
    if picked.is_none() && state.shown != index {
        // Changed elsewhere, by undo: select the matching option.
        state.shown = index;
        let entity = select.entity();
        select
            .commands()
            .queue(move |world: &mut World| show_variant(world, entity, index));
    }
    picked
}

/// Records the variant picked in an enum's select, for the panel to take on its next pass.
fn pick_variant(
    change: On<ValueChange<Entity>>,
    options: Query<&OptionIndex>,
    mut selects: Query<&mut PickedVariant>,
) {
    let (Ok(option), Ok(mut state)) = (
        options.get(change.value),
        selects.get_mut(change.event_target()),
    ) else {
        return;
    };
    state.picked = Some(option.0);
    state.shown = option.0;
}

/// Selects the option for variant `index` in the enum select `select`.
fn show_variant(world: &mut World, select: Entity, index: usize) {
    let mut options = world.query::<(Entity, &OptionIndex)>();
    let mut parents = world.query::<&ChildOf>();
    let row = options.iter(world).find_map(|(row, option)| {
        let mut ancestor = row;
        while let Ok(parent) = parents.get(world, ancestor) {
            ancestor = parent.parent();
            if ancestor == select {
                return (option.0 == index).then_some(row);
            }
        }
        None
    });
    if let Some(row) = row {
        world.trigger(SetSelected { entity: select, row });
    }
}

/// The value of the `Option` type `option`: `Some` holding its value type's default if `some`,
/// else `None`. A real value of the type, built through `registry`, not a dynamic stand-in.
pub(crate) fn option_value(
    registry: &TypeRegistry,
    option: &TypeInfo,
    some: bool,
) -> Option<Box<dyn PartialReflect>> {
    let registration = registry.get(option.type_id())?;
    if !some {
        let none = registration.data::<ReflectDefault>()?.default();
        return Some(none.into_partial_reflect());
    }
    let VariantInfo::Tuple(variant) = option.as_enum().ok()?.variant("Some")? else {
        return None;
    };
    let inner = registry
        .get_type_data::<ReflectDefault>(variant.field_at(0)?.type_id())?
        .default();
    let mut fields = DynamicTuple::default();
    fields.insert_boxed(inner.into_partial_reflect());
    let dynamic = DynamicEnum::new("Some", DynamicVariant::Tuple(fields));
    let value = registration
        .data::<ReflectFromReflect>()?
        .from_reflect(&dynamic)?;
    Some(value.into_partial_reflect())
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

/// A caption with a text input, named `input`, showing `value`. Returns the text the input
/// commits, on Enter or on losing focus (`text_commit`). The input shows a value changed
/// elsewhere, by undo, only while it is not being edited. With `remove`, a × ends the row.
fn text_row(
    row: ImmEntity<CapsPanel>,
    caption: &'static str,
    value: &str,
    remove: Option<&mut bool>,
) -> Option<String> {
    let mut committed = None;
    row.on_spawn_insert(move || (Name::new(caption), widget_line()))
        .add(|ui| {
            ui.ch().on_spawn_apply_scene(move || label(caption));
            ui.ch()
                .on_spawn_apply_scene(|| {
                    bsn! { @FeathersTextInputContainer Node { flex_grow: 1.0, min_width: px(0) } }
                })
                .add(|ui| {
                    let mut input = ui
                        .ch()
                        .on_spawn_apply_scene(|| bsn! { @FeathersTextInput #input })
                        .on_spawn_apply_commands(|commands| {
                            commands
                                .insert(CommittedText::default())
                                .observe(keep_committed_text);
                        });
                    if let Ok(Some(mut text)) = input.cap_get_component_mut::<CommittedText>() {
                        committed = text.0.take();
                    }
                    let focused = input
                        .cap_get_resource::<InputFocus>()
                        .ok()
                        .and_then(|focus| focus.get())
                        == Some(input.entity());
                    // A text just committed shows until the edit it asks for is made.
                    if !focused && committed.is_none() {
                        input.input_set(value);
                    }
                });
            remove_button(ui, remove);
        });
    committed.filter(|text| text != value)
}

/// With `remove`, a dim ×, named `remove`, at the end of a row's widget line, which sets it when
/// clicked: every optional row's × lines up at the panel's right.
fn remove_button(ui: &mut Imm<CapsPanel>, remove: Option<&mut bool>) {
    if let Some(remove) = remove {
        *remove |= icon_button(ui, "remove", icons::X, px(12), tokens::TEXT_DIM);
    }
}

/// A plain tool button named `name` showing the icon `image`, `height` high, tinted `color`.
/// Returns whether it was clicked.
// TODO: a dim button variant upstream, which would also brighten on hover.
fn icon_button(
    ui: &mut Imm<CapsPanel>,
    name: &'static str,
    image: &'static str,
    height: Val,
    color: ThemeToken,
) -> bool {
    let mut button = ui.ch_id(name).on_spawn_apply_scene(move || {
        bsn! {
            @FeathersToolButton { @variant: {ButtonVariant::Plain} }
            Name({name})
        }
    });
    let clicked = button.activated();
    button.add(|ui| {
        ui.ch_id(image)
            .on_spawn_apply_scene(move || {
                // `ThemeTextColor` colors only a `TextColor` already there as it is inserted.
                bsn! { @icon(image) Node { height: {height} } TextColor ThemeTextColor({color}) }
            });
    });
    clicked
}

/// A feathers menu popup aligned to its button's right edge, for a menu button at a row's right
/// end.
fn right_aligned_popup() -> impl Scene {
    bsn! {
        @FeathersMenuPopup
        Popover {
            positions: vec![
                PopoverPlacement {
                    side: PopoverSide::Bottom,
                    align: PopoverAlign::End,
                    gap: 2.0,
                },
                PopoverPlacement {
                    side: PopoverSide::Top,
                    align: PopoverAlign::End,
                    gap: 2.0,
                },
            ],
            window_margin: 10.0,
        }
    }
}

/// A feathers menu item showing `caption`, named `name`. Returns whether it was picked.
fn menu_item(item: ImmEntity<CapsPanel>, caption: &'static str, name: String) -> bool {
    item.on_spawn_apply_scene(move || {
        bsn! {
            @FeathersMenuItem { @caption: bsn! { Text(caption) ThemedText } }
            Name({name.clone()})
        }
    })
    .activated()
}

/// A row giving `caption` and `value` as text, for a value the panel cannot edit.
/// With `remove`, a × ends the row.
fn read_only_row(
    row: ImmEntity<CapsPanel>,
    caption: &'static str,
    value: &dyn PartialReflect,
    remove: Option<&mut bool>,
) {
    let text = format!("{caption}: {}", summary(value));
    row.on_spawn_insert(move || (Name::new(caption), caption_line()))
        .add(|ui| {
            ui.ch_id(text.clone())
                .on_spawn_apply_scene(move || label(text.clone()));
            remove_button(ui, remove);
        });
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

/// A nested value's rows, set in a little from its heading behind a dim stripe, which shows how
/// deep they are. The step is small and the same at every depth, so nesting leaves the values
/// most of the panel's width.
fn indented_node() -> impl Scene {
    bsn! {
        Node {
            flex_direction: FlexDirection::Column,
            row_gap: px(4),
            border: UiRect::left(px(4)),
            padding: UiRect::left(px(4)),
        }
        ThemeBorderColor(tokens::SUBPANE_HEADER_BORDER)
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

/// Where a number input shows its caption.
#[derive(Clone)]
enum NumberCaption {
    /// Inside the input, at its left end.
    Inside(&'static str),
    /// At its left, beside a sigil strip in the token's color, as for an axis.
    Sigil(&'static str, ThemeToken),
}

/// A number input named `name`, captioned as `caption` says, `width` wide, or taking a widget
/// line's spare width if `Val::Auto`.
fn number_input<T: NumberField>(
    input: ImmEntity<CapsPanel>,
    name: &'static str,
    caption: NumberCaption,
    look: NumberLook,
    width: Val,
    value: &mut T,
) {
    let grow: f32 = if width == Val::Auto { 1.0 } else { 0.0 };
    let (interior, label, sigil) = match caption {
        NumberCaption::Inside(text) => (Some(text), None, tokens::TEXT_INPUT_LABEL_BG),
        NumberCaption::Sigil(text, sigil) => (None, Some(text), sigil),
    };
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
            // Snapped to the step, so values nudged by dragging line up with each other.
            if let Some(step) = look.step {
                commands.insert((NumberInputStep(step), NumberInputSnapToStep));
            }
        })
        .on_spawn_apply_scene(move || {
            bsn! {
                @FeathersNumberInput {
                    @interior_label_text: {interior},
                    @label_text: {label},
                    @sigil_color: {sigil},
                }
                Node { flex_grow: {grow}, width: {width} }
            }
        });
    if !input.will_be_spawned() {
        input.number_input(value);
    }
}

/// A caption with a feathers color input, a swatch that opens a picker, at the row's right end.
/// The input is named `input`.
fn color_row(
    row: ImmEntity<CapsPanel>,
    caption: &'static str,
    color: &mut Color,
    remove: Option<&mut bool>,
) {
    row.on_spawn_insert(move || (Name::new(caption), caption_line()))
        .add(|ui| {
            ui.ch().on_spawn_apply_scene(move || label(caption));
            ui.ch().on_spawn_insert(widget_line).add(|ui| {
                color_input(ui, color);
                remove_button(ui, remove);
            });
        });
}

/// A feathers color input named `input`, showing `color`, and setting it to the color picked.
fn color_input(ui: &mut Imm<CapsPanel>, color: &mut Color) {
    let initial = *color;
    let mut input = ui
        .ch()
        .on_spawn_apply_scene(|| bsn! { #input @FeathersColorInput })
        .on_spawn_insert(move || (ColorInputValue(initial), NewValueChange::<Color>::default()));
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
}

#[cfg(test)]
mod tests {
    use super::shown_f32;

    #[test]
    fn an_f32_is_shown_as_its_decimal() {
        assert_eq!(shown_f32(0.35), 0.35);
        assert_eq!(shown_f32(0.3), 0.3);
        assert_eq!(shown_f32(-1.5e-7), -1.5e-7);
        assert_eq!(shown_f32(2.0), 2.0);
    }
}
