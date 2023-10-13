use bevy::{prelude::*, text::BreakLineOn, ui};
use lazy_static::lazy_static;
use regex::Regex;
use std::str::FromStr;

use crate::guise::coerce::Coerce;

use super::{
    color::ColorValue, expr::StyleExpr, expr_list::ExprList, AssetRef, ComputedStyle, UntypedExpr,
};

/** A single style-sheet property which can be applied to a computed style. */
#[derive(Debug, Clone, PartialEq)]
pub enum StyleAttr {
    BackgroundImage(StyleExpr<AssetRef>),
    // BackgroundColor(StyleExpr<ColorValue>),
    // BorderColor(StyleExpr<ColorValue>),
    // Color(StyleExpr<ColorValue>),

    // ZIndex(StyleExpr<i32>),

    // Display(StyleExpr<ui::Display>),
    // Position(StyleExpr<ui::PositionType>),
    Overflow(StyleExpr<ui::OverflowAxis>),
    OverflowX(StyleExpr<ui::OverflowAxis>),
    OverflowY(StyleExpr<ui::OverflowAxis>),
    Direction(StyleExpr<ui::Direction>),

    // Left(StyleExpr<ui::Val>),
    // Right(StyleExpr<ui::Val>),
    // Top(StyleExpr<ui::Val>),
    // Bottom(StyleExpr<ui::Val>),

    // Width(StyleExpr<ui::Val>),
    // Height(StyleExpr<ui::Val>),
    // MinWidth(StyleExpr<ui::Val>),
    // MinHeight(StyleExpr<ui::Val>),
    // MaxWidth(StyleExpr<ui::Val>),
    // MaxHeight(StyleExpr<ui::Val>),

    // pub aspect_ratio: StyleProp<f32>,
    AlignItems(StyleExpr<ui::AlignItems>),
    AlignSelf(StyleExpr<ui::AlignSelf>),
    AlignContent(StyleExpr<ui::AlignContent>),
    JustifyItems(StyleExpr<ui::JustifyItems>),
    JustifySelf(StyleExpr<ui::JustifySelf>),
    JustifyContent(StyleExpr<ui::JustifyContent>),

    // Allow margin sides to be set individually
    Margin(ExprList),
    MarginLeft(StyleExpr<ui::Val>),
    MarginRight(StyleExpr<ui::Val>),
    MarginTop(StyleExpr<ui::Val>),
    MarginBottom(StyleExpr<ui::Val>),

    Padding(ExprList),
    PaddingLeft(StyleExpr<ui::Val>),
    PaddingRight(StyleExpr<ui::Val>),
    PaddingTop(StyleExpr<ui::Val>),
    PaddingBottom(StyleExpr<ui::Val>),

    Border(ExprList),
    BorderLeft(StyleExpr<ui::Val>),
    BorderRight(StyleExpr<ui::Val>),
    BorderTop(StyleExpr<ui::Val>),
    BorderBottom(StyleExpr<ui::Val>),

    FlexDirection(StyleExpr<ui::FlexDirection>),
    FlexWrap(StyleExpr<ui::FlexWrap>),
    Flex(ExprList),
    FlexGrow(StyleExpr<f32>),
    FlexShrink(StyleExpr<f32>),
    FlexBasis(StyleExpr<ui::Val>),

    RowGap(StyleExpr<ui::Val>),
    ColumnGap(StyleExpr<ui::Val>),
    Gap(StyleExpr<ui::Val>),

    // TODO:
    GridAutoFlow(bevy::ui::GridAutoFlow),
    // pub grid_template_rows: Option<Vec<RepeatedGridTrack>>,
    // pub grid_template_columns: Option<Vec<RepeatedGridTrack>>,
    // pub grid_auto_rows: Option<Vec<GridTrack>>,
    // pub grid_auto_columns: Option<Vec<GridTrack>>,
    GridRow(bevy::ui::GridPlacement),
    GridRowStart(StyleExpr<i16>),
    GridRowSpan(StyleExpr<u16>),
    GridRowEnd(i16),
    GridColumn(bevy::ui::GridPlacement),
    GridColumnStart(i16),
    GridColumnSpan(u16),
    GridColumnEnd(i16),

    LineBreak(BreakLineOn),
}

