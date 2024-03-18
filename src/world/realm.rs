use bevy::asset::io::Reader;
use bevy::asset::{AssetLoader, LoadContext, LoadedFolder};
use bevy::prelude::*;
use bevy::reflect::TypePath;
use bevy::render::view::RenderLayers;
use futures_lite::AsyncReadExt;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Default, Serialize, Deserialize, Clone, Copy)]
pub enum RealmLighting {
    Interior,
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
    mut query: Query<(Entity, &mut Realm)>,
    mut ev_asset: EventReader<AssetEvent<RealmData>>,
) {
    for ev in ev_asset.read() {
        match ev {
            AssetEvent::Added { id } | AssetEvent::LoadedWithDependencies { id } => {
                let realm = assets.get(*id).unwrap();
                let realm_name = realm_name_from_handle(&server, id);

                // Assign first unused id.
                let mut layer: u8 = 1;
                loop {
                    let mut collision = false;
                    for (_, realm) in query.iter() {
                        if realm.layer.intersects(&RenderLayers::layer(layer)) {
                            layer += 1;
                            collision = true;
                        }
                    }
                    if !collision {
                        break;
                    }
                }

                let mut exists = false;
                for (_, mut comp) in query.iter_mut() {
                    if comp.name == realm_name {
                        comp.lighting = realm.lighting;
                        exists = true;
                    }
                }

                if !exists {
                    println!("Realm created: [{}].", realm_name);
                    commands.spawn(Realm {
                        layer: RenderLayers::layer(layer),
                        name: realm_name,
                        lighting: realm.lighting,
                        parcel_bounds: IRect::default(),
                        precinct_bounds: IRect::default(),
                    });
                }
            }

            AssetEvent::Modified { id } => {
                let realm = assets.get(*id).unwrap();
                let realm_name = realm_name_from_handle(&server, id);
                println!("Realm modified: [{}].", realm_name);
                for (_, mut comp) in query.iter_mut() {
                    if comp.name == realm_name {
                        comp.lighting = realm.lighting;
                    }
                }
            }

            AssetEvent::Removed { id } => {
                let realm_name = realm_name_from_handle(&server, id);
                println!("Realm removed: [{}].", realm_name);
                for (entity, comp) in query.iter_mut() {
                    if comp.name == realm_name {
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
