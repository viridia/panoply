use bevy::{
    asset::{io::Reader, Asset, AssetLoader, LoadContext},
    gltf::{Gltf, GltfError, GltfLoader, GltfLoaderSettings},
    reflect::TypePath,
    utils::thiserror::Error,
    utils::BoxedFuture,
};

/// A `ModelRef` is a reference to a GLTF model. The reference contains a handle to the
/// GLTF scene object, as well as a reference to the GLTF asset itself. The latter is kept
/// so that it stays in memory in case we need to load other models from the same file.
#[derive(TypePath, Asset)]
pub struct ModelRef(Gltf);

pub struct ModelLoader {
    inner: GltfLoader,
}

#[non_exhaustive]
#[derive(Debug, Error)]
pub enum ModelLoaderError {
    #[error("Could not load locations: {0}")]
    Io(#[from] std::io::Error),
}

/// A loader which is able to load individual scenes, by name, from within a GLTF file, and
/// perform transformations on the scene data.
impl AssetLoader for ModelLoader {
    type Asset = Gltf;
    type Error = GltfError;
    type Settings = ();

    fn load<'a>(
        &'a self,
        reader: &'a mut Reader,
        _settings: &'a Self::Settings,
        load_context: &'a mut LoadContext,
    ) -> BoxedFuture<'a, Result<Self::Asset, Self::Error>> {
        Box::pin(async move {
            let result = self
                .inner
                .load(
                    reader,
                    &GltfLoaderSettings {
                        load_cameras: false,
                        load_lights: false,
                        load_meshes: true,
                        include_source: false,
                    },
                    load_context,
                )
                .await;
            match result {
                Ok(gltf) => {
                    // info!("Loaded GLTF model: {:?}", gltf);
                    for _scene_handle in gltf.scenes.iter() {
                        // let scene = load_context.get_asset(&scene_handle).unwrap();
                        // info!("Scene: {:?}", scene_handle.name);
                    }
                    Ok(gltf)
                }
                Err(_) => result,
            }
        })
    }

    fn extensions(&self) -> &[&str] {
        &["glb", "gltf"]
    }
}

impl Default for ModelLoader {
    fn default() -> Self {
        Self {
            inner: GltfLoader {
                supported_compressed_formats: Default::default(),
                custom_vertex_attributes: Default::default(),
            },
        }
    }
}
