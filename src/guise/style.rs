use super::GuiseError;
use bevy::{
    reflect::{TypePath, TypeUuid},
    ui::*,
};
use lazy_static::lazy_static;
use quick_xml::{
    events::{BytesStart, Event},
    name::QName,
};
use quick_xml::{writer::Writer, Reader};
use regex::Regex;
use std::str::FromStr;

/** Set of style attributes that can be applied to construct a style. */
#[derive(Debug, Clone)]
pub enum StyleAttr {
    Display(bevy::ui::Display),
    Position(bevy::ui::PositionType),
    OverflowX(bevy::ui::OverflowAxis),
    OverflowY(bevy::ui::OverflowAxis),
    Direction(bevy::ui::Direction),

    Left(bevy::ui::Val),
    Right(bevy::ui::Val),
    Top(bevy::ui::Val),
    Bottom(bevy::ui::Val),

    Width(bevy::ui::Val),
    Height(bevy::ui::Val),
    MinWidth(bevy::ui::Val),
    MinHeight(bevy::ui::Val),
    MaxWidth(bevy::ui::Val),
    MaxHeight(bevy::ui::Val),

    // pub aspect_ratio: StyleProp<f32>,
    AlignItems(bevy::ui::AlignItems),
    JustifyItems(bevy::ui::JustifyItems),
    AlignSelf(bevy::ui::AlignSelf),
    JustifySelf(bevy::ui::JustifySelf),

    // Allow margin sides to be set individually
    Margin(bevy::ui::UiRect),
    MarginLeft(bevy::ui::Val),
    MarginRight(bevy::ui::Val),
    MarginTop(bevy::ui::Val),
    MarginBottom(bevy::ui::Val),

    Padding(bevy::ui::UiRect),
    PaddingLeft(bevy::ui::Val),
    PaddingRight(bevy::ui::Val),
    PaddingTop(bevy::ui::Val),
    PaddingBottom(bevy::ui::Val),

    Border(bevy::ui::UiRect),
    BorderLeft(bevy::ui::Val),
    BorderRight(bevy::ui::Val),
    BorderTop(bevy::ui::Val),
    BorderBottom(bevy::ui::Val),

    FlexDirection(bevy::ui::FlexDirection),
    FlexWrap(bevy::ui::FlexWrap),
    FlexGrow(f32),
    FlexShrink(f32),
    FlexBasis(bevy::ui::Val),

    RowGap(bevy::ui::Val),
    ColumnGap(bevy::ui::Val),
    Gap(bevy::ui::Val),
    // TODO:
    // pub grid_auto_flow: StyleProp<GridAutoFlow>,
    // pub grid_template_rows: Option<Vec<RepeatedGridTrack>>,
    // pub grid_template_columns: Option<Vec<RepeatedGridTrack>>,
    // pub grid_auto_rows: Option<Vec<GridTrack>>,
    // pub grid_auto_columns: Option<Vec<GridTrack>>,
    // pub grid_row: GridPlacement,
    // pub grid_column: GridPlacement,
}

