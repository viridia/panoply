mod asset;
mod controller;
mod plugin;
mod style;
mod template;

pub use plugin::*;

#[derive(Debug)]
pub enum GuiseError {
    PrematureEof,
    InvalidElement(String),
    MismatchedEnd(String),
    UnknownAttribute(String),
    UnknownAttributeValue(String),
    InvalidAttributeValue(String),
    MissingRequiredAttribute(String),
}

// impl dyn UiComponentList<()> {
//     fn items(&self) -> &[Box<dyn UiComponent>] {
//         &[]
//     }
// }

// pub trait UiComponent {
//     fn children(&self) -> &[Box<dyn UiComponent>];
//     // pick
//     // handle_event
//     // render
// }

// pub struct FlexBox {
//     // TODO: styles
//     children: Vec<Box<dyn UiComponent>>,
// }

// impl FlexBox {
//     pub fn new<M>(_children: impl UiComponentList<M>) -> FlexBox {
//         panic!("");
//         // Self {
//         //     children: Vec::from(children.items()),
//         // }
//     }
// }

// // impl UiComponent for FlexBox {
// //     fn children(&self) -> &[Box<dyn UiComponent>] {
// //         self.children
// //     }
// // }

// fn example() {
//     // let f1 = FlexBox::new(());
// }
