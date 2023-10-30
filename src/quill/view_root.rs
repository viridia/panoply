use bevy::prelude::*;

use super::{
    view::{Cx, ElementContext},
    NodeSpan, View,
};

#[derive(Resource)]
pub struct ViewRootResource(pub ViewRoot);

pub struct ViewRoot {
    pub handle: Box<dyn AnyViewHandle>,
}

impl ViewRoot {
    /// Construct a new ViewRoot from a presenter and props.
    pub fn new<V: View + 'static, Props: Send + Sync + 'static>(
        presenter: fn(cx: Cx<Props>) -> V,
        props: Props,
    ) -> Self {
        Self {
            handle: Box::new(ViewState::new(presenter, props)),
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

// pub struct ViewHandle {
//     pub(crate) state: Box<dyn AnyViewHandle>,
// }

pub struct ViewState<V: View, Props: Send + Sync> {
    presenter: fn(cx: Cx<Props>) -> V,
    nodes: NodeSpan,
    props: Props,
    needs_rebuild: bool,
}

impl<V: View, Props: Send + Sync> ViewState<V, Props> {
    pub fn new(presenter: fn(cx: Cx<Props>) -> V, props: Props) -> Self {
        Self {
            presenter,
            nodes: NodeSpan::Empty,
            props,
            needs_rebuild: true,
        }
    }
}

pub trait AnyViewHandle: Send + Sync {
    fn count(&self) -> usize;
    fn build<'w>(&mut self, cx: &'w mut ElementContext<'w>);
}

impl<V: View, Props: Send + Sync> AnyViewHandle for ViewState<V, Props> {
    fn count(&self) -> usize {
        self.nodes.count()
    }

    fn build<'w>(&mut self, ecx: &'w mut ElementContext<'w>) {
        if self.needs_rebuild {
            self.needs_rebuild = false;
            let cx = Cx::<Props> {
                sys: ecx,
                props: &self.props,
            };
            let v = (self.presenter)(cx);
            self.nodes = v.build(ecx, &self.nodes);
        }
    }
}
