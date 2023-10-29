use std::mem::swap;

use bevy::{
    prelude::*,
    text::{Text, TextStyle},
    utils::HashSet,
};

use super::node_span::NodeSpan;

pub struct ElementContext<'w> {
    // pub commands: Commands<'w>,
    pub(crate) world: &'w mut World,
}

pub struct Cx<'w, 'p, Props> {
    pub props: &'p Props,
    pub sys: ElementContext<'w>,
}

pub struct ClassList {
    classes: HashSet<String>,
}

pub trait View: Send + Sync {
    type State;

    /// Returns the number of actual entities created by this view.
    fn count(&self) -> usize;

    fn build<'w>(&self, cx: &mut ElementContext<'w>, prev: &NodeSpan) -> NodeSpan;
}

impl View for String {
    type State = ();

    fn count(&self) -> usize {
        1
    }

    fn build<'w>(&self, cx: &mut ElementContext<'w>, prev: &NodeSpan) -> NodeSpan {
        if let NodeSpan::Node(text_entity) = prev {
            if let Some(mut old_text) = cx.world.entity_mut(*text_entity).get_mut::<Text>() {
                // TODO: compare text for equality.
                old_text.sections.clear();
                old_text.sections.push(TextSection {
                    value: self.to_owned(),
                    style: TextStyle { ..default() },
                });
                return NodeSpan::Node(*text_entity);
            }
        }

        prev.despawn_recursive(cx.world);
        let new_entity = cx
            .world
            .spawn((TextBundle {
                text: Text::from_section(self.clone(), TextStyle { ..default() }),
                // TextStyle {
                //     font_size: 40.0,
                //     color: Color::rgb(0.9, 0.9, 0.9),
                //     ..Default::default()
                // },
                // background_color: Color::rgb(0.65, 0.75, 0.65).into(),
                // border_color: Color::BLUE.into(),
                // focus_policy: FocusPolicy::Pass,
                ..default()
            },))
            .id();

        return NodeSpan::Node(new_entity);
    }
}

impl View for &'static str {
    type State = ();

    fn count(&self) -> usize {
        1
    }

    fn build<'w>(&self, cx: &mut ElementContext<'w>, prev: &NodeSpan) -> NodeSpan {
        if let NodeSpan::Node(text_entity) = prev {
            if let Some(mut old_text) = cx.world.entity_mut(*text_entity).get_mut::<Text>() {
                // TODO: compare text for equality.
                old_text.sections.clear();
                old_text.sections.push(TextSection {
                    value: self.to_string(),
                    style: TextStyle { ..default() },
                });
                return NodeSpan::Node(*text_entity);
            }
        }

        prev.despawn_recursive(cx.world);
        let new_entity = cx
            .world
            .spawn((TextBundle {
                text: Text::from_section(self.to_string(), TextStyle { ..default() }),
                // TextStyle {
                //     font_size: 40.0,
                //     color: Color::rgb(0.9, 0.9, 0.9),
                //     ..Default::default()
                // },
                // background_color: Color::rgb(0.65, 0.75, 0.65).into(),
                // border_color: Color::BLUE.into(),
                // focus_policy: FocusPolicy::Pass,
                ..default()
            },))
            .id();

        return NodeSpan::Node(new_entity);
    }
}

pub struct Sequence<A: ViewTuple> {
    items: A,
}

impl<'a, A: ViewTuple> Sequence<A> {
    pub fn new(items: A) -> Self {
        Self { items }
    }
}

