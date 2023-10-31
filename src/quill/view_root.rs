use bevy::prelude::*;

use super::{
    view::{Cx, ElementContext, TrackedResources},
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
    id: usize,
}

impl<V: View, Props: Send + Sync> ViewState<V, Props> {
    pub fn new(presenter: fn(cx: Cx<Props>) -> V, props: Props) -> Self {
        Self {
            presenter,
            nodes: NodeSpan::Empty,
            props,
            needs_rebuild: true,
            // TODO generate an id based on something
            id: 8713479991066624,
        }
    }
}

pub trait AnyViewHandle: Send + Sync {
    fn count(&self) -> usize;
    fn build<'w>(&mut self, cx: &'w mut ElementContext<'w>);
    fn id(&self) -> usize;
}

impl<V: View, Props: Send + Sync> AnyViewHandle for ViewState<V, Props> {
    fn count(&self) -> usize {
        self.nodes.count()
    }

    fn id(&self) -> usize {
        self.id
    }

    fn build<'w>(&mut self, ecx: &'w mut ElementContext<'w>) {
        if let Some(x) = ecx.world.resource::<TrackedResources>().data.get(&self.id) {
            // Check if any resource used by this ViewState has changed
            self.needs_rebuild = x.iter().any(|x| x.is_changed(ecx.world));
        } else {
            // initialize with an empty array
            ecx.world
                .resource_mut::<TrackedResources>()
                .data
                .insert(self.id, Default::default());
        };
        if self.needs_rebuild {
            // reset the tracked resources
            ecx.world
                .resource_mut::<TrackedResources>()
                .data
                .get_mut(&self.id)
                .unwrap()
                .clear();
            println!("rebuild");

            self.needs_rebuild = false;
            let cx = Cx::<Props> {
                sys: ecx,
                props: &self.props,
                id: self.id,
            };
            let v = (self.presenter)(cx);
            self.nodes = v.build(ecx, &self.nodes);
        }
    }
}
