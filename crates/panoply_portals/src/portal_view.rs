//! What a portal's aperture shows: its far end, rendered by a camera of its own (D21).
//!
//! As faery did, a portal only translates: the portal camera is the primary camera moved by the
//! distance from the near aperture to the far one, keeping its rotation and lens. It renders only
//! the aperture's rectangle on screen ([`PortalProjection`]), with everything nearer than the far
//! aperture clipped, into the corner of an image the size of the primary camera's viewport. The
//! aperture, a rectangle on [`PORTAL_APERTURE_LAYER`], shows that image texel for pixel.

use bevy::{
    camera::{visibility::RenderLayers, RenderTarget, Viewport},
    ecs::{lifecycle::HookContext, world::DeferredWorld},
    light::{NotShadowCaster, NotShadowReceiver, ShadowFilteringMethod},
    picking::Pickable,
    prelude::*,
    render::{render_resource::TextureFormat, view::Tonemapping},
};
use panoply_core::{
    PortalViewRegion, PortalViewRegions, PrimaryCamera, Realm, Viewpoint, PORTAL_APERTURE_LAYER,
    PORTAL_VIEW_HALF_SIZE,
};
use panoply_scenery::scenery_element::SceneryElement;

use crate::{
    aperture_material::ApertureMaterial, portal_projection::PortalProjection, Portal, PortalSide,
    PortalTarget,
};

/// Portals whose near aperture is this close to the viewpoint, on x and z, have their far end
/// loaded, so that it is there when the aperture comes into view. The precincts beyond the
/// viewpoint's load radius (about 28 m) are not loaded, so neither are their portals.
const PORTAL_PRELOAD_DISTANCE: f32 = 24.;

/// Closer to the far aperture's plane than this, the portal camera keeps its near plane rather
/// than clipping at the aperture, which would be degenerate.
const MIN_CLIP_DISTANCE: f32 = 0.05;

/// A portal whose far end is in view: on the portal's scenery element. Removing it, or despawning
/// the element, despawns the camera and the aperture.
#[derive(Component)]
#[component(on_remove = end_portal_view)]
pub struct PortalView {
    camera: Entity,
    aperture: Entity,
    material: Handle<ApertureMaterial>,
    /// The size of the camera's image, the primary camera's viewport's when it was made.
    image_size: UVec2,
}

/// A portal's camera.
#[derive(Component)]
pub struct PortalCamera;

/// The surface that shows a portal camera's image, where the near aperture is.
#[derive(Component)]
pub struct PortalAperture;

fn end_portal_view(mut world: DeferredWorld, context: HookContext) {
    let view = world.get::<PortalView>(context.entity).unwrap();
    let (camera, aperture) = (view.camera, view.aperture);
    let mut commands = world.commands();
    commands.entity(camera).try_despawn();
    commands.entity(aperture).try_despawn();
}

/// A portal's aperture, from its element's transform: a rectangle in its local x-y plane, facing
/// +z, centered on the origin. faery's apertures lay in their element's y-z plane, facing +x, so
/// the rotation adds a quarter turn about y.
pub fn aperture_transform(portal: &Portal, element: &GlobalTransform) -> Transform {
    element.compute_transform()
        * Transform::from_translation(portal.offset).with_rotation(Quat::from_euler(
            EulerRot::XYZ,
            portal.x_rotation.unwrap_or(0.),
            portal.y_rotation.unwrap_or(0.) + std::f32::consts::FRAC_PI_2,
            portal.z_rotation.unwrap_or(0.),
        ))
}

/// A portal camera's name, as plans select it: `PortalCamera:` and its element's label, or its key
/// without one, or its entity without either.
fn camera_name(element: Entity, scenery: Option<&SceneryElement>) -> Name {
    let label = scenery.and_then(|scenery| scenery.label.as_deref());
    match (label, scenery.and_then(|scenery| scenery.iid.key())) {
        (Some(label), _) => Name::new(format!("PortalCamera:{label}")),
        (None, Some(key)) => Name::new(format!("PortalCamera:{key}")),
        (None, None) => Name::new(format!("PortalCamera:{element}")),
    }
}

