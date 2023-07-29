extern crate rmp_serde as rmps;
use bevy::{
    asset::{AssetLoader, LoadContext, LoadedAsset},
    math::IRect,
    prelude::*,
    reflect::{TypePath, TypeUuid},
    utils::BoxedFuture,
};
use serde::{Deserialize, Serialize};

use crate::world::Realm;

use super::parcel::{ShapeRef, ADJACENT_COUNT};

#[derive(Default, Serialize, Deserialize, TypeUuid, TypePath)]
#[uuid = "42827d83-1cb9-4d78-aa38-108ee87fbb2b"]
pub struct TerrainMapAsset {
    /** Boundary of the map relative to the world origin. */
    pub bounds: IRect,

    /** Array of indices to the terrain shapes table, includes both id and rotation. */
    pub shapes: Vec<u16>,

    /** Array of biome indices. */
    pub biomes: Vec<u8>,

    /** Terrain shape to use when off the edge of the map. */
    pub default_shape: u16,

    /** Biome to use when off the edge of the map. */
    pub default_biome: u8,
}

impl TerrainMapAsset {
    // Return the shape id at the given parcel coords
    pub fn shape_at(&self, pt: IVec2) -> ShapeRef {
        if self.bounds.contains(pt) {
            let d = self.shapes[((pt.y - self.bounds.min.y) * self.bounds.width() + pt.x
                - self.bounds.min.x) as usize];
            let shape = d >> 2;
            let rotation = (d & 3) as u8;
            return ShapeRef { shape, rotation };
        }
        ShapeRef {
            shape: self.default_shape,
            rotation: 0,
        }
    }

    // Return the shape id at the given parcel coords as well as all neighboring parcels.
    pub fn adjacent_shapes(&self, out: &mut [ShapeRef; ADJACENT_COUNT], pt: IVec2) {
        for z in [-1, -0, 1] {
            for x in [-1, -0, 1] {
                out[(z * 3 + x + 4) as usize] = self.shape_at(pt + IVec2::new(x, z))
            }
        }
    }
}

#[derive(Component, Default)]
pub struct TerrainMap {
    /** Asset data for terrain map. */
    pub handle: Handle<TerrainMapAsset>,

    /** Flag indicating we need to rebuild the biome texture. */
    pub needs_rebuild_biomes: bool,
    // private biomeTexture: DataTexture | null = null;
}

/// Marker component that a terrain map has changed.
#[derive(Component)]
pub struct TerrainMapChanged;

#[derive(Default)]
pub struct TerrainMapLoader;

impl AssetLoader for TerrainMapLoader {
    fn load<'a>(
        &'a self,
        bytes: &'a [u8],
        load_context: &'a mut LoadContext,
    ) -> BoxedFuture<'a, Result<(), bevy::asset::Error>> {
        Box::pin(async move {
            let map: TerrainMapAsset =
                rmps::from_slice(bytes).expect("unable to decode terrain map data");
            let area = (map.bounds.width() * map.bounds.height()) as usize;
            assert!(map.shapes.len() == area);
            assert!(map.biomes.len() == area);
            load_context.set_default_asset(LoadedAsset::new(map));
            Ok(())
        })
    }

    fn extensions(&self) -> &[&str] {
        &["terrain"]
    }
}

#[derive(Resource, Default)]
pub struct TerrainMapsHandleResource(pub Vec<HandleUntyped>);

pub fn load_terrain_maps(server: Res<AssetServer>, mut handle: ResMut<TerrainMapsHandleResource>) {
    handle.0 = server.load_folder("terrain/maps").unwrap();
}

pub fn insert_terrain_maps(
    mut commands: Commands,
    server: Res<AssetServer>,
    mut query: Query<(Entity, &mut Realm), Without<TerrainMap>>,
) {
    for (entity, realm) in query.iter_mut() {
        println!("Inserting terrain map: [{}].", realm.name);
        commands.entity(entity).insert(TerrainMap {
            handle: server.load(format!("terrain/maps/{}.terrain", realm.name)),
            needs_rebuild_biomes: false,
        });
    }
}

pub fn update_terrain_maps(
    mut commands: Commands,
    server: Res<AssetServer>,
    mut query: Query<(Entity, &mut Realm, Option<&mut TerrainMap>)>,
    mut ev_asset: EventReader<AssetEvent<TerrainMapAsset>>,
) {
    for ev in ev_asset.iter() {
        match ev {
            AssetEvent::Created { handle } => {
                let realm_name = asset_name_from_handle(&server, handle);
                if let Some(realm) = query.iter().find(|r| r.1.name == realm_name) {
                    commands.entity(realm.0).insert((
                        TerrainMap {
                            handle: handle.clone(),
                            needs_rebuild_biomes: false,
                        },
                        TerrainMapChanged,
                    ));
                    println!("Terrain map created: [{}].", realm_name);
                }
            }

            AssetEvent::Modified { handle } => {
                let realm_name = asset_name_from_handle(&server, handle);
                if let Some((entity, _, _)) = query.iter().find(|(_, r, _)| r.name == realm_name) {
                    println!("Terrain map modified: [{}].", realm_name);
                    commands.entity(entity).insert(TerrainMapChanged);
                }
            }

            AssetEvent::Removed { handle } => {
                let map_name = asset_name_from_handle(&server, handle);
                println!("Terrain map removed: [{}].", map_name);
                for (entity, realm, _terrain) in query.iter_mut() {
                    if realm.name == map_name {
                        commands.entity(entity).remove::<TerrainMap>();
                        commands.entity(entity).remove::<TerrainMapChanged>();
                    }
                }
            }
        }
    }
}

fn asset_name_from_handle(server: &Res<AssetServer>, handle: &Handle<TerrainMapAsset>) -> String {
    let asset_path = server.get_handle_path(handle).unwrap();
    let path = asset_path.path();
    let filename = path.file_name().expect("Asset has no file name!");
    let filename_str = filename.to_str().unwrap();
    let dot = filename_str.find(".").unwrap_or(filename_str.len());
    return filename_str[0..dot].to_string();
}
