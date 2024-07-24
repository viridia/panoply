use bevy::prelude::*;

mod layers;
mod model_loader;

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
