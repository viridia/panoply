use bevy::asset::{AssetLoader, LoadContext, LoadedAsset};
use bevy::prelude::*;
use bevy::utils::BoxedFuture;
use quick_xml::events::attributes::Attribute;
use quick_xml::events::{BytesStart, Event};
use quick_xml::name::QName;
use quick_xml::reader::Reader;

use crate::guise::style;
use crate::guise::template::ParamType;

use super::template::{Template, UiNodeList};
use super::GuiseError;

#[derive(Default)]
pub struct GuiseLoader;

const ATTR_ID: QName = QName(b"id");
const ATTR_NAME: QName = QName(b"name");
const ATTR_TYPE: QName = QName(b"type");

impl AssetLoader for GuiseLoader {
    fn load<'a>(
        &'a self,
        bytes: &'a [u8],
        load_context: &'a mut LoadContext,
    ) -> BoxedFuture<'a, Result<(), bevy::asset::Error>> {
        Box::pin(async move {
            let mut visitor = GuiseXmlVisitor::<'a> {
                reader: Reader::from_reader(bytes),
            };
            match visitor.visit(load_context) {
                Ok(()) => Ok(()),
                Err(e) => {
                    panic!("Parsing error: {:?}", e);
                }
            }
        })
    }

    fn extensions(&self) -> &[&str] {
        &["guise.xml"]
    }
}

struct GuiseXmlVisitor<'a> {
    reader: Reader<&'a [u8]>,
}

impl<'a> GuiseXmlVisitor<'a> {
    fn visit(&mut self, load_context: &'a mut LoadContext) -> Result<(), GuiseError> {
        loop {
            match self.reader.read_event() {
                Err(e) => panic!(
                    "Error at position {}: {:?}",
                    self.reader.buffer_position(),
                    e
                ),
                Ok(Event::Eof) => break,

                Ok(Event::Start(e)) => match e.name().as_ref() {
                    b"templates" => {
                        self.visit_templates(load_context)?;
                    }

                    _ => {
                        return Err(GuiseError::InvalidElement(
                            std::str::from_utf8(e.name().as_ref()).unwrap().to_string(),
                        ))
                    }
                },

                Ok(Event::Empty(e)) => match e.name().as_ref() {
                    b"templates" => {
                        self.reader.read_to_end(e.name()).unwrap();
                    }

                    _ => {
                        return Err(GuiseError::InvalidElement(
                            std::str::from_utf8(e.name().as_ref()).unwrap().to_string(),
                        ))
                    }
                },

                Ok(Event::End(e)) => match e.name().as_ref() {
                    b"templates" => break,

                    _ => {
                        panic!(
                            "Unrecognized end tag: </{}>",
                            std::str::from_utf8(e.name().as_ref()).unwrap()
                        );
                    }
                },

                _ => (),
            }
        }
        Ok(())
    }