impl<'a, A: ViewTuple> View for Sequence<A> {
    type State = A::State;

    fn count(&self) -> usize {
        1
    }

    fn build<'w>(&self, cx: &mut ElementContext<'w>, prev: &NodeSpan) -> NodeSpan {
        let count_spans = self.items.len();
        let mut child_spans: Vec<NodeSpan> = vec![NodeSpan::Empty; count_spans];

        // Get a copy of child spans from Component
        if let NodeSpan::Node(entity) = prev {
            if let Some(cmp) = cx.world.entity_mut(*entity).get_mut::<SequenceComponent>() {
                if cmp.children.len() == self.items.len() {
                    child_spans = cmp.children.clone();
                }
            }
        }

        // Rebuild span array, replacing ones that changed.
        self.items.build_spans(cx, &mut child_spans);
        let mut count_children: usize = 0;
        for node in child_spans.iter() {
            count_children += node.count()
        }
        let mut flat: Vec<Entity> = Vec::with_capacity(count_children);
        for node in child_spans.iter() {
            node.flatten(&mut flat);
        }

        if let NodeSpan::Node(entity) = prev {
            let mut em = cx.world.entity_mut(*entity);
            if let Some(mut cmp) = em.get_mut::<SequenceComponent>() {
                if cmp.children != child_spans {
                    swap(&mut cmp.children, &mut child_spans);
                    // TODO: Need to replace child entities
                    // em.push_children(&flat);
                }
                return NodeSpan::Node(*entity);
            }
        }

        // Remove previous entity
        prev.despawn_recursive(cx.world);

        let new_entity = cx
            .world
            .spawn((
                SequenceComponent {
                    children: child_spans,
                },
                NodeBundle {
                    // focus_policy: FocusPolicy::Pass,
                    visibility: Visibility::Visible,
                    ..default()
                },
            ))
            .push_children(&flat)
            .id();

        NodeSpan::Node(new_entity)
    }
}

/// Component for a sequence, tracks the list of children by span.
#[derive(Component)]
pub struct SequenceComponent {
    pub(crate) children: Vec<NodeSpan>,
}

// ViewTuple

pub trait ViewTuple: Send + Sync {
    type State;

    fn len(&self) -> usize;

    fn build_spans<'w>(&self, cx: &mut ElementContext<'w>, out: &mut [NodeSpan]);
}

impl ViewTuple for () {
    type State = ();

    fn len(&self) -> usize {
        0
    }

    fn build_spans<'w>(&self, cx: &mut ElementContext<'w>, out: &mut [NodeSpan]) {}
}

impl<A: View> ViewTuple for A {
    type State = A::State;

    fn len(&self) -> usize {
        1
    }

    fn build_spans<'w>(&self, cx: &mut ElementContext<'w>, out: &mut [NodeSpan]) {
        out[0] = self.build(cx, &out[0])
    }
}

impl<A: View> ViewTuple for (A,) {
    type State = (A::State,);

    fn len(&self) -> usize {
        1
    }

    fn build_spans<'w>(&self, cx: &mut ElementContext<'w>, out: &mut [NodeSpan]) {
        out[0] = self.0.build(cx, &out[0])
    }
}

impl<A0: View, A1: View> ViewTuple for (A0, A1) {
    type State = (A0::State, A1::State);

    fn len(&self) -> usize {
        2
    }

    fn build_spans<'w>(&self, cx: &mut ElementContext<'w>, out: &mut [NodeSpan]) {
        out[0] = self.0.build(cx, &out[0]);
        out[1] = self.1.build(cx, &out[1]);
    }
}

impl<A0: View, A1: View, A2: View> ViewTuple for (A0, A1, A2) {
    type State = (A0::State, A1::State, A2::State);

    fn len(&self) -> usize {
        3
    }

    fn build_spans<'w>(&self, cx: &mut ElementContext<'w>, out: &mut [NodeSpan]) {
        out[0] = self.0.build(cx, &out[0]);
        out[1] = self.1.build(cx, &out[1]);
        out[2] = self.2.build(cx, &out[1]);
    }
}

pub fn sequence_from<'a>(_args: impl ViewTuple) -> impl View + 'a {
    "Hello"
    // ViewSequence::<'a>::new(&["Hello"])
}

pub fn args_test() {
    let _a = sequence_from(());
    // let _b = sequence_from(("Hello",));
    let _b = sequence_from(("Hello",));
    let _c = sequence_from(("Hello", "World"));
    // let _d = sequence_from((sequence_from(())));
    let _s = Sequence::new((
        format!("Hello"),
        "Goodbye",
        Sequence::new((format!("Hello"), "Goodbye")),
    ));
}

pub fn aspect_chooser() -> impl View {
    Sequence::new(format!("Hello"))
}
