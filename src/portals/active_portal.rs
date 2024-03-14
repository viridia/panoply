use bevy::{prelude::*, render::view::RenderLayers};

use super::portal_aspect::{Portal, PortalTarget};

#[derive(Component)]
pub(crate) struct ActivePortal {
    /// The camera used to render the remote location.
    portal_camera: Option<Entity>,

    /// The mesh that renders the portal aperture.
    mesh: Handle<Mesh>,

    /// The material that renders the portal aperture.
    material: Handle<StandardMaterial>,

    /// Entity which renders the portal.
    portal_entity: Entity,
    // public readonly portalCamera: PerspectiveCamera;
    // public isOnscreen = false;

    // /** Location of source end. */
    // public readonly sourcePosition = new Vector3();
    // public sourceRealm: Realm | null = null;

    // /** Depth of the nearest point of the portal. */
    // public nearDepth = Infinity;

    // /** Size of the hole. */
    // private apertureSize = new Vector3();
    // private aperturePlane = new Plane();

    // /** Direction the hole is facing */
    // private apertureFacing = new Vector4();
    // private apertureSide: Side = FrontSide;

    // /** Difference between near and far end. Used to calculate relative camera position. */
    // private differential = new Vector3();
    // private displacement = 0;

    // // The geometric appearance of the portal.
    // private geometry = new BoxGeometry(1, 2, 0);
    // private material = new ActivePortalMaterial();
    // private mesh = new Mesh(this.geometry, this.material);

    // private renderTarget: WebGLRenderTarget | null = null;

    // /** Rectangular bounds of portal on screen. */
    // private mainScreenRect = new Box2();
    // private portalScreenRect = new Box2();

    // /** Same as screenRect, but in a form acceptable to scissor/viewport calls. */
    // private portalViewport = new Vector4();
    // private portalSize = new Vector2();
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

// We need to figure out which portals are in (or near) the current view frustum.

pub(crate) fn spawn_portals(
    mut commands: Commands,
    mut active_portal_query: Query<
        (
            Entity,
            &GlobalTransform,
            &Portal,
            &PortalTarget,
            &RenderLayers,
        ),
        Without<ActivePortal>,
    >,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (entity, _xform, portal, _portal_target, layers) in active_portal_query.iter_mut() {
        let material = materials.add(StandardMaterial {
            base_color: Color::rgb(1.0, 0.0, 1.0),
            unlit: true,
            ..default()
        });

        let mesh = meshes.add(Cuboid::from_size(portal.size));
        let portal_entity = commands
            .spawn((
                MaterialMeshBundle::<StandardMaterial> {
                    mesh: mesh.clone(),
                    material: material.clone(),
                    transform: Transform::from_translation(portal.offset),
                    ..Default::default()
                },
                *layers,
            ))
            .set_parent(entity)
            .id();

        commands.entity(entity).insert(ActivePortal {
            portal_camera: None,
            mesh,
            material: material.clone(),
            portal_entity,
        });
    }
}
