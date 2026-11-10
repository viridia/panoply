//! Terrain mode's selected parcel, which its tools edit: a click on the ground selects the parcel
//! there, if the realm's map holds it. The selection is saved, by realm name and parcel coordinates
//! rather than the parcel's entity, which the parcel cache spawns and despawns as the view moves; it
//! counts only while the view is in its realm.
//!
//! While editing, the ground is picked by its colliders, as play picks (D61), but only the ground:
//! the hit is reported on the parcel, with the point hit. The water's surface is picked too, by ray
//! math, so that a parcel with no ground, or ground far below the water, is picked where it is seen.

use bevy::{
    ecs::system::SystemParam,
    picking::{
        backend::{ray::RayMap, HitData, PointerHits},
        events::PointerClick,
        pointer::PointerButton,
        PickingSystems,
    },
    prelude::*,
    settings::{ReflectSettingsGroup, SaveSettingsDeferred, SettingsGroup},
};
use panoply_core::{
    realm_physics::collision_groups, GameMode, PrimaryCamera, Realm, RealmPhysics, Viewpoint,
};
use panoply_terrain::{
    Parcel, ParcelCache, ParcelKey, TerrainMap, TerrainMapAsset, PARCEL_SIZE_F, WATER_HEIGHT,
};
use rapier3d::prelude::{Group, InteractionGroups, InteractionTestMode, QueryFilter, Ray};

use crate::{mode_selector::EditorMode, shell::SAVE_DELAY, terrain_map_edit::TerrainMapEdits};

/// How far from the camera a pointer's ray can hit the ground, in meters.
const PICK_RANGE: f32 = 500.;

/// The parcel selected in terrain mode, and its realm.
#[derive(Resource, SettingsGroup, Reflect, Clone, PartialEq, Debug, Default)]
#[reflect(Resource, SettingsGroup, Default)]
#[settings_group(group = "parcel_selection")]
pub struct ParcelSelection {
    pub realm: Option<String>,
    pub parcel: Option<IVec2>,
}

impl ParcelSelection {
    /// The selected parcel, if it is in `realm`.
    pub(crate) fn in_realm(&self, realm: &Realm) -> Option<IVec2> {
        self.parcel
            .filter(|_| self.realm.as_deref() == Some(realm.name.as_str()))
    }
}

pub struct ParcelSelectionPlugin;

impl Plugin for ParcelSelectionPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PreUpdate,
            pick_ground
                .in_set(PickingSystems::Backend)
                .run_if(in_state(GameMode::Edit)),
        )
        .add_systems(OnEnter(EditorMode::Terrain), |mut commands: Commands| {
            commands.spawn((
                Name::new("SelectParcel"),
                Observer::new(select_parcel),
                DespawnOnExit(EditorMode::Terrain),
            ));
        });
    }
}

/// A picking backend while editing. Casts each pointer's ray at the view realm's ground and at the
/// water's surface, and reports the parcel under whichever it meets first, with the point.
/// Scenery does not stop the ray, so ground behind a wall can be picked.
fn pick_ground(
    rays: Res<RayMap>,
    cameras: Query<&Camera, With<PrimaryCamera>>,
    viewpoint: Res<Viewpoint>,
    realms: Query<&RealmPhysics>,
    cache: Res<ParcelCache>,
    mut hits: MessageWriter<PointerHits>,
) {
    let Some(realm) = viewpoint.realm else {
        return;
    };
    let Ok(physics) = realms.get(realm) else {
        return;
    };
    let world = physics.world();
    let filter = QueryFilter::default().groups(InteractionGroups::new(
        Group::ALL,
        collision_groups::TERRAIN,
        InteractionTestMode::And,
    ));
    for (&id, ray) in rays.iter() {
        let Ok(camera) = cameras.get(id.camera) else {
            continue;
        };
        let ground = world
            .cast_ray(&Ray::new(ray.origin, *ray.direction), PICK_RANGE, true, filter)
            .and_then(|(handle, distance)| {
                let collider = world.colliders.get(handle)?;
                Some((Entity::try_from_bits(collider.user_data as u64)?, distance))
            });
        let water = water_distance(ray.origin, *ray.direction)
            .filter(|&distance| distance <= PICK_RANGE)
            .and_then(|distance| {
                let point = ray.get_point(distance);
                let coords = (point.xz() / PARCEL_SIZE_F).floor().as_ivec2();
                let key = ParcelKey {
                    realm,
                    x: coords.x,
                    z: coords.y,
                };
                Some((cache.peek(&key)?, distance))
            });
        let nearest = match (ground, water) {
            (Some(g), Some(w)) => Some(if w.1 < g.1 { w } else { g }),
            (g, w) => g.or(w),
        };
        let Some((parcel, distance)) = nearest else {
            continue;
        };
        let hit = HitData::new(id.camera, distance, Some(ray.get_point(distance)), None);
        hits.write(PointerHits::new(
            id.pointer,
            vec![(parcel, hit)],
            camera.order as f32,
        ));
    }
}

