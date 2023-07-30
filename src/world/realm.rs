use bevy::asset::{AssetLoader, LoadContext, LoadedAsset};
use bevy::prelude::*;
use bevy::reflect::{TypePath, TypeUuid};
use bevy::utils::BoxedFuture;
use serde::{Deserialize, Serialize};

pub type RealmLayer = usize;

#[derive(Default, Serialize, Deserialize, Clone, Copy)]
pub enum RealmLighting {
    Interior,
    #[default]
    Exterior,
}

// #[derive(Default, Serialize, Deserialize)]
#[derive(Default, Serialize, Deserialize, TypeUuid, TypePath)]
#[uuid = "dace4039-ec2b-46ff-8dd1-92b8704e8b73"]
pub struct RealmData {
    /** Type of lighting for this realm. */
    pub lighting: RealmLighting,
}

#[derive(Component, Default)]
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
    fn load<'a>(
        &'a self,
        bytes: &'a [u8],
        load_context: &'a mut LoadContext,
    ) -> BoxedFuture<'a, Result<(), bevy::asset::Error>> {
        Box::pin(async move {
            let map: RealmData = serde_json::from_slice(bytes).expect("unable to decode RealmData");
            load_context.set_default_asset(LoadedAsset::new(map));
            Ok(())
        })
    }

    fn extensions(&self) -> &[&str] {
        &["realm.json"]
    }
}

#[derive(Resource, Default)]
pub struct RealmsHandleResource(pub Vec<HandleUntyped>);

pub fn load_realms(server: Res<AssetServer>, mut handle: ResMut<RealmsHandleResource>) {
    handle.0 = server.load_folder("realms").unwrap();
}

pub fn sync_realms(
    mut commands: Commands,
    server: Res<AssetServer>,
    assets: ResMut<Assets<RealmData>>,
    mut query: Query<(Entity, &mut Realm)>,
    mut ev_asset: EventReader<AssetEvent<RealmData>>,
) {
    for ev in ev_asset.iter() {
        match ev {
            AssetEvent::Created { handle } => {
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

                let realm = assets.get(handle).unwrap();
                let realm_name = realm_name_from_handle(&server, handle);
                println!("Realm created: [{}].", realm_name);
                commands.spawn(Realm {
                    layer,
                    name: realm_name,
                    lighting: realm.lighting,
                });
            }

            AssetEvent::Modified { handle } => {
                let realm = assets.get(handle).unwrap();
                let realm_name = realm_name_from_handle(&server, handle);
                println!("Realm modified: [{}].", realm_name);
                for (_, mut comp) in query.iter_mut() {
                    if comp.name == realm_name {
                        comp.lighting = realm.lighting;
                    }
                }
            }

            AssetEvent::Removed { handle } => {
                let realm_name = realm_name_from_handle(&server, handle);
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

fn realm_name_from_handle(server: &Res<AssetServer>, handle: &Handle<RealmData>) -> String {
    let asset_path = server.get_handle_path(handle).unwrap();
    let path = asset_path.path();
    let filename = path.file_name().expect("Asset has no file name!");
    let filename_str = filename.to_str().unwrap();
    let dot = filename_str.find(".").unwrap_or(filename_str.len());
    return filename_str[0..dot].to_string();
}
