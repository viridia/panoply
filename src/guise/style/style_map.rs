use std::fmt;

use crate::guise::style::{color::ColorValue, style_value::StyleValue};

use super::StyleAttr;
use serde::{de::Visitor, ser::SerializeStruct, Deserialize, Serialize};

pub struct StyleMap(Vec<StyleAttr>);

/// A collection of style attributes which can be merged to create a `ComputedStyle`.
/// Rather than storing the attributes in a struct full of optional fields, we store a flat
/// vector of enums, each of which stores a single style attribute. This "sparse" representation
/// allows for fast merging of styles, particularly for styles which have few or no attributes.
impl StyleMap {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self(Vec::with_capacity(capacity))
    }

    /// Construct a new `StyleMap` from a list of `StyleAttr`s.
    pub fn from_attrs(attrs: &[StyleAttr]) -> Self {
        Self(Vec::from(attrs))
    }

    /// Number of style attributes in the map.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn push(&mut self, attr: StyleAttr) {
        self.0.push(attr);
    }
}

impl Serialize for StyleMap {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut st = serializer.serialize_struct("StyleMap", self.0.len())?;
        for attr in self.0.iter() {
            match attr {
                StyleAttr::BackgroundColor(val) => st.serialize_field("background-color", val)?,
                StyleAttr::ZIndex(val) => st.serialize_field("z-index", val)?,
                StyleAttr::FlexGrow(val) => st.serialize_field("flex-grow", val)?,
                StyleAttr::FlexShrink(val) => st.serialize_field("flex-shrink", val)?,
                _ => todo!("Implement serialization"),
            };
        }
        st.end()
    }
}

const FIELDS: &'static [&'static str] = &[
    "background-color",
    "border-color",
    "z-index",
    "flex-grow",
    "flex-shrink",
];

impl<'de> Deserialize<'de> for StyleMap {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(field_identifier, rename_all = "kebab-case")]
        enum Field {
            BackgroundColor,
            BorderColor,
            // Color,
            ZIndex,
            // Display(bevy::ui::Display),
            // Position(bevy::ui::PositionType),
            // Overflow(bevy::ui::OverflowAxis),
            // OverflowX(bevy::ui::OverflowAxis),
            // OverflowY(bevy::ui::OverflowAxis),
            // Direction(bevy::ui::Direction),

            // Left(bevy::ui::Val),
            // Right(bevy::ui::Val),
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
        }

        struct StyleMapVisitor;

        impl<'de> Visitor<'de> for StyleMapVisitor {
            type Value = StyleMap;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a style definition")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::MapAccess<'de>,
            {
                let mut result = StyleMap::with_capacity(map.size_hint().unwrap_or(0));
                while let Some(key) = map.next_key()? {
                    match key {
                        Field::BackgroundColor => {
                            result.push(StyleAttr::BackgroundColor(
                                map.next_value::<StyleValue<ColorValue>>()?,
                            ));
                        }
                        Field::BorderColor => {
                            // result.0.push(StyleAttr::BorderColor(
                            //     map.next_value::<StyleColor>().unwrap(),
                            // ));
                        }
                        Field::ZIndex => {
                            result.push(StyleAttr::ZIndex(map.next_value::<StyleValue<i32>>()?));
                        }
                        Field::FlexGrow => {
                            result.push(StyleAttr::FlexGrow(map.next_value::<StyleValue<f32>>()?));
                        }
                        Field::FlexShrink => {
                            result
                                .push(StyleAttr::FlexShrink(map.next_value::<StyleValue<f32>>()?));
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
    use bevy::prelude::Color;

    use crate::guise::style::color::ColorValue;

    use super::*;

    #[test]
    fn test_serialize_basic_prop() {
        let map = StyleMap::from_attrs(&[StyleAttr::BackgroundColor(StyleValue::Constant(
            ColorValue::Color(Color::RED),
        ))]);
        let ser = serde_json::to_string(&map);
        assert_eq!(ser.unwrap(), r#"{"background-color":"rgba(255, 0, 0, 1)"}"#);
    }

    #[test]
    fn test_serialize_misc_props() {
        let map = StyleMap::from_attrs(&[
            StyleAttr::ZIndex(StyleValue::Constant(7)),
            StyleAttr::FlexGrow(StyleValue::Constant(2.)),
            StyleAttr::FlexShrink(StyleValue::Constant(3.)),
        ]);
        let ser = serde_json::to_string(&map);
        assert_eq!(
            ser.unwrap(),
            r#"{"z-index":7,"flex-grow":2.0,"flex-shrink":3.0}"#
        );
    }

    #[test]
    fn test_deserialize_basic_prop() {
        let des = serde_json::from_str::<StyleMap>(r#"{"background-color":"rgba(255, 0, 0, 1)"}"#)
            .unwrap();
        assert_eq!(des.len(), 1);
        let ser = serde_json::to_string(&des);
        assert_eq!(ser.unwrap(), r#"{"background-color":"rgba(255, 0, 0, 1)"}"#);
    }

    #[test]
    fn test_deserialize_misc_props() {
        let des =
            serde_json::from_str::<StyleMap>(r#"{"z-index":7,"flex-grow":2.0,"flex-shrink":3.0}"#)
                .unwrap();
        assert_eq!(des.len(), 3);
        let ser = serde_json::to_string(&des);
        assert_eq!(
            ser.unwrap(),
            r#"{"z-index":7,"flex-grow":2.0,"flex-shrink":3.0}"#
        );
    }
}