impl StyleAttr {
    /// Apply this style attribute to a computed style.
    pub fn apply(&self, computed: &mut Style) {
        match self {
            StyleAttr::Display(val) => {
                computed.display = *val;
            }
            StyleAttr::Position(val) => {
                computed.position_type = *val;
            }
            StyleAttr::OverflowX(val) => {
                computed.overflow.x = *val;
            }
            StyleAttr::OverflowY(val) => {
                computed.overflow.y = *val;
            }
            StyleAttr::Direction(val) => {
                computed.direction = *val;
            }

            StyleAttr::Left(val) => {
                computed.left = *val;
            }
            StyleAttr::Right(val) => {
                computed.right = *val;
            }
            StyleAttr::Top(val) => {
                computed.top = *val;
            }
            StyleAttr::Bottom(val) => {
                computed.bottom = *val;
            }

            StyleAttr::Width(val) => {
                computed.width = *val;
            }
            StyleAttr::Height(val) => {
                computed.height = *val;
            }
            StyleAttr::MinWidth(val) => {
                computed.min_width = *val;
            }
            StyleAttr::MinHeight(val) => {
                computed.min_height = *val;
            }
            StyleAttr::MaxWidth(val) => {
                computed.max_width = *val;
            }
            StyleAttr::MaxHeight(val) => {
                computed.max_height = *val;
            }

            StyleAttr::AlignItems(val) => {
                computed.align_items = *val;
            }
            StyleAttr::JustifyItems(val) => {
                computed.justify_items = *val;
            }
            StyleAttr::AlignSelf(val) => {
                computed.align_self = *val;
            }
            StyleAttr::JustifySelf(val) => {
                computed.justify_self = *val;
            }

            StyleAttr::Margin(val) => {
                computed.margin = *val;
            }
            StyleAttr::MarginLeft(val) => {
                computed.margin.left = *val;
            }
            StyleAttr::MarginRight(val) => {
                computed.margin.right = *val;
            }
            StyleAttr::MarginTop(val) => {
                computed.margin.top = *val;
            }
            StyleAttr::MarginBottom(val) => {
                computed.margin.bottom = *val;
            }

            StyleAttr::Padding(val) => {
                computed.padding = *val;
            }
            StyleAttr::PaddingLeft(val) => {
                computed.padding.left = *val;
            }
            StyleAttr::PaddingRight(val) => {
                computed.padding.right = *val;
            }
            StyleAttr::PaddingTop(val) => {
                computed.padding.top = *val;
            }
            StyleAttr::PaddingBottom(val) => {
                computed.padding.bottom = *val;
            }

            StyleAttr::Border(val) => {
                computed.border = *val;
            }
            StyleAttr::BorderLeft(val) => {
                computed.border.left = *val;
            }
            StyleAttr::BorderRight(val) => {
                computed.border.right = *val;
            }
            StyleAttr::BorderTop(val) => {
                computed.border.top = *val;
            }
            StyleAttr::BorderBottom(val) => {
                computed.border.bottom = *val;
            }

            StyleAttr::FlexDirection(val) => {
                computed.flex_direction = *val;
            }
            StyleAttr::FlexWrap(val) => {
                computed.flex_wrap = *val;
            }
            StyleAttr::FlexGrow(val) => {
                computed.flex_grow = *val;
            }
            StyleAttr::FlexShrink(val) => {
                computed.flex_shrink = *val;
            }
            StyleAttr::FlexBasis(val) => {
                computed.flex_basis = *val;
            }

            StyleAttr::RowGap(val) => {
                computed.row_gap = *val;
            }
            StyleAttr::ColumnGap(val) => {
                computed.column_gap = *val;
            }
            StyleAttr::Gap(val) => {
                computed.row_gap = *val;
                computed.column_gap = *val;
            }
        }
    }

