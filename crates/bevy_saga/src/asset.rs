use crate::CompilationUnit;
use bevy::{
    asset::{io::Reader, AssetLoader, LoadContext},
    prelude::*,
};
use core::str;
use std::sync::{Arc, Mutex};
use thiserror::Error;
use wasmprinter::print_bytes;
use wasmtime::Module;

type StoreData = ();

pub struct Vm {
    engine: wasmtime::Engine,
    store: wasmtime::Store<StoreData>,
    linker: wasmtime::Linker<StoreData>,
}

#[derive(Resource)]
pub struct SagaVmResource(pub Arc<Mutex<Vm>>);

impl SagaVmResource {}

#[derive(TypePath, Asset)]
pub struct ScriptAsset {
    module: Module,
}

impl ScriptAsset {
    pub fn call<Params, Results>(
        &self,
        vm: &mut Vm,
        func_name: &str,
        args: Params,
    ) -> Result<Results, wasmtime::Error>
    where
        Params: wasmtime::WasmParams,
        Results: wasmtime::WasmResults,
    {
        let instance = vm.linker.instantiate(&mut vm.store, &self.module)?;
        let func = instance
            .get_typed_func::<Params, Results>(&mut vm.store, func_name)
            .unwrap();
        func.call(&mut vm.store, args)
    }
}

// All wasm objects operate within the context of a "store". Each
// `Store` has a type parameter to store host-specific data, which in
// this case we're using `4` for.
// let mut store = Store::new(&engine, 4);
// let instance = linker.instantiate(&mut store, &module)?;
// let hello = instance.get_typed_func::<(), ()>(&mut store, "hello")?;

// // And finally we can call the wasm!
// hello.call(&mut store, ())?;

#[non_exhaustive]
#[derive(Debug, Error)]
pub enum SagaLoaderError {
    #[error("Could not load exemplar: {0}")]
    Io(#[from] std::io::Error),
    #[error("Could not decode Lua source from UTF-8: {0}")]
    DecodeUtf8(#[from] core::str::Utf8Error),
    #[error("{0:?}")]
    Wasm(#[from] wasmtime::Error),
    #[error("Compilation failed")]
    Compilation,
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
        load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        let path = load_context.path().to_str().unwrap();
        let src = str::from_utf8(&bytes)?;
        let mut unit = CompilationUnit::new(path, src);
        let err = unit.compile().await;
        if let Err(err) = err {
            unit.report_error(&err);
            return Err(SagaLoaderError::Compilation);
        }
        let wasm = unit.module.emit_wasm();
        println!("{}", wasmprinter::print_bytes(&wasm).unwrap());
        let vm = self.vm.lock().unwrap();
        let module = Module::new(&vm.engine, wasm)?;
        // let instance = vm.linker.instantiate(&mut vm.store, &module)?;
        Ok(ScriptAsset { module })
    }

    fn extensions(&self) -> &[&str] {
        &["saga"]
    }
}

pub struct WasmLoader {
    vm: Arc<Mutex<Vm>>,
}

impl FromWorld for WasmLoader {
    fn from_world(world: &mut World) -> Self {
        let resource = world.get_resource::<SagaVmResource>().unwrap();
        Self {
            vm: resource.0.clone(),
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
        let vm = self.vm.lock().unwrap();
        let module = Module::new(&vm.engine, bytes)?;
        Ok(ScriptAsset { module })
    }

    fn extensions(&self) -> &[&str] {
        &["wasm"]
    }
}

pub struct SagaPlugin;

impl Plugin for SagaPlugin {
    fn build(&self, app: &mut App) {
        let engine = wasmtime::Engine::default();
        let store = wasmtime::Store::<StoreData>::new(&engine, ());
        let linker = wasmtime::Linker::new(&engine);
        let vm = Arc::new(Mutex::new(Vm {
            engine,
            store,
            linker,
        }));
        app.init_asset::<ScriptAsset>()
            .register_asset_loader(SagaLoader { vm: vm.clone() })
            .register_asset_loader(WasmLoader { vm: vm.clone() })
            .insert_resource(SagaVmResource(vm.clone()));
    }
}
