mod asset;
mod controller;
mod controllers;
mod partial_style;
mod plugin;
mod style;
mod template;
mod view;

pub use plugin::*;

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
