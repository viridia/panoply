use std::fmt;

use super::{selectors_map::SelectorsMap, vars_map::VarsMap, ComputedStyle, StyleAttr, StyleExpr};
use bevy::{
    asset::AssetPath,
    prelude::{Asset, AssetServer},
    reflect::TypePath,
};
use serde::{de::Visitor, ser::SerializeStruct, Deserialize, Serialize};

/// A collection of style attributes which can be merged to create a `ComputedStyle`.
#[derive(Debug, Default, Clone, TypePath, PartialEq, Asset)]
#[type_path = "panoply::guise::StyleAsset"]
pub struct StyleAsset {
    /// List of style attributes.
    /// Rather than storing the attributes in a struct full of optional fields, we store a flat
    /// vector of enums, each of which stores a single style attribute. This "sparse" representation
    /// allows for fast merging of styles, particularly for styles which have few or no attributes.
    attrs: Vec<StyleAttr>,

    /// List of style variables to define when this style is invoked.
    vars: VarsMap,

    /// List of conditional styles
    selectors: SelectorsMap,
}

impl StyleAsset {
    pub fn new() -> Self {
        Self {
            attrs: Vec::new(),
            vars: VarsMap::new(),
            selectors: SelectorsMap::new(),
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            attrs: Vec::with_capacity(capacity),
            vars: VarsMap::new(),
            selectors: SelectorsMap::new(),
        }
    }

    /// Construct a new `StyleMap` from a list of `StyleAttr`s.
    pub fn from_attrs(attrs: &[StyleAttr]) -> Self {
        Self {
            attrs: Vec::from(attrs),
            vars: VarsMap::new(),
            selectors: SelectorsMap::new(),
        }
    }

    /// Number of style attributes in the map.
    pub fn len_attrs(&self) -> usize {
        self.attrs.len()
    }

    /// Merge the style properties into a computed `Style` object.
    pub fn apply_to(&self, computed: &mut ComputedStyle, server: &AssetServer) {
        for attr in self.attrs.iter() {
            attr.apply(computed, server);
        }
    }

    /// Find any relative asset paths and convert them to fully-resolved paths.
    pub fn resolve_asset_paths(&mut self, base: &AssetPath) {
        self.attrs.iter_mut().for_each(|attr| match attr {
            StyleAttr::BackgroundImage(StyleExpr::Asset(expr)) => expr.resolve_asset_path(base),
            _ => {}
        });
        self.selectors
            .0
            .iter_mut()
            .for_each(|(_sel, style)| style.resolve_asset_paths(base))
    }
}