/// Whether a portal open from `side` shows its far end to a viewer at `eye`.
fn faces_viewer(side: PortalSide, aperture: &Transform, eye: Vec3) -> bool {
    let toward = aperture.translation - eye;
    let normal = aperture.rotation * Vec3::Z;
    match side {
        PortalSide::Both => true,
        PortalSide::Front => toward.dot(normal) > 0.,
        PortalSide::Back => toward.dot(normal) < 0.,
    }
}

/// The primary camera's view, as `update_portal_views` reads it.
struct PrimaryView {
    transform: Transform,
    clip_from_world: Mat4,
    /// The viewport, in the window's physical pixels.
    viewport: URect,
    perspective: PerspectiveProjection,
}

impl PrimaryView {
    /// The aperture's rectangle on screen, in the window's physical pixels, whole pixels, clipped
    /// to the viewport; `None` if it is off screen. The part of the aperture behind the near
    /// plane is cut away first, so an aperture behind the camera is off screen.
    fn screen_rect(&self, aperture: &Transform, size: Vec2) -> Option<URect> {
        let corners = [
            Vec2::new(-0.5, -0.5),
            Vec2::new(0.5, -0.5),
            Vec2::new(0.5, 0.5),
            Vec2::new(-0.5, 0.5),
        ]
        .map(|corner| {
            let world = aperture.transform_point((corner * size).extend(0.));
            self.clip_from_world * world.extend(1.)
        });
        // Clip space's w is the depth in front of the camera.
        let near = self.perspective.near;
        let mut ndc = Rect::EMPTY;
        for (i, &a) in corners.iter().enumerate() {
            let b = corners[(i + 1) % corners.len()];
            if a.w >= near {
                ndc = ndc.union_point(a.xy() / a.w);
            }
            // Where the edge crosses the near plane.
            if (a.w >= near) != (b.w >= near) {
                let crossing = a.lerp(b, (near - a.w) / (b.w - a.w));
                ndc = ndc.union_point(crossing.xy() / crossing.w);
            }
        }
        let ndc = ndc.intersect(Rect::new(-1., -1., 1., 1.));
        if ndc.is_empty() {
            return None;
        }
        let origin = self.viewport.min.as_vec2();
        let size = self.viewport.size().as_vec2();
        let to_pixels = |ndc: Vec2| origin + Vec2::new(ndc.x + 1., 1. - ndc.y) / 2. * size;
        let min = to_pixels(Vec2::new(ndc.min.x, ndc.max.y))
            .floor()
            .as_uvec2();
        let max = to_pixels(Vec2::new(ndc.max.x, ndc.min.y)).ceil().as_uvec2();
        let pixels = URect::from_corners(min, max).intersect(self.viewport);
        (!pixels.is_empty()).then_some(pixels)
    }

    /// `pixels`, a rectangle of the window, in the primary camera's normalized device coordinates.
    fn to_ndc(&self, pixels: URect) -> Rect {
        let origin = self.viewport.min.as_vec2();
        let size = self.viewport.size().as_vec2();
        let to_ndc = |pixel: Vec2| {
            let unit = (pixel - origin) / size;
            Vec2::new(unit.x * 2. - 1., 1. - unit.y * 2.)
        };
        Rect::from_corners(to_ndc(pixels.min.as_vec2()), to_ndc(pixels.max.as_vec2()))
    }
}

/// The far aperture's plane in the portal camera's view space, as `PortalProjection::clip_plane`
/// takes it, facing away from the camera; `None` if the camera is all but on it.
fn far_clip_plane(camera: &Transform, far_center: Vec3, normal: Vec3) -> Option<Vec4> {
    let toward = far_center - camera.translation;
    let normal = if toward.dot(normal) >= 0. {
        normal
    } else {
        -normal
    };
    let distance = toward.dot(normal);
    (distance >= MIN_CLIP_DISTANCE).then(|| (camera.rotation.inverse() * normal).extend(-distance))
}

