use std::f32::consts::PI;

use bevy::asset::io::Reader;
use bevy::asset::{AssetLoader, LoadContext, LoadedFolder};
use bevy::pbr::CascadeShadowConfigBuilder;
use bevy::prelude::*;
use bevy::reflect::TypePath;
use bevy::render::view::RenderLayers;
use futures_lite::AsyncReadExt;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::view::layers::ReservedLayers;

#[derive(Default, Serialize, Deserialize, Clone, Copy)]
pub enum RealmLighting {
    /// Used for interior spaces like caves and dungeons.
    Interior,

    /// Used for overland realms.
    #[default]
    Exterior,
}

#[derive(Default, Serialize, Deserialize, TypePath, Asset)]
pub struct RealmData {
    /** Type of lighting for this realm. */
    pub lighting: RealmLighting,
}

#[derive(Component, Default, Asset, TypePath)]
pub struct Realm {
    /// Realm index, also used as layer index for rendering.
    pub layer_index: usize,

    /// Realm index, also used as layer index for rendering.
    pub layer: RenderLayers,

    /// Resource name of this realm.
    pub name: String,

    /// Type of lighting for this realm.
    pub lighting: RealmLighting,

    /// Boundary of the map, in parcels, relative to the world origin - sync'd from TerrainMap.
    pub parcel_bounds: IRect,

    /// Boundary of the map, in precincts, relative to the world origin - sync'd from TerrainMap.
    pub precinct_bounds: IRect,
}

/// Marker component for realms which are used for generating thumbnails and which aren't part of
/// the world.
#[derive(Component, Default, Asset, TypePath)]
pub struct HiddenRealm;

impl Realm {
    pub fn update_bounds(&mut self, parcel_bounds: IRect, precinct_bounds: IRect) {
        self.parcel_bounds = parcel_bounds;
        self.precinct_bounds = precinct_bounds;
    }
}

#[non_exhaustive]
#[derive(Debug, Error)]
pub enum RealmsLoaderError {
    #[error("Could not load realm: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Default)]
pub struct RealmsLoader;

impl AssetLoader for RealmsLoader {
    type Asset = RealmData;
    type Error = RealmsLoaderError;
    type Settings = ();

    async fn load<'a>(
        &'a self,
        reader: &'a mut Reader<'_>,
        _settings: &'a Self::Settings,
        _load_context: &'a mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        let map: RealmData = serde_json::from_slice(&bytes).expect("unable to decode RealmData");
        Ok(map)
    }

    fn extensions(&self) -> &[&str] {
        &["realm.json"]
    }
}

#[derive(Resource)]
pub struct RealmsHandleResource(pub Handle<LoadedFolder>);

impl FromWorld for RealmsHandleResource {
    fn from_world(world: &mut World) -> Self {
        let server = world.resource::<AssetServer>();
        RealmsHandleResource(server.load_folder("realms"))
    }
}

pub fn sync_realms(
    mut commands: Commands,
    server: Res<AssetServer>,
    assets: ResMut<Assets<RealmData>>,
    mut q_realms: Query<(Entity, &mut Realm)>,
    q_lights: Query<(Entity, &RenderLayers), With<DirectionalLight>>,
    mut r_layers: ResMut<ReservedLayers>,
    mut ev_asset: EventReader<AssetEvent<RealmData>>,
) {
    for ev in ev_asset.read() {
        match ev {
            AssetEvent::Added { id } | AssetEvent::LoadedWithDependencies { id } => {
                let realm = assets.get(*id).unwrap();
                let realm_name = realm_name_from_handle(&server, id);

                // Assign first unused id.
                let layer_index = r_layers.next_unused();

                let mut exists = false;
                for (_, mut comp) in q_realms.iter_mut() {
                    if comp.name == realm_name {
                        comp.lighting = realm.lighting;
                        exists = true;
                    }
                }

                if !exists {
                    println!("Realm created: [{}], layer={}.", realm_name, layer_index);
                    let render_layer = RenderLayers::layer(layer_index);
                    commands.spawn(Realm {
                        layer_index,
                        layer: render_layer.clone(),
                        name: realm_name.clone(),
                        lighting: realm.lighting,
                        parcel_bounds: IRect::default(),
                        precinct_bounds: IRect::default(),
                    });

                    // Light for realm
                    commands.spawn((
                        DirectionalLightBundle {
                            directional_light: DirectionalLight {
                                shadows_enabled: true,
                                color: Srgba::WHITE.into(),
                                illuminance: 3000.,
                                ..default()
                            },
                            transform: Transform {
                                translation: Vec3::new(0.0, 2.0, 0.0),
                                rotation: Quat::from_rotation_x(-PI / 3.),
                                ..default()
                            },
                            // The default cascade config is designed to handle large scenes.
                            // As this example has a much smaller world, we can tighten the shadow
                            // bounds for better visual quality.
                            cascade_shadow_config: CascadeShadowConfigBuilder {
                                first_cascade_far_bound: 4.0,
                                maximum_distance: 40.0,
                                ..default()
                            }
                            .into(),
                            ..default()
                        },
                        render_layer,
                    ));
                }
            }

            AssetEvent::Modified { id } => {
                let realm = assets.get(*id).unwrap();
                let realm_name = realm_name_from_handle(&server, id);
                println!("Realm modified: [{}].", realm_name);
                for (_, mut comp) in q_realms.iter_mut() {
                    if comp.name == realm_name {
                        comp.lighting = realm.lighting;
                    }
                }
            }

            AssetEvent::Removed { id } => {
                let realm_name = realm_name_from_handle(&server, id);
                println!("Realm removed: [{}].", realm_name);
                let mut layers_to_remove = RenderLayers::none();
                for (entity, comp) in q_realms.iter_mut() {
                    if comp.name == realm_name {
                        r_layers.release(comp.layer_index);
                        layers_to_remove = layers_to_remove.with(comp.layer_index);
                        commands.entity(entity).despawn_recursive();
                    }
                }
                for (entity, layers) in q_lights.iter() {
                    if layers.intersects(&layers_to_remove) {
                        commands.entity(entity).despawn_recursive();
                    }
                }
            }

            AssetEvent::Unused { id: _ } => {}
        }
    }
}

fn realm_name_from_handle(server: &Res<AssetServer>, handle: &AssetId<RealmData>) -> String {
    let asset_path = server.get_path(*handle).unwrap();
    let path = asset_path.path();
    let filename = path.file_name().expect("Asset has no file name!");
    let filename_str = filename.to_str().unwrap();
    let dot = filename_str.find('.').unwrap_or(filename_str.len());
    filename_str[0..dot].to_string()
}