    /// Parse a `StyleAttr` from an XML attribute name/value pair.
    pub fn from_xml<'a>(name: &'a [u8], value: &str) -> Result<Self, GuiseError> {
        Ok(match name {
            b"display" => StyleAttr::Display(match value {
                "none" => Display::None,
                "grid" => Display::Grid,
                "flex" => Display::Flex,
                _ => {
                    return Err(GuiseError::UnknownAttributeValue(value.to_string()));
                }
            }),

            b"position" => StyleAttr::Position(match value {
                "absolute" => PositionType::Absolute,
                "relative" => PositionType::Relative,
                _ => {
                    return Err(GuiseError::UnknownAttributeValue(value.to_string()));
                }
            }),

            b"overflow-x" => StyleAttr::OverflowX(match value {
                "clip" => OverflowAxis::Clip,
                "visible" => OverflowAxis::Visible,
                _ => {
                    return Err(GuiseError::UnknownAttributeValue(value.to_string()));
                }
            }),

            b"overflow-y" => StyleAttr::OverflowY(match value {
                "clip" => OverflowAxis::Clip,
                "visible" => OverflowAxis::Visible,
                _ => {
                    return Err(GuiseError::UnknownAttributeValue(value.to_string()));
                }
            }),

            //     Direction,

            //     Left,
            //     Right,
            //     Top,
            //     Bottom,

            //     Width,
            //     Height,
            //     MinWidth,
            //     MinHeight,
            //     MaxWidth,
            //     MaxHeight,

            //     // pub aspect_ratio: StyleProp<f32>,
            //     AlignItems,
            //     JustifyItems,
            //     AlignSelf,
            //     JustifySelf,
            b"margin" => StyleAttr::Margin(StyleAttr::parse_uirect(value)?),
            b"margin-left" => StyleAttr::MarginLeft(StyleAttr::parse_val(value)?),
            b"margin-right" => StyleAttr::MarginRight(StyleAttr::parse_val(value)?),
            b"margin-top" => StyleAttr::MarginTop(StyleAttr::parse_val(value)?),
            b"margin-bottom" => StyleAttr::MarginBottom(StyleAttr::parse_val(value)?),

            b"padding" => StyleAttr::Padding(StyleAttr::parse_uirect(value)?),
            b"padding-left" => StyleAttr::PaddingLeft(StyleAttr::parse_val(value)?),
            b"padding-right" => StyleAttr::PaddingRight(StyleAttr::parse_val(value)?),
            b"padding-top" => StyleAttr::PaddingTop(StyleAttr::parse_val(value)?),
            b"padding-bottom" => StyleAttr::PaddingBottom(StyleAttr::parse_val(value)?),

            b"border" => StyleAttr::Border(StyleAttr::parse_uirect(value)?),
            b"border-left" => StyleAttr::BorderLeft(StyleAttr::parse_val(value)?),
            b"border-right" => StyleAttr::BorderRight(StyleAttr::parse_val(value)?),
            b"border-top" => StyleAttr::BorderTop(StyleAttr::parse_val(value)?),
            b"border-bottom" => StyleAttr::BorderBottom(StyleAttr::parse_val(value)?),

            b"flex-direction" => StyleAttr::FlexDirection(match value {
                "row" => FlexDirection::Row,
                "column" => FlexDirection::Column,
                "row-reverse" => FlexDirection::RowReverse,
                "column-reverse" => FlexDirection::ColumnReverse,
                _ => {
                    return Err(GuiseError::UnknownAttributeValue(value.to_string()));
                }
            }),

            //     FlexWrap,
            //     FlexGrow,
            //     FlexShrink,
            //     FlexBasis,
            b"row-gap" => StyleAttr::RowGap(StyleAttr::parse_val(value)?),
            b"column-gap" => StyleAttr::ColumnGap(StyleAttr::parse_val(value)?),
            b"gap" => StyleAttr::Gap(StyleAttr::parse_val(value)?),

            //     // TODO:
            //     // pub grid_auto_flow: StyleProp<GridAutoFlow>,
            //     // pub grid_template_rows: Option<Vec<RepeatedGridTrack>>,
            //     // pub grid_template_columns: Option<Vec<RepeatedGridTrack>>,
            //     // pub grid_auto_rows: Option<Vec<GridTrack>>,
            //     // pub grid_auto_columns: Option<Vec<GridTrack>>,
            //     // pub grid_row: GridPlacement,
            //     // pub grid_column: GridPlacement,
            _ => {
                panic!(
                    "Unknown style attribute: {}",
                    std::str::from_utf8(name).unwrap()
                );
            }
        })
    }

    pub fn write_xml(&self, elem: &mut BytesStart) {
        match self {
            StyleAttr::Display(disp) => {
                elem.push_attribute((
                    "display",
                    match disp {
                        Display::None => "none",
                        Display::Flex => "flex",
                        Display::Grid => "grid",
                    },
                ));
            }

            StyleAttr::Position(pos) => {
                elem.push_attribute((
                    "position",
                    match pos {
                        PositionType::Absolute => "absolute",
                        PositionType::Relative => "relative",
                    },
                ));
            }

            StyleAttr::OverflowX(ov) => {
                elem.push_attribute((
                    "overflow-x",
                    match ov {
                        OverflowAxis::Clip => "clip",
                        OverflowAxis::Visible => "visible",
                    },
                ));
            }

            StyleAttr::OverflowY(ov) => {
                elem.push_attribute((
                    "overflow-y",
                    match ov {
                        OverflowAxis::Clip => "clip",
                        OverflowAxis::Visible => "visible",
                    },
                ));
            }

            // StyleAttr::Direction(val) => {
            //     computed.direction = *val;
            // }
            StyleAttr::Left(val) => {
                elem.push_attribute(("left", StyleAttr::val_to_str(*val).as_str()));
            }
            StyleAttr::Right(val) => {
                elem.push_attribute(("right", StyleAttr::val_to_str(*val).as_str()));
            }
            StyleAttr::Top(val) => {
                elem.push_attribute(("top", StyleAttr::val_to_str(*val).as_str()));
            }
            StyleAttr::Bottom(val) => {
                elem.push_attribute(("bottom", StyleAttr::val_to_str(*val).as_str()));
            }

            StyleAttr::Width(val) => {
                elem.push_attribute(("width", StyleAttr::val_to_str(*val).as_str()));
            }
            StyleAttr::Height(val) => {
                elem.push_attribute(("height", StyleAttr::val_to_str(*val).as_str()));
            }
            StyleAttr::MinWidth(val) => {
                elem.push_attribute(("min-width", StyleAttr::val_to_str(*val).as_str()));
            }
            StyleAttr::MinHeight(val) => {
                elem.push_attribute(("min-height", StyleAttr::val_to_str(*val).as_str()));
            }
            StyleAttr::MaxWidth(val) => {
                elem.push_attribute(("max-width", StyleAttr::val_to_str(*val).as_str()));
            }
            StyleAttr::MaxHeight(val) => {
                elem.push_attribute(("max-height", StyleAttr::val_to_str(*val).as_str()));
            }

            // StyleAttr::AlignItems(val) => {
            //     computed.align_items = *val;
            // }
            // StyleAttr::JustifyItems(val) => {
            //     computed.justify_items = *val;
            // }
            // StyleAttr::AlignSelf(val) => {
            //     computed.align_self = *val;
            // }
            // StyleAttr::JustifySelf(val) => {
            //     computed.justify_self = *val;
            // }
            StyleAttr::Margin(val) => {
                elem.push_attribute(("margin", StyleAttr::uirect_to_str(*val).as_str()));
            }
            StyleAttr::MarginLeft(val) => {
                elem.push_attribute(("margin-left", StyleAttr::val_to_str(*val).as_str()));
            }
            StyleAttr::MarginRight(val) => {
                elem.push_attribute(("margin-right", StyleAttr::val_to_str(*val).as_str()));
            }
            StyleAttr::MarginTop(val) => {
                elem.push_attribute(("margin-top", StyleAttr::val_to_str(*val).as_str()));
            }
            StyleAttr::MarginBottom(val) => {
                elem.push_attribute(("margin-bottom", StyleAttr::val_to_str(*val).as_str()));
            }

            StyleAttr::Padding(val) => {
                elem.push_attribute(("padding", StyleAttr::uirect_to_str(*val).as_str()));
            }
            StyleAttr::PaddingLeft(val) => {
                elem.push_attribute(("padding-left", StyleAttr::val_to_str(*val).as_str()));
            }
            StyleAttr::PaddingRight(val) => {
                elem.push_attribute(("padding-right", StyleAttr::val_to_str(*val).as_str()));
            }
            StyleAttr::PaddingTop(val) => {
                elem.push_attribute(("padding-top", StyleAttr::val_to_str(*val).as_str()));
            }
            StyleAttr::PaddingBottom(val) => {
                elem.push_attribute(("padding-bottom", StyleAttr::val_to_str(*val).as_str()));
            }

            StyleAttr::Border(val) => {
                elem.push_attribute(("border", StyleAttr::uirect_to_str(*val).as_str()));
            }
            StyleAttr::BorderLeft(val) => {
                elem.push_attribute(("border-left", StyleAttr::val_to_str(*val).as_str()));
            }
            StyleAttr::BorderRight(val) => {
                elem.push_attribute(("border-right", StyleAttr::val_to_str(*val).as_str()));
            }
            StyleAttr::BorderTop(val) => {
                elem.push_attribute(("border-top", StyleAttr::val_to_str(*val).as_str()));
            }
            StyleAttr::BorderBottom(val) => {
                elem.push_attribute(("border-bottom", StyleAttr::val_to_str(*val).as_str()));
            }

            // StyleAttr::FlexDirection(val) => {
            //     computed.flex_direction = *val;
            // }
            // StyleAttr::FlexWrap(val) => {
            //     computed.flex_wrap = *val;
            // }
            // StyleAttr::FlexGrow(val) => {
            //     computed.flex_grow = *val;
            // }
            // StyleAttr::FlexShrink(val) => {
            //     computed.flex_shrink = *val;
            // }
            // StyleAttr::FlexBasis(val) => {
            //     computed.flex_basis = *val;
            // }

            // StyleAttr::RowGap(val) => {
            //     computed.row_gap = *val;
            // }
            // StyleAttr::ColumnGap(val) => {
            //     computed.column_gap = *val;
            // }
            // StyleAttr::Gap(val) => {
            //     computed.row_gap = *val;
            //     computed.column_gap = *val;
            // }
            _ => {
                panic!("Unsupported tag")
            }
        }
    }

    /// Convert a CSS-style length string into a `Val`.
    fn parse_val(str: &str) -> Result<Val, GuiseError> {
        if str == "auto" {
            return Ok(Val::Auto);
        }
        lazy_static! {
            static ref RE: Regex = Regex::new(r"^([\d\.]+)(px|vw|vh|vmin|vmax|%)?$").unwrap();
        }
        RE.captures(str)
            .and_then(|cap| {
                let dist = f32::from_str(&cap[1]).unwrap();
                if cap.get(2).is_none() {
                    // Default to pixels if no unit
                    return Some(Val::Px(dist));
                }
                match &cap[2] {
                    "px" => Some(Val::Px(dist)),
                    "vw" => Some(Val::Vw(dist)),
                    "vh" => Some(Val::Vh(dist)),
                    "vmin" => Some(Val::VMin(dist)),
                    "vmax" => Some(Val::VMax(dist)),
                    _ => {
                        panic!("Invalid unit");
                    }
                }
            })
            .ok_or(GuiseError::InvalidAttributeValue(str.to_string()))
    }

    /// Convert a CSS-style string representing a sequences of "lengths" into a `UiRect`.
    /// These go in CSS order: (top, right, bottom, left).
    /// CSS shortcut forms are supported.
    fn parse_uirect(str: &str) -> Result<UiRect, GuiseError> {
        let mut rect = UiRect::new(Val::Auto, Val::Auto, Val::Auto, Val::Auto);
        let mut sides = str.split_whitespace();

        // Top
        if let Some(top) = sides.next() {
            rect.top = StyleAttr::parse_val(top)?;

            // Right defaults to top if not specified
            rect.right = match sides.next() {
                Some(val) => StyleAttr::parse_val(val)?,
                None => rect.top,
            };

            // Bottom defaults to top if not specified
            rect.bottom = match sides.next() {
                Some(val) => StyleAttr::parse_val(val)?,
                None => rect.top,
            };

            // Left defaults to right if not specified
            rect.left = match sides.next() {
                Some(val) => StyleAttr::parse_val(val)?,
                None => rect.right,
            };

            // Should be no more values.
            if sides.next().is_none() {
                Ok(rect)
            } else {
                Err(GuiseError::InvalidAttributeValue(str.to_string()))
            }
        } else {
            Err(GuiseError::InvalidAttributeValue(str.to_string()))
        }
    }

    /// Convert a `Val` into a CSS-style string.
    fn val_to_str(val: Val) -> String {
        match val {
            Val::Auto => "auto".to_string(),
            Val::Px(px) => format!("{}px", px),
            Val::Percent(pct) => format!("{}%", pct),
            Val::Vw(v) => format!("{}vw", v),
            Val::Vh(v) => format!("{}vh", v),
            Val::VMin(v) => format!("{}vmin", v),
            Val::VMax(v) => format!("{}vmax", v),
        }
    }

    /// Convert a `UiRect` into a CSS-style string. The order of the values is (top, right, bottom,
    /// left).
    fn uirect_to_str(val: UiRect) -> String {
        format!(
            "{} {} {} {}",
            StyleAttr::val_to_str(val.top),
            StyleAttr::val_to_str(val.right),
            StyleAttr::val_to_str(val.bottom),
            StyleAttr::val_to_str(val.left)
        )
    }
}

