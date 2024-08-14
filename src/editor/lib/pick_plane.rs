//! A backend for `bevy_mod_picking` that returns a hit on a horizontal plane.

#![allow(clippy::too_many_arguments, clippy::type_complexity)]
#![deny(missing_docs)]

use bevy::prelude::*;
use bevy_mod_picking::{
    backend::prelude::*,
    events::{Down, Drag, DragEnd, DragStart, Pointer},
    prelude::{ListenerMut, On},
};

use crate::{
    editor::ui::mode_scenery::SelectedTier,
    view::{
        picking::{PickAction, PickEvent, PickTarget},
        Viewpoint,
    },
};

/// Marks a camera that should be used in the backdrop picking backend.
/// Also marks the entity which is used as the backdrop.
#[derive(Debug, Clone, Default, Component, Reflect)]
#[reflect(Component, Default)]
pub struct PlanePick;

/// Adds the raycasting picking backend to your app.
#[derive(Clone)]
pub struct PlanePickBackend;
impl Plugin for PlanePickBackend {
    fn build(&self, app: &mut App) {
        app.add_systems(PreUpdate, update_hits.in_set(PickSet::Backend))
            .register_type::<PlanePick>();
        app.world_mut().commands().spawn((
            PlanePick,
            (
                On::<Pointer<Down>>::run(
                    move |mut ev: ListenerMut<Pointer<Down>>, mut commands: Commands| {
                        ev.stop_propagation();
                        commands.trigger(PickEvent {
                            action: PickAction::Down(ev.hit.position.unwrap()),
                            target: PickTarget::PickPlane,
                        });
                    },
                ),
                On::<Pointer<DragStart>>::run(
                    move |mut ev: ListenerMut<Pointer<DragStart>>,
                          mut commands: Commands,
                          view: Res<Viewpoint>| {
                        ev.stop_propagation();
                        commands.trigger(PickEvent {
                            action: PickAction::DragStart {
                                realm: view.realm.unwrap(),
                                pos: ev.hit.position.unwrap(),
                            },
                            target: PickTarget::PickPlane,
                        });
                    },
                ),
                On::<Pointer<Drag>>::run(
                    move |mut ev: ListenerMut<Pointer<Drag>>, mut commands: Commands| {
                        ev.stop_propagation();
                        commands.trigger(PickEvent {
                            action: PickAction::Drag,
                            target: PickTarget::PickPlane,
                        });
                    },
                ),
                On::<Pointer<DragEnd>>::run(
                    move |mut ev: ListenerMut<Pointer<DragEnd>>, mut commands: Commands| {
                        ev.stop_propagation();
                        commands.trigger(PickEvent {
                            action: PickAction::DragEnd,
                            target: PickTarget::PickPlane,
                        });
                    },
                ),
            ),
        ));
    }
}

/// Returns a hit on the camera backdrop.
pub fn update_hits(
    ray_map: Res<RayMap>,
    picking_cameras: Query<&Camera, With<PlanePick>>,
    picking_backdrop: Query<(Entity, &PlanePick), Without<Camera>>,
    selected_tier: Res<SelectedTier>,
    mut output_events: EventWriter<PointerHits>,
) {
    let backdrop = picking_backdrop.get_single().unwrap();

    for (&ray_id, &ray) in ray_map.map().iter() {
        let Ok(camera) = picking_cameras.get(ray_id.camera) else {
            continue;
        };

        let plane = InfinitePlane3d::new(Vec3::new(0.0, 1.0, 0.0));
        let Some(intersect) =
            ray.intersect_plane(Vec3::new(0.0, selected_tier.0 as f32, 0.0), plane)
        else {
            return;
        };

        let hit_data = HitData::new(
            ray_id.camera,
            intersect,
            Some(ray.get_point(intersect)),
            Some(plane.normal.as_vec3()),
        );
        let picks = Vec::from([(backdrop.0, hit_data)]);
        let order = (camera.order + 1) as f32;
        output_events.send(PointerHits::new(ray_id.pointer, picks, order));
    }
}
