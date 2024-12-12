use bevy::{
    asset::{io::Reader, AssetLoader, LoadContext},
    prelude::*,
};
use core::str;
use std::sync::{Arc, Mutex};
use thiserror::Error;
use wasmtime::Module;

use crate::{compiler::CompilationError, CompilationUnit};

pub struct Vm {
    engine: wasmtime::Engine,
    linker: wasmtime::Linker<wasmtime::Engine>,
}

#[derive(Resource)]
pub struct SagaVmResource(pub Arc<Mutex<Vm>>);

impl SagaVmResource {}

#[derive(TypePath, Asset)]
pub struct ScriptAsset(Module);

#[non_exhaustive]
#[derive(Debug, Error)]
pub enum SagaLoaderError {
    #[error("Could not load exemplar: {0}")]
    Io(#[from] std::io::Error),
    #[error("Could not decode Lua source from UTF-8: {0}")]
    DecodeUtf8(#[from] std::str::Utf8Error),
    #[error("Wasm Error: {0}")]
    Wasm(#[from] wasmtime::Error),
    #[error("Compilation Error: {0}")]
    Compilation(#[from] CompilationError),
}

pub struct SagaLoader {
    vm: Arc<Mutex<Vm>>,
}

impl FromWorld for SagaLoader {
    fn from_world(world: &mut World) -> Self {
        let resource = world.get_resource::<SagaVmResource>().unwrap();
        Self {
            vm: resource.0.clone(),
        }
    }
}

impl AssetLoader for SagaLoader {
    type Asset = ScriptAsset;
    type Error = SagaLoaderError;
    type Settings = ();

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &Self::Settings,
        _load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        let src = str::from_utf8(&bytes)?;
        let mut unit = CompilationUnit::new();
        unit.compile(src).await?;
        let vm = self.vm.lock().unwrap();
        let module = Module::new(&vm.engine, bytes)?;
        Ok(ScriptAsset(module))
    }

    fn extensions(&self) -> &[&str] {
        &["saga"]
    }
}

pub struct WasmLoader {
    engine: Arc<Mutex<Vm>>,
}

impl FromWorld for WasmLoader {
    fn from_world(world: &mut World) -> Self {
        let resource = world.get_resource::<SagaVmResource>().unwrap();
        Self {
            engine: resource.0.clone(),
        }
    }
}

impl AssetLoader for WasmLoader {
    type Asset = ScriptAsset;
    type Error = SagaLoaderError;
    type Settings = ();

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &Self::Settings,
        _load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        let vm = self.engine.lock().unwrap();
        let module = Module::new(&vm.engine, bytes)?;
        Ok(ScriptAsset(module))
    }

    fn extensions(&self) -> &[&str] {
        &["wasm"]
    }
}
