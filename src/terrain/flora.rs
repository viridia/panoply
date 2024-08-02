use std::sync::{Arc, Mutex};

use crate::{
    models::PropagateRenderLayers,
    random::{noise3, WeightedChoice},
    world::Realm,
};

use super::{
    biome::{BiomesAsset, BiomesHandle, BiomesTable},
    parcel::{Parcel, ParcelFloraChanged, ShapeRef},
    rotator::RotatingSquareArray,
    terrain_contours::{
        FloraType, TerrainContoursHandle, TerrainContoursTable, TerrainContoursTableAsset,
    },
    terrain_map::TerrainMap,
    ParcelTerrainFx, RebuildParcelTerrainFx, PARCEL_HEIGHT_SCALE, PARCEL_SIZE, PARCEL_SIZE_F,
    PARCEL_SIZE_U,
};
use bevy::{
    asset::LoadState,
    prelude::*,
    tasks::{AsyncComputeTaskPool, Task},
    utils::HashMap,
};
use futures_lite::future;

#[derive(Component)]
pub struct ComputeFloraTask(Task<Option<FloraPlacementResult>>);

#[derive(Component, Clone, Default)]
pub struct ParcelFlora;

pub struct FloraPlacementResult {
    /// Map of model resource names to instances, used in building the instance components.
    /// Each terrain parcel or scenery precinct will have one of these, which specifies how many
    /// instances of each model are placed in the world, and where they are located.
    models: HashMap<String, Vec<Transform>>,
}

#[derive(Debug, Component, Default)]
pub struct FloraInstance {
    pub handle: Handle<Gltf>,
    pub label: String,
    pub transform: Transform,
}

/// Spawns a task for each parcel to compute the ground mesh geometry.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn gen_flora(
    mut commands: Commands,
    mut q_parcels: Query<
        (Entity, &mut Parcel),
        (With<ParcelFloraChanged>, Without<RebuildParcelTerrainFx>),
    >,
    q_realms: Query<(&Realm, &TerrainMap)>,
    server: Res<AssetServer>,
    ts_handle: Res<TerrainContoursHandle>,
    ts_assets: Res<Assets<TerrainContoursTableAsset>>,
    bm_handle: Res<BiomesHandle>,
    bm_assets: Res<Assets<BiomesAsset>>,
) {
    let pool = AsyncComputeTaskPool::get();

    for (entity, parcel) in q_parcels.iter_mut() {
        let realm = q_realms.get(parcel.realm);
        if realm.is_err() {
            return;
        }

        if server.load_state(&ts_handle.0) != LoadState::Loaded
            || server.load_state(&bm_handle.0) != LoadState::Loaded
        {
            return;
        }

        let contours = ts_assets
            .get(&ts_handle.0)
            .expect("contours asset required")
            .0
            .clone();

        let biomes = bm_assets
            .get(&bm_handle.0)
            .expect("biomes asset required")
            .0
            .clone();

        let shape_ref = parcel.center_shape();
        let biome_indices = parcel.biomes;
        let coords = IVec2::new(parcel.coords.x * PARCEL_SIZE, parcel.coords.y * PARCEL_SIZE);
        let terrain_fx = parcel.terrain_fx;
        let task = pool.spawn(async move {
            let mut result = FloraPlacementResult {
                models: HashMap::new(),
            };
            if compute_flora_placement(
                coords,
                shape_ref,
                &contours,
                &terrain_fx,
                biome_indices,
                &biomes,
                &mut result,
            ) {
                Some(result)
            } else {
                None
            }
        });
        commands
            .entity(entity)
            .insert(ComputeFloraTask(task))
            .remove::<ParcelFloraChanged>();
    }
}

/// Consumes the output of the compute task and creates instances for trees and.
pub fn insert_flora(
    mut commands: Commands,
    mut q_parcels: Query<(Entity, &mut Parcel, &mut ComputeFloraTask)>,
    q_realms: Query<(&Realm, &TerrainMap)>,
    server: Res<AssetServer>,
) {
    for (entity, mut parcel, mut task) in q_parcels.iter_mut() {
        if let Ok((realm, _terrain)) = q_realms.get(parcel.realm) {
            if let Some(task_result) = future::block_on(future::poll_once(&mut task.0)) {
                // Remove existing flora
                if let Some(flora_entity) = parcel.flora_entity {
                    // println!("Dropping flora {}", flora_entity);
                    commands.entity(flora_entity).despawn_descendants();
                }

                // If there is flora
                if let Some(mut flora_placement) = task_result {
                    // Get or create flora entity.
                    let flora_entity = match parcel.flora_entity {
                        Some(flora_ent) => flora_ent,
                        None => {
                            let child = commands
                                .spawn((
                                    SpatialBundle { ..default() },
                                    ParcelFlora,
                                    realm.layer.clone(),
                                ))
                                .id();
                            commands.entity(entity).add_child(child);
                            parcel.flora_entity = Some(child);
                            child
                        }
                    };

                    // Generate model placements for flora.
                    let count = flora_placement.models.iter().fold(0, |n, c| n + c.1.len());
                    let mut children = Vec::<Entity>::with_capacity(count);
                    for (model, value) in flora_placement.models.drain() {
                        if let Some((fname, fragment)) = model.split_once('#') {
                            let handle: Handle<Gltf> = server.load(fname.to_owned());
                            for transform in value {
                                children.push(
                                    commands
                                        .spawn((
                                            FloraInstance {
                                                handle: handle.clone(),
                                                label: fragment.to_string(),
                                                transform,
                                            },
                                            realm.layer.clone(),
                                        ))
                                        .id(),
                                );
                            }
                        }
                    }
                    commands.entity(flora_entity).replace_children(&children);
                }

                commands.entity(entity).remove::<ComputeFloraTask>();
            }
        }
    }
}

