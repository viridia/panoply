mod asset;
mod controller;
mod controllers;
mod plugin;
mod style;
mod template;
mod view;

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
