use bevy::{asset::LoadedFolder, prelude::*};

#[derive(Resource)]
pub struct ExemplarsHandleResource(pub Handle<LoadedFolder>);

impl FromWorld for ExemplarsHandleResource {
    fn from_world(world: &mut World) -> Self {
        let server = world.resource::<AssetServer>();
        ExemplarsHandleResource(server.load_folder("exemplars"))
    }
}
