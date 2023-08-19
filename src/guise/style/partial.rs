use crate::guise::GuiseError;
use lazy_static::lazy_static;
use regex::Regex;

use super::attr::StyleAttr;
use super::ComputedStyle;
use bevy::reflect::{TypePath, TypeUuid};
use quick_xml::writer::Writer;
use quick_xml::{
    events::{BytesStart, Event},
    name::QName,
};

const ATTR_ID: QName = QName(b"id");

/// Selector expressions
#[derive(Debug, Default, Clone)]
pub enum Selector {
    /// Selector that is always true.
    #[default]
    Always,

    /// A dynamic class name such as "hover" or "selected"
    ClassName(String),

    /// Conjunction of several selectors
    Conjunction(Box<[Selector]>),

    /// Negation of a selector
    Negation(Box<Selector>),
    // TODO:
    // Child
    // Descendant
}

impl Selector {
    /// Parse a selector expression
    pub fn parse(input: &str) -> Result<Selector, GuiseError> {
        // TODO: Do a real parser later. For now, just support `self.<classname>`.
        lazy_static! {
            static ref RE_SEL: Regex = Regex::new(r"^self\.(\w+)$").unwrap();
        }

        RE_SEL
            .captures(input)
            .and_then(|cap| Some(Selector::ClassName(cap[1].to_string())))
            .ok_or(GuiseError::InvalidAttributeValue(input.to_string()))
    }

    /// Tests whether the selector expression matches the array of input names.
    pub fn test(&self, input: &[&str]) -> bool {
        Selector::test_expr(self, input)
    }

    fn test_expr(expr: &Self, input: &[&str]) -> bool {
        match expr {
            Selector::Always => true,
            Selector::ClassName(cls) => input.iter().any(|n| *n == cls),
            Selector::Conjunction(terms) => terms.iter().all(|t| Selector::test_expr(t, input)),
            Selector::Negation(expr) => !Selector::test_expr(expr, input),
        }
    }
}

/// A collection of style properties which can be merged to create a `Style`.
/// Rather than storing the attributes in a struct full of optional fields, we store a flat
/// vector of enums, each of which stores a single style attribute. This "sparse" representation
/// allows for fast (O(N) where N is the number of defined attributes) merging of styles,
/// particularly for styles which have few or no attributes.
#[derive(Debug, TypeUuid, TypePath, Default, Clone)]
#[uuid = "7d753986-2d0b-4e22-9ef3-166ffafa989e"]
pub struct PartialStyle {
    attrs: Vec<StyleAttr>,

    /// Optional child stylesheets which represent additional styles that can be enabled via
    /// a selector expression.
    selectors: Vec<(Selector, PartialStyle)>,
}

impl PartialStyle {
    pub const EMPTY: &'static PartialStyle = &PartialStyle::new();

    /// Construct a new, empty `PartialStyle`.
    pub const fn new() -> Self {
        Self {
            attrs: Vec::new(),
            selectors: Vec::new(),
        }
    }

    /// Construct a new, empty `PartialStyle` with capacity `size`.
    pub fn with_capacity(size: usize) -> Self {
        Self {
            attrs: Vec::with_capacity(size),
            selectors: Vec::new(),
        }
    }

    /// Construct a new `PartialStyle` from a list of `StyleAttr`s.
    pub fn from_attrs(attrs: &[StyleAttr]) -> Self {
        Self {
            attrs: Vec::from(attrs),
            selectors: Vec::new(),
        }
    }

    pub fn add_selector(&mut self, selector: Selector, style: PartialStyle) {
        self.selectors.push((selector, style));
    }

    /// True if there are no styles defined.
    pub fn is_empty(&self) -> bool {
        return self.attrs.is_empty();
    }

    /// Merge the style properties into a computed `Style` object.
    pub fn apply_to(&self, computed: &mut ComputedStyle) {
        for attr in self.attrs.iter() {
            attr.apply(computed);
        }
    }

    /// Merge the style properties into a computed `Style` object.
    pub fn apply_selected_to(&self, computed: &mut ComputedStyle, class_names: &[&str]) {
        for (selector, ss) in self.selectors.iter() {
            if selector.test(class_names) {
                for attr in ss.attrs.iter() {
                    attr.apply(computed);
                }
            }
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

impl PartialEq<PartialStyle> for PartialStyle {
    fn eq(&self, other: &PartialStyle) -> bool {
        self.attrs.len() == other.attrs.len()
            && self
                .attrs
                .iter()
                .enumerate()
                .all(|(i, attr)| *attr == other.attrs[i])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let style = PartialStyle::from_attrs(&[StyleAttr::Display(bevy::ui::Display::Flex)]);
        let mut writer = Writer::new(std::io::Cursor::new(Vec::new()));
        style.write_xml(&mut writer);
        assert_eq!(
            String::from_utf8(writer.into_inner().into_inner()).unwrap(),
            r#"<style display="flex"/>"#
        );
    }
}
