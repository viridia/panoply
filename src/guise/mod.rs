// mod asset;
mod asset;
mod coerce;
// mod controller;
// mod controllers;
mod computed;
mod element;
mod element_style;
mod expr;
mod from_ast;
mod parser;
mod path;
mod plugin;
// mod reconciler;
mod render_output;
mod renderable;
// mod style;
// mod template;
mod typed_expr;
mod view_element;
mod view_root;

// pub use controller::Controller;
pub use asset::GuiseAsset;
pub use element_style::ElementStyle;
pub use expr::Expr;
pub use plugin::*;
// pub use style::StyleAsset;
pub use render_output::RenderOutput;
pub use renderable::*;
// pub use view::ViewElement;
pub use view_root::ViewRoot;

// #[derive(Debug)]
// pub enum GuiseError {
//     PrematureEof,
//     InvalidElement(String),
//     MismatchedEnd(String),
//     UnknownAttribute(Vec<u8>),
//     UnknownAttributeValue(String),
//     InvalidAttributeValue(String),
//     MissingRequiredAttribute(String),
// }

// impl fmt::Display for GuiseError {
//     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//         match self {
//             GuiseError::PrematureEof => {
//                 write!(f, "Premature end of file")
//             }

//             GuiseError::InvalidElement(elt) => {
//                 write!(f, "Invalid element '{}'", elt)
//             }
//             GuiseError::MismatchedEnd(elt) => {
//                 write!(f, "Mismatched element end '{}'", elt)
//             }
//             GuiseError::UnknownAttribute(attr_name) => {
//                 write!(
//                     f,
//                     "Unknown attribute '{}'",
//                     std::str::from_utf8(&attr_name).unwrap()
//                 )
//             }
//             GuiseError::UnknownAttributeValue(attr_value) => {
//                 write!(f, "Unknown attribute value '{}'", attr_value)
//             }
//             GuiseError::InvalidAttributeValue(attr_value) => {
//                 write!(f, "Invalid attribute value '{}'", attr_value)
//             }
//             GuiseError::MissingRequiredAttribute(attr_value) => {
//                 write!(f, "Missing required attribute '{}'", attr_value)
//             }
//         }
//     }
// }

// impl std::error::Error for GuiseError {}
