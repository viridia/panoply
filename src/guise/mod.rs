mod asset;
mod asset2;
mod controller;
mod controllers;
mod path;
mod plugin;
mod style;
mod template;
mod view;

use std::fmt;

pub use controller::Controller;
pub use plugin::*;
pub use view::ViewElement;
pub use view::ViewRoot;

#[derive(Debug)]
pub enum GuiseError {
    PrematureEof,
    InvalidElement(String),
    MismatchedEnd(String),
    UnknownAttribute(Vec<u8>),
    UnknownAttributeValue(String),
    InvalidAttributeValue(String),
    MissingRequiredAttribute(String),
}

impl fmt::Display for GuiseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GuiseError::PrematureEof => {
                write!(f, "Premature end of file")
            }

            GuiseError::InvalidElement(elt) => {
                write!(f, "Invalid element '{}'", elt)
            }
            GuiseError::MismatchedEnd(elt) => {
                write!(f, "Mismatched element end '{}'", elt)
            }
            GuiseError::UnknownAttribute(attr_name) => {
                write!(
                    f,
                    "Unknown attribute '{}'",
                    std::str::from_utf8(&attr_name).unwrap()
                )
            }
            GuiseError::UnknownAttributeValue(attr_value) => {
                write!(f, "Unknown attribute value '{}'", attr_value)
            }
            GuiseError::InvalidAttributeValue(attr_value) => {
                write!(f, "Invalid attribute value '{}'", attr_value)
            }
            GuiseError::MissingRequiredAttribute(attr_value) => {
                write!(f, "Missing required attribute '{}'", attr_value)
            }
        }
    }
}

impl std::error::Error for GuiseError {}
