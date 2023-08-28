use std::fmt;

use crate::guise::style::expr::Expr;

use super::{Selector, StyleAttr};
use bevy::utils::HashMap;
use serde::{de::Visitor, ser::SerializeStruct, Deserialize, Serialize};

/// A collection of style attributes which can be merged to create a `ComputedStyle`.
pub struct Style {
    /// List of style attributes.
    /// Rather than storing the attributes in a struct full of optional fields, we store a flat
    /// vector of enums, each of which stores a single style attribute. This "sparse" representation
    /// allows for fast merging of styles, particularly for styles which have few or no attributes.
    attrs: Vec<StyleAttr>,

    /// List of style variables to define when this style is invoked.
    /// TODO: Var values are not strings...
    vars: HashMap<String, String>,

    /// List of conditional styles
    selectors: Vec<(Selector, Box<Style>)>,
}

impl Style {
    pub fn new() -> Self {
        Self {
            attrs: Vec::new(),
            vars: HashMap::new(),
            selectors: Vec::new(),
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            attrs: Vec::with_capacity(capacity),
            vars: HashMap::new(),
            selectors: Vec::new(),
        }
    }

    /// Construct a new `StyleMap` from a list of `StyleAttr`s.
    pub fn from_attrs(attrs: &[StyleAttr]) -> Self {
        Self {
            attrs: Vec::from(attrs),
            vars: HashMap::new(),
            selectors: Vec::new(),
        }
    }

    // Number of style attributes in the map.
    // pub fn len(&self) -> usize {
    //     self.att.len()
    // }

    // pub fn push(&mut self, attr: StyleAttr) {
    //     self.0.push(attr);
    // }
}

impl Serialize for Style {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut st = serializer.serialize_struct("StyleMap", self.attrs.len())?;
        for attr in self.attrs.iter() {
            match attr {
                StyleAttr::BackgroundColor(val) => st.serialize_field("background-color", val)?,
                StyleAttr::BorderColor(val) => st.serialize_field("border-color", val)?,
                StyleAttr::Color(val) => st.serialize_field("color", val)?,
                StyleAttr::ZIndex(val) => st.serialize_field("z-index", val)?,
                StyleAttr::Left(val) => st.serialize_field("left", val)?,
                StyleAttr::Right(val) => st.serialize_field("right", val)?,
                StyleAttr::FlexGrow(val) => st.serialize_field("flex-grow", val)?,
                StyleAttr::FlexShrink(val) => st.serialize_field("flex-shrink", val)?,
                _ => todo!("Implement serialization for {:?}", attr),
            };
        }
        st.end()
    }
}

const FIELDS: &'static [&'static str] = &[
    "background-color",
    "border-color",
    "color",
    "z-index",
    "left",
    "right",
    "flex-grow",
    "flex-shrink",
    "vars",
    "selectors",
];

