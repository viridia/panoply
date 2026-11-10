//! Conversions between Bevy's math types and rapier's, which come from different versions of glam:
//! Bevy's 0.34 and rapier's 0.33, through `glamx`. Each copies the components; the layouts match.
//! TODO(X83): delete once rapier is on Bevy's glam.

use bevy::math::{Quat, Vec2, Vec3};
use rapier3d::math as rapier;

/// A Bevy math value as rapier's.
pub trait ToRapier {
    type Rapier;

    fn to_rapier(self) -> Self::Rapier;
}

/// A rapier math value as Bevy's.
pub trait ToBevy {
    type Bevy;

    fn to_bevy(self) -> Self::Bevy;
}

impl ToRapier for Vec3 {
    type Rapier = rapier::Vec3;

    fn to_rapier(self) -> rapier::Vec3 {
        rapier::Vec3::from_array(self.to_array())
    }
}

impl ToBevy for rapier::Vec3 {
    type Bevy = Vec3;

    fn to_bevy(self) -> Vec3 {
        Vec3::from_array(self.to_array())
    }
}

impl ToRapier for Vec2 {
    type Rapier = rapier::Vec2;

    fn to_rapier(self) -> rapier::Vec2 {
        rapier::Vec2::from_array(self.to_array())
    }
}

impl ToBevy for rapier::Vec2 {
    type Bevy = Vec2;

    fn to_bevy(self) -> Vec2 {
        Vec2::from_array(self.to_array())
    }
}

impl ToRapier for Quat {
    type Rapier = rapier::Rotation;

    fn to_rapier(self) -> rapier::Rotation {
        rapier::Rotation::from_array(self.to_array())
    }
}

impl ToBevy for rapier::Rotation {
    type Bevy = Quat;

    fn to_bevy(self) -> Quat {
        Quat::from_array(self.to_array())
    }
}
