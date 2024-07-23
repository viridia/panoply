use bevy::{gltf::GltfMesh, prelude::*};

mod instance;
mod layers;
mod model_loader;
mod plugin;

pub use instance::*;
pub use layers::*;

// export interface IModelInstance {
//     component: IModelComponent;
//     context?: any;
//     animationGroup?: Group;
//   }

/** Specifies the position and placement of a model instance. */
pub struct ModelInstance {
    /// Path to gltf asset, including fragment identifier
    pub model: String,

    /// Transform for model
    pub transform: Transform,

    /// Visibility flag, default is true
    pub visible: bool,
}

// /** Used in archetypes to define the set of models displayed by that entity. */
// export interface IModelComponent {
//     /** ID of the model to display. */
//     model: string;

//     /** Model transformation parameters. */
//     xRotation?: number;
//     yRotation?: number;
//     zRotation?: number;
//     xRotationVariance?: number;
//     yRotationVariance?: number;
//     zRotationVariance?: number;
//     offset?: Vector3;
//     scale?: number;
//     scaleVariance?: number;
//   }

// /** List of instances for a given 3d model.

//     - A ModelRef can contains multiple components, each of which has a set of position
//       parameters.
//     - A ModelRef can be instantiated multiple times, with different parameters for each
//       instance.

//     Thus, the number of actual mesh instances is the product of these two sets.
// */
struct ModelInstanceList {
    mesh: Handle<GltfMesh>,
    // rebuild: boolean;
    static_instances: Vec<ModelInstance>,
    static_instance_count: usize,
    // animatedInstances: IModelInstance[];
    // animationGroups: Group[];
    // instanceMeshes: InstancedMesh[];
    // instanceTransforms: Matrix4[];
    // boundingSphere: Sphere;
    // boundingBox: Box3;
    // modified?: boolean;
}
