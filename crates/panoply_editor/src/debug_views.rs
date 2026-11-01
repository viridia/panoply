//! Debug views: rapier's debug render of the viewpoint realm's colliders, drawn with gizmos.

use std::time::Duration;

use bevy::{
    camera::visibility::RenderLayers,
    platform::collections::HashSet,
    prelude::*,
    settings::{ReflectSettingsGroup, SaveSettingsDeferred, SettingsGroup},
};
use bevy_action_map::prelude::*;
use panoply_core::{realm_physics::collision_groups, RealmPhysics, Viewpoint};
use panoply_terrain::Parcel;
use rapier3d::{
    pipeline::{DebugColor, DebugRenderBackend, DebugRenderMode, DebugRenderObject},
    prelude::{DebugRenderPipeline, DebugRenderStyle, Group, QueryFilter, Ray},
};

/// Which debug views are drawn, saved between runs.
#[derive(Resource, SettingsGroup, Reflect, Clone, PartialEq, Debug)]
#[reflect(Resource, SettingsGroup, Default)]
#[settings_group(group = "debug_views")]
pub struct DebugViews {
    /// Draw physics colliders.
    pub physics: bool,
    /// Collision groups to draw (`panoply_core::realm_physics::collision_groups`) as bits.
    pub physics_groups: u32,
}

impl Default for DebugViews {
    fn default() -> Self {
        Self {
            physics: false,
            physics_groups: Group::ALL.bits(),
        }
    }
}

/// What a ray cast straight down at the view center hits, while physics drawing is on; `None`
/// if nothing is under it. Drawn as a marker, and read by `plans/physics.py`.
#[derive(Resource, Reflect, Default, Debug)]
#[reflect(Resource)]
pub struct GroundProbe {
    pub hit: Option<Vec3>,
    /// Collision groups (`collision_groups`) the hit collider belongs to, as bits; 0 for no hit.
    pub groups: u32,
}

/// Gizmos for physics drawing, on every realm's layer.
#[derive(GizmoConfigGroup, Reflect, Default)]
struct PhysicsGizmos;

/// Toggle physics drawing.
#[derive(InputAction)]
#[action(path = "debug_views.toggle_physics", output = bool, intent = Button)]
pub struct TogglePhysics;

/// Debug view shortcuts.
#[derive(InputContext)]
#[context(path = "debug_views", tick = Render)]
pub struct DebugViewControl;

#[derive(Resource, Deref, DerefMut)]
struct PhysicsDebugRender(DebugRenderPipeline);

pub struct DebugViewsPlugin;

impl Plugin for DebugViewsPlugin {
    fn build(&self, app: &mut App) {
        app.insert_gizmo_config(
            PhysicsGizmos,
            GizmoConfig {
                // Lines lie on the ground they outline, so pull them forward to keep them visible.
                depth_bias: -0.001,
                // Realm layers only, which start at 1 (`ReservedLayers`): layer 0 is the HUD
                // camera's, which would draw the lines again through its 2D projection.
                render_layers: RenderLayers::from_layers(&(1..16).collect::<Vec<_>>()),
                ..default()
            },
        )
        .init_resource::<GroundProbe>()
        .insert_resource(PhysicsDebugRender(DebugRenderPipeline::new(
            DebugRenderStyle::default(),
            DebugRenderMode::COLLIDER_SHAPES,
        )))
        .add_context::<DebugViewControl>(|context| {
            context.bind::<TogglePhysics>(KeyCode::F3);
        })
        .add_systems(Startup, |mut commands: Commands| {
            commands.spawn((Name::new("DebugViewControl"), DebugViewControl));
        })
        .add_systems(Update, (toggle_physics, draw_physics).chain());
    }
}

fn toggle_physics(
    input: ContextActions<DebugViewControl>,
    mut views: ResMut<DebugViews>,
    mut commands: Commands,
) {
    if input.fired::<TogglePhysics>() {
        views.physics = !views.physics;
        commands.queue(SaveSettingsDeferred(Duration::from_secs_f32(0.5)));
    }
}

/// Draws the colliders of the viewpoint's realm: ground only for shown parcels (D20), since all
/// of it would be millions of lines, and every scenery collider.
fn draw_physics(
    views: Res<DebugViews>,
    viewpoint: Res<Viewpoint>,
    q_realms: Query<&RealmPhysics>,
    q_parcels: Query<(Entity, &Visibility), With<Parcel>>,
    mut pipeline: ResMut<PhysicsDebugRender>,
    mut probe: ResMut<GroundProbe>,
    mut gizmos: Gizmos<PhysicsGizmos>,
) {
    let physics = views
        .physics
        .then(|| viewpoint.realm.and_then(|realm| q_realms.get(realm).ok()))
        .flatten();
    let Some(physics) = physics else {
        *probe = GroundProbe::default();
        return;
    };

    let shown_parcels = q_parcels
        .iter()
        .filter(|(_, visibility)| **visibility != Visibility::Hidden)
        .map(|(entity, _)| entity.to_bits() as u128)
        .collect();
    let mut backend = GizmoBackend {
        gizmos: &mut gizmos,
        groups: Group::from_bits_truncate(views.physics_groups),
        shown_parcels,
    };
    physics.world().debug_render(&mut pipeline, &mut backend);

    let origin = viewpoint.position + Vec3::Y * 1000.;
    let ray = Ray::new(origin, Vec3::NEG_Y);
    let hit = physics
        .world()
        .cast_ray(&ray, 2000., true, QueryFilter::default());
    probe.hit = hit.map(|(_, distance)| ray.point_at(distance));
    probe.groups = hit
        .and_then(|(handle, _)| physics.world().colliders.get(handle))
        .map_or(0, |collider| collider.collision_groups().memberships.bits());
    if let Some(hit) = probe.hit {
        gizmos.sphere(Isometry3d::from_translation(hit), 0.25, Color::WHITE);
    }
}

struct GizmoBackend<'a, 'w, 's> {
    gizmos: &'a mut Gizmos<'w, 's, PhysicsGizmos>,
    groups: Group,
    /// Entity bits of shown parcels, matched against each collider's `user_data`.
    shown_parcels: HashSet<u128>,
}

impl DebugRenderBackend for GizmoBackend<'_, '_, '_> {
    fn filter_object(&self, object: DebugRenderObject) -> bool {
        match object {
            DebugRenderObject::Collider(_, collider) => {
                let memberships = collider.collision_groups().memberships;
                // Scenery colliders are a few lines each, so all of them are drawn.
                memberships.intersects(self.groups)
                    && (!memberships.contains(collision_groups::TERRAIN)
                        || self.shown_parcels.contains(&collider.user_data))
            }
            _ => true,
        }
    }

    fn draw_line(&mut self, _object: DebugRenderObject, a: Vec3, b: Vec3, color: DebugColor) {
        self.gizmos
            .line(a, b, Color::hsla(color[0], color[1], color[2], color[3]));
    }
}
