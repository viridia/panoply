use bevy::{
    asset::{io::Reader, Asset, AssetLoader, Handle, LoadContext},
    gltf::{Gltf, GltfLoader},
    reflect::TypePath,
    scene::Scene,
    utils::BoxedFuture,
};
use thiserror::Error;

/// A `ModelRef` is a reference to a GLTF model. The reference contains a handle to the
/// GLTF scene object, as well as a reference to the GLTF asset itself. The latter is kept
/// so that it stays in memory in case we need to load other models from the same file.
#[derive(Default, TypePath, Asset)]
pub struct ModelRef {
    pub model: Handle<Scene>,
    pub asset: Handle<Gltf>,
}

pub struct ModelLoader;

#[non_exhaustive]
#[derive(Debug, Error)]
pub enum ModelLoaderError {
    #[error("Could load locations: {0}")]
    Io(#[from] std::io::Error),
    // #[error("Could not extract image: {0}")]
    // Image(#[from] image::ImageError),
}

/// A loader which is able to load individual scenes, by name, from within a GLTF file, and
/// perform transformations on the scene data.
impl AssetLoader for ModelLoader {
    type Asset = ModelRef;
    type Error = ModelLoaderError;
    type Settings = ();

    fn load<'a>(
        &'a self,
        reader: &'a mut Reader,
        _settings: &'a Self::Settings,
        load_context: &'a mut LoadContext,
    ) -> BoxedFuture<'a, Result<Self::Asset, Self::Error>> {
        Box::pin(async move {
            let mut gltf = load_context.load::<Gltf>(load_context.asset_path()).await?;
            // let mut bytes = Vec::new();
            // reader.read_to_end(&mut bytes).await?;
            // let biomes: Vec<BiomeData> =
            //     serde_json::from_slice(&bytes).expect("unable to decode biomes");

            Ok(ModelRef {
                model: gltf.default_scene.clone(),
                asset: gltf.into(),
            })
        })
    }

    fn extensions(&self) -> &[&str] {
        &["glb", "gltf"]
    }
}
