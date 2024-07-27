use std::io::Cursor;

use base64::{
    alphabet,
    engine::{general_purpose::NO_PAD, GeneralPurpose},
    DecodeError, Engine,
};
use bevy::{
    asset::{embedded_asset, io::AssetReader},
    pbr::ExtendedMaterial,
    prelude::*,
    render::render_resource::Face,
};
use futures_lite::{AsyncRead, AsyncSeek};
use serde::{de::DeserializeOwned, Serialize};
use thiserror::Error;

mod flame;
mod floor_noisy;
mod floor_std;
mod outline;

pub use flame::FlameMaterial;
pub use floor_noisy::FloorNoisyMaterial;
use floor_noisy::FloorNoisyMaterialLoader;
pub use floor_noisy::FloorNoisyMaterialParams;
use floor_std::FloorStdMaterialLoader;
pub use floor_std::FloorStdMaterialParams;
pub use outline::{OutlineMaterial, OutlineMaterialExtension};

#[derive(Debug, Clone, Resource, Default)]
pub struct OutlineMaterialHandle(pub Handle<OutlineMaterial>);

#[derive(Debug, Clone, Resource, Default)]
pub struct BlackMaterialHandle(pub Handle<StandardMaterial>);

#[derive(Debug, Clone, Resource, Default)]
pub struct FlameMaterialHandle(pub Handle<FlameMaterial>);

pub struct MaterialsPlugin;

impl Plugin for MaterialsPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "flame.wgsl");
        embedded_asset!(app, "flame_prepass.wgsl");
        embedded_asset!(app, "outline.wgsl");
        app.init_resource::<OutlineMaterialHandle>()
            .init_resource::<BlackMaterialHandle>()
            .init_resource::<FlameMaterialHandle>()
            .init_asset_loader::<FloorStdMaterialLoader>()
            .init_asset_loader::<FloorNoisyMaterialLoader>()
            .add_plugins(MaterialPlugin::<FloorNoisyMaterial>::default())
            .add_plugins(MaterialPlugin::<OutlineMaterial>::default())
            .add_plugins(MaterialPlugin::<FlameMaterial>::default())
            .add_systems(
                Startup,
                (
                    create_outline_material,
                    create_black_material,
                    create_flame_material,
                ),
            );
    }
}

pub const URL_SAFE_NO_PAD: GeneralPurpose = GeneralPurpose::new(&alphabet::URL_SAFE, NO_PAD);

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

impl AsyncSeek for NullReader {
    fn poll_seek(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
        _pos: std::io::SeekFrom,
    ) -> std::task::Poll<std::io::Result<u64>> {
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

#[non_exhaustive]
#[derive(Debug, Error)]
pub enum InlineAssetError {
    #[error("Could not decode inline asset: {0}")]
    DecodeMsgpack(#[from] rmp_serde::decode::Error),
    #[error("Could not decode base64: {0}")]
    DecodeBase64(#[from] DecodeError),
    #[error("Could not encode inline asset: {0}")]
    EncodeMsgpack(#[from] rmp_serde::encode::Error),
}

pub trait InlineAssetParams
where
    Self: Sized + Serialize + DeserializeOwned,
{
    fn encode(&self) -> Result<String, InlineAssetError> {
        let bytes = rmp_serde::to_vec(self)?;
        Ok(URL_SAFE_NO_PAD.encode(bytes))
    }

    fn decode(encoded: &str) -> Result<Self, InlineAssetError> {
        let bytes = URL_SAFE_NO_PAD.decode(encoded)?;
        Ok(rmp_serde::from_read(Cursor::new(bytes))?)
    }
}

fn create_outline_material(
    mut materials: ResMut<Assets<OutlineMaterial>>,
    mut r_outline: ResMut<OutlineMaterialHandle>,
) {
    r_outline.0 = materials.add(ExtendedMaterial {
        base: StandardMaterial {
            base_color: Color::BLACK,
            unlit: true,
            cull_mode: Some(Face::Front),
            ..Default::default()
        },
        extension: OutlineMaterialExtension { width: 0.015 },
    });
}

fn create_black_material(
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut r_black: ResMut<BlackMaterialHandle>,
) {
    r_black.0 = materials.add(StandardMaterial {
        base_color: Srgba::BLACK.into(),
        unlit: true,
        ..Default::default()
    });
}

pub fn create_flame_material(
    mut materials: ResMut<Assets<FlameMaterial>>,
    mut resource: ResMut<FlameMaterialHandle>,
    asset_server: Res<AssetServer>,
) {
    resource.0 = materials.add(FlameMaterial {
        noise: asset_server.load("textures/noise_1.png"),
    });
}
