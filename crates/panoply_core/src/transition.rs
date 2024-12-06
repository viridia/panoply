use bevy::{
    math::{Curve, VectorSpace},
    prelude::{EaseFunction, EasingCurve},
};

/// An animated transition between two values.
#[derive(Debug, Clone)]
pub struct Transition<T> {
    start: T,
    end: T,
    duration: f32,
    elapsed: f32,
    ease: EaseFunction,
}

impl<T: Clone + Copy + VectorSpace> Transition<T> {
    pub fn new(start: T, end: T, duration: f32, ease: EaseFunction) -> Self {
        Self {
            start,
            end,
            duration,
            elapsed: 0.,
            ease,
        }
    }

    pub fn advance(&mut self, delta: f32) {
        self.elapsed += delta;
    }

    pub fn current(&self) -> T {
        let t = (self.elapsed / self.duration).min(1.);
        let e = EasingCurve::new(0.0, 1.0, self.ease);
        let value = e.sample(t).unwrap();
        self.start.lerp(self.end, value)
    }

    pub fn is_finished(&self) -> bool {
        self.elapsed >= self.duration
    }
}