impl StyleAttr {
    /// Apply this style attribute to a computed style.
    pub fn apply(&self, computed: &mut ComputedStyle, server: &AssetServer) {
        match self {
            StyleAttr::BackgroundImage(val) => {
                if let StyleExpr::Constant(ref asset) = val {
                    // TODO: Get rid of clone here.
                    computed.image = Some(server.load(asset.resolved().clone()));
                } else {
                    warn!("Incorrect expression type for background-image: {:?}", val);
                }
            }
            StyleAttr::BackgroundColor(val) => {
                if let Some(c) = val.coerce() {
                    computed.background_color = c;
                }
            }
            StyleAttr::BorderColor(val) => {
                if let Some(c) = val.coerce() {
                    computed.border_color = c;
                }
            }
            StyleAttr::Color(val) => {
                if let Some(c) = val.coerce() {
                    computed.color = c;
                }
            }
            StyleAttr::ZIndex(val) => {
                if let Some(z) = val.coerce() {
                    computed.z_index = Some(z);
                }
            }

            StyleAttr::Display(val) => {
                if let Some(d) = val.coerce() {
                    computed.style.display = d;
                }
            }
            StyleAttr::Position(val) => {
                if let Some(d) = val.coerce() {
                    computed.style.position_type = d;
                }
            }
            StyleAttr::Overflow(val) => {
                if let Some(d) = val.coerce() {
                    computed.style.overflow.x = d;
                    computed.style.overflow.y = d;
                }
            }
            StyleAttr::OverflowX(val) => {
                if let Some(d) = val.coerce() {
                    computed.style.overflow.x = d;
                }
            }
            StyleAttr::OverflowY(val) => {
                if let Some(d) = val.coerce() {
                    computed.style.overflow.y = d;
                }
            }
            StyleAttr::Direction(val) => {
                if let Some(d) = val.coerce() {
                    computed.style.direction = d;
                }
            }

            StyleAttr::Left(val) => {
                if let Some(l) = val.coerce() {
                    computed.style.left = l;
                }
            }
            StyleAttr::Right(val) => {
                if let Some(l) = val.coerce() {
                    computed.style.right = l;
                }
            }
            StyleAttr::Top(val) => {
                if let Some(l) = val.coerce() {
                    computed.style.top = l;
                }
            }
            StyleAttr::Bottom(val) => {
                if let Some(l) = val.coerce() {
                    computed.style.bottom = l;
                }
            }

            StyleAttr::Width(val) => {
                if let Some(l) = val.coerce() {
                    computed.style.width = l;
                }
            }
            StyleAttr::Height(val) => {
                if let Some(l) = val.coerce() {
                    computed.style.height = l;
                }
            }
            StyleAttr::MinWidth(val) => {
                if let Some(l) = val.coerce() {
                    computed.style.min_width = l;
                }
            }
            StyleAttr::MinHeight(val) => {
                if let Some(l) = val.coerce() {
                    computed.style.min_height = l;
                }
            }
            StyleAttr::MaxWidth(val) => {
                if let Some(l) = val.coerce() {
                    computed.style.max_width = l;
                }
            }
            StyleAttr::MaxHeight(val) => {
                if let Some(l) = val.coerce() {
                    computed.style.max_height = l;
                }
            }

            StyleAttr::AlignItems(val) => {
                if let Some(l) = val.coerce() {
                    computed.style.align_items = l;
                }
            }
            StyleAttr::JustifyItems(val) => {
                if let Some(l) = val.coerce() {
                    computed.style.justify_items = l;
                }
            }
            StyleAttr::AlignSelf(val) => {
                if let Some(l) = val.coerce() {
                    computed.style.align_self = l;
                }
            }
            StyleAttr::JustifySelf(val) => {
                if let Some(l) = val.coerce() {
                    computed.style.justify_self = l;
                }
            }
            StyleAttr::AlignContent(val) => {
                if let Some(l) = val.coerce() {
                    computed.style.align_content = l;
                }
            }
            StyleAttr::JustifyContent(val) => {
                if let Some(l) = val.coerce() {
                    computed.style.justify_content = l;
                }
            }

            StyleAttr::Margin(val) => {
                if let Some(r) = val.coerce() {
                    computed.style.margin = r;
                }
            }
            StyleAttr::MarginLeft(val) => {
                if let Some(l) = val.coerce() {
                    computed.style.margin.left = l;
                }
            }
            StyleAttr::MarginRight(val) => {
                if let Some(l) = val.coerce() {
                    computed.style.margin.right = l;
                }
            }
            StyleAttr::MarginTop(val) => {
                if let Some(l) = val.coerce() {
                    computed.style.margin.top = l;
                }
            }
            StyleAttr::MarginBottom(val) => {
                if let Some(l) = val.coerce() {
                    computed.style.margin.bottom = l;
                }
            }

            StyleAttr::Padding(val) => {
                if let Some(r) = val.coerce() {
                    computed.style.padding = r;
                }
            }
            StyleAttr::PaddingLeft(val) => {
                if let Some(l) = val.coerce() {
                    computed.style.padding.left = l;
                }
            }
            StyleAttr::PaddingRight(val) => {
                if let Some(l) = val.coerce() {
                    computed.style.padding.right = l;
                }
            }
            StyleAttr::PaddingTop(val) => {
                if let Some(l) = val.coerce() {
                    computed.style.padding.top = l;
                }
            }
            StyleAttr::PaddingBottom(val) => {
                if let Some(l) = val.coerce() {
                    computed.style.padding.bottom = l;
                }
            }

            StyleAttr::Border(val) => {
                if let Some(r) = val.coerce() {
                    computed.style.border = r;
                }
            }
            StyleAttr::BorderLeft(val) => {
                if let Some(l) = val.coerce() {
                    computed.style.border.left = l;
                }
            }
            StyleAttr::BorderRight(val) => {
                if let Some(l) = val.coerce() {
                    computed.style.border.right = l;
                }
            }
            StyleAttr::BorderTop(val) => {
                if let Some(l) = val.coerce() {
                    computed.style.border.top = l;
                }
            }
            StyleAttr::BorderBottom(val) => {
                if let Some(l) = val.coerce() {
                    computed.style.border.bottom = l
                }
            }

            StyleAttr::FlexDirection(val) => {
                if let Some(l) = val.coerce() {
                    computed.style.flex_direction = l;
                }
            }
            StyleAttr::FlexWrap(val) => {
                if let Some(l) = val.coerce() {
                    computed.style.flex_wrap = l;
                }
            }

            StyleAttr::Flex(items) => {
                if items.len() == 3 {
                    match items.0[0] {
                        UntypedExpr::Number(n) => {
                            computed.style.flex_grow = n;
                        }
                        _ => (),
                    };
                    match items.0[1] {
                        UntypedExpr::Number(n) => {
                            computed.style.flex_shrink = n;
                        }
                        _ => (),
                    };
                    if let Some(basis) = items.0[2].coerce() {
                        computed.style.flex_basis = basis;
                    }
                } else if items.len() == 1 {
                    match items.0[0] {
                        UntypedExpr::Number(n) => {
                            computed.style.flex_grow = n;
                            computed.style.flex_shrink = n;
                            computed.style.flex_basis = Val::Auto;
                        }
                        _ => (),
                    };
                } else {
                    warn!("Invalid flex value: {:?}", items);
                }
            }

            StyleAttr::FlexGrow(val) => {
                if let Some(flex) = val.coerce() {
                    computed.style.flex_grow = flex;
                }
            }
            StyleAttr::FlexShrink(val) => {
                if let Some(flex) = val.coerce() {
                    computed.style.flex_shrink = flex;
                }
            }
            StyleAttr::FlexBasis(val) => {
                if let Some(len) = val.coerce() {
                    computed.style.flex_basis = len;
                }
            }

            StyleAttr::RowGap(val) => {
                if let Some(len) = val.coerce() {
                    computed.style.row_gap = len;
                }
            }
            StyleAttr::ColumnGap(val) => {
                if let Some(len) = val.coerce() {
                    computed.style.column_gap = len;
                }
            }
            StyleAttr::Gap(val) => {
                if let Some(len) = val.coerce() {
                    computed.style.row_gap = len;
                    computed.style.column_gap = len;
                }
            }

            StyleAttr::GridAutoFlow(val) => {
                computed.style.grid_auto_flow = *val;
            }
            StyleAttr::GridRow(val) => {
                computed.style.grid_row = *val;
            }
            StyleAttr::GridRowStart(val) => {
                if let Some(len) = val.coerce() {
                    computed.style.grid_row.set_start(len);
                }
            }
            StyleAttr::GridRowSpan(val) => {
                if let Some(len) = val.coerce() {
                    computed.style.grid_row.set_span(len);
                }
            }
            StyleAttr::GridRowEnd(val) => {
                computed.style.grid_row.set_end(*val);
            }

            StyleAttr::GridColumn(val) => {
                computed.style.grid_column = *val;
            }
            StyleAttr::GridColumnStart(val) => {
                computed.style.grid_column.set_start(*val);
            }
            StyleAttr::GridColumnSpan(val) => {
                computed.style.grid_column.set_span(*val);
            }
            StyleAttr::GridColumnEnd(val) => {
                computed.style.grid_column.set_end(*val);
            }
            StyleAttr::LineBreak(val) => {
                computed.line_break = Some(*val);
            }
        }
    }

