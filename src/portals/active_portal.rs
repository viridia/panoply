use bevy::{
    prelude::*,
    render::{
        camera::RenderTarget,
        primitives::{Frustum, Sphere},
        render_resource::{
            Extent3d, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
        },
        view::RenderLayers,
    },
};

use crate::view::PrimaryCamera;

use super::portal_aspect::{Portal, PortalSide, PortalTarget};

#[derive(Component)]
pub(crate) struct ActivePortal {
    /// The camera used to render the remote location.
    camera: Entity,

    /// The mesh that renders the portal aperture.
    aperture_mesh: Handle<Mesh>,

    /// Aperture transform, from aperture local to world space.
    transform: Transform,

    /// The material that renders the portal aperture.
    material: Handle<StandardMaterial>,

    /// Entity which renders the portal.
    portal_entity: Entity,

    /// The image buffer
    image: Handle<Image>,
    // public readonly portalCamera: PerspectiveCamera;
    // public isOnscreen = false;

    // /** Size of the hole. */
    // private apertureSize = new Vector3();
    // private aperturePlane = new Plane();

    // /** Rectangular bounds of portal on screen. */
    // private mainScreenRect = new Box2();
    // private portalScreenRect = new Box2();

    // private portalBufferSize = new Vector2();

    // // Used in calculations
    // private lookAtPt = new Vector3();
    // private worldPt = new Vector3();
    // private screenPt = new Vector2();
    // private cameraFacing = new Vector3();
    // private sourceScene?: Scene;
    // private clippingPlane = new Plane();

    // private needsUpdate = false;
}

#[derive(Component)]
pub(crate) struct PortalCamera;

// We need to figure out which portals are in (or near) the current view frustum.

#[allow(clippy::type_complexity)]
pub(crate) fn spawn_portals(
    mut commands: Commands,
    query_camera: Query<(&GlobalTransform, &Frustum), With<PrimaryCamera>>,
    mut active_portal_query: Query<(
        Entity,
        &GlobalTransform,
        &Portal,
        &RenderLayers,
        Option<&ActivePortal>,
    )>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    let Ok((primary_xform, frustum)) = query_camera.get_single() else {
        warn!("No primary camera found");
        return;
    };
    let primary_transform = primary_xform.compute_transform();

    for (entity, portal_xform, portal, layers, active) in active_portal_query.iter_mut() {
        // Compute the transform for the portal aperture.
        let portal_transform = portal_xform.compute_transform();
        let aperture_transform = portal_transform
            * Transform::from_translation(portal.offset).with_rotation(Quat::from_euler(
                EulerRot::XYZ,
                portal.x_rotation.unwrap_or(0.),
                portal.y_rotation.unwrap_or(0.) + std::f32::consts::FRAC_PI_2,
                portal.z_rotation.unwrap_or(0.),
            ));
        let aperture_center = aperture_transform.transform_point(Vec3::ZERO);

        // Portal bounding sphere
        let bounds = Sphere {
            center: aperture_center.into(),
            radius: portal.size.length(),
        };

        let is_visible = {
            if !frustum.intersects_sphere(&bounds, false) {
                // Portal is offscreen
                false
            } else if portal.side == PortalSide::Both {
                // Visible from both sides
                true
            } else {
                // Visible if the aperture normal is pointing toward the camera
                let aperture_normal = aperture_transform.rotation.mul_vec3(Vec3::new(0., 0., 1.));
                let camera_dir = primary_transform.translation - aperture_center;
                let dot = aperture_normal.dot(camera_dir);
                if portal.side == PortalSide::Front {
                    dot <= 0.
                } else {
                    dot >= 0.
                }
            }
        };

        // Test if portal is within camera frustum. If not, we can despawn it.
        if is_visible {
            if active.is_some() {
                continue;
            }

            // println!("Portal in view: {:?}", entity);
            // TODO: recalculate when Portal component changed.
            let mut transform = Transform::from_translation(portal.offset);
            transform.rotate(Quat::from_rotation_y(-std::f32::consts::FRAC_PI_2));
            let mesh = meshes.add(Rectangle::from_size(portal.size));

            let size = Extent3d {
                width: 100,
                height: 100,
                ..Extent3d::default()
            };

            let mut image = Image {
                texture_descriptor: TextureDescriptor {
                    label: None,
                    size,
                    dimension: TextureDimension::D2,
                    format: TextureFormat::Bgra8UnormSrgb,
                    mip_level_count: 1,
                    sample_count: 1,
                    usage: TextureUsages::TEXTURE_BINDING
                        | TextureUsages::COPY_DST
                        | TextureUsages::RENDER_ATTACHMENT,
                    view_formats: &[],
                },
                ..Image::default()
            };

            image.resize(size);

            let image_handle = images.add(image);

            let material = materials.add(StandardMaterial {
                base_color: Color::rgb(1.0, 0.0, 1.0),
                // base_color_texture: Some(image_handle.clone()),
                unlit: true,
                double_sided: true,
                cull_mode: None,
                ..default()
            });

            let portal_entity = commands
                .spawn((
                    MaterialMeshBundle::<StandardMaterial> {
                        mesh: mesh.clone(),
                        material: material.clone(),
                        transform,
                        ..Default::default()
                    },
                    *layers,
                ))
                .set_parent(entity)
                .id();

            let camera = commands
                .spawn((
                    Camera3dBundle {
                        transform: Transform::from_translation(Vec3::new(0., 0., 0.)),
                        camera: Camera {
                            order: -1,
                            clear_color: ClearColorConfig::Custom(Color::BLACK),
                            target: RenderTarget::Image(image_handle.clone()),
                            ..default()
                        },
                        ..default()
                    },
                    RenderLayers::all(),
                    PortalCamera,
                ))
                .id();

            commands.entity(entity).insert(ActivePortal {
                camera,
                aperture_mesh: mesh,
                transform: aperture_transform,
                material: material.clone(),
                portal_entity,
                image: image_handle,
            });
        } else if let Some(active) = active {
            // TODO: Despawn portal if scene element despawned
            commands.entity(active.portal_entity).despawn();
            commands.entity(active.camera).despawn();
            commands.entity(entity).remove::<ActivePortal>();
        }
    }
}

