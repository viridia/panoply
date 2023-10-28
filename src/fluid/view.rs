use bevy::{
    prelude::*,
    text::{Text, TextStyle},
    utils::HashSet,
};

use super::node_span::NodeSpan;

pub struct ElementContext<'w> {
    // pub commands: Commands<'w, 's>,
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
    /// Returns the number of actual entities created by this view.
    fn count(&self) -> usize;

    fn build<'w>(&self, cx: &mut ElementContext<'w>, prev: &NodeSpan) -> NodeSpan;
}

impl View for String {
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
    fn count(&self) -> usize {
        1
    }

    fn build<'w, 's>(&self, cx: &mut ElementContext<'w>, prev: &NodeSpan) -> NodeSpan {
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

struct Sequence<A: ViewTuple> {
    // items: Vec<&'a dyn View>,
    items: A,
}

impl<'a, A: ViewTuple> Sequence<A> {
    fn new(items: A) -> Self {
        Self { items }
    }
}

impl<'a, A: ViewTuple> View for Sequence<A> {
    fn count(&self) -> usize {
        1
    }

    fn build<'w, 's>(&self, cx: &mut ElementContext<'w>, prev: &NodeSpan) -> NodeSpan {
        todo!()
    }

    // fn despawn(&self) {
    //     todo!()
    // }
}

pub trait ViewTuple: Send + Sync {
    fn to_vec<'a>(&self) -> Vec<&'a dyn View>;
}

impl ViewTuple for () {
    fn to_vec<'a>(&self) -> Vec<&'a dyn View> {
        Vec::new()
    }
}

impl<A: View> ViewTuple for A {
    fn to_vec<'a>(&self) -> Vec<&'a dyn View> {
        Vec::new()
        // vec![self]
    }
}

impl<A: View> ViewTuple for (A,) {
    fn to_vec<'a>(&self) -> Vec<&'a dyn View> {
        Vec::new()
        // vec![&self.0]
    }
}

impl<A0: View, A1: View> ViewTuple for (A0, A1) {
    fn to_vec<'a>(&self) -> Vec<&'a dyn View> {
        Vec::new()
        // vec![&self.0, &self.1]
    }
}

impl<A0: View, A1: View, A2: View> ViewTuple for (A0, A1, A2) {
    fn to_vec<'a>(&self) -> Vec<&'a dyn View> {
        // vec![&self.0, &self.1, &self.2]
        vec![]
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