    // TODO: Remove all this parsing code and tests once migrated to willow / serde.

    // / Parse a `StyleAttr` from an XML attribute name/value pair.
    // pub fn parse<'a>(name: &'a [u8], value: &str) -> Result<Option<Self>, GuiseError> {
    //     Ok(Some(match name {
    //         b"grid-auto-flow" => StyleAttr::GridAutoFlow(match value {
    //             "row" => GridAutoFlow::Row,
    //             "column" => GridAutoFlow::Column,
    //             "row-dense" => GridAutoFlow::RowDense,
    //             "column-dense" => GridAutoFlow::ColumnDense,
    //             _ => {
    //                 return Err(GuiseError::UnknownAttributeValue(value.to_string()));
    //             }
    //         }),
    //         //     // TODO:
    //         //     // pub grid_template_rows: Option<Vec<RepeatedGridTrack>>,
    //         //     // pub grid_template_columns: Option<Vec<RepeatedGridTrack>>,
    //         //     // pub grid_auto_rows: Option<Vec<GridTrack>>,
    //         //     // pub grid_auto_columns: Option<Vec<GridTrack>>,
    //         b"grid-row" => StyleAttr::GridRow(StyleAttr::parse_grid_placement(value)?),
    //         // b"grid-row-start" => StyleAttr::GridRowStart(StyleAttr::parse_i16(value)?),
    //         // b"grid-row-span" => StyleAttr::GridRowSpan(StyleAttr::parse_u16(value)?),
    //         b"grid-row-end" => StyleAttr::GridRowEnd(StyleAttr::parse_i16(value)?),
    //         b"grid-column" => StyleAttr::GridColumn(StyleAttr::parse_grid_placement(value)?),
    //         b"grid-column-start" => StyleAttr::GridColumnStart(StyleAttr::parse_i16(value)?),
    //         b"grid-column-span" => StyleAttr::GridColumnSpan(StyleAttr::parse_u16(value)?),
    //         b"grid-column-end" => StyleAttr::GridColumnEnd(StyleAttr::parse_i16(value)?),