const ATTR_ID: QName = QName(b"id");

/// A collection of style properties which can be merged to create a `Style`.
/// Rather than storing the attributes in a struct full of optional fields, we store a flat
/// vector of enums, each of which stores a single style attribute. This "sparse" representation
/// allows for fast (O(N) where N is the number of defined attributes) merging of styles,
/// particularly for styles which have few or no attributes.
#[derive(Debug, TypeUuid, TypePath, Default)]
#[uuid = "7d753986-2d0b-4e22-9ef3-166ffafa989e"]
pub struct PartialStyle {
    attrs: Vec<StyleAttr>,
}

impl PartialStyle {
    pub const EMPTY: &'static PartialStyle = &PartialStyle::new();

    /// Construct a new, empty `PartialStyle`.
    pub const fn new() -> Self {
        Self { attrs: Vec::new() }
    }

    /// Construct a new, empty `PartialStyle` with capacity `size`.
    pub fn with_capacity(size: usize) -> Self {
        Self {
            attrs: Vec::with_capacity(size),
        }
    }

    /// Construct a new `PartialStyle` from a list of `StyleAttr`s.
    pub fn from_attrs(attrs: &[StyleAttr]) -> Self {
        Self {
            attrs: Vec::from(attrs),
        }
    }

    /// Construct a new `PartialStyle` from a Quick-XML `Attributes` collection.
    pub fn from_xml<'a>(e: &'a BytesStart) -> Result<Self, GuiseError> {
        let mut style = Self {
            attrs: Vec::with_capacity(e.attributes().count()),
        };
        for a in e.attributes() {
            if let Ok(attr) = a {
                if attr.key != ATTR_ID && attr.key.prefix().is_none() {
                    let attr_name: &[u8] = attr.key.local_name().into_inner();
                    let attr_value: &str = &attr.unescape_value().unwrap();
                    style
                        .attrs
                        .push(StyleAttr::from_xml(attr_name, attr_value.trim())?)
                }
            }
        }
        Ok(style)
    }

    /// Merge the style properties into a computed `Style` object.
    pub fn apply_to(&self, computed: &mut Style) {
        for attr in self.attrs.iter() {
            attr.apply(computed);
        }
    }

    /// Returns either the current style or an empty style based on a condition.
    /// Used for dynamic styling in response to state changes.
    pub fn if_cond(&self, cond: bool) -> &PartialStyle {
        if cond {
            &self
        } else {
            PartialStyle::EMPTY
        }
    }

    pub fn write_xml(&self, writer: &mut Writer<std::io::Cursor<Vec<u8>>>) {
        let mut elem = BytesStart::new("style");
        for attr in self.attrs.iter() {
            attr.write_xml(&mut elem);
        }
        assert!(writer.write_event(Event::Empty(elem)).is_ok());
    }
}

