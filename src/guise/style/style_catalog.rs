use bevy::{
    reflect::{TypePath, TypeUuid},
    utils::HashMap,
};
use serde_derive::{Deserialize, Serialize};

use super::style::Style;

#[derive(Debug, Default, Clone, Serialize, Deserialize, TypeUuid, TypePath)]
#[uuid = "4af40b07-f427-46f5-bdb2-f4b6f6c8ccef"]
pub struct StyleCatalog {
    #[serde(skip_serializing_if = "Option::is_none")]
    extends: Option<String>,

    #[serde(flatten)]
    styles: HashMap<String, Style>,
}

impl StyleCatalog {
    pub fn new() -> Self {
        Self {
            extends: None,
            styles: HashMap::new(),
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            extends: None,
            styles: HashMap::with_capacity(capacity),
        }
    }

    pub fn len(&self) -> usize {
        self.styles.len()
    }
}

#[cfg(test)]
mod tests {
    use crate::guise::style::{expr::Expr, StyleAttr};

    use super::*;

    #[test]
    fn test_serialize_style_catalog() {
        let style = Style::from_attrs(&[
            StyleAttr::ZIndex(Expr::Number(7.)),
            StyleAttr::FlexGrow(Expr::Number(2.)),
            StyleAttr::FlexShrink(Expr::Number(3.)),
        ]);
        let mut catalog = StyleCatalog::new();
        catalog.styles.insert("base".into(), style);
        let ser = serde_json::to_string(&catalog);
        assert_eq!(
            ser.unwrap(),
            r#"{"base":{"z-index":7,"flex-grow":2,"flex-shrink":3}}"#
        );
    }

    #[test]
    fn test_deserialize_style_catalog() {
        let des = serde_json::from_str::<StyleCatalog>(
            r#"{"base":{"z-index":7,"flex-grow":2,"flex-shrink":3}}"#,
        )
        .unwrap();
        assert_eq!(des.len(), 1);
        let base = des.styles.get("base").unwrap();
        assert_eq!(base.len_attrs(), 3);
    }
}
