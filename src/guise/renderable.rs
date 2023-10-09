use bevy::ecs::system::SystemState;
use bevy::prelude::*;

use super::view_element::ViewElement;
use super::RenderOutput;

// pub st
// commands: &'a mut Commands<'w, 's>,
// query_elements: &'a mut Query<'w, 's, &'a mut ViewElement>,
// query_text: &'a mut Query<'w, 's, &'a mut Text>,
// server: &'a AssetServer,
// assets: &'a Assets<TemplateAsset>,

#[derive(Resource)]
pub struct CachedRenderState {
    system_state: SystemState<Query<'static, 'static, &'static mut ViewElement>>,
}

// impl CachedRenderState {
//     pub fn get_view_element(&self, entity: Entity) -> Option<ViewElement> {
//         return self.system_state.get(entity);
//     }

//     pub fn get_mut_view_element(&mut self, entity: Entity) -> Option<ViewElement> {}
// }

pub struct RenderContext<'a, 'w, 's> {
    pub(crate) query_elements: &'a mut Query<'w, 's, &'static mut ViewElement>,
}

impl<'a, 'w, 's> RenderContext<'a, 'w, 's> {
    pub fn render(&self) -> RenderOutput {
        println!("Render!");
        RenderOutput::Empty
    }

    pub fn get_view_element(&self, entity: Entity) -> Option<&ViewElement> {
        self.query_elements.get(entity).ok()
    }

    pub fn get_mut_view_element(&mut self, entity: Entity) -> Option<Mut<ViewElement>> {
        self.query_elements.get_mut(entity).ok()
    }
}

pub trait Renderable {
    fn render<'a>(&self, output: &RenderOutput, context: &'a mut RenderContext) -> RenderOutput;
}
