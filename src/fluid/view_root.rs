use bevy::prelude::*;

use super::{view::ElementContext, NodeSpan, View};

/// Component that defines the root of a view hierarchy and a template invocation.
// #[derive(Component)]
// pub struct ViewRoot {
//     handle: Box<dyn AnyViewHandle>,
// }

// impl ViewRoot {
//     pub fn new<V: View + 'static>(presenter: fn() -> V) -> Self {
//         Self {
//             handle: Box::new(ViewHandle::new(presenter)),
//         }
//     }

//     pub fn build(&mut self, cx: &mut World) {
//         self.handle.test();
//     }
// }

#[derive(Resource)]
pub struct ViewRootResource {
    pub handle: Box<dyn AnyViewHandle>,
}

impl ViewRootResource {
    /// Construct a new ViewRootResource from a presenter.
    pub fn new<V: View + 'static>(presenter: fn() -> V) -> Self {
        Self {
            handle: Box::new(ViewHandle::new(presenter)),
        }
    }

    /// Return the count of top-level UiNodes
    pub fn count(&self) -> usize {
        self.handle.count()
    }

    /// Rebuild the UiNodes.
    pub fn build(&mut self, world: &mut World) {
        let mut ec = ElementContext { world };
        self.handle.build(&mut ec);
    }
}

pub struct ViewHandle<V: View> {
    presenter: fn() -> V,
    nodes: NodeSpan,
}

impl<V: View> ViewHandle<V> {
    pub fn new(presenter: fn() -> V) -> Self {
        Self {
            presenter,
            nodes: NodeSpan::Empty,
        }
    }
}

pub trait AnyViewHandle: Send + Sync {
    fn count(&self) -> usize;
    fn build<'w>(&mut self, cx: &mut ElementContext<'w>);
}

impl<V: View> AnyViewHandle for ViewHandle<V> {
    fn count(&self) -> usize {
        self.nodes.count()
    }

    fn build<'w>(&mut self, cx: &mut ElementContext<'w>) {
        let v = (self.presenter)();
        self.nodes = v.build(cx, &self.nodes);
    }
}
