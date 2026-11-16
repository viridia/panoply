//! Scenery mode's current tier, the floor level its tools work on, and where the pointer meets the
//! tier's level plane, as faery's tools picked: never the ground, so a wall drawn on an upper floor
//! lands where the pointer is over that floor. Scenery mode's keys are here too.

use std::ops::RangeInclusive;

use bevy::{
    picking::{backend::ray::RayMap, pointer::PointerId},
    prelude::*,
};
use bevy_action_map::prelude::*;
use panoply_core::{PrimaryCamera, Viewpoint};
use panoply_scenery::{precinct::PrecinctKey, PrecinctCache, PRECINCT_SIZE_F, TIER_OFFSET};

use crate::{mode_selector::EditorMode, terrain_cursor::level_point};

/// The tiers the tools can choose, which the precincts' levels lie within.
pub const TIER_RANGE: RangeInclusive<i32> = -8..=8;

/// The floor level scenery tools place on, and the tile grid shows. Not saved, as faery's was not.
#[derive(Resource, Reflect, Clone, Copy, PartialEq, Eq, Debug, Default)]
#[reflect(Resource)]
pub struct SceneryTier(pub i32);

impl SceneryTier {
    /// The height of the tier's level plane: a scenery instance on the tier stands at it.
    pub fn height(self) -> f32 {
        self.0 as f32 + TIER_OFFSET
    }

    /// The tier `by` levels away, kept within [`TIER_RANGE`].
    fn step(self, by: i32) -> Self {
        Self((self.0 + by).clamp(*TIER_RANGE.start(), *TIER_RANGE.end()))
    }
}

/// Where the pointer meets the current tier's level plane in the view realm, and the precinct
/// there, if it is loaded.
#[derive(Resource, Clone, Copy, PartialEq, Debug, Default)]
pub struct SceneryPointer {
    /// The pointer's ray from the view's camera.
    pub ray: Option<Ray3d>,
    /// In the world's coordinates.
    pub point: Option<Vec3>,
    /// The precinct's entity and coordinates.
    pub precinct: Option<(Entity, IVec2)>,
}

/// The loaded precinct of `realm` at `point`, in the world's coordinates: its entity and
/// coordinates.
pub(crate) fn precinct_at(
    cache: &PrecinctCache,
    realm: Entity,
    point: Vec3,
) -> Option<(Entity, IVec2)> {
    let coords = (point.xz() / PRECINCT_SIZE_F).floor().as_ivec2();
    let key = PrecinctKey {
        realm,
        x: coords.x,
        z: coords.y,
    };
    Some((cache.peek(&key)?, coords))
}

/// Scenery mode's keys. A focused text field or a dialog holds the keys (`widget_keys`), so these
/// do not reach the tools from there.
#[derive(InputContext)]
#[context(path = "editor.scenery", tick = Render)]
pub struct SceneryKeys;

/// Turn what the tool places a quarter turn left.
#[derive(InputAction)]
#[action(path = "editor.scenery.rotate_left", output = bool, intent = Button)]
pub struct RotateLeft;

/// Turn what the tool places a quarter turn right.
#[derive(InputAction)]
#[action(path = "editor.scenery.rotate_right", output = bool, intent = Button)]
pub struct RotateRight;

/// Work on the tier below.
#[derive(InputAction)]
#[action(path = "editor.scenery.tier_down", output = bool, intent = Button)]
pub struct TierDown;

/// Work on the tier above.
#[derive(InputAction)]
#[action(path = "editor.scenery.tier_up", output = bool, intent = Button)]
pub struct TierUp;

/// Updates the [`SceneryPointer`], which the tools and the tile grid read.
#[derive(SystemSet, Clone, PartialEq, Eq, Hash, Debug)]
pub struct SceneryPointerSystems;

pub struct SceneryPointerPlugin;

impl Plugin for SceneryPointerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SceneryTier>()
            .init_resource::<SceneryPointer>()
            .register_type::<SceneryTier>()
            .add_context::<SceneryKeys>(|context| {
                context.active_in_state(EditorMode::Scenery);
                // As faery's: brackets turn, and comma and period, or Shift and the brackets,
                // change the tier. Shift makes a binding more specific, so it takes over.
                context.bind::<RotateLeft>(KeyCode::BracketLeft);
                context.bind::<RotateRight>(KeyCode::BracketRight);
                context.bind::<TierDown>(KeyCode::Comma);
                context.bind::<TierUp>(KeyCode::Period);
                context
                    .bind::<TierDown>(KeyCode::BracketLeft)
                    .with(ModifierKey::Shift);
                context
                    .bind::<TierUp>(KeyCode::BracketRight)
                    .with(ModifierKey::Shift);
            })
            .add_systems(Startup, |mut commands: Commands| {
                commands.spawn((Name::new("SceneryKeys"), SceneryKeys));
            })
            .add_systems(
                Update,
                (tier_keys, update_scenery_pointer)
                    .chain()
                    .in_set(SceneryPointerSystems)
                    .run_if(in_state(EditorMode::Scenery)),
            );
    }
}

/// Steps the tier by `by`; for the tier buttons.
pub(crate) fn step_tier(tier: &mut ResMut<SceneryTier>, by: i32) {
    let next = tier.step(by);
    tier.set_if_neq(next);
}

fn tier_keys(input: ContextActions<SceneryKeys>, mut tier: ResMut<SceneryTier>) {
    if input.fired::<TierDown>() {
        step_tier(&mut tier, -1);
    }
    if input.fired::<TierUp>() {
        step_tier(&mut tier, 1);
    }
}

fn update_scenery_pointer(
    tier: Res<SceneryTier>,
    viewpoint: Res<Viewpoint>,
    rays: Res<RayMap>,
    cameras: Query<(), With<PrimaryCamera>>,
    cache: Res<PrecinctCache>,
    mut pointer: ResMut<SceneryPointer>,
) {
    let ray = rays
        .iter()
        .find(|(id, _)| id.pointer == PointerId::Mouse && cameras.contains(id.camera))
        .map(|(_, &ray)| ray);
    let point = ray.and_then(|ray| level_point(ray, tier.height()));
    let precinct = point
        .zip(viewpoint.realm)
        .and_then(|(point, realm)| precinct_at(&cache, realm, point));
    pointer.set_if_neq(SceneryPointer {
        ray,
        point,
        precinct,
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_tier_stays_within_its_range() {
        assert_eq!(SceneryTier(0).step(1), SceneryTier(1));
        assert_eq!(SceneryTier(*TIER_RANGE.end()).step(1).0, *TIER_RANGE.end());
        assert_eq!(
            SceneryTier(*TIER_RANGE.start()).step(-1).0,
            *TIER_RANGE.start()
        );
    }
}