impl Serialize for StyleAsset {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let has_vars: usize = if self.vars.len() > 0 { 1 } else { 0 };
        let has_selectors: usize = if self.selectors.len() > 0 { 1 } else { 0 };
        let mut st =
            serializer.serialize_struct("StyleMap", self.attrs.len() + has_vars + has_selectors)?;
        for attr in self.attrs.iter() {
            match attr {
                StyleAttr::BackgroundImage(val) => st.serialize_field("background-image", val)?,
                StyleAttr::BackgroundColor(val) => st.serialize_field("background-color", val)?,
                StyleAttr::BorderColor(val) => st.serialize_field("border-color", val)?,
                StyleAttr::Color(val) => st.serialize_field("color", val)?,
                StyleAttr::ZIndex(val) => st.serialize_field("z-index", val)?,

                StyleAttr::Display(val) => st.serialize_field("display", val)?,
                StyleAttr::Position(val) => st.serialize_field("position", val)?,
                StyleAttr::Overflow(val) => st.serialize_field("overflow", val)?,
                StyleAttr::OverflowX(val) => st.serialize_field("overflow-x", val)?,
                StyleAttr::OverflowY(val) => st.serialize_field("overflow-y", val)?,
                StyleAttr::Direction(val) => st.serialize_field("direction", val)?,

                StyleAttr::Left(val) => st.serialize_field("left", val)?,
                StyleAttr::Right(val) => st.serialize_field("right", val)?,
                StyleAttr::Top(val) => st.serialize_field("top", val)?,
                StyleAttr::Bottom(val) => st.serialize_field("bottom", val)?,

                StyleAttr::Width(val) => st.serialize_field("width", val)?,
                StyleAttr::Height(val) => st.serialize_field("height", val)?,
                StyleAttr::MinWidth(val) => st.serialize_field("min-width", val)?,
                StyleAttr::MinHeight(val) => st.serialize_field("min-height", val)?,
                StyleAttr::MaxWidth(val) => st.serialize_field("max-width", val)?,
                StyleAttr::MaxHeight(val) => st.serialize_field("max-height", val)?,

                StyleAttr::AlignItems(val) => st.serialize_field("align-items", val)?,
                StyleAttr::AlignContent(val) => st.serialize_field("align-content", val)?,
                StyleAttr::AlignSelf(val) => st.serialize_field("align-self", val)?,
                StyleAttr::JustifyItems(val) => st.serialize_field("justify-items", val)?,
                StyleAttr::JustifyContent(val) => st.serialize_field("justify-content", val)?,
                StyleAttr::JustifySelf(val) => st.serialize_field("justify-self", val)?,

                StyleAttr::Margin(val) => st.serialize_field("margin", val)?,
                StyleAttr::MarginLeft(val) => st.serialize_field("margin-left", val)?,
                StyleAttr::MarginRight(val) => st.serialize_field("margin-right", val)?,
                StyleAttr::MarginTop(val) => st.serialize_field("margin-top", val)?,
                StyleAttr::MarginBottom(val) => st.serialize_field("margin-bottom", val)?,

                StyleAttr::Padding(val) => st.serialize_field("padding", val)?,
                StyleAttr::PaddingLeft(val) => st.serialize_field("padding-left", val)?,
                StyleAttr::PaddingRight(val) => st.serialize_field("padding-right", val)?,
                StyleAttr::PaddingTop(val) => st.serialize_field("padding-top", val)?,
                StyleAttr::PaddingBottom(val) => st.serialize_field("padding-bottom", val)?,

                StyleAttr::Border(val) => st.serialize_field("border", val)?,
                StyleAttr::BorderLeft(val) => st.serialize_field("border-left", val)?,
                StyleAttr::BorderRight(val) => st.serialize_field("border-right", val)?,
                StyleAttr::BorderTop(val) => st.serialize_field("border-top", val)?,
                StyleAttr::BorderBottom(val) => st.serialize_field("border-bottom", val)?,

                StyleAttr::Flex(val) => st.serialize_field("flex", val)?,
                StyleAttr::FlexGrow(val) => st.serialize_field("flex-grow", val)?,
                StyleAttr::FlexShrink(val) => st.serialize_field("flex-shrink", val)?,
                StyleAttr::FlexBasis(val) => st.serialize_field("flex-basis", val)?,
                StyleAttr::FlexWrap(val) => st.serialize_field("flex-wrap", val)?,
                StyleAttr::FlexDirection(val) => st.serialize_field("flex-direction", val)?,

                StyleAttr::RowGap(val) => st.serialize_field("row-gap", val)?,
                StyleAttr::ColumnGap(val) => st.serialize_field("column-gap", val)?,
                StyleAttr::Gap(val) => st.serialize_field("gap", val)?,
                _ => todo!("Implement serialization for {:?}", attr),
            };
        }

        if self.vars.len() > 0 {
            st.serialize_field("vars", &self.vars)?;
        }

        if self.selectors.len() > 0 {
            st.serialize_field("selectors", &self.selectors)?;
        }

        st.end()
    }
}

const FIELDS: &'static [&'static str] = &[
    // Colors
    "background-image",
    "background-color",
    "border-color",
    "color",
    // Positioning
    "z-index",
    "display",
    "position",
    "overflow",
    "overflow-x",
    "overflow-y",
    "direction",
    // Rect
    "left",
    "right",
    "top",
    "bottom",
    // Size
    "width",
    "height",
    "min-width",
    "min-height",
    "max-width",
    "max-height",
    // Flex-alignment
    "align-items",
    "align-self",
    "align-content",
    "justify-items",
    "justify-self",
    "justify-content",
    // Margins
    "margin",
    "margin-left",
    "margin-right",
    "margin-top",
    "margin-bottom",
    // Padding
    "padding",
    "padding-left",
    "padding-right",
    "padding-top",
    "padding-bottom",
    // Border
    "border",
    "border-left",
    "border-right",
    "border-top",
    "border-bottom",
    // Flex
    "flex",
    "flex-grow",
    "flex-shrink",
    "flex-basis",
    // Gap
    "row-gap",
    "column-gap",
    "gap",
    // Grid
    // GridAutoFlow(bevy::ui::GridAutoFlow),
    // pub grid_template_rows: Option<Vec<RepeatedGridTrack>>,
    // pub grid_template_columns: Option<Vec<RepeatedGridTrack>>,
    // pub grid_auto_rows: Option<Vec<GridTrack>>,
    // pub grid_auto_columns: Option<Vec<GridTrack>>,
    // GridRow(bevy::ui::GridPlacement),
    "grid-row-start",
    "grid-row-span",
    // GridRowEnd(i16),
    // GridColumn(bevy::ui::GridPlacement),
    // GridColumnStart(i16),
    // GridColumnSpan(u16),
    // GridColumnEnd(i16),

    // Other
    "vars",
    "selectors",
];

