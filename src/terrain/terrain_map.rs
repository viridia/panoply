extern crate rmp_serde as rmps;
use bevy::{
    asset::{
        io::{AssetWriterError, Reader},
        saver::AssetSaver,
        AssetLoader, LoadContext, LoadedFolder, RecursiveDependencyLoadState,
    },
    math::IRect,
    prelude::*,
    reflect::TypePath,
    render::{
        render_asset::RenderAssetUsages,
        render_resource::{Extent3d, TextureDimension, TextureFormat},
        texture::ImageSampler,
    },
};
use futures_lite::{AsyncReadExt, AsyncWriteExt};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{scenery::PRECINCT_SIZE, world::Realm};

use super::{
    biome::{BiomesAsset, BiomesHandle},
    ground_material::GroundMaterial,
    parcel::{ShapeRef, ADJACENT_COUNT},
    PARCEL_SIZE,
};

#[derive(Default, Serialize, Deserialize, TypePath, Asset, Clone)]
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
    /// Returns true if the terrain map includes the given coordinates. The bounds is
    /// considered a half-open interval: a point at `min` is considered inside, at `max` is
    /// considered outside.
    pub fn contains_pt(&self, pt: IVec2) -> bool {
        pt.x >= self.bounds.min.x
            && pt.x < self.bounds.max.x
            && pt.y >= self.bounds.min.y
            && pt.y < self.bounds.max.y
    }

    /// Return the shape id and rotation at the given parcel coords
    pub fn shape_at(&self, pt: IVec2) -> ShapeRef {
        if self.contains_pt(pt) {
            let index = ((pt.y - self.bounds.min.y) * self.bounds.width() + pt.x
                - self.bounds.min.x) as usize;
            if index >= self.shapes.len() {
                println!("OOB: {:?} {:?}", pt, self.bounds);
            }
            let d = self.shapes[index];
            let shape = d >> 2;
            let rotation = (d & 3) as u8;
            return ShapeRef { shape, rotation };
        }
        ShapeRef {
            shape: self.default_shape,
            rotation: 0,
        }
    }

    /// Set the shape id and rotation at the given parcel coords.
    pub fn set_shape_at(&mut self, pt: IVec2, shape: ShapeRef) {
        if self.contains_pt(pt) {
            let index = ((pt.y - self.bounds.min.y) * self.bounds.width() + pt.x
                - self.bounds.min.x) as usize;
            self.shapes[index] = (shape.shape << 2) | shape.rotation as u16;
        }
    }

    /// Return the shape id at the given parcel coords as well as all neighboring parcels.
    pub fn adjacent_shapes(&self, out: &mut [ShapeRef; ADJACENT_COUNT], pt: IVec2) {
        for z in [-1, 0, 1] {
            for x in [-1, 0, 1] {
                out[(z * 3 + x + 4) as usize] = self.shape_at(pt + IVec2::new(x, z))
            }
        }
    }

    /// Return the biome index at the given parcel coords.
    pub fn biome_at(&self, pt: IVec2) -> u8 {
        if self.contains_pt(pt) {
            return self.biomes[((pt.y - self.bounds.min.y) * self.bounds.width() + pt.x
                - self.bounds.min.x) as usize];
        }
        self.default_biome
    }

    /// Return the biomes at the 4 corners of the parcel.
    pub fn adjacent_biomes(&self, pt: IVec2) -> [u8; 4] {
        [
            self.biome_at(pt + IVec2::new(0, 0)),
            self.biome_at(pt + IVec2::new(1, 0)),
            self.biome_at(pt + IVec2::new(0, 1)),
            self.biome_at(pt + IVec2::new(1, 1)),
        ]
    }
}

#[derive(Component, Default)]
pub struct TerrainMap {
    /** Asset data for terrain map. */
    pub handle: Handle<TerrainMapAsset>,

    /** Material to use when rendering terrain. */
    pub ground_material: Handle<GroundMaterial>,

    /** Flag indicating we need to rebuild the biome texture. */
    pub needs_rebuild_biomes: bool,
}

/// Marker component that a terrain map has changed.
#[derive(Component)]
#[component(storage = "SparseSet")]
pub struct TerrainMapChanged;

#[non_exhaustive]
#[derive(Debug, Error)]
pub enum TerrainMapLoaderError {
    #[error("Could not load terrain map: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Default)]
pub struct TerrainMapLoader;

impl AssetLoader for TerrainMapLoader {
    type Asset = TerrainMapAsset;
    type Error = TerrainMapLoaderError;
    type Settings = ();

    async fn load<'a>(
        &'a self,
        reader: &'a mut Reader<'_>,
        _settings: &'a Self::Settings,
        _load_context: &'a mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        let map: TerrainMapAsset =
            rmps::from_slice(&bytes).expect("unable to decode terrain map data");
        let area = (map.bounds.width() * map.bounds.height()) as usize;
        assert!(map.shapes.len() == area);
        assert!(map.biomes.len() == area);
        Ok(map)
    }

    fn extensions(&self) -> &[&str] {
        &["terrain"]
    }
}

