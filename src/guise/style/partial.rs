use super::attr::StyleAttr;
use super::ComputedStyle;
use bevy::reflect::{TypePath, TypeUuid};
use quick_xml::writer::Writer;
use quick_xml::{
    events::{BytesStart, Event},
    name::QName,
};

const ATTR_ID: QName = QName(b"id");

/// A collection of style properties which can be merged to create a `Style`.
/// Rather than storing the attributes in a struct full of optional fields, we store a flat
/// vector of enums, each of which stores a single style attribute. This "sparse" representation
/// allows for fast (O(N) where N is the number of defined attributes) merging of styles,
/// particularly for styles which have few or no attributes.
#[derive(Debug, TypeUuid, TypePath, Default, Clone)]
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

#[cfg(test)]
mod tests {
    use bevy::ui::{UiRect, Val};

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
        let style = PartialStyle::from_attrs(&[StyleAttr::Display(bevy::ui::Display::Flex)]);
        let mut writer = Writer::new(std::io::Cursor::new(Vec::new()));
        style.write_xml(&mut writer);
        assert_eq!(
            String::from_utf8(writer.into_inner().into_inner()).unwrap(),
            r#"<style display="flex"/>"#
        );
    }
}
