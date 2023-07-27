extern crate rmp_serde as rmps;
use bevy::{
    asset::{AssetLoader, LoadContext, LoadedAsset},
    math::IRect,
    prelude::*,
    reflect::{TypePath, TypeUuid},
    utils::BoxedFuture,
};
use serde::{Deserialize, Serialize};

#[derive(Default, Serialize, Deserialize, TypeUuid, TypePath)]
#[uuid = "42827d83-1cb9-4d78-aa38-108ee87fbb2b"]
pub struct TerrainMapData {
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

#[derive(Component, Default)]
pub struct TerrainMap {
    /** Asset data for terrain map. */
    pub data: TerrainMapData,

    /** Whether this map has been modified in the editor. */
    pub modified: bool,

    /** Flag indicating we need to rebuild the biome texture. */
    pub needs_rebuild_biomes: bool,
    // private biomeTexture: DataTexture | null = null;
}

impl TerrainMap {
    pub fn _new(bounds: IRect) -> TerrainMap {
        Self {
            data: TerrainMapData {
                bounds,
                shapes: vec![0; (bounds.width() * bounds.height()) as usize],
                biomes: vec![0; (bounds.width() * bounds.height()) as usize],
                ..default()
            },
            ..default()
        }
    }
}

// pub fn load_terrain_maps() {}

#[derive(Default)]
pub struct TerrainMapLoader;

impl AssetLoader for TerrainMapLoader {
    fn load<'a>(
        &'a self,
        bytes: &'a [u8],
        load_context: &'a mut LoadContext,
    ) -> BoxedFuture<'a, Result<(), bevy::asset::Error>> {
        Box::pin(async move {
            let map: TerrainMapData =
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
