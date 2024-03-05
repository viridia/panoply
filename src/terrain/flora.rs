use std::sync::{Arc, Mutex};

use crate::{
    instancing::{InstanceMap, ModelPlacement, ModelPlacementChanged, ModelPlacements},
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
    PARCEL_SIZE, PARCEL_SIZE_F,
};
use bevy::{
    asset::LoadState,
    prelude::*,
    tasks::{AsyncComputeTaskPool, Task},
};
use futures_lite::future;

#[derive(Component)]
pub struct ComputeFloraTask(Task<Option<FloraPlacementResult>>);

#[derive(Component, Clone, Default)]
pub struct ParcelFlora;

pub struct FloraPlacementResult {
    models: InstanceMap,
}

pub const HEIGHT_SCALE: f32 = 0.5;

/// Spawns a task for each parcel to compute the ground mesh geometry.
#[allow(clippy::too_many_arguments)]
pub fn gen_flora(
    mut commands: Commands,
    mut query: Query<(Entity, &mut Parcel), With<ParcelFloraChanged>>,
    realms_query: Query<(&Realm, &TerrainMap)>,
    server: Res<AssetServer>,
    ts_handle: Res<TerrainContoursHandle>,
    ts_assets: Res<Assets<TerrainContoursTableAsset>>,
    bm_handle: Res<BiomesHandle>,
    bm_assets: Res<Assets<BiomesAsset>>,
) {
    let pool = AsyncComputeTaskPool::get();

    for (entity, parcel) in query.iter_mut() {
        let realm = realms_query.get(parcel.realm);
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

        let shape_ref = parcel.contours[4];
        let biome_indices = parcel.biomes;
        let coords = IVec2::new(parcel.coords.x * PARCEL_SIZE, parcel.coords.y * PARCEL_SIZE);
        let task = pool.spawn(async move {
            let mut result = FloraPlacementResult {
                models: InstanceMap::new(),
            };
            if compute_flora_placement(
                coords,
                shape_ref,
                &contours,
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
    mut query: Query<(Entity, &mut Parcel, &mut ComputeFloraTask)>,
    realms_query: Query<(&Realm, &TerrainMap)>,
) {
    for (entity, mut parcel, mut task) in query.iter_mut() {
        let realm = realms_query.get(parcel.realm);
        if realm.is_ok() {
            if let Some(task_result) = future::block_on(future::poll_once(&mut task.0)) {
                // Remove existing flora
                if let Some(flora_entity) = parcel.flora_entity {
                    println!("Dropping flora");
                    commands.entity(flora_entity).despawn_descendants();
                }

                // If there is flora
                if let Some(mut flora_placement) = task_result {
                    // Get or create flora entity.
                    let flora_entity = match parcel.flora_entity {
                        Some(entity) => entity,
                        None => {
                            let child = commands
                                .spawn((SpatialBundle { ..default() }, ParcelFlora))
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
                        children.push(
                            commands
                                .spawn((
                                    SpatialBundle { ..default() },
                                    ModelPlacements {
                                        model: model.clone(),
                                        placement_list: value,
                                    },
                                    ModelPlacementChanged,
                                ))
                                .id(),
                        );
                    }
                    commands.entity(flora_entity).replace_children(&children);
                } else if let Some(flora_entity) = parcel.flora_entity {
                    // There was no flora so remove all entities.
                    commands.entity(flora_entity).despawn_descendants();
                }

                commands.entity(entity).remove::<ComputeFloraTask>();
            }
        }
    }
}

fn compute_flora_placement(
    origin: IVec2,
    shape_ref: ShapeRef,
    contours: &Arc<Mutex<TerrainContoursTable>>,
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
        PARCEL_SIZE as usize,
        shape_ref.rotation as i32,
        center.flora.elts(),
    );

    let heights = RotatingSquareArray::new(
        center.height.size(),
        shape_ref.rotation as i32,
        center.height.elts(),
    );

    for x in 0..PARCEL_SIZE {
        for z in 0..PARCEL_SIZE {
            //     const fxOffset = ftAccessor.indexOf(x + 1, z + 1) * 8;
            //     if (
            //       effect &&
            //       (effect[fxOffset + FX_ROAD] > 10 ||
            //         effect[fxOffset + FX_PATH] > 10 ||
            //         effect[fxOffset + FX_STONE] > 10)
            //     ) {
            //       continue;
            //     }
            let gx = origin.x + x;
            let gz = origin.y + z;

            let feature = flora.get(x, z);
            if feature == FloraType::None {
                continue;
            }

            // Weighted random selection of biome N or N+1.
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
            let ty: f32 = heights.get_interpolated(tx, tz) * HEIGHT_SCALE;
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
                    entry.push(ModelPlacement {
                        transform: Transform {
                            translation,
                            rotation,
                            scale: Vec3::new(scale, scale, scale),
                        },
                        visible: true,
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