/// How far along a ray from `origin` in `direction` it meets the water's surface, from above.
fn water_distance(origin: Vec3, direction: Vec3) -> Option<f32> {
    let height = origin.y - WATER_HEIGHT;
    (height > 0. && direction.y < 0.).then(|| height / -direction.y)
}

/// Observer while in terrain mode: a click on a parcel selects it, or clears the selection if the
/// parcel is off the map, where there is nothing to edit.
fn select_parcel(
    click: On<PointerClick>,
    parcels: Query<&Parcel>,
    realms: Query<(&Realm, &TerrainMap)>,
    maps: Res<Assets<TerrainMapAsset>>,
    mut selection: ResMut<ParcelSelection>,
    mut commands: Commands,
) {
    if click.button != PointerButton::Primary {
        return;
    }
    let Ok(parcel) = parcels.get(click.event_target()) else {
        return;
    };
    let Ok((realm, terrain)) = realms.get(parcel.realm) else {
        return;
    };
    let on_map = maps
        .get(&terrain.handle)
        .is_some_and(|map| map.contains_pt(parcel.coords));
    let chosen = ParcelSelection {
        realm: Some(realm.name.clone()),
        parcel: on_map.then_some(parcel.coords),
    };
    if selection.set_if_neq(chosen) {
        commands.queue(SaveSettingsDeferred(SAVE_DELAY));
    }
}

/// The selected parcel in the view realm, and the edits made to it.
#[derive(SystemParam)]
pub(crate) struct SelectedParcelEdits<'w, 's> {
    selection: Res<'w, ParcelSelection>,
    viewpoint: Res<'w, Viewpoint>,
    realms: Query<'w, 's, &'static Realm>,
    edits: TerrainMapEdits<'w, 's>,
}

impl SelectedParcelEdits<'_, '_> {
    /// The selected parcel, if it is in the view realm and on its map.
    pub(crate) fn parcel(&self) -> Option<IVec2> {
        let realm = self.realms.get(self.viewpoint.realm?).ok()?;
        let parcel = self.selection.in_realm(realm)?;
        self.edits.map()?.contains_pt(parcel).then_some(parcel)
    }

    /// Turns the selected parcel's contour by `turns` quarter turns.
    pub(crate) fn rotate(&mut self, turns: u8) {
        let Some(parcel) = self.parcel() else {
            return;
        };
        self.edits.edit("Rotate contour", |map| {
            let mut shape = map.shape_at(parcel);
            shape.rotation = (shape.rotation + turns) % 4;
            map.set_shape_at(parcel, shape);
            true
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_ray_down_meets_the_water_below_it() {
        let origin = Vec3::new(0., 10. + WATER_HEIGHT, 0.);
        let down = Vec3::new(0.6, -0.8, 0.);
        let distance = water_distance(origin, down).unwrap();
        assert!((distance - 12.5).abs() < 1e-4, "{distance}");
    }

    #[test]
    fn a_ray_up_or_from_below_misses_the_water() {
        let above = Vec3::new(0., 5., 0.);
        let below = Vec3::new(0., WATER_HEIGHT - 1., 0.);
        assert_eq!(water_distance(above, Vec3::Y), None);
        assert_eq!(water_distance(below, Vec3::NEG_Y), None);
    }
}
