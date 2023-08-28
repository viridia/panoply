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

                StyleAttr::Display(val) => st.serialize_field("display", val)?,

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
    "display",
    "left",
    "right",
    "top",
    "bottom",
    "width",
    "height",
    "min-width",
    "min-height",
    "max-width",
    "max-height",
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
            Display,
            // Position(bevy::ui::PositionType),
            // Overflow(bevy::ui::OverflowAxis),
            // OverflowX(bevy::ui::OverflowAxis),
            // OverflowY(bevy::ui::OverflowAxis),
            // Direction(bevy::ui::Direction),
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
                type SA = StyleAttr;
                let mut st = Style::with_capacity(map.size_hint().unwrap_or(0));
                let attrs = &mut st.attrs;
                while let Some(key) = map.next_key()? {
                    match key {
                        Field::BackgroundColor => {
                            attrs.push(SA::BackgroundColor(map.next_value::<Expr>()?))
                        }
                        Field::BorderColor => {
                            attrs.push(SA::BorderColor(map.next_value::<Expr>()?))
                        }
                        Field::Color => attrs.push(SA::Color(map.next_value::<Expr>()?)),
                        Field::ZIndex => attrs.push(SA::ZIndex(map.next_value::<Expr>()?)),

                        Field::Display => {
                            attrs.push(SA::Display(map.next_value::<Expr>()?.into_display_const()))
                        }

                        Field::Left => attrs.push(SA::Left(map.next_value::<Expr>()?)),
                        Field::Right => attrs.push(SA::Right(map.next_value::<Expr>()?)),
                        Field::Top => attrs.push(SA::Top(map.next_value::<Expr>()?)),
                        Field::Bottom => attrs.push(SA::Bottom(map.next_value::<Expr>()?)),

                        Field::Width => attrs.push(SA::Width(map.next_value::<Expr>()?)),
                        Field::Height => attrs.push(SA::Height(map.next_value::<Expr>()?)),
                        Field::MinWidth => attrs.push(SA::MinWidth(map.next_value::<Expr>()?)),
                        Field::MinHeight => attrs.push(SA::MinHeight(map.next_value::<Expr>()?)),
                        Field::MaxWidth => attrs.push(SA::MaxWidth(map.next_value::<Expr>()?)),
                        Field::MaxHeight => attrs.push(SA::MaxHeight(map.next_value::<Expr>()?)),

                        Field::FlexGrow => attrs.push(SA::FlexGrow(map.next_value::<Expr>()?)),
                        Field::FlexShrink => attrs.push(SA::FlexShrink(map.next_value::<Expr>()?)),
                        Field::Vars => {
                            todo!()
                        }
                        Field::Selectors => {
                            todo!()
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

    #[test]
    fn test_serialize_display() {
        let map = Style::from_attrs(&[StyleAttr::Display(Expr::Ident("grid".to_string()))]);
        let ser = serde_json::to_string(&map);
        assert_eq!(ser.unwrap(), r#"{"display":"grid"}"#);

        let map2 = Style::from_attrs(&[StyleAttr::Display(Expr::Display(ui::Display::Grid))]);
        let ser2 = serde_json::to_string(&map2);
        assert_eq!(ser2.unwrap(), r#"{"display":"grid"}"#);
    }

    #[test]
    fn test_deserialize_display() {
        let des = serde_json::from_str::<Style>(r#"{"display":"grid"}"#).unwrap();
        assert_eq!(des.attrs.len(), 1);
        assert_eq!(
            des.attrs[0],
            StyleAttr::Display(Expr::Display(ui::Display::Grid))
        );
    }
}