pub(crate) fn update_portals(
    query_primary_camera: Query<(&Camera, &Transform, &GlobalTransform), With<PrimaryCamera>>,
    mut query_portal_camera: Query<&mut Transform, (With<PortalCamera>, Without<PrimaryCamera>)>,
    mut active_portal_query: Query<(&GlobalTransform, &Portal, &PortalTarget, &ActivePortal)>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut gizmos: Gizmos,
) {
    let Ok((primary_camera, primary_transform, primary_global)) = query_primary_camera.get_single()
    else {
        return;
    };
    let _primary_global_transform = primary_global.compute_transform();

    for (_portal_xform, portal, portal_target, active_portal) in active_portal_query.iter_mut() {
        let Ok(mut portal_camera_xform) = query_portal_camera.get_mut(active_portal.camera) else {
            println!("No portal camera found");
            continue;
        };
        let source_position = portal_camera_xform.translation;
        let target_position = Vec3::from(portal_target.pos);
        let differential = target_position - source_position;
        portal_camera_xform.translation = primary_transform.translation + differential;
        portal_camera_xform.rotation = primary_transform.rotation;

        let center = active_portal
            .transform
            .transform_point(Vec3::new(0., 0., 0.));
        gizmos.rect(
            center,
            active_portal.transform.rotation,
            portal.size,
            Color::GREEN,
        );

        let normal = active_portal
            .transform
            .rotation
            .mul_vec3(Vec3::new(0., 0., 1.));
        if portal.side != PortalSide::Back {
            gizmos.arrow(center, center + normal, Color::GOLD);
        }
        if portal.side != PortalSide::Front {
            gizmos.arrow(center, center - normal, Color::AQUAMARINE);
        }

        let _material = materials.get_mut(&active_portal.material).unwrap();
        // material.base_color_texture = Some(active_portal.image.clone());

        let viewport_size = primary_camera.logical_viewport_size().unwrap();
        let mut rect = Rect {
            min: Vec2::new(f32::MAX, f32::MAX),
            max: Vec2::new(f32::MIN, f32::MIN),
        };
        if let Some(pos) = primary_camera.world_to_viewport(primary_global, center) {
            // println!("pos: {:?}", pos);
            rect = rect.union_point(pos);
        }

        for x in [-portal.size.x * 0.5, portal.size.x * 0.5].iter() {
            for y in [-portal.size.y * 0.5, portal.size.y * 0.5].iter() {
                let pos = active_portal
                    .transform
                    .transform_point(Vec3::new(*x, *y, 0.));
                gizmos.arrow(pos, pos + normal, Color::GOLD);
                if let Some(pos) = primary_camera.world_to_viewport(primary_global, pos) {
                    rect = rect.union_point(pos);
                }
            }
        }

        let viewport_rect = primary_camera.logical_viewport_rect().unwrap();
        rect = rect.intersect(viewport_rect);
        if !rect.is_empty() {
            let screen_rect = Rect {
                min: Vec2::new(
                    rect.min.x - viewport_size.x * 0.5,
                    viewport_size.y * 0.5 - rect.min.y,
                ),
                max: Vec2::new(
                    rect.max.x - viewport_size.x * 0.5,
                    viewport_size.y * 0.5 - rect.max.y,
                ),
            };
            gizmos.rect_2d(screen_rect.center(), 0., screen_rect.size(), Color::RED);
        }
    }
}