    //         b"line-break" => StyleAttr::LineBreak(match value {
    //             "nowrap" => bevy::text::BreakLineOn::NoWrap,
    //             "word" => bevy::text::BreakLineOn::WordBoundary,
    //             "char" => bevy::text::BreakLineOn::AnyCharacter,
    //             _ => {
    //                 return Err(GuiseError::UnknownAttributeValue(value.to_string()));
    //             }
    //         }),

    //         _ => return Ok(None),
    //     }))
    // }

    // pub fn write_xml(&self, elem: &mut BytesStart) {
    //     match self {
    //         StyleAttr::GridAutoFlow(val) => {
    //             elem.push_attribute((
    //                 "grid-auto-flow",
    //                 match val {
    //                     GridAutoFlow::Row => "row",
    //                     GridAutoFlow::Column => "column",
    //                     GridAutoFlow::RowDense => "row-dense",
    //                     GridAutoFlow::ColumnDense => "column-dense",
    //                 },
    //             ));
    //         }

    //         StyleAttr::GridRow(_) => {
    //             panic!("Unsupported, can't write GridPlacement");
    //         }
    //         StyleAttr::GridRowStart(val) => {
    //             elem.push_attribute(("grid-row-start", i16::to_string(val).as_str()));
    //         }
    //         StyleAttr::GridRowSpan(val) => {
    //             elem.push_attribute(("grid-row-span", u16::to_string(val).as_str()));
    //         }
    //         StyleAttr::GridRowEnd(val) => {
    //             elem.push_attribute(("grid-row-end", i16::to_string(val).as_str()));
    //         }

    //         StyleAttr::GridColumn(_) => {
    //             panic!("Unsupported, can't write GridPlacement");
    //         }
    //         StyleAttr::GridColumnStart(val) => {
    //             elem.push_attribute(("grid-column-start", i16::to_string(val).as_str()));
    //         }
    //         StyleAttr::GridColumnSpan(val) => {
    //             elem.push_attribute(("grid-column-span", u16::to_string(val).as_str()));
    //         }
    //         StyleAttr::GridColumnEnd(val) => {
    //             elem.push_attribute(("grid-column-end", i16::to_string(val).as_str()));
    //         }

