//! The Show menu's Portals: each portal's aperture in the viewpoint's realm, outlined, with an
//! arrow for each side it is open from, pointing the way it is looked through; and, for a portal
//! into the same realm, its far aperture. Bright while its far end is drawn (`PortalView`).

use bevy::{camera::visibility::RenderLayers, color::palettes::css, prelude::*};
use panoply_core::{Realm, Viewpoint, PORTAL_APERTURE_LAYER};
use panoply_portals::{aperture_transform, Portal, PortalSide, PortalTarget, PortalView};

use crate::debug_views::DebugViews;

/// Gizmos for portals, on the apertures' layer, which only the primary camera renders, so they
/// stay out of the portal cameras' views.
#[derive(GizmoConfigGroup, Reflect, Default)]
struct PortalGizmos;

pub struct PortalGizmosPlugin;

impl Plugin for PortalGizmosPlugin {
    fn build(&self, app: &mut App) {
        app.insert_gizmo_config(
            PortalGizmos,
            GizmoConfig {
                render_layers: RenderLayers::layer(PORTAL_APERTURE_LAYER),
                // Over everything: an aperture is usually larger than its doorway, whose frame
                // would hide the outline, and it is judged against that frame.
                depth_bias: -1.,
                ..default()
            },
        )
        .add_systems(Update, draw_portals);
    }
}

fn draw_portals(
    views: Res<DebugViews>,
    viewpoint: Res<Viewpoint>,
    q_realms: Query<&Realm>,
    q_portals: Query<(
        &Portal,
        &PortalTarget,
        &GlobalTransform,
        &RenderLayers,
        Has<PortalView>,
    )>,
    mut gizmos: Gizmos<PortalGizmos>,
) {
    if !views.portals {
        return;
    }
    let Some(realm) = viewpoint.realm.and_then(|realm| q_realms.get(realm).ok()) else {
        return;
    };
    for (portal, target, element, layers, in_view) in q_portals.iter() {
        if !layers.intersects(&realm.layer) {
            continue;
        }
        let aperture = aperture_transform(portal, element);
        let (outline, front, back) = if in_view {
            (css::GOLD, css::GOLD, css::AQUAMARINE)
        } else {
            (css::GRAY, css::DIM_GRAY, css::DIM_GRAY)
        };
        let center = aperture.translation;
        gizmos.rect(
            Isometry3d::new(center, aperture.rotation),
            portal.size,
            outline,
        );
        let normal = aperture.rotation * Vec3::Z;
        if portal.side != PortalSide::Back {
            gizmos.arrow(center, center + normal, front);
        }
        if portal.side != PortalSide::Front {
            gizmos.arrow(center, center - normal, back);
        }
        if target.realm == realm.name {
            let far = target.pos + Vec3::Y * portal.size.y * 0.5;
            gizmos.rect(
                Isometry3d::new(far, aperture.rotation),
                portal.size,
                css::ORCHID,
            );
        }
    }
}