pub fn from_xml<'a>(
    _reader: &mut Reader<&[u8]>,
    e: &'a BytesStart,
) -> Result<PartialStyle, GuiseError> {
    let mut result = PartialStyle {
        attrs: Vec::with_capacity(e.attributes().count()),
    };
    for a in e.attributes() {
        if let Ok(attr) = a {
            if attr.key != ATTR_ID && attr.key.prefix().is_none() {
                let attr_name: &[u8] = attr.key.local_name().into_inner();
                let attr_value: &str = &attr.unescape_value().unwrap();
                result
                    .attrs
                    .push(StyleAttr::from_xml(attr_name, attr_value.trim())?)
            }
        }
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_val() {
        assert_eq!(StyleAttr::parse_val("auto").unwrap(), Val::Auto);
        assert_eq!(StyleAttr::parse_val("1").unwrap(), Val::Px(1.));
        assert_eq!(StyleAttr::parse_val("1px").unwrap(), Val::Px(1.));
        assert_eq!(StyleAttr::parse_val("1vw").unwrap(), Val::Vw(1.));
        assert_eq!(StyleAttr::parse_val("1vh").unwrap(), Val::Vh(1.));
        assert_eq!(StyleAttr::parse_val("1.1px").unwrap(), Val::Px(1.1));

        assert!(StyleAttr::parse_val("1.1bad").is_err());
        assert!(StyleAttr::parse_val("bad").is_err());
        assert!(StyleAttr::parse_val("1.1.1bad").is_err());
    }

    #[test]
    fn test_parse_uirect() {
        assert_eq!(
            StyleAttr::parse_uirect("1px 2px 3px 4px").unwrap(),
            UiRect::new(Val::Px(4.), Val::Px(2.), Val::Px(1.), Val::Px(3.))
        );
        assert_eq!(
            StyleAttr::parse_uirect("1px 2px 3px").unwrap(),
            UiRect::new(Val::Px(2.), Val::Px(2.), Val::Px(1.), Val::Px(3.))
        );
        assert_eq!(
            StyleAttr::parse_uirect("1px 2px").unwrap(),
            UiRect::new(Val::Px(2.), Val::Px(2.), Val::Px(1.), Val::Px(1.))
        );
        assert_eq!(
            StyleAttr::parse_uirect("1px").unwrap(),
            UiRect::new(Val::Px(1.), Val::Px(1.), Val::Px(1.), Val::Px(1.))
        );

        assert!(StyleAttr::parse_uirect("1.1bad").is_err());
    }

    #[test]
    fn test_serialize_empty() {
        let style = PartialStyle::new();
        let mut writer = Writer::new(std::io::Cursor::new(Vec::new()));
        style.write_xml(&mut writer);
        assert_eq!(
            String::from_utf8(writer.into_inner().into_inner()).unwrap(),
            r#"<style/>"#
        );
    }

    #[test]
    fn test_serialize_display() {
        let style = PartialStyle::from_attrs(&[StyleAttr::Display(Display::Flex)]);
        let mut writer = Writer::new(std::io::Cursor::new(Vec::new()));
        style.write_xml(&mut writer);
        assert_eq!(
            String::from_utf8(writer.into_inner().into_inner()).unwrap(),
            r#"<style display="flex"/>"#
        );
    }
}
