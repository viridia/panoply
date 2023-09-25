use serde::{Deserialize, Serialize};

use crate::guise::view::TemplateOutput;

use super::TemplateNodeRef;

/// Dynamically-typed expression for parameters that can be passed to a template Call.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TemplateExpr {
    /// No value
    None,

    /// A boolean value
    Bool(bool),

    /// A number
    Number(f32),

    /// A quoted string
    String(String),

    /// An identifier
    Ident(String),

    /// A template node reference
    Node(TemplateNodeRef),

    /// A list of param values
    List(Box<[TemplateExpr]>),
}

impl TemplateExpr {
    pub fn render(&self, _context: &EvalContext) -> TemplateOutput {
        match self {
            TemplateExpr::None => TemplateOutput::Empty,
            TemplateExpr::Bool(b) => todo!("Render bool"),
            TemplateExpr::Number(_) => todo!("Render number"),
            TemplateExpr::String(_) => todo!("Render string"),
            TemplateExpr::Ident(_) => TemplateOutput::Empty,
            TemplateExpr::Node(_) => todo!("Render node"),
            TemplateExpr::List(_) => todo!("Render list"),
        }
    }
}

pub struct EvalContext {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_bool() {
        let des = serde_json::from_str::<TemplateExpr>(r#"false"#).unwrap();
        assert!(matches!(des, TemplateExpr::Bool(false)));
    }

    #[test]
    fn test_serialize_bool() {
        let ser = serde_json::to_string(&TemplateExpr::Bool(false));
        assert_eq!(ser.unwrap(), r#"false"#);
    }

    #[test]
    fn test_deserialize_number() {
        let des = serde_json::from_str::<TemplateExpr>(r#"1"#).unwrap();
        match des {
            TemplateExpr::Number(n) => {
                assert_eq!(n, 1.0);
            }
            _ => panic!("match failed"),
        }
    }

    #[test]
    fn test_serialize_number() {
        let ser = serde_json::to_string(&TemplateExpr::Number(1.0));
        assert_eq!(ser.unwrap(), r#"1.0"#);
    }

    #[test]
    fn test_deserialize_list() {
        let des = serde_json::from_str::<TemplateExpr>(r#"[false, true]"#).unwrap();
        assert!(matches!(des, TemplateExpr::List(_)));
    }

    #[test]
    fn test_serialize_list() {
        let ser = serde_json::to_string(&TemplateExpr::List(Box::from([
            TemplateExpr::Bool(true),
            TemplateExpr::Bool(false),
        ])));
        assert_eq!(ser.unwrap(), r#"[true,false]"#);
    }
}