impl<'de> Deserialize<'de> for Style {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(field_identifier, rename_all = "kebab-case")]
        enum Field {
            BackgroundColor,
            BorderColor,
            Color,

            ZIndex,
            // Display(bevy::ui::Display),
            // Position(bevy::ui::PositionType),
            // Overflow(bevy::ui::OverflowAxis),
            // OverflowX(bevy::ui::OverflowAxis),
            // OverflowY(bevy::ui::OverflowAxis),
            // Direction(bevy::ui::Direction),
            Left,
            Right,
            // Top(bevy::ui::Val),
            // Bottom(bevy::ui::Val),

            // Width(bevy::ui::Val),
            // Height(bevy::ui::Val),
            // MinWidth(bevy::ui::Val),
            // MinHeight(bevy::ui::Val),
            // MaxWidth(bevy::ui::Val),
            // MaxHeight(bevy::ui::Val),

            // // pub aspect_ratio: StyleProp<f32>,
            // AlignItems(bevy::ui::AlignItems),
            // JustifyItems(bevy::ui::JustifyItems),
            // AlignSelf(bevy::ui::AlignSelf),
            // JustifySelf(bevy::ui::JustifySelf),
            // AlignContent(bevy::ui::AlignContent),
            // JustifyContent(bevy::ui::JustifyContent),

            // // Allow margin sides to be set individually
            // Margin(bevy::ui::UiRect),
            // MarginLeft(bevy::ui::Val),
            // MarginRight(bevy::ui::Val),
            // MarginTop(bevy::ui::Val),
            // MarginBottom(bevy::ui::Val),

            // Padding(bevy::ui::UiRect),
            // PaddingLeft(bevy::ui::Val),
            // PaddingRight(bevy::ui::Val),
            // PaddingTop(bevy::ui::Val),
            // PaddingBottom(bevy::ui::Val),

            // Border(bevy::ui::UiRect),
            // BorderLeft(bevy::ui::Val),
            // BorderRight(bevy::ui::Val),
            // BorderTop(bevy::ui::Val),
            // BorderBottom(bevy::ui::Val),

            // FlexDirection(bevy::ui::FlexDirection),
            // FlexWrap(bevy::ui::FlexWrap),
            FlexGrow,
            FlexShrink,
            // FlexBasis(bevy::ui::Val),

            // RowGap(bevy::ui::Val),
            // ColumnGap(bevy::ui::Val),
            // Gap(bevy::ui::Val),

            // // TODO:
            // GridAutoFlow(bevy::ui::GridAutoFlow),
            // // pub grid_template_rows: Option<Vec<RepeatedGridTrack>>,
            // // pub grid_template_columns: Option<Vec<RepeatedGridTrack>>,
            // // pub grid_auto_rows: Option<Vec<GridTrack>>,
            // // pub grid_auto_columns: Option<Vec<GridTrack>>,
            // GridRow(bevy::ui::GridPlacement),
            // GridRowStart(i16),
            // GridRowSpan(u16),
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
            type Value = Style;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("style definition")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::MapAccess<'de>,
            {
                let mut result = Style::with_capacity(map.size_hint().unwrap_or(0));
                while let Some(key) = map.next_key()? {
                    match key {
                        Field::BackgroundColor => {
                            result
                                .attrs
                                .push(StyleAttr::BackgroundColor(map.next_value::<Expr>()?));
                        }
                        Field::BorderColor => {
                            result
                                .attrs
                                .push(StyleAttr::BorderColor(map.next_value::<Expr>()?));
                        }
                        Field::Color => {
                            result
                                .attrs
                                .push(StyleAttr::Color(map.next_value::<Expr>()?));
                        }
                        Field::ZIndex => {
                            result
                                .attrs
                                .push(StyleAttr::ZIndex(map.next_value::<Expr>()?));
                        }
                        Field::Left => {
                            result
                                .attrs
                                .push(StyleAttr::Left(map.next_value::<Expr>()?));
                        }
                        Field::Right => {
                            result
                                .attrs
                                .push(StyleAttr::Right(map.next_value::<Expr>()?));
                        }
                        Field::FlexGrow => {
                            result
                                .attrs
                                .push(StyleAttr::FlexGrow(map.next_value::<Expr>()?));
                        }
                        Field::FlexShrink => {
                            result
                                .attrs
                                .push(StyleAttr::FlexShrink(map.next_value::<Expr>()?));
                        }
                        Field::Vars => {
                            todo!()
                        }
                        Field::Selectors => {
                            todo!()
                        }
                    }
                }
                Ok(result)
            }
        }

        deserializer.deserialize_struct("StyleMap", FIELDS, StyleMapVisitor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialize_misc_props() {
        let map = Style::from_attrs(&[
            StyleAttr::ZIndex(Expr::Number(7.)),
            StyleAttr::FlexGrow(Expr::Number(2.)),
            StyleAttr::FlexShrink(Expr::Number(3.)),
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
            serde_json::from_str::<Style>(r#"{"background-color":"rgba(255, 0, 0, 1)"}"#).unwrap();
        assert_eq!(des.attrs.len(), 1);
        let ser = serde_json::to_string(&des);
        assert_eq!(ser.unwrap(), r#"{"background-color":"rgba(255, 0, 0, 1)"}"#);
    }

    #[test]
    fn test_deserialize_misc_props() {
        let des =
            serde_json::from_str::<Style>(r#"{"z-index":7,"flex-grow":2.0,"flex-shrink":3.1}"#)
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
        let des = serde_json::from_str::<Style>(r#"{"right":7}"#).unwrap();
        assert_eq!(des.attrs.len(), 1);
        let ser = serde_json::to_string(&des);
        assert_eq!(ser.unwrap(), r#"{"right":7}"#);
    }

    #[test]
    fn test_deserialize_length_px() {
        let des = serde_json::from_str::<Style>(r#"{"right":"7px"}"#).unwrap();
        assert_eq!(des.attrs.len(), 1);
        let ser = serde_json::to_string(&des);
        assert_eq!(ser.unwrap(), r#"{"right":7}"#);
    }
}