pub fn spawn_flora_model_instances(
    mut commands: Commands,
    mut q_flora_instance: Query<(Entity, &FloraInstance), Without<Handle<Scene>>>,
    assets_gltf: Res<Assets<Gltf>>,
    server: Res<AssetServer>,
) {
    for (entity, mesh) in q_flora_instance.iter_mut() {
        let result = server.load_state(&mesh.handle);
        if result == LoadState::Loaded {
            let asset = assets_gltf.get(&mesh.handle);
            if let Some(gltf) = asset {
                if let Some(scene_handle) = gltf.named_scenes.get(mesh.label.as_str()) {
                    commands.entity(entity).insert((
                        SceneBundle {
                            scene: scene_handle.clone(),
                            transform: mesh.transform,
                            ..Default::default()
                        },
                        PropagateRenderLayers,
                    ));
                } else {
                    error!("Model not found: [{}]", mesh.label);
                    info!("Available scenes: [{:?}]", gltf.named_scenes.keys());
                    commands.entity(entity).despawn();
                    // panic!();
                }
            }
        }
    }
}

fn compute_flora_placement(
    origin: IVec2,
    shape_ref: ShapeRef,
    contours: &Arc<Mutex<TerrainContoursTable>>,
    terrain_fx: &ParcelTerrainFx,
    biome_indices: [u8; 4],
    biomes: &Arc<Mutex<BiomesTable>>,
    out: &mut FloraPlacementResult,
) -> bool {
    let contours_table = contours.lock().unwrap();
    let biomes_table = biomes.lock().unwrap();
    let center = contours_table.get(shape_ref.shape as usize);
    if !center.has_terrain {
        return false;
    }

    // Flora array
    let flora = RotatingSquareArray::new(
        PARCEL_SIZE_U,
        shape_ref.rotation as i32,
        center.flora.elts(),
    );

    let heights = RotatingSquareArray::new(
        center.height.size(),
        shape_ref.rotation as i32,
        center.height.elts(),
    );

    for x in 0..PARCEL_SIZE_U {
        for z in 0..PARCEL_SIZE_U {
            // Don't place flora on roads or other terrain fx.
            let fx = terrain_fx.get(x + 1, z + 1);
            if !fx.effect.is_empty() {
                continue;
            }
            let feature = flora.get(x, z);
            if feature == FloraType::None {
                continue;
            }

            // Weighted random selection of biome N or N+1.
            let gx = origin.x + x as i32;
            let gz = origin.y + z as i32;
            let xt: usize = if (x as f32 / PARCEL_SIZE_F + noise3(gx, gz, 5)) < 1. {
                0
            } else {
                1
            };
            let zt: usize = if (z as f32 / PARCEL_SIZE_F + noise3(gx, gz, 6)) < 1. {
                0
            } else {
                2
            };
            let biome_index = biome_indices[xt + zt] as usize;
            if biome_index >= biomes_table.biomes.len() {
                continue;
            }
            let biome = &biomes_table.biomes[biome_index];
            let feature_selection = noise3(gx, gz, 3);
            let feature_model = match feature {
                FloraType::None => unreachable!(),
                FloraType::RandomTree => WeightedChoice::choice(&biome.trees, feature_selection),
                FloraType::RandomShrub => WeightedChoice::choice(&biome.shrubs, feature_selection),
                FloraType::RandomHerb => WeightedChoice::choice(&biome.herbs, feature_selection),
            };

            let tx = x as f32 + 0.2 + noise3(gx, gz, 4) * 0.6;
            let tz = z as f32 + 0.2 + noise3(gx, gz, 5) * 0.6;
            let ty: f32 = heights.get_interpolated(tx, tz) * PARCEL_HEIGHT_SCALE;
            // + match feature {
            //     FloraType::None => unreachable!(),
            //     FloraType::RandomTree => 1.25,
            //     FloraType::RandomShrub => 0.5,
            //     FloraType::RandomHerb => 0.75,
            // };

            let translation = Vec3::new(tx, ty, tz);
            let rotation = Quat::IDENTITY;
            // let mut scale = Vec3::new(1., 1., 1.);
            if let Some(entry) = feature_model {
                if let Some(ref model) = entry.proto {
                    let scale = noise3(gx, gz, 7) * 0.3 + 0.5;
                    // println!("Biome: {gx} {gz} {}", model);
                    let entry = out
                        .models
                        .entry(model.clone())
                        .or_insert(Vec::with_capacity(6));
                    entry.push(Transform {
                        translation,
                        rotation,
                        scale: Vec3::new(scale, scale, scale),
                    })
                }
            }

            //     const flora = floraSelector.get(x, z);
            //     if (!flora) {
            //       continue;
            //     }
            //     const [type, model] = flora;
            //     if (model?.proto) {
            //       const tx = x + 0.2 + noise3(gx, gy, 4) * 0.6;
            //       const ty = z + 0.2 + noise3(gx, gy, 5) * 0.6;
            //       const position = new Vector3(tx, interp(tx, ty) * 0.5 - 0.05, ty);
            //       if (this.engine.models) {
            //         this.featureModels.addStaticComponents([
            //           {
            //             component: {
            //               model: model.proto,
            //               yRotation: noise3(gx, gy, 6) * Math.PI * 2,
            //               scale: noise3(gx, gy, 7) * 0.3 + 0.5,
            //             },
            //             position,
            //           },
            //         ]);
            //       }
            //       if (this.engine.models) {
            //         treePosition.copy(position);
            //         if (type === FloraType.RandomTree) {
            //           treePosition.y += 1.25;
            //           this.featureBillboards.add('pine.png', treePosition, new Vector2(1.5, 2.5));
            //         } else if (type === FloraType.RandomShrub) {
            //           treePosition.y += 0.5;
            //           this.featureBillboards.add('shrub.png', treePosition, new Vector2(1.0, 1.0));
            //         } else if (type === FloraType.RandomHerb) {
            //           treePosition.y += 0.75;
            //           this.featureBillboards.add('herb.png', treePosition, new Vector2(1.0, 1.5));
            //         }
            //       }
            //       if (type === FloraType.RandomTree) {
            //         const index = (z * 2 + 1) * TREE_ARRAY_STRIDE + x * 2 + 1;
            //         this.treeLocations[index] = 1;
            //         treePositions.push([x, z]);
            //       }
            //     }
        }
    }
    // for (let y = 0; y < PLOT_LENGTH; y += 1) {
    //   for (let x = 0; x < PLOT_LENGTH; x += 1) {
    //     const tx = x * 2 + 1;
    //     const ty = y * 2 + 1;
    //     const index = ty * TREE_ARRAY_STRIDE + tx;
    //     this.treeLocations[index + 1] = this.treeLocations[index] & this.treeLocations[index + 2];
    //     this.treeLocations[index + TREE_ARRAY_STRIDE] =
    //       this.treeLocations[index] & this.treeLocations[index + TREE_ARRAY_STRIDE * 2];
    //   }
    // }
    // for (let y = 1; y < PLOT_LENGTH; y += 1) {
    //   for (let x = 1; x < PLOT_LENGTH; x += 1) {
    //     const tx = x * 2;
    //     const ty = y * 2;
    //     const index = ty * TREE_ARRAY_STRIDE + tx;
    //     this.treeLocations[index] =
    //       (this.treeLocations[index - 1] & this.treeLocations[index + 1]) |
    //       (this.treeLocations[index - TREE_ARRAY_STRIDE] &
    //         this.treeLocations[index + TREE_ARRAY_STRIDE]);
    //   }
    // }
    // this.featureModels.buildMeshInstances();
    // this.featureBillboards.build();
    // this.treeCoords.set(treePositions);
    // // Kindof brute force here.
    // this.engine.navigation.invalidateTract(
    //   this.realm.name,
    //   Math.floor(this.origin.x / TRACT_SIZE),
    //   Math.floor(this.origin.y / TRACT_SIZE)
    // );

    // center.flora

    //     const floraArray = this.plot.flora;
    //     // Absolute coordinates for use with noise.
    //     const gx = this.origin.x + x;
    //     const gy = this.origin.y + y;

    //     // Weighted random selection of biome N or N+1.
    //     const xt = Math.floor(gx / PLOT_LENGTH + noise3(gx, gy, 5));
    //     const yt = Math.floor(gy / PLOT_LENGTH + noise3(gx, gy, 6));
    //     const biomeIndex = this.realm.getBiomeAt(xt, yt);
    //     const biome = this.world.getBiomeByIndex(biomeIndex) || this.world.getBiomeByIndex(0)!;

    //     // Get the flora type specified by the plot's flora map
    //     const { dx, dy, baseIndex } = this.accessor;
    //     const floraSelection = floraArray[baseIndex + x * dx + y * dy];

    //     // Pick a random flora model based on weighted probability in the biome table.
    //     if (floraSelection !== FloraType.None) {
    //       switch (floraSelection) {
    //         case FloraType.RandomTree:
    //           return [floraSelection, biome.getTree(gx, gy, 3)];
    //         case FloraType.RandomShrub:
    //           return [floraSelection, biome.getShrub(gx, gy, 3)];
    //         case FloraType.RandomHerb:
    //           return [floraSelection, biome.getHerb(gx, gy, 3)];
    //         default:
    //           return null;
    //       }
    //     }
    //     return null;
    //   }

    true
}