    fn visit_templates<'b>(&mut self, load_context: &'b mut LoadContext) -> Result<(), GuiseError> {
        loop {
            match self.reader.read_event() {
                Err(e) => panic!(
                    "Error at position {}: {:?}",
                    self.reader.buffer_position(),
                    e
                ),
                Ok(Event::Eof) => return Err(GuiseError::PrematureEof),

                Ok(Event::Start(e)) => match e.name().as_ref() {
                    b"template" => {
                        self.visit_template(&e, load_context)?;
                    }

                    b"style" => {
                        self.visit_style(&e, load_context)?;
                        self.reader.read_to_end(e.name()).unwrap();
                    }

                    _ => {
                        return Err(GuiseError::InvalidElement(
                            std::str::from_utf8(e.name().as_ref()).unwrap().to_string(),
                        ))
                    }
                },

                Ok(Event::Empty(e)) => match e.name().as_ref() {
                    b"style" => {
                        self.visit_style(&e, load_context)?;
                    }

                    _ => {
                        return Err(GuiseError::InvalidElement(
                            std::str::from_utf8(e.name().as_ref()).unwrap().to_string(),
                        ))
                    }
                },

                Ok(Event::End(e)) => match e.name().as_ref() {
                    b"templates" => break,

                    _ => {
                        panic!(
                            "Unrecognized end tag: </{}>",
                            std::str::from_utf8(e.name().as_ref()).unwrap()
                        );
                    }
                },

                _ => (),
            }
        }
        Ok(())
    }

    fn visit_template<'b>(
        &mut self,
        e: &'b BytesStart,
        load_context: &'b mut LoadContext,
    ) -> Result<(), GuiseError> {
        let id = require_attr(e, ATTR_ID)?.unescape_value().unwrap();

        let mut result = Template::new();

        loop {
            match self.reader.read_event() {
                Err(e) => panic!(
                    "Error at position {}: {:?}",
                    self.reader.buffer_position(),
                    e
                ),
                Ok(Event::Eof) => return Err(GuiseError::PrematureEof),

                Ok(Event::Start(e)) => match e.name().as_ref() {
                    b"params" => {
                        self.visit_params(&mut result)?;
                    }

                    b"content" => {
                        self.visit_node_list(&e, &mut result.children)?;
                    }

                    _ => {
                        return Err(GuiseError::InvalidElement(
                            std::str::from_utf8(e.name().as_ref()).unwrap().to_string(),
                        ))
                    }
                },

                Ok(Event::Empty(e)) => match e.name().as_ref() {
                    b"style" => {
                        self.visit_style(&e, load_context)?;
                    }

                    _ => {
                        return Err(GuiseError::InvalidElement(
                            std::str::from_utf8(e.name().as_ref()).unwrap().to_string(),
                        ))
                    }
                },

                Ok(Event::End(e)) => match e.name().as_ref() {
                    b"template" => break,

                    _ => {
                        panic!(
                            "Unrecognized end tag: </{}>",
                            std::str::from_utf8(e.name().as_ref()).unwrap()
                        );
                    }
                },

                _ => (),
            }
        }

        println!("Template element loaded: {}", id);
        load_context.set_labeled_asset(&id, LoadedAsset::new(result));
        Ok(())
    }

    fn visit_style<'b>(
        &mut self,
        e: &'b BytesStart,
        load_context: &'b mut LoadContext,
    ) -> Result<(), GuiseError> {
        let id = require_attr(e, ATTR_ID)?.unescape_value().unwrap();
        let style = style::from_xml(&mut self.reader, e)?;
        println!("Style element loaded: {}", id);
        load_context.set_labeled_asset(&id, LoadedAsset::new(style));
        Ok(())
    }

    fn visit_params<'b>(&mut self, template: &mut Template) -> Result<(), GuiseError> {
        loop {
            match self.reader.read_event() {
                Err(e) => panic!(
                    "Error at position {}: {:?}",
                    self.reader.buffer_position(),
                    e
                ),
                Ok(Event::Eof) => return Err(GuiseError::PrematureEof),

                Ok(Event::Start(e)) => match e.name().as_ref() {
                    b"param" => {
                        self.visit_param(&e, template)?;
                    }

                    _ => {
                        return Err(GuiseError::InvalidElement(
                            std::str::from_utf8(e.name().as_ref()).unwrap().to_string(),
                        ))
                    }
                },

                Ok(Event::Empty(e)) => match e.name().as_ref() {
                    b"param" => {
                        self.visit_param(&e, template)?;
                    }

                    _ => {
                        return Err(GuiseError::InvalidElement(
                            std::str::from_utf8(e.name().as_ref()).unwrap().to_string(),
                        ))
                    }
                },

                Ok(Event::End(e)) => match e.name().as_ref() {
                    b"template" => break,

                    _ => {
                        panic!(
                            "Unrecognized end tag: </{}>",
                            std::str::from_utf8(e.name().as_ref()).unwrap()
                        );
                    }
                },

                _ => (),
            }
        }
        Ok(())
    }

    fn visit_param<'b>(
        &mut self,
        e: &'b BytesStart,
        template: &mut Template,
    ) -> Result<(), GuiseError> {
        let name = require_attr(e, ATTR_NAME)?.unescape_value().unwrap();
        let typ: &str = &require_attr(e, ATTR_TYPE)?.unescape_value().unwrap();
        println!("Param element: {}:{}", name, typ);
        template.params.insert(
            name.to_string(),
            match typ {
                "bool" => ParamType::Bool,
                "i32" => ParamType::I32,
                "u32" => ParamType::U32,
                "f32" => ParamType::F32,
                "string" => ParamType::String,
                _ => {
                    return Err(GuiseError::InvalidAttributeValue(typ.to_string()));
                }
            },
        );
        Ok(())
    }

    fn visit_node_list<'b>(
        &mut self,
        e: &'b BytesStart,
        nodes: &mut UiNodeList,
    ) -> Result<(), GuiseError> {
        // if let Some(Ok(id_attr)) = e.attributes().find(|a| match a {
        //     Ok(a) => a.key == ATTR_ID,
        //     _ => false,
        // }) {
        //     let id = id_attr.unescape_value().unwrap();
        //     let template = template::from_xml(&mut self.reader, e)?;
        //     println!("Template element loaded: {}", id);
        //     load_context.set_labeled_asset(&id, LoadedAsset::new(template));
        //     Ok(())
        // } else {
        //     panic!("<template> 'id' attribute is required'.");
        // }
        Ok(())
    }

    fn visit_node<'b>(&mut self, e: &'b BytesStart) -> Result<(), GuiseError> {
        if let Some(Ok(id_attr)) = e.attributes().find(|a| match a {
            Ok(a) => a.key == ATTR_ID,
            _ => false,
        }) {
            let id = id_attr.unescape_value().unwrap();
            // let template = template::from_xml(&mut self.reader, e)?;
            println!("Template element loaded: {}", id);
            // load_context.set_labeled_asset(&id, LoadedAsset::new(template));
            Ok(())
        } else {
            panic!("<template> 'id' attribute is required'.");
        }
    }
}

pub fn require_attr<'a>(e: &'a BytesStart, name: QName) -> Result<Attribute<'a>, GuiseError> {
    for attr in e.attributes() {
        if attr.is_ok() {
            let attr = attr.unwrap();
            if attr.key == name {
                return Ok(attr);
            }
        }
    }

    Err(GuiseError::MissingRequiredAttribute(
        std::str::from_utf8(name.into_inner()).unwrap().to_string(),
    ))
}

#[derive(Resource)]
pub struct ControlPanelResource(pub Handle<Template>);

impl FromWorld for ControlPanelResource {
    fn from_world(world: &mut World) -> Self {
        let server = world.resource::<AssetServer>();
        ControlPanelResource(server.load("editor/ui/test.guise.xml#main"))
    }
}
