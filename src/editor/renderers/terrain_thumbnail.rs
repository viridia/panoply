use bevy::{
    asset::LoadState,
    pbr::CascadeShadowConfigBuilder,
    prelude::*,
    render::{
        camera::RenderTarget,
        render_resource::{
            Extent3d, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
        },
        view::RenderLayers,
    },
    utils::HashMap,
};

use crate::{
    editor::events::{ChangeContourEvent, ThumbnailsReady},
    terrain::{
        create_ground_material,
        terrain_contours::{TerrainContoursHandle, TerrainContoursTableAsset},
        ComputeGroundMeshTask, ComputeWaterMeshTask, GroundMaterial, Parcel, ParcelFloraChanged,
        ParcelTerrainFx, ParcelThumbnail, ParcelWaterChanged, RebuildParcelGroundMesh, ShapeRef,
        TerrainFxVertexAttr, TerrainMap, PARCEL_TERRAIN_FX_AREA,
    },
    view::layers::ReservedLayers,
    world::{HiddenRealm, Realm, RealmLighting},
};

#[derive(Component)]
pub struct TerrainThumbnail {
    pub contour_id: usize,
    pub render_target: Handle<Image>,
    preview_parcel: Option<Entity>,
    // clip bounds
}

/// Marker component that indicates that a terrain thumbnail needs to be rebuilt.
#[derive(Component)]
pub struct RebuildTerrainThumbnail;

/// Resource that contains the realm used for previewing terrain thumbnails.
#[derive(Resource)]
pub struct TerrainThumbnailRealm(pub Entity);

#[derive(Component)]
pub(crate) struct ThumbnailCamera;

#[derive(Resource)]
pub struct TerrainThumbnailBuilder {
    camera: Entity,
    dir_light: Entity,
}

const PARCEL_SPACING: f32 = 40.0;

#[allow(clippy::too_many_arguments)]
pub fn setup_thumbnail_realm(
    mut commands: Commands,
    r_realm: Option<ResMut<TerrainThumbnailRealm>>,
    mut r_materials: ResMut<Assets<GroundMaterial>>,
    mut r_images: ResMut<Assets<Image>>,
    mut r_layers: ResMut<ReservedLayers>,
    r_server: Res<AssetServer>,
) {
    if r_realm.is_none() {
        let layer = r_layers.next_unused();
        let realm = commands
            .spawn((
                Realm {
                    layer: RenderLayers::layer(layer),
                    layer_index: layer,
                    name: "ThumbnailRealm".to_string(),
                    lighting: RealmLighting::Exterior,
                    parcel_bounds: IRect::from_corners(IVec2::ZERO, IVec2::new(1, 1)),
                    precinct_bounds: IRect::from_corners(IVec2::ZERO, IVec2::ZERO),
                },
                TerrainMap {
                    handle: Handle::default(),
                    ground_material: create_ground_material(
                        &mut r_materials,
                        &mut r_images,
                        &r_server,
                    ),
                    needs_rebuild_biomes: false,
                },
                HiddenRealm,
            ))
            .id();
        commands.insert_resource(TerrainThumbnailRealm(realm));
    }
}

pub fn setup_thumbnail_camera(
    mut commands: Commands,
    q_realms: Query<&Realm>,
    r_realm: ResMut<TerrainThumbnailRealm>,
    r_builder: Option<ResMut<TerrainThumbnailBuilder>>,
) {
    let realm = q_realms.get(r_realm.0).unwrap();
    if r_builder.is_none() {
        let camera = commands
            .spawn((
                Camera3dBundle {
                    transform: Transform::from_translation(Vec3::new(-40. + 256., 40., -40.))
                        .looking_at(Vec3::new(8. + 256., 0., 8.), Vec3::Y),
                    camera: Camera {
                        order: -2,
                        clear_color: ClearColorConfig::Custom(Srgba::new(0., 0., 0., 0.).into()),
                        is_active: false,
                        ..default()
                    },
                    projection: Projection::Perspective(PerspectiveProjection {
                        fov: std::f32::consts::PI / 18.0,
                        near: 0.1,
                        far: 1000.0,
                        aspect_ratio: 1.0,
                    }),
                    ..default()
                },
                ThumbnailCamera,
                realm.layer.clone(),
            ))
            .id();
        let dir_light = commands
            .spawn((
                DirectionalLightBundle {
                    directional_light: DirectionalLight {
                        shadows_enabled: true,
                        color: Srgba::WHITE.into(),
                        illuminance: 3000.,
                        ..default()
                    },
                    cascade_shadow_config: CascadeShadowConfigBuilder {
                        first_cascade_far_bound: 4.0,
                        maximum_distance: 40.0,
                        ..default()
                    }
                    .into(),
                    transform: Transform::from_rotation(Quat::from_rotation_x(-0.9)),
                    ..default()
                },
                realm.layer.clone(),
            ))
            .id();
        commands.insert_resource(TerrainThumbnailBuilder { camera, dir_light });
    }
}

pub fn setup_thumbnail_observer(mut commands: Commands) {
    commands.observe(
        |event: Trigger<ChangeContourEvent>,
         mut commands: Commands,
         q_thumbnails: Query<(Entity, &TerrainThumbnail)>| {
            if let Some((e, _)) = q_thumbnails
                .iter()
                .find(|(_, t)| t.contour_id == event.event().0)
            {
                commands.entity(e).insert(RebuildTerrainThumbnail);
            }
        },
    );
}

