use thiserror::Error;

use bevy::asset::io::Reader;
use bevy::asset::{AssetLoader, LoadContext};
use bevy::prelude::*;

// pub fn test_script() -> Result<(), rustyscript::Error> {
//     Ok(())
// }

// #[derive(Resource)]
// pub struct ScriptVM(pub Runtime);

// #[derive(TypePath, Asset)]
// pub struct ScriptAsset(pub Module);

// #[non_exhaustive]
// #[derive(Debug, Error)]
// pub enum ScriptAssetError {
//     #[error("Could not load Lua asset: {0}")]
//     Load(#[from] std::io::Error),
//     #[error("Could not decode Lua source from UTF-8: {0}")]
//     DecodeUtf8(#[from] std::string::FromUtf8Error),
// }

// #[derive(Default)]
// pub struct ScriptAssetLoader;

// impl AssetLoader for ScriptAssetLoader {
//     type Asset = ScriptAsset;
//     type Error = ScriptAssetError;
//     type Settings = ();

//     async fn load(
//         &self,
//         reader: &mut dyn Reader,
//         _settings: &Self::Settings,
//         load_context: &mut LoadContext<'_>,
//     ) -> Result<Self::Asset, Self::Error> {
//         let mut bytes = Vec::new();
//         reader.read_to_end(&mut bytes).await?;
//         let src = String::from_utf8(bytes)?;
//         let module = Module::new(load_context.path().to_str().unwrap(), src);

//         // let runtime = settings.lock().unwrap();
//         // let module_handle = runtime.load_module(&module)?;

//         // let lua = Lua::new();
//         // let chunk = lua.load(&bytes);
//         Ok(ScriptAsset(module))
//     }

//     fn extensions(&self) -> &[&str] {
//         &["ts"]
//     }
// }

pub struct ScriptsPlugin;

impl Plugin for ScriptsPlugin {
    fn build(&self, app: &mut App) {
        // app.register_asset_loader(ScriptAssetLoader)
        //     .init_asset::<ScriptAsset>()
        //     .add_systems(Startup, (run_lua_test, run_js_test))
        //     .add_systems(Update, receive_lua_test);
    }
}

// #[derive(Component)]
// struct EntityScript {
//     script: Handle<ScriptAsset>,
//     done: bool,
// }

// pub fn run_js_test() {
//     test_script().unwrap();
// }

// pub fn run_lua_test(mut commands: Commands, r_server: ResMut<AssetServer>) {
//     let script: Handle<ScriptAsset> = r_server.load("scripts/hello.ts");
//     commands.spawn(EntityScript {
//         script,
//         done: false,
//     });
// }

// fn receive_lua_test(
//     mut q_scripts: Query<&mut EntityScript>,
//     r_lua_assets: Res<Assets<ScriptAsset>>,
// ) {
//     for mut script in q_scripts.iter_mut() {
//         if !script.done {
//             // if let Some(lua_asset) = r_lua_assets.get(&script.script) {
//             //     let lua = Lua::new();
//             //     let chunk = lua.load(&lua_asset.0);
//             //     let () = chunk.call(()).unwrap();
//             //     script.done = true;
//             // }
//         }
//     }
// }