    //         StyleAttr::LineBreak(dir) => {
    //             elem.push_attribute((
    //                 "line-break",
    //                 match dir {
    //                     bevy::text::BreakLineOn::NoWrap => "nowrap",
    //                     bevy::text::BreakLineOn::WordBoundary => "word",
    //                     bevy::text::BreakLineOn::AnyCharacter => "char",
    //                 },
    //             ));
    //         }

    //         _ => {
    //             todo!("Implement attr")
    //         }
    //     }
    // }

    // / Convert a CSS-style color into a Color. Supports #hex, rgba() and hsla().
    // fn parse_grid_placement(str: &str) -> Result<GridPlacement, GuiseError> {
    //     lazy_static! {
    //         static ref RE_GRID_1: Regex = Regex::new(r"^([\d\.]+)\s*/\s*([\d\.]+)$").unwrap();
    //         static ref RE_GRID_2: Regex =
    //             Regex::new(r"^([\d\.]+)\s*/\s*span\s*([\d\.]+)$").unwrap();
    //     }

    //     RE_GRID_1
    //         .captures(str)
    //         .map(|cap| {
    //             GridPlacement::default()
    //                 .set_start(i16::from_str(&cap[1]).unwrap())
    //                 .set_end(i16::from_str(&cap[2]).unwrap())
    //         })
    //         .or(RE_GRID_2.captures(str).map(|cap| {
    //             GridPlacement::default()
    //                 .set_start(i16::from_str(&cap[1]).unwrap())
    //                 .set_span(u16::from_str(&cap[2]).unwrap())
    //         }))
    //         .ok_or(GuiseError::InvalidAttributeValue(str.to_string()))
    // }

    // /// Convert a CSS-style length string into a `Val`.
    // pub(crate) fn parse_val(str: &str) -> Result<Val, GuiseError> {
    //     if str == "auto" {
    //         return Ok(Val::Auto);
    //     }
    //     lazy_static! {
    //         static ref RE: Regex = Regex::new(r"^([\-\d\.]+)(px|vw|vh|vmin|vmax|%)?$").unwrap();
    //     }
    //     RE.captures(str)
    //         .and_then(|cap| {
    //             let dist = f32::from_str(&cap[1]).unwrap();
    //             if cap.get(2).is_none() {
    //                 // Default to pixels if no unit
    //                 return Some(Val::Px(dist));
    //             }
    //             match &cap[2] {
    //                 "px" => Some(Val::Px(dist)),
    //                 "%" => Some(Val::Percent(dist)),
    //                 "vw" => Some(Val::Vw(dist)),
    //                 "vh" => Some(Val::Vh(dist)),
    //                 "vmin" => Some(Val::VMin(dist)),
    //                 "vmax" => Some(Val::VMax(dist)),
    //                 _ => {
    //                     panic!("Invalid unit");
    //                 }
    //             }
    //         })
    //         .ok_or(GuiseError::InvalidAttributeValue(str.to_string()))
    // }

    // /// Convert a CSS-style string representing a sequences of "lengths" into a `UiRect`.
    // /// These go in CSS order: (top, right, bottom, left).
    // /// CSS shortcut forms are supported.
    // pub(crate) fn parse_uirect(str: &str) -> Result<UiRect, GuiseError> {
    //     let mut rect = UiRect::new(Val::Auto, Val::Auto, Val::Auto, Val::Auto);
    //     let mut sides = str.split_whitespace();

    //     // Top
    //     if let Some(top) = sides.next() {
    //         rect.top = StyleAttr::parse_val(top)?;

    //         // Right defaults to top if not specified
    //         rect.right = match sides.next() {
    //             Some(val) => StyleAttr::parse_val(val)?,
    //             None => rect.top,
    //         };

    //         // Bottom defaults to top if not specified
    //         rect.bottom = match sides.next() {
    //             Some(val) => StyleAttr::parse_val(val)?,
    //             None => rect.top,
    //         };

    //         // Left defaults to right if not specified
    //         rect.left = match sides.next() {
    //             Some(val) => StyleAttr::parse_val(val)?,
    //             None => rect.right,
    //         };

    //         // Should be no more values.
    //         if sides.next().is_none() {
    //             Ok(rect)
    //         } else {
    //             Err(GuiseError::InvalidAttributeValue(str.to_string()))
    //         }
    //     } else {
    //         Err(GuiseError::InvalidAttributeValue(str.to_string()))
    //     }
    // }

