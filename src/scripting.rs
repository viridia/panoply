use bevy::prelude::*;

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
