use std::sync::Arc;

use bevy::asset::{AssetLoader, LoadContext, LoadedAsset};
use bevy::prelude::default;
use bevy::utils::BoxedFuture;
use quick_xml::events::attributes::Attribute;
use quick_xml::events::{BytesStart, Event};
use quick_xml::name::QName;
use quick_xml::reader::Reader;

use crate::guise::template::TemplateParam;

use super::style::{PartialStyle, StyleAttr};
use super::template::{Template, TemplateNode, TemplateNodeList, TemplateNodeType};
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

        // println!("Template element loaded: {}", id);
        load_context.set_labeled_asset(&id, LoadedAsset::new(result));
        Ok(())
    }

    fn visit_style<'b>(
        &mut self,
        e: &'b BytesStart,
        load_context: &'b mut LoadContext,
    ) -> Result<(), GuiseError> {
        let id = require_attr(e, ATTR_ID)?.unescape_value().unwrap();
        let mut attrs: Vec<StyleAttr> = Vec::with_capacity(10);

        for a in e.attributes() {
            if let Ok(attr) = a {
                if attr.key != ATTR_ID && attr.key.prefix().is_none() {
                    let attr_name: &[u8] = attr.key.local_name().into_inner();
                    let attr_value: &str = &attr.unescape_value().unwrap();
                    match StyleAttr::parse(attr_name, attr_value.trim()) {
                        Ok(Some(attr)) => attrs.push(attr),
                        Ok(None) => {
                            // We didn't recognize the style attribute. That's an error
                            // for <style> element but not an error for inline styles, since
                            // nodes can have other attributes.
                            return Err(GuiseError::UnknownAttribute(attr_name.to_vec()));
                        }
                        Err(err) => return Err(err),
                    }
                }
            }
        }

        // println!("Style element loaded: {}", id);
        let style = PartialStyle::from_attrs(&attrs);
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
                    b"params" => break,

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
        // println!("Template param: {}: {}", name, typ);
        template
            .params
            .insert(name.to_string(), TemplateParam::new(typ));
        Ok(())
    }

    fn visit_node_list<'b>(
        &mut self,
        e: &'b BytesStart,
        nodes: &mut TemplateNodeList,
    ) -> Result<(), GuiseError> {
        let name = e.name();
        loop {
            match self.reader.read_event() {
                Err(e) => panic!(
                    "Error at position {}: {:?}",
                    self.reader.buffer_position(),
                    e
                ),
                Ok(Event::Eof) => return Err(GuiseError::PrematureEof),
                Ok(Event::Start(e)) => self.visit_node(&e, nodes, false)?,
                Ok(Event::Empty(e)) => self.visit_node(&e, nodes, true)?,
                Ok(Event::End(e)) => {
                    if e.name() == name {
                        break;
                    }
                    panic!(
                        "Unrecognized end tag: </{}>",
                        std::str::from_utf8(e.name().as_ref()).unwrap()
                    );
                }

                _ => (),
            }
        }
        Ok(())
    }

    fn visit_node<'b>(
        &mut self,
        e: &'b BytesStart,
        parent: &mut TemplateNodeList,
        empty: bool,
    ) -> Result<(), GuiseError> {
        let mut node = TemplateNode {
            tag: match e.name().as_ref() {
                b"node" => TemplateNodeType::Node,
                b"flex" => TemplateNodeType::Flex,
                b"grid" => TemplateNodeType::Grid,
                b"fragment" => TemplateNodeType::Fragment,
                _ => {
                    return Err(GuiseError::InvalidElement(
                        std::str::from_utf8(e.name().as_ref()).unwrap().to_string(),
                    ))
                }
            },
            ..default()
        };

        // Parse inline style attributes
        let mut style_attrs = Vec::<StyleAttr>::with_capacity(20);
        for attr in e.attributes() {
            if let Ok(attr) = attr {
                let attr_name: &[u8] = attr.key.local_name().into_inner();
                let attr_value: &str = &attr.unescape_value().unwrap();

                match StyleAttr::parse(attr_name, attr_value) {
                    // If we recognized the attribute as a style, then add it to the style list.
                    Ok(Some(attr)) => style_attrs.push(attr),

                    // Otherwise, if we didn't recognize it, that's OK - treat it as a generic
                    // atribute for this template node.
                    Ok(None) => {
                        node.attrs.insert(
                            std::str::from_utf8(attr_name).unwrap().to_string(),
                            attr_value.to_string(),
                        );
                    }

                    // If the parser returned an error, then propagate it.
                    Err(err) => return Err(err),
                }
            }
        }

        if style_attrs.len() > 0 {
            node.inline_styles = Some(Arc::new(PartialStyle::from_attrs(&style_attrs)))
        }

        if !empty {
            self.visit_node_list(e, &mut node.children)?;
        }

        parent.push(Box::new(node));
        Ok(())
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
