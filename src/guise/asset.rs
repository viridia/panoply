use std::sync::Arc;

use bevy::asset::{AssetLoader, LoadContext, LoadedAsset};
use bevy::prelude::*;
use bevy::utils::BoxedFuture;
use quick_xml::events::attributes::Attribute;
use quick_xml::events::{BytesStart, Event};
use quick_xml::name::QName;
use quick_xml::reader::Reader;

use crate::guise::template::TemplateParam;

use super::style::{PartialStyle, Selector, StyleAttr};
use super::template::{ElementNode, Template, TemplateNode, TemplateNodeList, TextNode};
use super::GuiseError;

#[derive(Default)]
pub struct GuiseLoader;

const ATTR_ID: QName = QName(b"id");
const ATTR_NAME: QName = QName(b"name");
const ATTR_TYPE: QName = QName(b"type");
const ATTR_SELECTOR: QName = QName(b"selector");
const ATTR_CONTROLLER: QName = QName(b"controller");

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
                        self.visit_root(load_context)?;
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

    fn visit_root<'b>(&mut self, load_context: &'b mut LoadContext) -> Result<(), GuiseError> {
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
                        let id = require_attr(&e, ATTR_ID)?.unescape_value().unwrap();
                        let style = self.visit_style(&e, false)?;
                        if load_context.has_labeled_asset(&id) {
                            error!("Duplicate id for style: {}", id);
                        }
                        load_context.set_labeled_asset(&id, LoadedAsset::new(style));
                    }

                    _ => {
                        return Err(GuiseError::InvalidElement(
                            std::str::from_utf8(e.name().as_ref()).unwrap().to_string(),
                        ))
                    }
                },

                Ok(Event::Empty(e)) => match e.name().as_ref() {
                    b"style" => {
                        let id = require_attr(&e, ATTR_ID)?.unescape_value().unwrap();
                        let style = self.visit_style(&e, true)?;
                        if load_context.has_labeled_asset(&id) {
                            error!("Duplicate id for template: {}", id);
                        }
                        load_context.set_labeled_asset(&id, LoadedAsset::new(style));
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
                    b"param" => {
                        self.visit_param(&e, &mut result, false)?;
                    }

                    // b"params" => {
                    //     self.visit_params(&mut result)?;
                    // }
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
                    b"param" => {
                        self.visit_param(&e, &mut result, true)?;
                    }

                    // b"style" => {
                    //     let style = self.visit_style(&e, true)?;
                    //     if load_context.has_labeled_asset(&id) {
                    //         error!("Duplicate id for style: {}", id);
                    //     }
                    //     load_context.set_labeled_asset(&id, LoadedAsset::new(style));
                    // }
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
        if load_context.has_labeled_asset(&id) {
            error!("Duplicate id for template: {}", id);
        }
        load_context.set_labeled_asset(&id, LoadedAsset::new(result));
        Ok(())
    }

    fn visit_style<'b>(
        &mut self,
        e: &'b BytesStart,
        empty: bool,
    ) -> Result<PartialStyle, GuiseError> {
        let mut attrs: Vec<StyleAttr> = Vec::with_capacity(10);
        self.visit_style_attrs(e, &mut attrs)?;
        let mut style = PartialStyle::from_attrs(&attrs);
        if !empty {
            self.visit_style_children(&mut style)?;
        }
        Ok(style)
    }

    fn visit_style_attrs<'b>(
        &mut self,
        e: &'b BytesStart,
        attrs: &mut Vec<StyleAttr>,
    ) -> Result<(), GuiseError> {
        for a in e.attributes() {
            if let Ok(attr) = a {
                if attr.key != ATTR_ID && attr.key != ATTR_SELECTOR && attr.key.prefix().is_none() {
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

        Ok(())
    }

    fn visit_style_children<'b>(&mut self, parent: &mut PartialStyle) -> Result<(), GuiseError> {
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
                    b"style" => {
                        let selector = require_attr(&e, ATTR_SELECTOR)?.unescape_value().unwrap();
                        let selector = Selector::parse(&selector)?;
                        let style = self.visit_style(&e, false)?;
                        parent.add_selector(selector, style);
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
                        let selector = require_attr(&e, ATTR_SELECTOR)?.unescape_value().unwrap();
                        let selector = Selector::parse(&selector)?;
                        let style = self.visit_style(&e, true)?;
                        parent.add_selector(selector, style);
                    }

                    _ => {
                        return Err(GuiseError::InvalidElement(
                            std::str::from_utf8(e.name().as_ref()).unwrap().to_string(),
                        ))
                    }
                },

                Ok(Event::End(e)) => match e.name().as_ref() {
                    b"style" => break,

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
        empty: bool,
    ) -> Result<(), GuiseError> {
        let name = require_attr(e, ATTR_NAME)?.unescape_value().unwrap();
        let typ: &str = &require_attr(e, ATTR_TYPE)?.unescape_value().unwrap();
        // println!("Template param: {}: {}", name, typ);
        template
            .params
            .insert(name.to_string(), TemplateParam::new(typ));

        if !empty {
            self.reader.read_to_end(e.name()).unwrap();
        }
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
                Ok(Event::Start(e)) => match e.name().as_ref() {
                    b"node" => self.visit_element_node(&e, nodes, false)?,
                    _ => {
                        return Err(GuiseError::InvalidElement(
                            std::str::from_utf8(e.name().as_ref()).unwrap().to_string(),
                        ))
                    }
                },
                Ok(Event::Empty(e)) => self.visit_element_node(&e, nodes, true)?,
                Ok(Event::End(e)) => {
                    if e.name() == name {
                        break;
                    }
                    panic!(
                        "Unrecognized end tag: </{}>",
                        std::str::from_utf8(e.name().as_ref()).unwrap()
                    );
                }

                // TODO: Stateful trimming of whitespace.
                Ok(Event::Text(e)) => {
                    let mut node = TextNode { ..default() };
                    let content = e.unescape().expect("string expected");
                    let content = content.trim();
                    if content.len() > 0 {
                        node.content = content.to_string();
                        nodes.push(Box::new(TemplateNode::Text(node)));
                    }
                }

                _ => (),
            }
        }
        Ok(())
    }

    fn visit_element_node<'b>(
        &mut self,
        e: &'b BytesStart,
        parent: &mut TemplateNodeList,
        empty: bool,
    ) -> Result<(), GuiseError> {
        let mut node = ElementNode { ..default() };

        // Parse inline style attributes
        let mut style_attrs = Vec::<StyleAttr>::with_capacity(20);
        for attr in e.attributes() {
            if let Ok(attr) = attr {
                let attr_name: &[u8] = attr.key.local_name().into_inner();
                let attr_value: &str = &attr.unescape_value().unwrap();

                if attr.key == ATTR_ID {
                    // Node id
                    node.id = Some(attr_value.to_string());
                } else if attr.key == ATTR_CONTROLLER {
                    // Controller type name
                    node.controller = Some(attr_value.to_string());
                } else {
                    match StyleAttr::parse(attr_name, attr_value) {
                        // If we recognized the attribute as a style, then add it to the style list.
                        Ok(Some(attr)) => style_attrs.push(attr),

                        // Otherwise, if we didn't recognize it, that's OK - treat it as a generic
                        // atribute for this template node.
                        // TODO: Use 'ui' or 'ctrl' namespace here.
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
        }

        if style_attrs.len() > 0 {
            node.inline_styles = Some(Arc::new(PartialStyle::from_attrs(&style_attrs)))
        }

        if !empty {
            self.visit_node_list(e, &mut node.children)?;
        }

        parent.push(Box::new(TemplateNode::Element(node)));
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