impl<'de, 'a> Deserialize<'de> for StyleAsset {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(field_identifier, rename_all = "kebab-case")]
        enum Field {
            BackgroundImage,
            BackgroundColor,
            BorderColor,
            Color,

            ZIndex,
            Display,
            Position,
            Overflow,
            OverflowX,
            OverflowY,
            Direction,
            Left,
            Right,
            Top,
            Bottom,

            Width,
            Height,
            MinWidth,
            MinHeight,
            MaxWidth,
            MaxHeight,

            // pub aspect_ratio: StyleProp<f32>,
            AlignItems,
            JustifyItems,
            AlignSelf,
            JustifySelf,
            AlignContent,
            JustifyContent,

            Margin,
            MarginLeft,
            MarginRight,
            MarginTop,
            MarginBottom,

            Padding,
            PaddingLeft,
            PaddingRight,
            PaddingTop,
            PaddingBottom,

            Border,
            BorderLeft,
            BorderRight,
            BorderTop,
            BorderBottom,

            Flex,
            FlexDirection,
            FlexWrap,
            FlexGrow,
            FlexShrink,
            FlexBasis,

            RowGap,
            ColumnGap,
            Gap,

            // // TODO:
            // GridAutoFlow(bevy::ui::GridAutoFlow),
            // // pub grid_template_rows: Option<Vec<RepeatedGridTrack>>,
            // // pub grid_template_columns: Option<Vec<RepeatedGridTrack>>,
            // // pub grid_auto_rows: Option<Vec<GridTrack>>,
            // // pub grid_auto_columns: Option<Vec<GridTrack>>,
            // GridRow(bevy::ui::GridPlacement),
            GridRowStart,
            GridRowSpan,
            // GridRowEnd(i16),
            // GridColumn(bevy::ui::GridPlacement),
            // GridColumnStart(i16),
            // GridColumnSpan(u16),
            // GridColumnEnd(i16),

            // LineBreak(BreakLineOn),
            Vars,
            Selectors,
        }

        struct StyleMapVisitor;

        impl<'de> Visitor<'de> for StyleMapVisitor {
            type Value = StyleAsset;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("style definition")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::MapAccess<'de>,
            {
                type SA = StyleAttr;
                let mut st = StyleAsset::with_capacity(map.size_hint().unwrap_or(0));
                let attrs = &mut st.attrs;

                while let Some(key) = map.next_key()? {
                    match key {
                        Field::BackgroundImage => {
                            attrs.push(SA::BackgroundImage(map.next_value()?))
                        }
                        Field::BackgroundColor => {
                            attrs.push(SA::BackgroundColor(map.next_value()?))
                        }
                        Field::BorderColor => attrs.push(SA::BorderColor(map.next_value()?)),
                        Field::Color => attrs.push(SA::Color(map.next_value()?)),
                        Field::ZIndex => attrs.push(SA::ZIndex(map.next_value()?)),

                        Field::Display => attrs.push(SA::Display(map.next_value()?)),
                        Field::Position => attrs.push(SA::Position(map.next_value()?)),
                        Field::Overflow => attrs.push(SA::Overflow(map.next_value()?)),
                        Field::OverflowX => attrs.push(SA::OverflowX(map.next_value()?)),
                        Field::OverflowY => attrs.push(SA::OverflowY(map.next_value()?)),
                        Field::Direction => attrs.push(SA::Direction(map.next_value()?)),

                        Field::Left => attrs.push(SA::Left(map.next_value()?)),
                        Field::Right => attrs.push(SA::Right(map.next_value()?)),
                        Field::Top => attrs.push(SA::Top(map.next_value()?)),
                        Field::Bottom => attrs.push(SA::Bottom(map.next_value()?)),
                        Field::Width => attrs.push(SA::Width(map.next_value()?)),
                        Field::Height => attrs.push(SA::Height(map.next_value()?)),

                        Field::MinWidth => attrs.push(SA::MinWidth(map.next_value()?)),
                        Field::MinHeight => attrs.push(SA::MinHeight(map.next_value()?)),
                        Field::MaxWidth => attrs.push(SA::MaxWidth(map.next_value()?)),
                        Field::MaxHeight => attrs.push(SA::MaxHeight(map.next_value()?)),

                        Field::AlignItems => attrs.push(SA::AlignItems(map.next_value()?)),
                        Field::AlignContent => attrs.push(SA::AlignContent(map.next_value()?)),
                        Field::AlignSelf => attrs.push(SA::AlignSelf(map.next_value()?)),

                        Field::JustifyItems => attrs.push(SA::JustifyItems(map.next_value()?)),
                        Field::JustifyContent => attrs.push(SA::JustifyContent(map.next_value()?)),
                        Field::JustifySelf => attrs.push(SA::JustifySelf(map.next_value()?)),

                        Field::Margin => attrs.push(SA::Margin(map.next_value()?)),
                        Field::MarginLeft => attrs.push(SA::MarginLeft(map.next_value()?)),
                        Field::MarginRight => attrs.push(SA::MarginRight(map.next_value()?)),
                        Field::MarginTop => attrs.push(SA::MarginTop(map.next_value()?)),
                        Field::MarginBottom => attrs.push(SA::MarginBottom(map.next_value()?)),

                        Field::Padding => attrs.push(SA::Padding(map.next_value()?)),
                        Field::PaddingLeft => attrs.push(SA::PaddingLeft(map.next_value()?)),
                        Field::PaddingRight => attrs.push(SA::PaddingRight(map.next_value()?)),
                        Field::PaddingTop => attrs.push(SA::PaddingTop(map.next_value()?)),
                        Field::PaddingBottom => attrs.push(SA::PaddingBottom(map.next_value()?)),

                        Field::Border => attrs.push(SA::Border(map.next_value()?)),
                        Field::BorderLeft => attrs.push(SA::BorderLeft(map.next_value()?)),
                        Field::BorderRight => attrs.push(SA::BorderRight(map.next_value()?)),
                        Field::BorderTop => attrs.push(SA::BorderTop(map.next_value()?)),
                        Field::BorderBottom => attrs.push(SA::BorderBottom(map.next_value()?)),

                        Field::Flex => attrs.push(SA::Flex(map.next_value()?)),
                        Field::FlexDirection => {
                            attrs.push(SA::FlexDirection(map.next_value()?));
                        }
                        Field::FlexWrap => attrs.push(SA::FlexWrap(map.next_value()?)),
                        Field::FlexGrow => attrs.push(SA::FlexGrow(map.next_value()?)),
                        Field::FlexShrink => attrs.push(SA::FlexShrink(map.next_value()?)),
                        Field::FlexBasis => attrs.push(SA::FlexBasis(map.next_value()?)),
                        Field::RowGap => attrs.push(SA::RowGap(map.next_value()?)),
                        Field::ColumnGap => attrs.push(SA::ColumnGap(map.next_value()?)),
                        Field::Gap => attrs.push(SA::Gap(map.next_value()?)),

                        Field::GridRowStart => attrs.push(SA::GridRowStart(map.next_value()?)),
                        Field::GridRowSpan => attrs.push(SA::GridRowSpan(map.next_value()?)),

                        Field::Vars => {
                            st.vars = map.next_value::<VarsMap>()?;
                        }
                        Field::Selectors => {
                            st.selectors = map.next_value::<SelectorsMap>()?;
                        }
                    }
                }
                Ok(st)
            }
        }

        deserializer.deserialize_struct("StyleMap", FIELDS, StyleMapVisitor)
    }
}

