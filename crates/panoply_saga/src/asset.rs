//! `.saga` modules as assets.

use core::str;
use std::sync::Arc;

use bevy::{
    asset::{AssetLoader, LoadContext, io::Reader},
    prelude::*,
};
use thiserror::Error;

use crate::{
    HostState, NoImports, ScriptModule,
    compiler::{self, compile_module, format_error},
};

#[derive(TypePath, Asset)]
pub struct ScriptModuleAsset {
    pub module: ScriptModule,
}

#[non_exhaustive]
#[derive(Debug, Error)]
pub enum ModuleLoaderError {
    #[error("Could not load script module: {0}")]
    Io(#[from] std::io::Error),
    #[error("Could not decode script source from UTF-8: {0}")]
    DecodeUtf8(#[from] core::str::Utf8Error),
    #[error("{1}")]
    Compilation(compiler::CompilationError, String),
}

/// Loads `.saga` modules, compiled against one host.
#[derive(TypePath)]
pub struct ScriptModuleLoader {
    pub host: Arc<HostState>,
}

impl AssetLoader for ScriptModuleLoader {
    type Asset = ScriptModuleAsset;
    type Error = ModuleLoaderError;
    type Settings = ();

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &Self::Settings,
        load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        let path = load_context.path().to_string();
        let src = str::from_utf8(&bytes)?;
        // TODO(PAN71): a resolver that loads imports through the load context.
        match compile_module(&path, src, &self.host, &NoImports).await {
            Ok(module) => Ok(ScriptModuleAsset { module }),
            Err(err) => {
                let message = format_error(&path, src, &err);
                Err(ModuleLoaderError::Compilation(err, message))
            }
        }
    }

    fn extensions(&self) -> &[&str] {
        &["saga"]
    }
}