/// Starts, places and ends each portal's view, and lists the far ends to load. After the primary
/// camera is placed for the frame, and before transforms propagate, so the portal camera moves
/// with it.
pub(crate) fn update_portal_views(
    mut commands: Commands,
    viewpoint: Res<Viewpoint>,
    mut regions: ResMut<PortalViewRegions>,
    q_primary: Query<(&Transform, &Camera, &Projection), With<PrimaryCamera>>,
    q_realms: Query<(Entity, &Realm)>,
    mut q_portals: Query<(
        Entity,
        &Portal,
        &PortalTarget,
        &GlobalTransform,
        &RenderLayers,
        Option<&SceneryElement>,
        Option<&mut PortalView>,
    )>,
    mut q_cameras: Query<
        (
            &mut Transform,
            &mut Projection,
            &mut Camera,
            &mut RenderTarget,
        ),
        (With<PortalCamera>, Without<PrimaryCamera>),
    >,
    mut q_apertures: Query<
        &mut Transform,
        (
            With<PortalAperture>,
            Without<PortalCamera>,
            Without<PrimaryCamera>,
        ),
    >,
    mut materials: ResMut<Assets<ApertureMaterial>>,
    mut images: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    let source_realm = viewpoint
        .realm
        .and_then(|realm| q_realms.get(realm).ok())
        .map(|(_, realm)| realm.layer.clone());
    let primary = q_primary
        .single()
        .ok()
        .and_then(|(transform, camera, projection)| {
            let Projection::Perspective(perspective) = projection else {
                return None;
            };
            Some(PrimaryView {
                transform: *transform,
                clip_from_world: camera.clip_from_view() * transform.to_matrix().inverse(),
                viewport: camera.physical_viewport_rect()?,
                perspective: perspective.clone(),
            })
        });

    let mut wanted_regions = Vec::new();
    for (entity, portal, target, element, layers, scenery, view) in q_portals.iter_mut() {
        let far_realm = q_realms
            .iter()
            .find(|(_, realm)| !target.realm.is_empty() && realm.name == target.realm)
            .map(|(entity, realm)| (entity, realm.layer.clone()));
        let (Some(source_layer), Some((far_realm, far_layer)), Some(primary)) =
            (&source_realm, far_realm, &primary)
        else {
            if view.is_some() {
                commands.entity(entity).remove::<PortalView>();
            }
            continue;
        };
        // Only portals in the viewpoint's realm are looked through.
        if !layers.intersects(source_layer) {
            if view.is_some() {
                commands.entity(entity).remove::<PortalView>();
            }
            continue;
        }

        let aperture = aperture_transform(portal, element);
        // The target is the ground under the far aperture's center.
        let far_center = target.pos + Vec3::Y * portal.size.y * 0.5;
        let offset = far_center - aperture.translation;
        if aperture.translation.xz().distance(viewpoint.position.xz()) <= PORTAL_PRELOAD_DISTANCE {
            wanted_regions.push(PortalViewRegion {
                realm: far_realm,
                center: target.pos,
            });
        }

        let rect = faces_viewer(portal.side, &aperture, primary.transform.translation)
            .then(|| primary.screen_rect(&aperture, portal.size))
            .flatten();
        let Some(rect) = rect else {
            if view.is_some() {
                commands.entity(entity).remove::<PortalView>();
            }
            continue;
        };

        let camera_transform = primary
            .transform
            .with_translation(primary.transform.translation + offset);
        let projection = Projection::custom(PortalProjection {
            fov: primary.perspective.fov,
            aspect_ratio: primary.perspective.aspect_ratio,
            near: primary.perspective.near,
            // Culling stops at the far end's region, which is all that loads there: without the
            // cap, a portal into its own realm culled against the primary camera's 100 m.
            far: (far_center - camera_transform.translation).length() + 2. * PORTAL_VIEW_HALF_SIZE,
            rect: primary.to_ndc(rect),
            clip_plane: far_clip_plane(&camera_transform, far_center, aperture.rotation * Vec3::Z),
        });
        let viewport = Viewport {
            physical_position: UVec2::ZERO,
            physical_size: rect.size(),
            ..default()
        };
        let origin = rect.min.as_vec2().extend(0.).extend(0.);
        let image_size = primary.viewport.size();

        match view {
            Some(mut view) => {
                if let Ok((mut transform, mut camera_projection, mut camera, mut render_target)) =
                    q_cameras.get_mut(view.camera)
                {
                    *transform = camera_transform;
                    *camera_projection = projection;
                    camera.viewport = Some(viewport);
                    if view.image_size != image_size {
                        let image = images.add(portal_image(image_size));
                        *render_target = RenderTarget::Image(image.clone().into());
                        if let Some(mut material) = materials.get_mut(&view.material) {
                            material.image = image;
                        }
                        view.image_size = image_size;
                    }
                }
                if let Some(mut material) = materials.get_mut(&view.material) {
                    if material.origin != origin {
                        material.origin = origin;
                    }
                }
                if let Ok(mut transform) = q_apertures.get_mut(view.aperture) {
                    transform.set_if_neq(aperture);
                }
            }
            None => {
                let image = images.add(portal_image(image_size));
                let material = materials.add(ApertureMaterial {
                    origin,
                    image: image.clone(),
                });
                let camera = commands
                    .spawn((
                        camera_name(entity, scenery),
                        PortalCamera,
                        Camera3d::default(),
                        Camera {
                            // Before the primary camera, which shows its image.
                            order: -1,
                            clear_color: ClearColorConfig::Custom(Color::BLACK),
                            viewport: Some(viewport),
                            ..default()
                        },
                        RenderTarget::Image(image.into()),
                        projection,
                        camera_transform,
                        // The far realm only, without the apertures.
                        far_layer,
                        // As the primary camera has them, so the two views match; no SSAO, which
                        // would cost a depth prepass for a postage stamp of the screen.
                        Tonemapping::Linear,
                        ShadowFilteringMethod::Hardware2x2,
                        Msaa::Off,
                    ))
                    .id();
                let aperture = commands
                    .spawn((
                        Name::new("PortalAperture"),
                        PortalAperture,
                        Mesh3d(meshes.add(Rectangle::from_size(portal.size))),
                        MeshMaterial3d(material.clone()),
                        aperture,
                        RenderLayers::layer(PORTAL_APERTURE_LAYER),
                        NotShadowCaster,
                        NotShadowReceiver,
                        // Clicks go to the element behind it.
                        Pickable::IGNORE,
                    ))
                    .id();
                commands.entity(entity).insert(PortalView {
                    camera,
                    aperture,
                    material,
                    image_size,
                });
            }
        }
    }
    regions.set_if_neq(PortalViewRegions(wanted_regions));
}

