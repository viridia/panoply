use bevy::{asset::io::AssetReader, pbr::ExtendedMaterial, prelude::*};
use futures_lite::AsyncRead;

mod floor;
mod outline;

use self::floor::FloorMaterialLoader;
pub use self::outline::OutlineMaterial;
pub use floor::FloorMaterialParams;

pub struct MaterialsPlugin;

impl Plugin for MaterialsPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset_loader::<FloorMaterialLoader>()
            .add_plugins(MaterialPlugin::<
                ExtendedMaterial<StandardMaterial, OutlineMaterial>,
            >::default());
    }
}

/// An asset reader that doesn't actually read anything, merely passes through the asset path
/// as the asset data.
pub struct InlineAssetReader;

/// An AsyncReader that always returns EOF.
struct NullReader;

impl AsyncRead for NullReader {
    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
        _buf: &mut [u8],
    ) -> std::task::Poll<std::io::Result<usize>> {
        std::task::Poll::Ready(Ok(0))
    }
}

impl AssetReader for InlineAssetReader {
    async fn read<'a>(
        &'a self,
        _path: &'a std::path::Path,
    ) -> Result<Box<bevy::asset::io::Reader<'a>>, bevy::asset::io::AssetReaderError> {
        Ok(Box::new(NullReader))
    }

    async fn read_meta<'a>(
        &'a self,
        path: &'a std::path::Path,
    ) -> Result<Box<bevy::asset::io::Reader<'a>>, bevy::asset::io::AssetReaderError> {
        Err(bevy::asset::io::AssetReaderError::NotFound(path.to_owned()))
    }

    async fn read_directory<'a>(
        &'a self,
        _path: &'a std::path::Path,
    ) -> Result<Box<bevy::asset::io::PathStream>, bevy::asset::io::AssetReaderError> {
        unreachable!("Reading directories is not supported by ComputedAssetReader.")
    }

    async fn is_directory<'a>(
        &'a self,
        _path: &'a std::path::Path,
    ) -> Result<bool, bevy::asset::io::AssetReaderError> {
        Ok(false)
    }
}