    // /// Parse a scalar float.
    // fn parse_f32(str: &str) -> Result<f32, GuiseError> {
    //     f32::from_str(str).or_else(|_| Err(GuiseError::InvalidAttributeValue(str.to_string())))
    // }

    // /// Parse a scalar i32.
    // fn parse_i16(str: &str) -> Result<i16, GuiseError> {
    //     i16::from_str(str).or_else(|_| Err(GuiseError::InvalidAttributeValue(str.to_string())))
    // }

    // /// Parse a scalar i32.
    // fn parse_u16(str: &str) -> Result<u16, GuiseError> {
    //     u16::from_str(str).or_else(|_| Err(GuiseError::InvalidAttributeValue(str.to_string())))
    // }
}

#[cfg(test)]
mod tests {
    use super::*;

    // #[test]
    // fn test_parse_val() {
    //     assert_eq!(StyleAttr::parse_val("auto").unwrap(), Val::Auto);
    //     assert_eq!(StyleAttr::parse_val("1").unwrap(), Val::Px(1.));
    //     assert_eq!(StyleAttr::parse_val("1px").unwrap(), Val::Px(1.));
    //     assert_eq!(StyleAttr::parse_val("1vw").unwrap(), Val::Vw(1.));
    //     assert_eq!(StyleAttr::parse_val("1vh").unwrap(), Val::Vh(1.));
    //     assert_eq!(StyleAttr::parse_val("1.1px").unwrap(), Val::Px(1.1));

    //     assert!(StyleAttr::parse_val("1.1bad").is_err());
    //     assert!(StyleAttr::parse_val("bad").is_err());
    //     assert!(StyleAttr::parse_val("1.1.1bad").is_err());
    // }

    #[test]
    fn test_parse_attrs() {

        //         //     FlexWrap,
        //         // TODO: Allow shortcut forms for flex.
        //         b"flex" => StyleAttr::FlexGrow(StyleAttr::parse_f32(value)?),
        //         b"flex-grow" => StyleAttr::FlexGrow(StyleAttr::parse_f32(value)?),
        //         b"flex-shrink" => StyleAttr::FlexShrink(StyleAttr::parse_f32(value)?),
        //         b"flex-basis" => StyleAttr::FlexBasis(StyleAttr::parse_val(value)?),

        //         b"row-gap" => StyleAttr::RowGap(StyleAttr::parse_val(value)?),
        //         b"column-gap" => StyleAttr::ColumnGap(StyleAttr::parse_val(value)?),
        //         b"gap" => StyleAttr::Gap(StyleAttr::parse_val(value)?),

        //         b"grid-auto-flow" => StyleAttr::GridAutoFlow(match value {
        //             "row" => GridAutoFlow::Row,
        //             "column" => GridAutoFlow::Column,
        //             "row-dense" => GridAutoFlow::RowDense,
        //             "column-dense" => GridAutoFlow::ColumnDense,
        //             _ => {
        //                 return Err(GuiseError::UnknownAttributeValue(value.to_string()));
        //             }
        //         }),
        //         //     // TODO:
        //         //     // pub grid_template_rows: Option<Vec<RepeatedGridTrack>>,
        //         //     // pub grid_template_columns: Option<Vec<RepeatedGridTrack>>,
        //         //     // pub grid_auto_rows: Option<Vec<GridTrack>>,
        //         //     // pub grid_auto_columns: Option<Vec<GridTrack>>,
        //         b"grid-row" => StyleAttr::GridRow(StyleAttr::parse_grid_placement(value)?),
        //         b"grid-row-start" => StyleAttr::GridRowStart(StyleAttr::parse_i16(value)?),
        //         b"grid-row-span" => StyleAttr::GridRowSpan(StyleAttr::parse_u16(value)?),
        //         b"grid-row-end" => StyleAttr::GridRowEnd(StyleAttr::parse_i16(value)?),
        //         b"grid-column" => StyleAttr::GridColumn(StyleAttr::parse_grid_placement(value)?),
        //         b"grid-column-start" => StyleAttr::GridColumnStart(StyleAttr::parse_i16(value)?),
        //         b"grid-column-span" => StyleAttr::GridColumnSpan(StyleAttr::parse_u16(value)?),
        //         b"grid-column-end" => StyleAttr::GridColumnEnd(StyleAttr::parse_i16(value)?),
        //         _ => return Ok(None),
        //     }))
        // }
    }
}