#[derive(Default)]
pub struct TerrainMapSaver;

#[non_exhaustive]
#[derive(Debug, Error)]
pub enum TerrainMapSaverError {
    #[error("Could not save terrain map: {0}")]
    Io(#[from] std::io::Error),
    #[error("Could not encode terrain map: {0}")]
    Encode(#[from] rmps::encode::Error),
    #[error("Could not commit terrain map: {0}")]
    Commit(#[from] AssetWriterError),
}

impl AssetSaver for TerrainMapSaver {
    type Asset = TerrainMapAsset;
    type Error = TerrainMapSaverError;
    type Settings = ();

    type OutputLoader = TerrainMapLoader;

    async fn save<'a>(
        &'a self,
        writer: &'a mut bevy::asset::io::Writer,
        asset: bevy::asset::saver::SavedAsset<'a, Self::Asset>,
        _settings: &'a Self::Settings,
    ) -> Result<(), TerrainMapSaverError> {
        let v = rmps::encode::to_vec_named(&*asset)?;
        writer.write_all(&v).await?;
        Ok(())
    }
}

#[derive(Resource)]
pub struct TerrainMapsHandleResource(pub Handle<LoadedFolder>);

impl FromWorld for TerrainMapsHandleResource {
    fn from_world(world: &mut World) -> Self {
        let server = world.resource::<AssetServer>();
        TerrainMapsHandleResource(server.load_folder("terrain/maps"))
    }
}

/** Discover terrain map assets, load them, and bind them to realm entities. */
pub fn insert_terrain_maps(
    mut commands: Commands,
    server: Res<AssetServer>,
    mut query: Query<(Entity, &mut Realm), Without<TerrainMap>>,
    mut materials: ResMut<Assets<GroundMaterial>>,
    mut images: ResMut<Assets<Image>>,
    terrain_folder: Res<TerrainMapsHandleResource>,
    terrain_folder_asset: Res<Assets<LoadedFolder>>,
) {
    if let Some(st) = server.get_recursive_dependency_load_state(&terrain_folder.0) {
        if st != RecursiveDependencyLoadState::Loaded {
            return;
        }
    }

    let files: &LoadedFolder = terrain_folder_asset.get(&terrain_folder.0).unwrap();
    for (entity, realm) in query.iter_mut() {
        let terrain_path = format!("terrain/maps/{}.terrain", realm.name);
        if !files.handles.iter().any(|handle| {
            server
                .get_path(handle.id())
                .unwrap()
                .path()
                .to_str()
                .unwrap()
                == terrain_path
        }) {
            continue;
        }
        // println!("Inserting terrain map: [{}].", realm.name);
        commands.entity(entity).insert((
            TerrainMap {
                handle: server.load(terrain_path.to_string()),
                ground_material: create_ground_material(&mut materials, &mut images, &server),
                needs_rebuild_biomes: false,
            },
            TerrainMapChanged,
        ));
    }
}

/** React when terrain map assets change and update the realm entities. */
#[allow(clippy::too_many_arguments)]
pub fn update_terrain_maps(
    mut commands: Commands,
    server: Res<AssetServer>,
    mut query: Query<(Entity, &mut Realm, Option<&mut TerrainMap>)>,
    mut ev_asset: EventReader<AssetEvent<TerrainMapAsset>>,
    mut materials: ResMut<Assets<GroundMaterial>>,
    mut images: ResMut<Assets<Image>>,
    tm_assets: Res<Assets<TerrainMapAsset>>,
    asset_server: Res<AssetServer>,
) {
    for ev in ev_asset.read() {
        match ev {
            AssetEvent::Added { id } | AssetEvent::LoadedWithDependencies { id } => {
                let realm_name = asset_name_from_id(&server, id);
                if let Some((re, mut realm, terrain)) =
                    query.iter_mut().find(|r| r.1.name == realm_name)
                {
                    let tm = tm_assets.get(*id).unwrap();
                    if realm.parcel_bounds != tm.bounds {
                        realm.update_bounds(tm.bounds, convert_parcel_to_precinct(&tm.bounds))
                    }
                    if terrain.is_none() {
                        commands.entity(re).insert((
                            TerrainMap {
                                handle: server.get_id_handle(*id).unwrap(),
                                ground_material: create_ground_material(
                                    &mut materials,
                                    &mut images,
                                    &asset_server,
                                ),
                                needs_rebuild_biomes: false,
                            },
                            TerrainMapChanged,
                        ));
                        println!("Terrain map created: [{}].", realm_name);
                    } else {
                        println!("Terrain map changed: [{}].", realm_name);
                        commands.entity(re).insert(TerrainMapChanged);
                    }
                }
            }

            AssetEvent::Modified { id } => {
                let realm_name = asset_name_from_id(&server, id);
                if let Some((entity, mut realm, _)) =
                    query.iter_mut().find(|(_, r, _)| r.name == realm_name)
                {
                    // println!("Terrain map modified: [{}].", realm_name);
                    let tm = tm_assets.get(*id).unwrap();
                    if realm.parcel_bounds != tm.bounds {
                        realm.update_bounds(tm.bounds, convert_parcel_to_precinct(&tm.bounds))
                    }
                    commands.entity(entity).insert(TerrainMapChanged);
                }
            }

            AssetEvent::Removed { id } => {
                let map_name = asset_name_from_id(&server, id);
                println!("Terrain map removed: [{}].", map_name);
                for (entity, realm, _terrain) in query.iter_mut() {
                    if realm.name == map_name {
                        commands.entity(entity).remove::<TerrainMap>();
                        commands.entity(entity).remove::<TerrainMapChanged>();
                    }
                }
            }

            AssetEvent::Unused { id: _ } => {}
        }
    }
}

fn asset_name_from_id(server: &Res<AssetServer>, id: &AssetId<TerrainMapAsset>) -> String {
    let asset_path = server.get_path(*id).unwrap();
    let path = asset_path.path();
    let filename = path.file_name().expect("Asset has no file name!");
    let filename_str = filename.to_str().unwrap();
    let dot = filename_str.find('.').unwrap_or(filename_str.len());
    filename_str[0..dot].to_string()
}

pub fn update_ground_material(
    mut commands: Commands,
    mut query: Query<(Entity, &Realm, &mut TerrainMap), With<TerrainMapChanged>>,
    mut materials: ResMut<Assets<GroundMaterial>>,
    mut r_images: ResMut<Assets<Image>>,
    bm_handle: Res<BiomesHandle>,
    bm_assets: Res<Assets<BiomesAsset>>,
    tm_assets: Res<Assets<TerrainMapAsset>>,
) {
    if let Some(biomes_asset) = bm_assets.get(&bm_handle.0) {
        if let Ok(biomes) = biomes_asset.0.try_lock() {
            let biomes_table = &biomes.biomes;
            for (entity, _realm, terrain) in query.iter_mut() {
                if let Some(terr) = tm_assets.get(&terrain.handle) {
                    if let Some(m) = materials.get_mut(&terrain.ground_material) {
                        // println!("Updating material {}", realm.name);

                        if terr.bounds.width() > 0 && terr.bounds.height() > 0 {
                            let mut texture_data = Vec::<u8>::new();
                            let rows = terr.bounds.height() as usize;
                            let stride = terr.bounds.width() as usize;
                            texture_data.resize(rows * stride, 0);
                            for z in 0..rows {
                                for x in 0..stride {
                                    let bi = terr.biomes[z * stride + x];
                                    let surface = biomes_table[bi as usize].surface;
                                    texture_data[z * stride + x] = surface as u8;
                                }
                            }
                            let mut res = Image::new_fill(
                                Extent3d {
                                    width: terr.bounds.width() as u32,
                                    height: terr.bounds.height() as u32,
                                    depth_or_array_layers: 1,
                                },
                                TextureDimension::D2,
                                &texture_data,
                                TextureFormat::R8Uint,
                                RenderAssetUsages::default(),
                            );
                            res.sampler = ImageSampler::nearest();
                            r_images.insert(m.biomes.id(), res);
                        }

                        m.realm_offset =
                            Vec2::new(terr.bounds.min.x as f32 - 1., terr.bounds.min.y as f32 - 1.);
                    }
                }

                commands.entity(entity).remove::<TerrainMapChanged>();
            }
        }
    }
}

pub fn create_ground_material(
    materials: &mut Assets<GroundMaterial>,
    images: &mut Assets<Image>,
    asset_server: &AssetServer,
) -> Handle<GroundMaterial> {
    let mut res = Image::new_fill(
        Extent3d {
            width: 1u32,
            height: 1u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0u8],
        TextureFormat::R8Uint,
        RenderAssetUsages::default(),
    );
    res.sampler = ImageSampler::nearest();
    let biomes = images.add(res);

    materials.add(GroundMaterial {
        noise: asset_server.load("terrain/textures/noise.png"),
        grass: asset_server.load("terrain/textures/grass.png"),
        dirt: asset_server.load("terrain/textures/dirt.png"),
        moss: asset_server.load("terrain/textures/moss.png"),
        cobbles: asset_server.load("terrain/textures/cobbles.png"),
        water_color: Srgba::rgb(0.0, 0.1, 0.3).into(),
        realm_offset: Vec2::new(0., 0.),
        biomes,
    })
}

const PARCELS_PER_PRECINCT: i32 = PRECINCT_SIZE / PARCEL_SIZE;

fn convert_parcel_to_precinct(parcel_bounds: &IRect) -> IRect {
    IRect {
        min: IVec2::new(
            parcel_bounds.min.x / PARCELS_PER_PRECINCT,
            parcel_bounds.min.y / PARCELS_PER_PRECINCT,
        ),
        max: IVec2::new(
            (parcel_bounds.max.x + PARCELS_PER_PRECINCT - 1) / PARCELS_PER_PRECINCT,
            (parcel_bounds.max.y + PARCELS_PER_PRECINCT - 1) / PARCELS_PER_PRECINCT,
        ),
    }
}