#[cfg(test)]
mod tests {
    use bevy::ui;

    use crate::guise::style::{selector::Selector, ExprList, StyleExpr, UntypedExpr};

    use super::*;

    #[test]
    fn test_serialize_misc_props() {
        let map = StyleAsset::from_attrs(&[
            StyleAttr::ZIndex(StyleExpr::Constant(7)),
            StyleAttr::FlexGrow(StyleExpr::Constant(2.)),
            StyleAttr::FlexShrink(StyleExpr::Constant(3.)),
        ]);
        let ser = serde_json::to_string(&map);
        assert_eq!(
            ser.unwrap(),
            r#"{"z-index":7,"flex-grow":2,"flex-shrink":3}"#
        );
    }

    #[test]
    fn test_deserialize_basic_prop() {
        let des =
            serde_json::from_str::<StyleAsset>(r#"{"background-color":"rgba(255, 0, 0, 1)"}"#)
                .unwrap();
        assert_eq!(des.attrs.len(), 1);
        let ser = serde_json::to_string(&des);
        assert_eq!(ser.unwrap(), r#"{"background-color":"rgba(255, 0, 0, 1)"}"#);
    }

    #[test]
    fn test_deserialize_misc_props() {
        let des = serde_json::from_str::<StyleAsset>(
            r#"{"z-index":7,"flex-grow":2.0,"flex-shrink":3.1}"#,
        )
        .unwrap();
        assert_eq!(des.attrs.len(), 3);
        let ser = serde_json::to_string(&des);
        assert_eq!(
            ser.unwrap(),
            r#"{"z-index":7,"flex-grow":2,"flex-shrink":3.1}"#
        );
    }

    #[test]
    fn test_deserialize_length_no_unit() {
        let des = serde_json::from_str::<StyleAsset>(r#"{"right":7}"#).unwrap();
        assert_eq!(des.attrs.len(), 1);
        assert_eq!(
            des.attrs[0],
            StyleAttr::Right(StyleExpr::Constant(ui::Val::Px(7.)))
        );
        // let ser = serde_json::to_string(&des);
        // assert_eq!(ser.unwrap(), r#"{"right":7}"#);
    }

    #[test]
    fn test_deserialize_length_px() {
        let des = serde_json::from_str::<StyleAsset>(r#"{"right":"7px"}"#).unwrap();
        assert_eq!(des.attrs.len(), 1);
        assert_eq!(
            des.attrs[0],
            StyleAttr::Right(StyleExpr::Constant(ui::Val::Px(7.)))
        );
        // let ser = serde_json::to_string(&des);
        // assert_eq!(ser.unwrap(), r#"{"right":7}"#);
    }

    #[test]
    fn test_deserialize_auto() {
        let des = serde_json::from_str::<StyleAsset>(r#"{"right":"auto"}"#).unwrap();
        assert_eq!(des.attrs.len(), 1);
        assert_eq!(
            des.attrs[0],
            StyleAttr::Right(StyleExpr::Constant(ui::Val::Auto))
        );
    }

    #[test]
    fn test_serialize_display() {
        let map =
            StyleAsset::from_attrs(&[StyleAttr::Display(StyleExpr::Constant(ui::Display::Grid))]);
        let ser = serde_json::to_string(&map);
        assert_eq!(ser.unwrap(), r#"{"display":"grid"}"#);

        let map =
            StyleAsset::from_attrs(&[StyleAttr::Display(StyleExpr::Constant(ui::Display::Grid))]);
        let ser = serde_json::to_string(&map);
        assert_eq!(ser.unwrap(), r#"{"display":"grid"}"#);
    }

    #[test]
    fn test_deserialize_display() {
        let des = serde_json::from_str::<StyleAsset>(r#"{"display":"grid"}"#).unwrap();
        assert_eq!(des.attrs.len(), 1);
        assert_eq!(
            des.attrs[0],
            StyleAttr::Display(StyleExpr::Constant(ui::Display::Grid))
        );
    }

    #[test]
    fn test_serialize_position() {
        let map = StyleAsset::from_attrs(&[StyleAttr::Position(StyleExpr::Constant(
            ui::PositionType::Relative,
        ))]);
        let ser = serde_json::to_string(&map);
        assert_eq!(ser.unwrap(), r#"{"position":"relative"}"#);
    }

    #[test]
    fn test_deserialize_position() {
        let des = serde_json::from_str::<StyleAsset>(r#"{"position":"relative"}"#).unwrap();
        assert_eq!(des.attrs.len(), 1);
        assert_eq!(
            des.attrs[0],
            StyleAttr::Position(StyleExpr::Constant(ui::PositionType::Relative,))
        );
    }

    #[test]
    fn test_serialize_overflow() {
        let map = StyleAsset::from_attrs(&[StyleAttr::Overflow(StyleExpr::Constant(
            ui::OverflowAxis::Clip,
        ))]);
        let ser = serde_json::to_string(&map);
        assert_eq!(ser.unwrap(), r#"{"overflow":"clip"}"#);

        let map = StyleAsset::from_attrs(&[StyleAttr::OverflowX(StyleExpr::Constant(
            ui::OverflowAxis::Clip,
        ))]);
        let ser = serde_json::to_string(&map);
        assert_eq!(ser.unwrap(), r#"{"overflow-x":"clip"}"#);

        let map = StyleAsset::from_attrs(&[StyleAttr::OverflowY(StyleExpr::Constant(
            ui::OverflowAxis::Clip,
        ))]);
        let ser = serde_json::to_string(&map);
        assert_eq!(ser.unwrap(), r#"{"overflow-y":"clip"}"#);
    }

    #[test]
    fn test_deserialize_overflow() {
        let des = serde_json::from_str::<StyleAsset>(r#"{"overflow":"clip"}"#).unwrap();
        assert_eq!(des.attrs.len(), 1);
        assert_eq!(
            des.attrs[0],
            StyleAttr::Overflow(StyleExpr::Constant(ui::OverflowAxis::Clip,))
        );

        let des = serde_json::from_str::<StyleAsset>(r#"{"overflow-x":"clip"}"#).unwrap();
        assert_eq!(des.attrs.len(), 1);
        assert_eq!(
            des.attrs[0],
            StyleAttr::OverflowX(StyleExpr::Constant(ui::OverflowAxis::Clip,))
        );

        let des = serde_json::from_str::<StyleAsset>(r#"{"overflow-y":"clip"}"#).unwrap();
        assert_eq!(des.attrs.len(), 1);
        assert_eq!(
            des.attrs[0],
            StyleAttr::OverflowY(StyleExpr::Constant(ui::OverflowAxis::Clip,))
        );
    }

    #[test]
    fn test_serialize_direction() {
        let map = StyleAsset::from_attrs(&[StyleAttr::Direction(StyleExpr::Constant(
            ui::Direction::LeftToRight,
        ))]);
        let ser = serde_json::to_string(&map);
        assert_eq!(ser.unwrap(), r#"{"direction":"ltr"}"#);
    }

    #[test]
    fn test_deserialize_direction() {
        let des = serde_json::from_str::<StyleAsset>(r#"{"direction":"ltr"}"#).unwrap();
        assert_eq!(des.attrs.len(), 1);
        assert_eq!(
            des.attrs[0],
            StyleAttr::Direction(StyleExpr::Constant(ui::Direction::LeftToRight,))
        );
    }

    #[test]
    fn test_serialize_align_items() {
        let map = StyleAsset::from_attrs(&[StyleAttr::AlignItems(StyleExpr::Constant(
            ui::AlignItems::Start,
        ))]);
        let ser = serde_json::to_string(&map);
        assert_eq!(ser.unwrap(), r#"{"align-items":"start"}"#);
    }

    #[test]
    fn test_deserialize_align_items() {
        let des = serde_json::from_str::<StyleAsset>(r#"{"align-items":"start"}"#).unwrap();
        assert_eq!(des.attrs.len(), 1);
        assert_eq!(
            des.attrs[0],
            StyleAttr::AlignItems(StyleExpr::Constant(ui::AlignItems::Start))
        );
    }

    #[test]
    fn test_serialize_align_content() {
        let map = StyleAsset::from_attrs(&[StyleAttr::AlignContent(StyleExpr::Constant(
            ui::AlignContent::Start,
        ))]);
        let ser = serde_json::to_string(&map);
        assert_eq!(ser.unwrap(), r#"{"align-content":"start"}"#);
    }

    #[test]
    fn test_deserialize_align_content() {
        let des = serde_json::from_str::<StyleAsset>(r#"{"align-content":"start"}"#).unwrap();
        assert_eq!(des.attrs.len(), 1);
        assert_eq!(
            des.attrs[0],
            StyleAttr::AlignContent(StyleExpr::Constant(ui::AlignContent::Start))
        );
    }

    #[test]
    fn test_serialize_align_self() {
        let map = StyleAsset::from_attrs(&[StyleAttr::AlignSelf(StyleExpr::Constant(
            ui::AlignSelf::Start,
        ))]);
        let ser = serde_json::to_string(&map);
        assert_eq!(ser.unwrap(), r#"{"align-self":"start"}"#);
    }

    #[test]
    fn test_deserialize_align_self() {
        let des = serde_json::from_str::<StyleAsset>(r#"{"align-self":"start"}"#).unwrap();
        assert_eq!(des.attrs.len(), 1);
        assert_eq!(
            des.attrs[0],
            StyleAttr::AlignSelf(StyleExpr::Constant(ui::AlignSelf::Start))
        );
    }

    #[test]
    fn test_serialize_justify_items() {
        let map = StyleAsset::from_attrs(&[StyleAttr::JustifyItems(StyleExpr::Constant(
            ui::JustifyItems::Start,
        ))]);
        let ser = serde_json::to_string(&map);
        assert_eq!(ser.unwrap(), r#"{"justify-items":"start"}"#);
    }

    #[test]
    fn test_deserialize_justify_items() {
        let des = serde_json::from_str::<StyleAsset>(r#"{"justify-items":"start"}"#).unwrap();
        assert_eq!(des.attrs.len(), 1);
        assert_eq!(
            des.attrs[0],
            StyleAttr::JustifyItems(StyleExpr::Constant(ui::JustifyItems::Start))
        );
    }

    #[test]
    fn test_serialize_justify_content() {
        let map = StyleAsset::from_attrs(&[StyleAttr::JustifyContent(StyleExpr::Constant(
            ui::JustifyContent::Start,
        ))]);
        let ser = serde_json::to_string(&map);
        assert_eq!(ser.unwrap(), r#"{"justify-content":"start"}"#);
    }

    #[test]
    fn test_deserialize_justify_content() {
        let des = serde_json::from_str::<StyleAsset>(r#"{"justify-content":"start"}"#).unwrap();
        assert_eq!(des.attrs.len(), 1);
        assert_eq!(
            des.attrs[0],
            StyleAttr::JustifyContent(StyleExpr::Constant(ui::JustifyContent::Start))
        );
    }

    #[test]
    fn test_serialize_justify_self() {
        let map = StyleAsset::from_attrs(&[StyleAttr::JustifySelf(StyleExpr::Constant(
            ui::JustifySelf::Start,
        ))]);
        let ser = serde_json::to_string(&map);
        assert_eq!(ser.unwrap(), r#"{"justify-self":"start"}"#);
    }

    #[test]
    fn test_deserialize_justify_self() {
        let des = serde_json::from_str::<StyleAsset>(r#"{"justify-self":"start"}"#).unwrap();
        assert_eq!(des.attrs.len(), 1);
        assert_eq!(
            des.attrs[0],
            StyleAttr::JustifySelf(StyleExpr::Constant(ui::JustifySelf::Start))
        );
    }

    #[test]
    fn test_serialize_uirect() {
        let map = StyleAsset::from_attrs(&[StyleAttr::Margin(ExprList::from_exprs(&[
            UntypedExpr::Number(0.),
        ]))]);
        let ser = serde_json::to_string(&map);
        assert_eq!(ser.unwrap(), r#"{"margin":[0]}"#);

        let map = StyleAsset::from_attrs(&[StyleAttr::Margin(ExprList::from_exprs(&[
            UntypedExpr::Number(0.),
            UntypedExpr::Number(0.),
        ]))]);
        let ser = serde_json::to_string(&map);
        assert_eq!(ser.unwrap(), r#"{"margin":[0,0]}"#);

        let map = StyleAsset::from_attrs(&[StyleAttr::Margin(ExprList::from_exprs(&[
            UntypedExpr::Length(ui::Val::Auto),
            UntypedExpr::Length(ui::Val::Px(7.)),
        ]))]);
        let ser = serde_json::to_string(&map);
        assert_eq!(ser.unwrap(), r#"{"margin":["auto",7]}"#);
    }

    #[test]
    fn test_deserialize_uirect() {
        // Unitless number
        let des = serde_json::from_str::<StyleAsset>(r#"{"margin":0}"#).unwrap();
        assert_eq!(des.attrs.len(), 1);
        assert_eq!(
            des.attrs[0],
            StyleAttr::Margin(ExprList::from_exprs(&[UntypedExpr::Number(0.)]))
        );

        // Unitless string
        let des = serde_json::from_str::<StyleAsset>(r#"{"margin":"0"}"#).unwrap();
        assert_eq!(des.attrs.len(), 1);
        assert_eq!(
            des.attrs[0],
            StyleAttr::Margin(ExprList::from_exprs(&[UntypedExpr::Number(0.)]))
        );

        // Pixel units
        let des = serde_json::from_str::<StyleAsset>(r#"{"margin":"0px"}"#).unwrap();
        assert_eq!(des.attrs.len(), 1);
        assert_eq!(
            des.attrs[0],
            StyleAttr::Margin(ExprList::from_exprs(&[UntypedExpr::Length(ui::Val::Px(
                0.
            ))]))
        );

        // Multiple values
        let des = serde_json::from_str::<StyleAsset>(r#"{"margin":"0px 0px"}"#).unwrap();
        assert_eq!(des.attrs.len(), 1);
        assert_eq!(
            des.attrs[0],
            StyleAttr::Margin(ExprList::from_exprs(&[
                UntypedExpr::Length(ui::Val::Px(0.)),
                UntypedExpr::Length(ui::Val::Px(0.))
            ]))
        );

        // Optimize ident to 'auto'
        let des = serde_json::from_str::<StyleAsset>(r#"{"margin":"0px auto"}"#).unwrap();
        assert_eq!(des.attrs.len(), 1);
        assert_eq!(
            des.attrs[0],
            StyleAttr::Margin(ExprList::from_exprs(&[
                UntypedExpr::Length(ui::Val::Px(0.)),
                UntypedExpr::Ident("auto".to_string())
            ]))
        );
    }

    #[test]
    fn test_deserialize_vars() {
        let des = serde_json::from_str::<StyleAsset>(r#"{"vars":{}}"#).unwrap();
        assert_eq!(des.vars.len(), 0);

        let des = serde_json::from_str::<StyleAsset>(r#"{"vars":{"--x":1}}"#).unwrap();
        assert_eq!(des.vars.len(), 1);
        assert_eq!(des.vars.get("x").unwrap(), &UntypedExpr::Number(1.));

        let des = serde_json::from_str::<StyleAsset>(r#"{"vars":{"--bg":"auto"}}"#).unwrap();
        assert_eq!(des.vars.len(), 1);
        assert_eq!(
            des.vars.get("bg").unwrap(),
            &UntypedExpr::Ident("auto".to_string())
        );
    }

    #[test]
    fn test_serialize_vars() {
        let mut style = StyleAsset::new();
        style.vars.insert("x".into(), UntypedExpr::Number(7.));
        let ser = serde_json::to_string(&style);
        assert_eq!(ser.unwrap(), r#"{"vars":{"--x":7}}"#);
    }

    #[test]
    fn test_deserialize_selectors() {
        let des = serde_json::from_str::<StyleAsset>(r#"{"selectors":{}}"#).unwrap();
        assert_eq!(des.selectors.len(), 0);

        let des = serde_json::from_str::<StyleAsset>(r#"{"selectors":{"&.name": {"margin":0}}}"#)
            .unwrap();
        assert_eq!(des.selectors.len(), 1);
        let (ref sel, ref style) = des.selectors.entries()[0];
        assert_eq!(
            sel,
            &Selector::Current(Box::new(Selector::Class(
                "name".into(),
                Box::new(Selector::Accept)
            )))
        );
        assert_eq!(style.len_attrs(), 1);
    }

    #[test]
    fn test_serialize_selectors() {
        let mut style = StyleAsset::new();
        style.selectors.insert(
            Selector::Current(Box::new(Selector::Accept)),
            Box::new(StyleAsset::new()),
        );
        let ser = serde_json::to_string(&style);
        assert_eq!(ser.unwrap(), r#"{"selectors":{"&":{}}}"#);
    }
}
