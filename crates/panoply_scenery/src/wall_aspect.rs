use bevy::{annotations::DisplayName, prelude::*};
use panoply_exemplar::{annotations::AspectIcon, *};

/// Wall Size aspect
#[derive(Component, Debug, Reflect, Clone, Default)]
#[reflect(Aspect, Default)]
#[reflect(@AspectIcon("editor/icons/lucide/ruler.png"))]
pub struct WallSize {
    /// How many tile spaces does this wall take up in the x direction
    #[reflect(@DisplayName("Width"))]
    x: u32,

    /// How many tile spaces does this wall take up in the y (actually z) direction
    #[reflect(@DisplayName("Depth"))]
    y: u32,
}

impl NativeAspect for WallSize {
    type Config = Self;

    fn config(&self) -> Self {
        self.clone()
    }

    fn can_attach(&self, kind: InstanceKind) -> bool {
        kind == InstanceKind::Wall
    }
}