/// The image a portal camera renders into, as the primary camera's target would be.
fn portal_image(size: UVec2) -> Image {
    Image::new_target_texture(size.x, size.y, TextureFormat::Bgra8UnormSrgb, None)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn primary() -> PrimaryView {
        let transform = Transform::from_xyz(0., 8., 8.).looking_at(Vec3::ZERO, Vec3::Y);
        let mut perspective = PerspectiveProjection {
            fov: 0.69,
            near: 0.5,
            far: 100.,
            ..default()
        };
        perspective.aspect_ratio = 1600. / 900.;
        let projection = Projection::Perspective(perspective.clone());
        PrimaryView {
            clip_from_world: projection.get_clip_from_view() * transform.to_matrix().inverse(),
            transform,
            viewport: URect::new(100, 50, 1700, 950),
            perspective,
        }
    }

    /// An aperture at the view's center covers the viewport's center (in perspective, its
    /// rectangle is not centered on it), and its rectangle maps back to normalized device
    /// coordinates covering the view's center.
    #[test]
    fn aperture_at_center_is_on_screen_at_center() {
        let view = primary();
        let aperture = Transform::IDENTITY;
        let rect = view.screen_rect(&aperture, Vec2::new(1.8, 1.6)).unwrap();
        assert!(rect.contains(UVec2::new(900, 500)), "{rect:?}");
        assert!(view.viewport.contains(rect.min) && rect.max.cmple(view.viewport.max).all());
        let ndc = view.to_ndc(rect);
        assert!(ndc.contains(Vec2::ZERO), "{ndc:?}");
    }

    /// Off to the side and out of the view, no rectangle.
    #[test]
    fn aperture_out_of_view_is_off_screen() {
        let view = primary();
        let aperture = Transform::from_xyz(60., 0., 0.);
        assert!(view.screen_rect(&aperture, Vec2::new(1.8, 1.6)).is_none());
    }

    /// Behind the camera, no rectangle, where taking the whole view would start a camera for an
    /// aperture no one sees.
    #[test]
    fn aperture_behind_camera_is_off_screen() {
        let view = primary();
        let aperture = Transform::from_xyz(0., 2., 30.);
        assert!(view.screen_rect(&aperture, Vec2::new(1.8, 1.6)).is_none());
    }

    /// Reaching from behind the near plane to in front of it, the part in front counts: here a
    /// floor under the camera, which covers the bottom of the view.
    #[test]
    fn aperture_across_near_plane_is_cut() {
        let view = primary();
        let forward = view.transform.forward().as_vec3();
        let up = view.transform.up().as_vec3();
        // Parallel to the view direction, 1 m below the camera, 6 m either side of the near plane.
        let center = view.transform.translation + forward * view.perspective.near - up;
        let aperture = Transform::from_translation(center).looking_to(up, forward);
        let rect = view.screen_rect(&aperture, Vec2::new(4., 12.)).unwrap();
        assert!(rect.min.y > view.viewport.center().y, "{rect:?}");
        assert_eq!(rect.max.y, view.viewport.max.y, "{rect:?}");
    }

    /// The front side is seen from behind the normal, as faery had it: the aperture faces away
    /// from its viewer.
    #[test]
    fn front_is_seen_against_the_normal() {
        let aperture = Transform::IDENTITY; // Facing +z.
        assert!(faces_viewer(
            PortalSide::Front,
            &aperture,
            Vec3::new(0., 0., -5.)
        ));
        assert!(!faces_viewer(
            PortalSide::Front,
            &aperture,
            Vec3::new(0., 0., 5.)
        ));
        assert!(faces_viewer(
            PortalSide::Back,
            &aperture,
            Vec3::new(0., 0., 5.)
        ));
        assert!(faces_viewer(
            PortalSide::Both,
            &aperture,
            Vec3::new(0., 0., 5.)
        ));
    }

    /// The far plane faces away from the camera whichever way the aperture faces.
    #[test]
    fn clip_plane_faces_away() {
        let camera = Transform::from_xyz(0., 8., 8.).looking_at(Vec3::ZERO, Vec3::Y);
        for normal in [Vec3::Z, Vec3::NEG_Z] {
            let plane = far_clip_plane(&camera, Vec3::ZERO, normal).unwrap();
            // The plane's point in view space is ahead (negative z), and on the plane.
            let point = camera.to_matrix().inverse().transform_point3(Vec3::ZERO);
            assert!(point.z < 0.);
            assert!((plane.xyz().dot(point) + plane.w).abs() < 1e-4);
            assert!(plane.w < 0.);
        }
    }
}
