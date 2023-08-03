#![allow(dead_code)]
mod noise;
mod weighted_random;

pub use noise::*;
pub use weighted_random::*;

pub trait Choice {
    fn probability(&self) -> f32;
}
