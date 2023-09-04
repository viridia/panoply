use bevy::asset::io::Reader;
use bevy::asset::{AssetLoader, LoadContext, LoadedFolder};
use bevy::prelude::*;
use bevy::reflect::TypePath;
use bevy::utils::BoxedFuture;
use futures_lite::AsyncReadExt;
use serde::{Deserialize, Serialize};

pub type RealmLayer = usize;

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
    /** Realm index, also used as layer index for rendering. */
    pub layer: RealmLayer,

    /** Resource name of this realm. */
    pub name: String,

    /** Type of lighting for this realm. */
    pub lighting: RealmLighting,
}

#[derive(Default)]
pub struct RealmsLoader;

impl AssetLoader for RealmsLoader {
    type Asset = RealmData;
    type Settings = ();

    fn load<'a>(
        &'a self,
        reader: &'a mut Reader,
        _settings: &'a Self::Settings,
        _load_context: &'a mut LoadContext,
    ) -> BoxedFuture<'a, Result<Self::Asset, anyhow::Error>> {
        Box::pin(async move {
            let mut bytes = Vec::new();
            reader.read_to_end(&mut bytes).await?;
            let map: RealmData =
                serde_json::from_slice(&bytes).expect("unable to decode RealmData");
            Ok(map)
        })
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
                // Assign first unused id.
                let mut layer: RealmLayer = 1;
                loop {
                    let mut collision = false;
                    for (_, realm) in query.iter() {
                        if realm.layer == layer {
                            layer += 1;
                            collision = true;
                        }
                    }
                    if !collision {
                        break;
                    }
                }

                let realm = assets.get(*id).unwrap();
                let realm_name = realm_name_from_handle(&server, id);
                println!("Realm created: [{}].", realm_name);
                commands.spawn(Realm {
                    layer,
                    name: realm_name,
                    lighting: realm.lighting,
                });
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
        }
    }
}

fn realm_name_from_handle(server: &Res<AssetServer>, handle: &AssetId<RealmData>) -> String {
    let asset_path = server.get_path(*handle).unwrap();
    let path = asset_path.path();
    let filename = path.file_name().expect("Asset has no file name!");
    let filename_str = filename.to_str().unwrap();
    let dot = filename_str.find(".").unwrap_or(filename_str.len());
    return filename_str[0..dot].to_string();
}
