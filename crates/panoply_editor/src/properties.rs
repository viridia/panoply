//! The property editor's rows (D83): a subpane per inspected value, headed by its aspect's icon
//! (`AspectIcon`) and caption, and in it a row per field. A row is a caption in a column of fixed
//! width, then its widget, then a column for its ×, empty if it has none, so widgets and ×s line
//! up down the panel. Booleans get a checkbox, numbers a number input, a `Vec3` three of them on
//! one line, colors a swatch that opens a picker, strings a text input committed on Enter or on
//! leaving it, a model (`Skin`, `SceneryModel`, `ClipsFile`) a button that opens a picker of models
//! of its kind, an exemplar's `extends` a button that opens a chooser of the exemplars it may
//! extend, an enum a select of its variants with the variant's fields below it. Nested structs and
//! list items are a heading over their own rows, set in behind a stripe; a heading, and a pane's
//! header, collapse their rows behind a chevron and then show a summary: a list's or map's length,
//! or the value's `PropertySummary`; adding a row from one expands it. A map with string keys is a row per entry, in key order: a
//! text input for its key, its value's widget beside it, and a ×; a key committed empty or
//! another entry's is put back. An empty list or map says "No items" or "No entries" in place of
//! rows. Any other newtype shows its one field's row. An `Option` is its
//! value's row, with a × that removes the value, or no row while it is `None`: a struct's heading
//! has a menu of its absent fields, which adds one with its type's default. Every other field is a
//! row that shows its value without editing it. Annotations from `bevy::annotations` set a row's
//! caption, a number's range and step, and whether it can be edited; on an `Option`, they apply to
//! its value.
//!
//! A row is named after its field, never its caption, and a list item "#1", "#2", so plans' paths
//! keep working when captions change.
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
            ButtonVariant, ColorInputValue, FeathersButton, FeathersCheckbox, FeathersColorInput,
            FeathersDisclosureToggle, FeathersListRow, FeathersMenu, FeathersMenuItem,
            FeathersMenuPopup,
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
        ParsedPath, ReflectFromReflect, ReflectRef, TypeInfo, TypeRegistry,
    },
    platform::collections::HashSet,
    text::{FontSourceTemplate, LineBreak, TextLayout},
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
        selected, slider_base_color, slider_value,
        text::{self, ImmUiText},
        text_input::{self, ImmUiTextInput},
        track_value_change_plugin::{NewValueChange, TrackValueChangePlugin},
        ImplCapsUiFeathers,
    },
    CapSet, Imm, ImmCapAccessRequests, ImmCapability, ImmEntity,
};
use panoply_core::HexColor;
use panoply_exemplar::annotations::{AspectIcon, ReflectPropertySummary};

use crate::{
    model_picker::{model_field, open_model_picker, ChosenModel, ModelKind},
    scenery_properties::{open_extends_chooser, ExemplarRef},
};
use edit::{EditKind, PathSteps, PropertyEdits, PropertyRoot};

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
        CapabilityModelPicker,
    )
);

/// Lets the panel take the model a model field's picker last chose.
pub struct CapabilityModelPicker;

impl ImmCapability for CapabilityModelPicker {
    fn build<Caps: CapSet>(app: &mut App, cap_req: &mut ImmCapAccessRequests<Caps>) {
        cap_req.request_component_write::<ChosenModel>(app.world_mut());
    }
}

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

/// The width of a row's caption column at the top level. A nested row's is narrower by its
/// indent, so widgets line up at every depth.
const CAPTION_WIDTH: f32 = 88.0;

/// How far each level of nesting sets its rows in: the stripe and the gap after it
/// (`indented_node`).
const INDENT: f32 = 8.0;

/// The width of the column every row ends in, holding its × or nothing.
const REMOVE_WIDTH: f32 = 16.0;

/// The pane icon of an aspect type with no `AspectIcon`.
const DEFAULT_ASPECT_ICON: &str = "editor/icons/lucide/puzzle.png";

/// The panes, headings and items whose rows are collapsed, by key: a pane by its caption, so an
/// aspect stays collapsed from exemplar to exemplar, as Blender's panels do; a heading by its
/// root's key and path. Everything starts expanded.
#[derive(Resource, Default)]
pub struct CollapsedRows(HashSet<String>);

/// A subpane captioned with `root`'s caption beside its type's icon, holding the rows of `value`,
/// `root`'s value; a struct's absent fields are in a menu at the caption's right. Its chevron
/// collapses the rows, which `collapsed` remembers; collapsed, the header shows a summary. With
/// `remove`, a × ends the header, which sets it. The edits its widgets ask for go to `edits`; new
/// values are built from the types in `registry`.
pub fn root_pane(
    ui: &mut Imm<CapsPanel>,
    root: &Arc<dyn PropertyRoot>,
    value: &dyn PartialReflect,
    registry: &TypeRegistry,
    edits: &mut PropertyEdits,
    collapsed: &mut CollapsedRows,
    remove: Option<&mut bool>,
) {
    let caption = root.caption();
    let image = aspect_icon(value);
    let key = format!("pane {caption}");
    let mut rows = Rows {
        root,
        registry,
        edits,
        collapsed,
        depth: 0,
        added: false,
    };
    let name = caption.clone();
    let mut expanded = true;
    ui.ch_id(root.key())
        .on_spawn_apply_scene(subpane)
        .on_spawn_insert(move || Name::new(name.clone()))
        .add(|ui| {
            // The header's right end padded as the body is, so its × lines up with the rows'.
            let header = || {
                bsn! {
                    @subpane_header()
                    Node { padding: UiRect { left: px(4), right: px(6), top: px(0), bottom: px(0) } }
                    Name("header")
                }
            };
            ui.ch().on_spawn_apply_scene(header).add(|ui| {
                ui.ch().on_spawn_insert(heading_start).add(|ui| {
                    expanded = disclosure(ui, rows.collapsed, key.clone());
                    ui.ch()
                        .on_spawn_apply_scene(move || bsn! { @icon(image) Node { height: px(14) } });
                    ui.ch()
                        .on_spawn_apply_scene(move || header_caption(caption.clone()));
                    rows.summary(ui, value, expanded);
                });
                ui.ch().on_spawn_insert(widget_line).add(|ui| {
                    rows.add_menu(ui, value, &ParsedPath::empty(), None);
                    let (inner, path) = through_newtypes(value, ParsedPath::empty());
                    rows.add_button(ui, inner, &path);
                    remove_button(ui, remove);
                });
            });
            rows.expand_if_added(&key);
            if !expanded {
                return;
            }
            ui.ch().on_spawn_apply_scene(subpane_body).add(|ui| {
                if let Some(note) = no_rows_note(value) {
                    ui.ch_id(note).on_spawn_apply_scene(move || placeholder(note));
                }
                rows.root(ui, value);
            });
        });
}

