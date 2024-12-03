#![allow(dead_code)]
mod noise;
mod weighted_choice;

pub use noise::*;
pub use weighted_choice::*;

pub trait Choice {
    fn probability(&self) -> f32;
}