pub fn create_terrain_thumbnails(
    mut commands: Commands,
    r_handle: Res<TerrainContoursHandle>,
    r_asset: Res<Assets<TerrainContoursTableAsset>>,
    r_thumbnails: Query<(Entity, &TerrainThumbnail)>,
    r_server: Res<AssetServer>,
    mut r_images: ResMut<Assets<Image>>,
) {
    if r_server.load_state(&r_handle.0) != LoadState::Loaded {
        return;
    }

    // Build index by shape index
    let mut thumbnails: HashMap<usize, Entity> = r_thumbnails
        .iter()
        .map(|(e, t)| (t.contour_id, e))
        .collect();

    // Get the contours
    let tc_asset = r_asset.get(&r_handle.0).unwrap();
    let tc_table = tc_asset.0.read().unwrap();
    let mut changed = false;
    for (contour_id, _contour) in tc_table.list().iter().enumerate() {
        if thumbnails.remove(&contour_id).is_none() {
            // Create new thumbnail if it wasn't already in the map.
            let size = Extent3d {
                width: 128,
                height: 64,
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
                ..default()
            };
            image.resize(size); // This line is required, otherwise panic.
            let render_target = r_images.add(image);
            commands.spawn((
                TerrainThumbnail {
                    contour_id,
                    render_target,
                    preview_parcel: None,
                },
                RebuildTerrainThumbnail,
            ));
            changed = true;
        }
    }

    // Remove thumbnails that are no longer needed
    for (_, e) in thumbnails.drain() {
        println!("Despawning thumbnail {:?}", e);
        commands.entity(e).despawn();
        changed = true;
    }

    if changed {
        commands.trigger(ThumbnailsReady);
    }
}

pub fn update_terrain_thumbnails(
    mut commands: Commands,
    mut r_thumbnails: Query<&mut TerrainThumbnail, With<RebuildTerrainThumbnail>>,
    r_realm: Option<Res<TerrainThumbnailRealm>>,
) {
    let Some(realm_id) = r_realm else {
        return;
    };
    for mut t in r_thumbnails.iter_mut() {
        if t.preview_parcel.is_none() {
            // Create preview parcel
            let parcel = Parcel {
                realm: realm_id.0,
                coords: IVec2::ZERO,
                visible: false,
                contours: [
                    ShapeRef::default(),
                    ShapeRef::default(),
                    ShapeRef::default(),
                    ShapeRef::default(),
                    ShapeRef {
                        shape: t.contour_id as u16,
                        rotation: 0,
                    },
                    ShapeRef::default(),
                    ShapeRef::default(),
                    ShapeRef::default(),
                    ShapeRef::default(),
                ],
                biomes: [0, 0, 0, 0],
                ground_entity: None,
                water_entity: None,
                flora_entity: None,
                terrain_fx: ParcelTerrainFx(
                    [TerrainFxVertexAttr::default(); PARCEL_TERRAIN_FX_AREA],
                ),
            };

            let shape_pos = t.contour_id as f32 * PARCEL_SPACING;
            t.preview_parcel = Some(
                commands
                    .spawn((
                        parcel,
                        Name::new(format!("Parcel.Thumbnail:{}", t.contour_id)),
                        SpatialBundle {
                            transform: Transform::from_xyz(shape_pos, 0., 0.),
                            visibility: Visibility::Visible,
                            ..default()
                        },
                        RebuildParcelGroundMesh,
                        ParcelFloraChanged,
                        ParcelWaterChanged,
                        ParcelThumbnail,
                    ))
                    .id(),
            );
        }
    }
}

pub fn assign_thumbnails_to_camera(
    mut commands: Commands,
    q_thumbnails: Query<(Entity, &TerrainThumbnail), With<RebuildTerrainThumbnail>>,
    q_parcels: Query<
        &Parcel,
        (
            With<ParcelThumbnail>,
            Without<RebuildParcelGroundMesh>,
            Without<ComputeGroundMeshTask>,
            Without<ComputeWaterMeshTask>,
            Without<ParcelFloraChanged>,
            Without<ParcelWaterChanged>,
        ),
    >,
    mut q_cameras: Query<(&mut Camera, &mut Transform), With<ThumbnailCamera>>,
    r_builder: ResMut<TerrainThumbnailBuilder>,
) {
    let Ok((mut camera, mut camera_transform)) = q_cameras.get_mut(r_builder.camera) else {
        return;
    };

    for (e, thumbnail) in q_thumbnails.iter() {
        let Some(parcel_id) = thumbnail.preview_parcel else {
            continue;
        };
        if q_parcels.get(parcel_id).is_ok() {
            // Assign parcel to camera
            let xpos = thumbnail.contour_id as f32 * PARCEL_SPACING;
            camera.target = RenderTarget::Image(thumbnail.render_target.clone());
            camera.is_active = true;
            *camera_transform = Transform::from_translation(Vec3::new(-40. + xpos, 30., -40.))
                .looking_at(Vec3::new(8. + xpos, 0., 8.), Vec3::Y);

            commands.entity(e).remove::<RebuildTerrainThumbnail>();
            return;
        }
    }

    camera.is_active = false;
}