/// A pane's caption, in the header's font: the header's own font reaches only its direct
/// children, and the caption sits in a nested node beside its chevron and icon.
fn header_caption(caption: String) -> impl Scene {
    bsn! {
        Text(caption)
        TextFont {
            font: FontSourceTemplate::Families(fonts::SANS),
            font_size: size::MEDIUM_FONT,
            weight: FontWeight::NORMAL,
        }
        PropagateOver<TextFont>
        TextLayout { linebreak: LineBreak::NoWrap }
        ThemeTextColor(HEADER_TEXT)
    }
}

/// The icon `value`'s type names in its `AspectIcon`, or the default.
fn aspect_icon(value: &dyn PartialReflect) -> &'static str {
    let attributes = match value.get_represented_type_info() {
        Some(TypeInfo::Struct(info)) => info.custom_attributes(),
        Some(TypeInfo::TupleStruct(info)) => info.custom_attributes(),
        Some(TypeInfo::Enum(info)) => info.custom_attributes(),
        _ => return DEFAULT_ASPECT_ICON,
    };
    attributes
        .get::<AspectIcon>()
        .map_or(DEFAULT_ASPECT_ICON, |icon| icon.0)
}

/// A chevron, named `toggle`, that collapses the rows under the header or heading it starts,
/// remembered in `collapsed` by `key`. Returns whether the rows show.
fn disclosure(ui: &mut Imm<CapsPanel>, collapsed: &mut CollapsedRows, key: String) -> bool {
    let shown = !collapsed.0.contains(&key);
    let mut expanded = shown;
    ui.ch_id("toggle")
        .on_spawn_apply_scene(|| bsn! { @FeathersDisclosureToggle #toggle })
        .checked(&mut expanded);
    if expanded != shown {
        if expanded {
            collapsed.0.remove(&key);
        } else {
            collapsed.0.insert(key);
        }
    }
    expanded
}

/// What a pane says in place of rows when `value` has none to show: a struct with no fields, or
/// one whose fields are all absent, which is in effect at their defaults (an `Armature` whose
/// skeleton and clips are those of the actor's own file).
fn no_rows_note(value: &dyn PartialReflect) -> Option<&'static str> {
    let ReflectRef::Struct(fields) = value.reflect_ref() else {
        return None;
    };
    if fields.field_len() == 0 {
        return Some("No fields");
    }
    let absent = |field: &dyn PartialReflect| {
        is_option(field)
            && matches!(field.reflect_ref(), ReflectRef::Enum(option) if option.variant_name() == "None")
    };
    (0..fields.field_len())
        .filter_map(|index| fields.field_at(index))
        .all(absent)
        .then_some("All defaults")
}

/// `text`, dim and italic, named `placeholder`, standing in for rows.
fn placeholder(text: &'static str) -> impl Scene {
    bsn! {
        Text(text)
        TextFont {
            font: FontSourceTemplate::Families(fonts::SANS),
            font_size: size::MEDIUM_FONT,
            style: FontStyle::Italic,
        }
        PropagateOver<TextFont>
        ThemeTextColor(tokens::TEXT_DIM)
        Name("placeholder")
    }
}

/// The menu of `value`'s absent fields, `value` being `root`'s value, ending in an item named
/// `more.0` and captioned `more.1`, for whatever else the root's pane can add. Returns whether
/// that item was picked. The edits its items ask for go to `edits`.
pub fn root_add_menu(
    ui: &mut Imm<CapsPanel>,
    root: &Arc<dyn PropertyRoot>,
    value: &dyn PartialReflect,
    registry: &TypeRegistry,
    edits: &mut PropertyEdits,
    collapsed: &mut CollapsedRows,
    more: (&'static str, &'static str),
) -> bool {
    Rows {
        root,
        registry,
        edits,
        collapsed,
        depth: 0,
        added: false,
    }
    .add_menu(ui, value, &ParsedPath::empty(), Some(more))
}

/// The rows of `value`, `root`'s value, in whatever holds them: a struct's fields with no heading
/// of their own, or another value's one row. The edits its widgets ask for go to `edits`; new
/// values are built from the types in `registry`; `collapsed` holds the headings collapsed.
pub fn root_rows(
    ui: &mut Imm<CapsPanel>,
    root: &Arc<dyn PropertyRoot>,
    value: &dyn PartialReflect,
    registry: &TypeRegistry,
    edits: &mut PropertyEdits,
    collapsed: &mut CollapsedRows,
) {
    Rows {
        root,
        registry,
        edits,
        collapsed,
        depth: 0,
        added: false,
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
    collapsed: &'a mut CollapsedRows,
    /// How many headings the rows being built are nested under.
    depth: usize,
    /// Whether a header's or heading's add button or menu just asked for an item, entry or field,
    /// which the header or heading then shows ([`Rows::expand_if_added`]).
    added: bool,
}

impl Rows<'_> {
    /// Expands the header or heading collapsed under `key` if its widgets just asked for
    /// something to be added, so the new row shows once the edit is made.
    fn expand_if_added(&mut self, key: &str) {
        if std::mem::take(&mut self.added) {
            self.collapsed.0.remove(key);
        }
    }

    /// The width of the caption column at the current depth.
    fn caption_width(&self) -> f32 {
        caption_width(self.depth)
    }

    /// While the rows under a header are collapsed, a dim summary of `value`, named `summary`: a
    /// list's length, or its type's `PropertySummary`. Nothing while they show, or for a value with
    /// no summary.
    fn summary(&self, ui: &mut Imm<CapsPanel>, value: &dyn PartialReflect, expanded: bool) {
        let Some(text) = (!expanded).then(|| self.summary_text(value)).flatten() else {
            return;
        };
        ui.ch_id("summary")
            .on_spawn_apply_scene(|| {
                bsn! {
                    Text("")
                    TextFont {
                        font: FontSourceTemplate::Families(fonts::SANS),
                        font_size: size::MEDIUM_FONT,
                        weight: FontWeight::NORMAL,
                    }
                    PropagateOver<TextFont>
                    TextLayout { linebreak: LineBreak::NoWrap }
                    ThemeTextColor(tokens::TEXT_DIM)
                    Name("summary")
                    Node { min_width: px(0), overflow: Overflow::clip_x() }
                }
            })
            .on_hash_change_text_fn(&text, || text.clone());
    }

    /// The words that stand for `value` while its rows are collapsed, if any.
    fn summary_text(&self, value: &dyn PartialReflect) -> Option<String> {
        let (inner, _) = through_newtypes(value, ParsedPath::empty());
        match inner.reflect_ref() {
            ReflectRef::List(list) => {
                return Some(match list.len() {
                    1 => "1 item".to_string(),
                    count => format!("{count} items"),
                })
            }
            ReflectRef::Map(map) => {
                return Some(match map.len() {
                    1 => "1 entry".to_string(),
                    count => format!("{count} entries"),
                })
            }
            _ => {}
        }
        let reflect = value.try_as_reflect()?;
        let summary = self
            .registry
            .get_type_data::<ReflectPropertySummary>(reflect.reflect_type_info().type_id())?;
        Some(summary.get(reflect)?.property_summary())
    }

    /// A heading over rows, named `name`, keyed by `path`: on a line named `heading`, a chevron that
    /// collapses them, `start` (the heading's caption, or a map entry's key), and its summary
    /// while collapsed, then `widgets` and the × column at its right; below it, while expanded,
    /// `body`'s rows, set in behind a stripe, one level deeper.
    #[expect(clippy::too_many_arguments, reason = "a heading's parts, each used once")]
    fn heading(
        &mut self,
        row: ImmEntity<CapsPanel>,
        name: Name,
        value: &dyn PartialReflect,
        path: &ParsedPath,
        remove: Option<&mut bool>,
        start: impl FnOnce(&mut Self, &mut Imm<CapsPanel>),
        widgets: impl FnOnce(&mut Self, &mut Imm<CapsPanel>),
        body: impl FnOnce(&mut Self, &mut Imm<CapsPanel>),
    ) {
        let key = format!("{} {path}", self.root.key());
        row.on_spawn_insert(move || (name, heading_node()))
            .add(|ui| {
                let mut expanded = true;
                let line = || (Name::new("heading"), caption_line());
                ui.ch().on_spawn_insert(line).add(|ui| {
                    ui.ch().on_spawn_insert(heading_start).add(|ui| {
                        expanded = disclosure(ui, self.collapsed, key.clone());
                        start(self, ui);
                        self.summary(ui, value, expanded);
                    });
                    ui.ch().on_spawn_insert(widget_line).add(|ui| {
                        widgets(self, ui);
                        remove_button(ui, remove);
                    });
                    self.expand_if_added(&key);
                });
                if expanded {
                    ui.ch_id("rows").on_spawn_apply_scene(indented_node).add(|ui| {
                        self.depth += 1;
                        body(self, ui);
                        self.depth -= 1;
                    });
                }
            });
    }
    /// Asks for `value` to be written at `path`.
    fn set(&mut self, path: &ParsedPath, value: impl PartialReflect) {
        self.edits.set(self.root, path.clone(), value);
    }

    /// Asks for the `Option` of type `option` at `path` to be set to `Some` holding its value
    /// type's default (`add`), or to `None`.
    fn add_or_remove(&mut self, path: &ParsedPath, option: &TypeInfo, add: bool) {
        let kind = if add { EditKind::Add } else { EditKind::Remove };
        match option_value(self.registry, option, add) {
            Some(value) => self.edits.push(self.root, path.clone(), kind, Some(value)),
            None => error!("Cannot make a {} value", option.type_path()),
        }
    }

    /// The rows of the root's value: a struct's fields, a list's items or a map's entries (through
    /// newtypes), or another value's one row.
    fn root(&mut self, ui: &mut Imm<CapsPanel>, value: &dyn PartialReflect) {
        let (inner, path) = through_newtypes(value, ParsedPath::empty());
        match (value.reflect_ref(), inner.reflect_ref()) {
            (ReflectRef::Struct(_), _) => self.fields(ui, value, &ParsedPath::empty()),
            (_, ReflectRef::List(_)) => self.items(ui, inner, &path),
            _ if string_keyed(inner) => self.entries(ui, inner, &path),
            _ => self.row(
                ui,
                short_type_name(value),
                &CustomAttributes::default(),
                value,
                ParsedPath::empty(),
                None,
            ),
        }
    }

    /// A boxed-plus menu of the absent fields of `value`, a struct at `path`: its `Option`
    /// fields that are `None` and can be edited. Picking one adds it. With `more`, an item named
    /// `more.0` and captioned `more.1` ends the menu; returns whether it was picked. Nothing if
    /// the menu would be empty.
    fn add_menu(
        &mut self,
        ui: &mut Imm<CapsPanel>,
        value: &dyn PartialReflect,
        path: &ParsedPath,
        more: Option<(&'static str, &'static str)>,
    ) -> bool {
        let (Some(TypeInfo::Struct(info)), ReflectRef::Struct(fields)) =
            (value.get_represented_type_info(), value.reflect_ref())
        else {
            return false;
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
        if absent.is_empty() && more.is_none() {
            return false;
        }
        let mut picked = None;
        let mut more_picked = false;
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
                        // The menu tool button is always Normal, boxed; plain, as the list's
                        // boxed plus is.
                        ButtonVariant::Plain
                    }
                });
                ui.ch().on_spawn_apply_scene(right_aligned_popup).add(|ui| {
                    for &(name, caption, option) in &absent {
                        if menu_item(ui.ch_id(name), caption, format!("add_{name}")) {
                            picked = Some((name, option));
                        }
                    }
                    if let Some((name, caption)) = more {
                        more_picked = menu_item(ui.ch_id(name), caption, format!("add_{name}"));
                    }
                });
            });
        if let Some((name, option)) = picked {
            self.add_or_remove(&path.with_field(name), option, true);
            self.added = true;
        }
        more_picked
    }

    /// The rows of the fields of `value`, a struct at `path`.
    fn fields(&mut self, ui: &mut Imm<CapsPanel>, value: &dyn PartialReflect, path: &ParsedPath) {
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
                    path.with_field(name),
                    None,
                );
            }
        }
    }

    /// The row of `value`, at `path`, named `name`, and captioned so unless its attributes `attrs`
    /// name it. With `remove`, the row's × column holds a × that sets it.
    fn row(
        &mut self,
        ui: &mut Imm<CapsPanel>,
        name: &'static str,
        attrs: &CustomAttributes,
        value: &dyn PartialReflect,
        path: ParsedPath,
        remove: Option<&mut bool>,
    ) {
        let head = RowHead {
            name,
            caption: Some(attrs.get::<DisplayName>().map_or(name, |name| name.0)),
            width: self.caption_width(),
        };
        self.row_with(ui, head, attrs, value, path, remove);
    }

    /// The row of `value`, at `path`, named and captioned as `head` says.
    fn row_with(
        &mut self,
        ui: &mut Imm<CapsPanel>,
        head: RowHead,
        attrs: &CustomAttributes,
        value: &dyn PartialReflect,
        path: ParsedPath,
        remove: Option<&mut bool>,
    ) {
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
            let inner_path = path.with_tuple(0);
            if attrs.get::<ReadOnly>().is_some() {
                self.row_with(ui, head, attrs, inner, inner_path, remove);
                return;
            }
            let mut removed = false;
            self.row_with(ui, head, attrs, inner, inner_path, Some(&mut removed));
            if removed {
                self.add_or_remove(&path, info, false);
            }
            return;
        }
        let name = head.name;
        let caption = head.caption.unwrap_or(name);
        // A model, picked from a list of its kind rather than typed.
        if let Some((kind, model)) = model_field(value) {
            if attrs.get::<ReadOnly>().is_none() {
                let row = ui.ch_id(path.clone());
                if let Some(chosen) = model_row(row, head, kind, model, remove) {
                    self.set(&path.with_tuple(0), chosen);
                }
                return;
            }
        }
        // The exemplar another extends, chosen from a list of them.
        if let Some(reference) = value.try_downcast_ref::<ExemplarRef>() {
            if attrs.get::<ReadOnly>().is_none() {
                let row = ui.ch_id(path.clone());
                let mut clear = false;
                let clearable = reference.0.is_some().then_some(&mut clear);
                exemplar_ref_row(row, head, reference, self.root, clearable);
                if clear {
                    self.set(&path, ExemplarRef(None));
                }
                return;
            }
        }
        // A newtype is its field's row.
        if let Some(inner) = newtype_field(value) {
            if value.try_downcast_ref::<HexColor>().is_none() {
                self.row_with(ui, head, attrs, inner, path.with_tuple(0), remove);
                return;
            }
        }
        // Rows are keyed by path, so they keep their entities from frame to frame.
        let row = ui.ch_id(path.clone());
        if attrs.get::<ReadOnly>().is_some() {
            read_only_row(row, head, value, remove);
            return;
        }
        if let Some(&flag) = value.try_downcast_ref::<bool>() {
            let mut edited = flag;
            row.on_spawn_insert(move || head.node()).add(|ui| {
                head.caption_cell(ui);
                ui.ch()
                    .on_spawn_apply_scene(|| bsn! { @FeathersCheckbox #check })
                    .checked(&mut edited);
                ui.ch().on_spawn_insert(spacer);
                remove_button(ui, remove);
            });
            if edited != flag {
                self.set(&path, edited);
            }
        } else if let Some(&number) = value.try_downcast_ref::<f32>() {
            let shown = shown_f32(number);
            if let Some(edited) = self.number_row(row, head, attrs, shown, remove) {
                self.set(&path, edited as f32);
            }
        } else if let Some(&number) = value.try_downcast_ref::<f64>() {
            if let Some(edited) = self.number_row(row, head, attrs, number, remove) {
                self.set(&path, edited);
            }
        } else if let Some(&number) = value.try_downcast_ref::<i32>() {
            if let Some(edited) = self.number_row(row, head, attrs, number, remove) {
                self.set(&path, edited);
            }
        } else if let Some(&number) = value.try_downcast_ref::<i64>() {
            if let Some(edited) = self.number_row(row, head, attrs, number, remove) {
                self.set(&path, edited);
            }
        } else if let Some(&vector) = value.try_downcast_ref::<Vec3>() {
            self.vec3_row(row, head, attrs, vector, &path, remove);
        } else if let Some(&color) = value.try_downcast_ref::<HexColor>() {
            let mut edited = Color::Srgba(color.0);
            color_row(row, head, &mut edited, remove);
            if edited != Color::Srgba(color.0) {
                self.set(&path, HexColor(edited.to_srgba()));
            }
        } else if let Some(text) = value.try_downcast_ref::<String>() {
            if let Some(edited) = text_row(row, head, text, remove) {
                self.set(&path, edited);
            }
        } else if let ReflectRef::Enum(_) = value.reflect_ref() {
            self.enum_row(row, head, value, &path, remove);
        } else if let ReflectRef::List(_) = value.reflect_ref() {
            self.heading(
                row,
                name.into(),
                value,
                &path,
                remove,
                |_, ui| heading_caption(ui, caption),
                |rows, ui| rows.add_button(ui, value, &path),
                |rows, ui| rows.items(ui, value, &path),
            );
        } else if string_keyed(value) {
            self.heading(
                row,
                name.into(),
                value,
                &path,
                remove,
                |_, ui| heading_caption(ui, caption),
                |rows, ui| rows.add_button(ui, value, &path),
                |rows, ui| rows.entries(ui, value, &path),
            );
        } else if let ReflectRef::Struct(_) = value.reflect_ref() {
            self.heading(
                row,
                name.into(),
                value,
                &path,
                remove,
                |_, ui| heading_caption(ui, caption),
                |rows, ui| {
                    rows.add_menu(ui, value, &path, None);
                },
                |rows, ui| rows.fields(ui, value, &path),
            );
        } else {
            read_only_row(row, head, value, remove);
        }
    }

    /// A number input, named `input` so plans can read its value, after the row's caption.
    /// Returns the number edited in it, if it changed.
    fn number_row<T: NumberField>(
        &mut self,
        row: ImmEntity<CapsPanel>,
        head: RowHead,
        attrs: &CustomAttributes,
        value: T,
        remove: Option<&mut bool>,
    ) -> Option<T> {
        let look = NumberLook::from_attributes::<T>(attrs);
        let mut edited = value;
        row.on_spawn_insert(move || head.node()).add(|ui| {
            head.caption_cell(ui);
            number_input(ui.ch(), "input", None, look, &mut edited);
            remove_button(ui, remove);
        });
        (edited != value).then_some(edited)
    }

    /// Three number inputs after the row's caption, labeled X, Y and Z beside their axes' colored
    /// sigils.
    fn vec3_row(
        &mut self,
        row: ImmEntity<CapsPanel>,
        head: RowHead,
        attrs: &CustomAttributes,
        value: Vec3,
        path: &ParsedPath,
        remove: Option<&mut bool>,
    ) {
        let look = NumberLook::from_attributes::<f64>(attrs);
        let shown = value.to_array().map(shown_f32);
        let mut edited = shown;
        row.on_spawn_insert(move || head.node()).add(|ui| {
            head.caption_cell(ui);
            let [x, y, z] = &mut edited;
            let axes = [
                ("x", "X", tokens::TEXT_INPUT_X_AXIS, x),
                ("y", "Y", tokens::TEXT_INPUT_Y_AXIS, y),
                ("z", "Z", tokens::TEXT_INPUT_Z_AXIS, z),
            ];
            for (name, label, sigil, component) in axes {
                number_input(ui.ch(), name, Some((label, sigil)), look.clone(), component);
            }
            remove_button(ui, remove);
        });
        for ((axis, old), new) in ["x", "y", "z"].into_iter().zip(shown).zip(edited) {
            if new != old {
                self.set(&path.with_field(axis), new as f32);
            }
        }
    }

    /// The rows of the items of `value`, a list at `path`, captioned and named "#1" and on, each
    /// with a × that removes it.
    fn items(&mut self, ui: &mut Imm<CapsPanel>, value: &dyn PartialReflect, path: &ParsedPath) {
        let ReflectRef::List(list) = value.reflect_ref() else {
            return;
        };
        if list.is_empty() {
            ui.ch_id("No items").on_spawn_apply_scene(|| placeholder("No items"));
            return;
        }
        let mut removed = None;
        for (index, item) in list.iter().enumerate() {
            let mut remove = false;
            self.row(
                ui,
                item_caption(index),
                &CustomAttributes::default(),
                item,
                path.with_item(index),
                Some(&mut remove),
            );
            if remove {
                removed = Some(index);
            }
        }
        if let Some(index) = removed {
            let kind = EditKind::RemoveItem(index);
            self.edits.push(self.root, path.clone(), kind, None);
        }
    }

    /// A boxed plus for `value`, at `path`: if it is a list, named `add_item`, adding an item at
    /// its end, its type's default; if a map with string keys, named `add_entry`, adding an entry
    /// with an empty key and its value type's default, unless it has one with an empty key
    /// already, whose key is still to be typed. Nothing for any other value.
    fn add_button(&mut self, ui: &mut Imm<CapsPanel>, value: &dyn PartialReflect, path: &ParsedPath) {
        let (name, value_type) = match (value.reflect_ref(), value.get_represented_type_info()) {
            (ReflectRef::List(_), Some(TypeInfo::List(info))) => ("add_item", info.item_ty()),
            (ReflectRef::Map(_), Some(TypeInfo::Map(info))) if string_keyed(value) => {
                ("add_entry", info.value_ty())
            }
            _ => return,
        };
        if !icon_button(ui, name, ADD_ICON, px(18), tokens::TEXT_MAIN) {
            return;
        }
        let Some(new) = self
            .registry
            .get_type_data::<ReflectDefault>(value_type.id())
            .map(|default| default.default().into_partial_reflect())
        else {
            error!("Cannot make a {}", value_type.path());
            return;
        };
        match value.reflect_ref() {
            ReflectRef::List(list) => {
                let kind = EditKind::AddItem(list.len());
                self.edits.push(self.root, path.clone(), kind, Some(new));
            }
            ReflectRef::Map(map) if map.get(&String::new()).is_none() => {
                self.edits.add_entry(self.root, path.clone(), String::new(), new);
            }
            _ => {}
        }
        // Shown even while it adds nothing: the entry whose key is still empty.
        self.added = true;
    }

    /// The rows of the entries of `value`, a map with string keys at `path`, in the order of
    /// their keys, an entry whose key is still empty last, where a new one is added. Each has a ×
    /// that removes it.
    fn entries(&mut self, ui: &mut Imm<CapsPanel>, value: &dyn PartialReflect, path: &ParsedPath) {
        let ReflectRef::Map(map) = value.reflect_ref() else {
            return;
        };
        if map.is_empty() {
            ui.ch_id("No entries").on_spawn_apply_scene(|| placeholder("No entries"));
            return;
        }
        let mut entries: Vec<(&str, &dyn PartialReflect)> = map
            .iter()
            .filter_map(|(key, value)| Some((key.try_downcast_ref::<String>()?.as_str(), value)))
            .collect();
        entries.sort_by_key(|&(key, _)| (key.is_empty(), key));
        let (mut removed, mut renamed) = (None, None);
        for (key, value) in entries {
            let mut remove = false;
            if let Some(new) = self.entry(ui, key, value, path, &mut remove) {
                // A key that is empty, or another entry's, is not committed: the input shows the
                // entry's own again.
                if !new.is_empty() && map.get(&new).is_none() {
                    renamed = Some((key.to_string(), new));
                }
            }
            if remove {
                removed = Some(key.to_string());
            }
        }
        if let Some(key) = removed {
            self.edits.remove_entry(self.root, path.clone(), key);
        }
        if let Some((from, to)) = renamed {
            self.edits.rename_key(self.root, path.clone(), from, to);
        }
    }

    /// The row of the entry `key`, holding `value`, of the map at `path`, named after its key, or
    /// `new_entry` while its key is empty. A text input, named `key`, holds its key, then a row
    /// named `value` its value's widget: in a column of fixed width for a narrow widget, else
    /// sharing the width with the key. A value of several rows is a heading instead, its key in
    /// place of a caption. The entry's × sets `remove`. Returns the key the input commits.
    fn entry(
        &mut self,
        ui: &mut Imm<CapsPanel>,
        key: &str,
        value: &dyn PartialReflect,
        path: &ParsedPath,
        remove: &mut bool,
    ) -> Option<String> {
        let path = path.with_key(key);
        let name = Name::new(match key.is_empty() {
            true => "new_entry".to_string(),
            false => key.to_string(),
        });
        let row = ui.ch_id(path.clone());
        let mut committed = None;
        let layout = entry_layout(value);
        if layout == EntryLayout::Rows {
            let (inner, inner_path) = through_newtypes(value, path.clone());
            self.heading(
                row,
                name,
                value,
                &path,
                Some(remove),
                |_, ui| committed = text_field(ui, "key", key),
                |rows, ui| {
                    rows.add_menu(ui, inner, &inner_path, None);
                    rows.add_button(ui, inner, &inner_path);
                },
                |rows, ui| match inner.reflect_ref() {
                    ReflectRef::List(_) => rows.items(ui, inner, &inner_path),
                    ReflectRef::Struct(_) => rows.fields(ui, inner, &inner_path),
                    _ if string_keyed(inner) => rows.entries(ui, inner, &inner_path),
                    _ => {
                        let head = RowHead {
                            name: "value",
                            caption: Some("Value"),
                            width: rows.caption_width(),
                        };
                        let attrs = CustomAttributes::default();
                        rows.row_with(ui, head, &attrs, inner, inner_path.clone(), None);
                    }
                },
            );
            return committed;
        }
        row.on_spawn_insert(move || (name, widget_line())).add(|ui| {
            committed = text_field(ui, "key", key);
            ui.ch().on_spawn_insert(move || layout.value_node()).add(|ui| {
                let head = RowHead {
                    name: "value",
                    caption: None,
                    width: 0.0,
                };
                let attrs = CustomAttributes::default();
                self.row_with(ui, head, &attrs, value, path, Some(remove));
            });
        });
        committed
    }

    /// The row's caption with a select of the variants of `value`, an enum at `path`; below it,
    /// the fields of the variant chosen, as its siblings, not set in: they are the enum's value,
    /// not a part of it. Picking another variant asks for it, carrying over the fields of the same
    /// name and type.
    fn enum_row(
        &mut self,
        row: ImmEntity<CapsPanel>,
        head: RowHead,
        value: &dyn PartialReflect,
        path: &ParsedPath,
        remove: Option<&mut bool>,
    ) {
        let (ReflectRef::Enum(current), Some(TypeInfo::Enum(info))) =
            (value.reflect_ref(), value.get_represented_type_info())
        else {
            return;
        };
        let index = current.variant_index();
        let mut picked = None;
        row.on_spawn_insert(move || (Name::new(head.name), heading_node()))
            .add(|ui| {
                ui.ch().on_spawn_insert(widget_line).add(|ui| {
                    head.caption_cell(ui);
                    picked = variant_select(ui, info.variant_names(), index);
                    remove_button(ui, remove);
                });
                if current.field_len() > 0 {
                    // Keyed by variant, so another variant's fields get rows of their own.
                    ui.ch_id(current.variant_name())
                        .on_spawn_insert(column)
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
                    .push(self.root, path.clone(), EditKind::Set, Some(value))
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
        path: &ParsedPath,
    ) {
        let Some(variant) = info.variant(value.variant_name()) else {
            return;
        };
        for index in 0..value.field_len() {
            let Some(field) = value.field_at(index) else {
                continue;
            };
            let (name, attrs, field_path) = match variant {
                VariantInfo::Struct(variant) => {
                    let Some(field_info) = variant.field_at(index) else {
                        continue;
                    };
                    let name = field_info.name();
                    (name, field_info.custom_attributes(), path.with_field(name))
                }
                VariantInfo::Tuple(variant) => {
                    let Some(field_info) = variant.field_at(index) else {
                        continue;
                    };
                    let name = short_type_name(field);
                    (name, field_info.custom_attributes(), path.with_tuple(index))
                }
                VariantInfo::Unit(_) => return,
            };
            self.row(ui, name, attrs, field, field_path, None);
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

/// "#1", "#2" and on: a list item's caption and name, for its `index`. Each is made once and
/// kept, since rows take their captions for good.
fn item_caption(index: usize) -> &'static str {
    static CAPTIONS: std::sync::Mutex<Vec<&'static str>> = std::sync::Mutex::new(Vec::new());
    let mut captions = CAPTIONS.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    while captions.len() <= index {
        let caption = format!("#{}", captions.len() + 1);
        captions.push(Box::leak(caption.into_boxed_str()));
    }
    captions[index]
}

/// `value` and its `path` within the root, or, while it is a newtype other than a color, its one
/// field and that field's path.
fn through_newtypes(value: &dyn PartialReflect, path: ParsedPath) -> (&dyn PartialReflect, ParsedPath) {
    match newtype_field(value) {
        Some(inner) if value.try_downcast_ref::<HexColor>().is_none() => {
            through_newtypes(inner, path.with_tuple(0))
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
            bsn! { @FeathersSelect { @options: {rows} } #select Node { flex_grow: 1.0, min_width: px(0) } }
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

/// A caption with a text input, named `input`, showing `value` (`text_field`). Returns the text
/// the input commits. With `remove`, a × ends the row.
fn text_row(
    row: ImmEntity<CapsPanel>,
    head: RowHead,
    value: &str,
    remove: Option<&mut bool>,
) -> Option<String> {
    let mut committed = None;
    row.on_spawn_insert(move || head.node())
        .add(|ui| {
            head.caption_cell(ui);
            committed = text_field(ui, "input", value);
            remove_button(ui, remove);
        });
    committed
}

/// A text input named `name`, showing `value`, sharing its line's spare width with any other
/// input there. Returns the text the input commits, on Enter or on losing focus
/// (`text_commit`), if it differs from `value`. The input shows a value changed elsewhere, by
/// undo, only while it is not being edited, and `value` again after a commit the panel did not
/// make.
fn text_field(ui: &mut Imm<CapsPanel>, name: &'static str, value: &str) -> Option<String> {
    let mut committed = None;
    ui.ch()
        .on_spawn_apply_scene(|| {
            bsn! {
                @FeathersTextInputContainer
                Node { flex_grow: 1.0, flex_basis: px(0), min_width: px(0) }
            }
        })
        .add(|ui| {
            let mut input = ui
                .ch()
                .on_spawn_apply_scene(move || bsn! { @FeathersTextInput Name({name}) })
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
    committed.filter(|text| text != value)
}

/// Whether `value` is a map whose keys are strings: the maps the panel edits.
fn string_keyed(value: &dyn PartialReflect) -> bool {
    matches!(value.get_represented_type_info(), Some(TypeInfo::Map(info)) if info.key_ty().is::<String>())
}

/// The width of a map entry's value column, for a value whose widget is narrow.
const VALUE_WIDTH: f32 = 72.0;

/// How a map entry lays out its value beside its key.
#[derive(Clone, Copy, PartialEq, Eq)]
enum EntryLayout {
    /// A narrow widget (a checkbox, a number, a swatch), in a column of fixed width.
    Narrow,
    /// A wide widget (a text, a vector, a select, a picker's button), sharing the line evenly
    /// with the key.
    Wide,
    /// Rows: a list, a struct, a map or an enum with fields, under a heading holding the key.
    Rows,
}

impl EntryLayout {
    /// The node holding the value's row, ending in the entry's × column.
    fn value_node(self) -> Node {
        let width = match self {
            EntryLayout::Narrow => Node {
                width: px(VALUE_WIDTH + 4.0 + REMOVE_WIDTH),
                flex_shrink: 0.0,
                ..default()
            },
            EntryLayout::Wide | EntryLayout::Rows => Node {
                flex_grow: 1.0,
                flex_basis: px(0),
                min_width: px(0),
                ..default()
            },
        };
        Node {
            flex_direction: FlexDirection::Column,
            ..width
        }
    }
}

/// How a map entry lays out `value`, from its type alone, so an entry keeps its layout as its
/// value changes.
fn entry_layout(value: &dyn PartialReflect) -> EntryLayout {
    if model_field(value).is_some() || value.try_downcast_ref::<ExemplarRef>().is_some() {
        return EntryLayout::Wide;
    }
    let (value, _) = through_newtypes(value, ParsedPath::empty());
    let narrow = value.try_downcast_ref::<bool>().is_some()
        || value.try_downcast_ref::<f32>().is_some()
        || value.try_downcast_ref::<f64>().is_some()
        || value.try_downcast_ref::<i32>().is_some()
        || value.try_downcast_ref::<i64>().is_some()
        || value.try_downcast_ref::<HexColor>().is_some();
    if narrow {
        return EntryLayout::Narrow;
    }
    if value.try_downcast_ref::<Vec3>().is_some() {
        return EntryLayout::Wide;
    }
    match value.get_represented_type_info() {
        Some(TypeInfo::List(_) | TypeInfo::Struct(_) | TypeInfo::Map(_)) => EntryLayout::Rows,
        Some(TypeInfo::Enum(info))
            if info
                .iter()
                .any(|variant| !matches!(variant, VariantInfo::Unit(_))) =>
        {
            EntryLayout::Rows
        }
        _ => EntryLayout::Wide,
    }
}

/// A caption with a button, named `pick`, whose text, named `model`, is the model `value` names
/// without its folders. The button opens a picker of `kind`'s models (`model_picker`). Returns
/// the model chosen in it. With `remove`, a × ends the row.
fn model_row(
    row: ImmEntity<CapsPanel>,
    head: RowHead,
    kind: ModelKind,
    value: &str,
    remove: Option<&mut bool>,
) -> Option<String> {
    let mut chosen = None;
    let shown = value.rsplit_once('/').map_or(value, |(_, file)| file).to_string();
    row.on_spawn_insert(move || head.node())
        .add(|ui| {
            head.caption_cell(ui);
            let mut button = ui.ch_id("pick").on_spawn_apply_scene(|| {
                bsn! {
                    @FeathersButton
                    #pick
                    ChosenModel
                    Node {
                        flex_grow: 1.0,
                        min_width: px(0),
                        justify_content: JustifyContent::Start,
                        overflow: Overflow::clip_x(),
                    }
                }
            });
            if let Ok(Some(mut model)) = button.cap_get_component_mut::<ChosenModel>() {
                chosen = model.0.take();
            }
            if button.activated() {
                let (entity, current) = (button.entity(), value.to_string());
                button
                    .commands()
                    .run_system_cached_with(open_model_picker, (entity, kind, current));
            }
            button.add(|ui| {
                ui.ch()
                    .on_spawn_apply_scene(|| button_text("model"))
                    .on_hash_change_text_fn(&shown, || shown.clone());
            });
            remove_button(ui, remove);
        });
    chosen.filter(|model| model != value)
}

/// A caption with a button, named `pick`, whose text, named `name`, is the label of the exemplar
/// `value` names, or "none". The button opens a chooser of the exemplars `root` may extend
/// (`open_extends_chooser`), which sets the field itself. With `remove`, a × ends the row.
fn exemplar_ref_row(
    row: ImmEntity<CapsPanel>,
    head: RowHead,
    value: &ExemplarRef,
    root: &Arc<dyn PropertyRoot>,
    remove: Option<&mut bool>,
) {
    let shown = value
        .0
        .as_deref()
        .map_or("none", |path| path.rsplit_once('#').map_or(path, |(_, label)| label))
        .to_string();
    row.on_spawn_insert(move || head.node())
        .add(|ui| {
            head.caption_cell(ui);
            let mut button = ui.ch_id("pick").on_spawn_apply_scene(|| {
                bsn! {
                    @FeathersButton
                    #pick
                    Node {
                        flex_grow: 1.0,
                        min_width: px(0),
                        justify_content: JustifyContent::Start,
                        overflow: Overflow::clip_x(),
                    }
                }
            });
            if button.activated() {
                let root = root.clone();
                button
                    .commands()
                    .run_system_cached_with(open_extends_chooser, root);
            }
            button.add(|ui| {
                ui.ch()
                    .on_spawn_apply_scene(|| button_text("name"))
                    .on_hash_change_text_fn(&shown, || shown.clone());
            });
            remove_button(ui, remove);
        });
}

/// The text of a picker's button, named `name`: regular, since it is a value rather than an
/// action, and on one line, since a model path has hyphens to break at and a second line would
/// spill out of the button, which clips only its sides.
fn button_text(name: &'static str) -> impl Scene {
    bsn! {
        Text("")
        ThemedText
        TextFont {
            font: FontSourceTemplate::Handle(fonts::REGULAR),
            font_size: size::MEDIUM_FONT,
            weight: FontWeight::NORMAL,
        }
        PropagateOver<TextFont>
        TextLayout { linebreak: LineBreak::NoWrap }
        Name(name)
    }
}

/// The × column that ends a row or heading: with `remove`, a dim ×, named `remove`, which sets it
/// when clicked; without, the column empty, so the widgets before it end where every other row's
/// do.
fn remove_button(ui: &mut Imm<CapsPanel>, remove: Option<&mut bool>) {
    let Some(remove) = remove else {
        ui.ch_id("no_remove").on_spawn_insert(|| Node {
            width: px(REMOVE_WIDTH),
            flex_shrink: 0.0,
            ..default()
        });
        return;
    };
    let mut button = ui.ch_id("remove").on_spawn_apply_scene(|| {
        bsn! {
            @FeathersToolButton { @variant: {ButtonVariant::Plain} }
            Name("remove")
            Node {
                padding: UiRect::ZERO,
                min_width: px(REMOVE_WIDTH),
                width: px(REMOVE_WIDTH),
                flex_shrink: 0.0,
            }
        }
    });
    *remove |= button.activated();
    button.add(|ui| {
        ui.ch_id(icons::X).on_spawn_apply_scene(|| {
            bsn! { @icon(icons::X) Node { height: px(12) } TextColor ThemeTextColor(tokens::TEXT_DIM) }
        });
    });
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
    head: RowHead,
    value: &dyn PartialReflect,
    remove: Option<&mut bool>,
) {
    let text = summary(value);
    row.on_spawn_insert(move || head.node()).add(|ui| {
        head.caption_cell(ui);
        ui.ch_id(text.clone())
            .on_spawn_apply_scene(move || bsn! { @label(text.clone()) Node { flex_grow: 1.0 } });
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

/// What a row is named and shows at its left: its field's name, its caption, and the width of
/// the caption column at its depth. A map entry's value has no caption: its key is beside it.
#[derive(Clone, Copy)]
struct RowHead {
    name: &'static str,
    caption: Option<&'static str>,
    width: f32,
}

impl RowHead {
    /// The row's node, named after its field, laying out caption, widget and × column in a line.
    fn node(self) -> (Name, Node) {
        (Name::new(self.name), widget_line())
    }

    /// The caption in its column, clipped rather than wrapped when it is too long; nothing for a
    /// row without one.
    fn caption_cell(self, ui: &mut Imm<CapsPanel>) {
        let (Some(caption), width) = (self.caption, self.width) else {
            return;
        };
        ui.ch().on_spawn_apply_scene(move || {
            bsn! {
                Node {
                    width: px(width),
                    flex_shrink: 0.0,
                    overflow: Overflow::clip_x(),
                }
                Children [
                    @label(caption)
                    TextLayout { linebreak: LineBreak::NoWrap }
                ]
            }
        });
    }
}

/// The caption column's width `depth` headings deep: narrower by each level's indent, so the
/// widgets after it line up at every depth.
fn caption_width(depth: usize) -> f32 {
    (CAPTION_WIDTH - depth as f32 * INDENT).max(INDENT)
}

/// Takes a line's spare width, so the widgets after it sit at its right end.
fn spacer() -> Node {
    Node {
        flex_grow: 1.0,
        ..default()
    }
}

/// Rows stacked, spaced as a pane's rows are.
fn column() -> Node {
    Node {
        flex_direction: FlexDirection::Column,
        row_gap: px(4),
        ..default()
    }
}

/// The left of a header or heading: its chevron, caption and summary, taking the spare width and
/// clipping a long summary.
fn heading_start() -> Node {
    Node {
        flex_grow: 1.0,
        min_width: px(0),
        overflow: Overflow::clip_x(),
        ..widget_line()
    }
}

/// A heading's caption, in the main text color.
fn heading_caption(ui: &mut Imm<CapsPanel>, caption: &'static str) {
    ui.ch()
        .on_spawn_apply_scene(move || heading_text(caption, tokens::TEXT_MAIN));
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

/// A number input named `name`, taking its line's spare width. With `axis`, it is labeled at its
/// left beside a sigil strip in the token's color, as X, Y and Z are.
fn number_input<T: NumberField>(
    input: ImmEntity<CapsPanel>,
    name: &'static str,
    axis: Option<(&'static str, ThemeToken)>,
    look: NumberLook,
    value: &mut T,
) {
    let (label, sigil) = match axis {
        Some((text, sigil)) => (Some(text), sigil),
        None => (None, tokens::TEXT_INPUT_LABEL_BG),
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
                    @label_text: {label},
                    @sigil_color: {sigil},
                }
                Node { flex_grow: 1.0, min_width: px(0) }
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
    head: RowHead,
    color: &mut Color,
    remove: Option<&mut bool>,
) {
    row.on_spawn_insert(move || head.node()).add(|ui| {
        head.caption_cell(ui);
        color_input(ui, color);
        ui.ch().on_spawn_insert(spacer);
        remove_button(ui, remove);
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
    use super::{caption_width, shown_f32, CAPTION_WIDTH, INDENT};

    #[test]
    fn a_nested_caption_column_ends_where_the_top_level_one_does() {
        for depth in 0..4 {
            assert_eq!(caption_width(depth) + depth as f32 * INDENT, CAPTION_WIDTH);
        }
    }

    #[test]
    fn an_f32_is_shown_as_its_decimal() {
        assert_eq!(shown_f32(0.35), 0.35);
        assert_eq!(shown_f32(0.3), 0.3);
        assert_eq!(shown_f32(-1.5e-7), -1.5e-7);
        assert_eq!(shown_f32(2.0), 2.0);
    }
}
