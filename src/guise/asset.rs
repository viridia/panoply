use std::{any::TypeId, sync::Arc};

use bevy::{
    asset::{io::Reader, AssetLoader, AssetPath, LoadContext},
    prelude::*,
    reflect::{TypePath, TypeRegistryArc, TypeRegistryInternal},
    utils::{BoxedFuture, HashMap},
};
use futures_lite::AsyncReadExt;
use pest::{
    error::ErrorVariant,
    iterators::{Pair, Pairs},
    Parser,
};

use crate::guise::{expr::TemplateParam, from_ast::ReflectFromAst, path::relative_asset_path};

use super::{
    expr::{Expr, Template, TemplateParams},
    parser::{GuiseParser, Rule},
};

#[derive(Debug, Asset, TypePath, Clone)]
pub struct GuiseAsset(pub Expr);

pub struct GuiseAssetLoader {
    registry: TypeRegistryArc,
}

impl GuiseAssetLoader {
    fn new(registry: TypeRegistryArc) -> Self {
        Self { registry }
    }
}

impl AssetLoader for GuiseAssetLoader {
    type Asset = GuiseAsset;
    type Settings = ();

    fn load<'a>(
        &'a self,
        reader: &'a mut Reader,
        _settings: &'a Self::Settings,
        load_context: &'a mut LoadContext,
    ) -> BoxedFuture<'a, Result<Self::Asset, anyhow::Error>> {
        Box::pin(async move {
            let mut bytes = Vec::new();
            reader.read_to_end(&mut bytes).await?;
            let result = GuiseParser::parse(Rule::guise, std::str::from_utf8(&bytes).unwrap());
            match result {
                Ok(pairs) => {
                    let registry: &TypeRegistryInternal = &self.registry.read();
                    let mut visitor = AstVisitor::new(registry, load_context);
                    visitor.visit_file(pairs)?;
                }
                Err(e) => {
                    return Err(anyhow::Error::new(e));
                }
            }

            // entries.styles.drain().for_each(|(key, mut style)| {
            //     let label = format!("styles/{}", key);
            //     let base = AssetPath::from_path(load_context.path().to_path_buf())
            //         .with_label(label.clone());
            //     self.visit_stylesheet(&mut style, &base);
            //     load_context.add_labeled_asset(label, style);
            // });
            Ok(GuiseAsset(Expr::Null))
        })
    }

    fn extensions(&self) -> &[&str] {
        &["guise"]
    }
}

impl FromWorld for GuiseAssetLoader {
    fn from_world(world: &mut World) -> Self {
        GuiseAssetLoader::new(world.resource::<AppTypeRegistry>().0.clone())
    }
}

pub struct AstVisitor<'a, 'b> {
    registry: &'a TypeRegistryInternal,
    load_context: &'a mut LoadContext<'b>,
}

impl<'a, 'c> AstVisitor<'a, 'c> {
    fn new(registry: &'a TypeRegistryInternal, load_context: &'a mut LoadContext<'c>) -> Self {
        Self {
            registry,
            load_context,
        }
    }

    fn visit_file(&mut self, pairs: Pairs<'_, Rule>) -> Result<(), anyhow::Error> {
        let mut imports = Imports::new();
        for decl in pairs.into_iter() {
            match decl.as_rule() {
                Rule::use_decl => {
                    let qname = decl.into_inner().next().unwrap().as_str();
                    let short_name = qname.rsplit_once("::").ok_or(qname).unwrap().1;
                    let ty = self.registry.get_with_name(qname);
                    if ty.is_some() {
                        imports.entries.insert(
                            short_name.to_string(),
                            ImportEntry::Native(ty.unwrap().type_id()),
                        );
                        // println!("USE {} as {}", qname, short_name);
                    } else {
                        error!("Not found {}", qname);
                        // for t in registry.iter() {
                        //     info!("Type: {}", t.type_name());
                        // }
                    }
                }
                Rule::decl => {
                    self.visit_decl(&mut decl.into_inner(), "", &mut imports)?;
                }
                Rule::EOI => {}
                _ => {
                    panic!("{:?}", decl);
                }
            }
        }
        Ok(())
    }

    fn visit_decl<'b>(
        &mut self,
        pairs: &'b mut Pairs<'b, Rule>,
        base: &str,
        mut imports: &'b mut Imports,
    ) -> Result<(), anyhow::Error> {
        let id = pairs.next().unwrap();
        let key = self.visit_key(&id);
        let label = match base {
            "" => key.to_string(),
            _ => format!("{}/{}", base, key),
        };

        let args = if pairs.len() > 1 { pairs.next() } else { None };
        let mut value = pairs.next().unwrap();
        match value.as_rule() {
            Rule::group => {
                if args.is_some() {
                    return Err(anyhow::Error::new(
                        pest::error::Error::new_from_span(
                            ErrorVariant::<()>::CustomError {
                                message: String::from("Argument list not allowed for groups"),
                            },
                            args.unwrap().as_span(),
                        )
                        .with_path(self.load_context.path().to_str().unwrap()),
                    ));
                }

                for decl in value.into_inner().into_iter() {
                    self.visit_decl(&mut decl.into_inner(), &label, &mut imports)?
                }
            }

            Rule::object => self.visit_asset(&label, args, &mut value, base, &mut imports)?,

            _ => panic!("Invalid rule {:?}", value.as_rule()),
        }
        Ok(())
    }

    fn visit_asset<'b>(
        &mut self,
        label: &str,
        params: Option<Pair<'b, Rule>>,
        pair: &'b mut Pair<'b, Rule>,
        base: &str,
        imports: &'b mut Imports,
    ) -> Result<(), anyhow::Error> {
        self.load_context.begin_labeled_asset();
        let has_params = params.is_some();
        let params_map: TemplateParams = match params {
            Some(param_pairs) => {
                let params_inner = param_pairs.into_inner();
                let mut map: TemplateParams = HashMap::with_capacity(params_inner.len());
                for param in params_inner.into_iter() {
                    let mut param_inner = param.into_inner();
                    let param_name = param_inner.next().unwrap().as_str();
                    let param_type =
                        self.visit_param_type(&param_inner.next().unwrap(), imports)?;
                    // println!("Param {:?} {:?}", arg_name, arg_type);
                    map.insert(param_name.to_string(), TemplateParam::new(param_type));
                }
                map
            }
            None => HashMap::new(),
        };
        let expr = self.visit_expr(label, &params_map, pair, base, imports)?;
        if has_params {
            self.load_context.add_labeled_asset(
                label.to_string(),
                GuiseAsset(Expr::Template(Box::new(Template {
                    params: params_map,
                    expr,
                }))),
            );
            imports.entries.insert(
                label.to_string(),
                ImportEntry::Asset(
                    self.load_context
                        .asset_path()
                        .with_label(label.to_owned().clone()),
                ),
            );
        } else {
            self.load_context
                .add_labeled_asset(label.to_string(), GuiseAsset(expr));
            imports.entries.insert(
                label.to_string(),
                ImportEntry::Asset(
                    self.load_context
                        .asset_path()
                        .with_label(label.to_owned().clone()),
                ),
            );
        }
        Ok(())
    }

    fn visit_expr<'b, 'd>(
        &mut self,
        label: &str,
        params: &TemplateParams,
        expr: &'b mut Pair<'b, Rule>,
        base: &str,
        imports: &'d Imports,
    ) -> Result<Expr, anyhow::Error> {
        match expr.as_rule() {
            Rule::object => self.visit_object(params, expr, base, imports),
            Rule::boolean => Ok(Expr::Bool(expr.as_str() == "true")),
            Rule::identifier => Ok(Expr::Ident(expr.as_str().to_string())),
            Rule::number => Ok(Expr::Number(expr.as_str().parse::<f32>().unwrap())),
            Rule::string => {
                let mut pairs = expr.clone().into_inner();
                let raw_text = pairs.next().unwrap().as_str();
                let mut unescaped = String::with_capacity(raw_text.len());
                let mut chars = raw_text.chars().enumerate();
                while let Some((_idx, c)) = chars.next() {
                    if c == '\\' {
                        match chars.next() {
                            None => {
                                return Err(anyhow::Error::new(
                                    pest::error::Error::new_from_span(
                                        ErrorVariant::<()>::CustomError {
                                            message: String::from("Invalid escape sequence"),
                                        },
                                        expr.as_span(),
                                    )
                                    .with_path(self.load_context.path().to_str().unwrap()),
                                ));
                            }
                            Some((_idx, c2)) => unescaped.push(match c2 {
                                'n' => '\n',
                                'r' => '\r',
                                't' => '\t',
                                '\\' => '\\',
                                '\'' => '\'',
                                '"' => '"',
                                '$' => '$',
                                '`' => '`',
                                ' ' => ' ',
                                // TODO: Unicode
                                // https://docs.rs/snailquote/latest/src/snailquote/lib.rs.html#231-308
                                _ => {
                                    return Err(anyhow::Error::new(
                                        pest::error::Error::new_from_span(
                                            ErrorVariant::<()>::CustomError {
                                                message: String::from("Invalid escape sequence"),
                                            },
                                            expr.as_span(),
                                        )
                                        .with_path(self.load_context.path().to_str().unwrap()),
                                    ));
                                }
                            }),
                        }
                    } else {
                        unescaped.push(c);
                    }
                }
                Ok(Expr::Text(unescaped))
            }
            Rule::color => {
                let hex = &expr.as_str()[1..];
                match hex.len() {
                    3 | 4 | 6 | 8 => Ok(Expr::Color(Color::hex(hex).unwrap())),
                    _ => {
                        return Err(anyhow::Error::new(
                            pest::error::Error::new_from_span(
                                ErrorVariant::<()>::CustomError {
                                    message: String::from("Incorrect number of digits for color"),
                                },
                                expr.as_span(),
                            )
                            .with_path(self.load_context.path().to_str().unwrap()),
                        ));
                    }
                }
            }
            Rule::length => {
                let mut pairs = expr.clone().into_inner();
                let magnitude = pairs.next().unwrap().as_str().parse::<f32>().unwrap();
                let unit = pairs.next().unwrap();
                Ok(match unit.as_str() {
                    "px" => Expr::Length(bevy::ui::Val::Px(magnitude)),
                    "vh" => Expr::Length(bevy::ui::Val::Vh(magnitude)),
                    "vw" => Expr::Length(bevy::ui::Val::Vw(magnitude)),
                    "vmin" => Expr::Length(bevy::ui::Val::VMin(magnitude)),
                    "vmax" => Expr::Length(bevy::ui::Val::VMax(magnitude)),
                    "%" => Expr::Length(bevy::ui::Val::Percent(magnitude)),
                    _ => unreachable!(),
                })
            }
            Rule::array => {
                let elements = expr.clone().into_inner();
                let mut elts: Vec<Expr> = Vec::with_capacity(elements.len());
                for mut element in elements.into_iter() {
                    elts.push(self.visit_expr(label, params, &mut element, base, imports)?);
                }
                Ok(Expr::List(elts.into_boxed_slice()))
            }
            Rule::asset => {
                let pairs = expr
                    .clone()
                    .into_inner()
                    .next()
                    .unwrap()
                    .into_inner()
                    .next()
                    .unwrap();
                let asset_path = pairs.as_str();
                Ok(Expr::AssetPath(relative_asset_path(
                    self.load_context.asset_path(),
                    asset_path,
                )))
            }
            _ => panic!("Invalid rule {:?}", expr.as_rule()),
        }
    }

    fn visit_object<'b, 'd>(
        &mut self,
        params: &TemplateParams,
        obj_ast: &'b mut Pair<'b, Rule>,
        base: &str,
        imports: &'d Imports,
    ) -> Result<Expr, anyhow::Error> {
        let mut pairs = obj_ast.clone().into_inner();
        let type_name = pairs.next().unwrap(); // qualified name, string, etc.
        let type_name_str = type_name.as_str(); // qualified name, string, etc.
                                                // println!("OBJECT {} -> {}", label, type_name_str);
        let members = if pairs.len() > 0 {
            let body = pairs.next().unwrap().into_inner();
            let mut members = HashMap::with_capacity(body.len());
            for member in body.into_iter() {
                match member.as_rule() {
                    Rule::key_value => {
                        let mut member_pairs = member.into_inner();
                        let id = member_pairs.next().unwrap();
                        let key = self.visit_key(&id);
                        let mut value = member_pairs.next().unwrap();
                        // println!("  MEMBER {}", key);
                        let expr = self.visit_expr(key, params, &mut value, base, imports)?;
                        members.insert(key.to_string(), expr);
                    }

                    _ => panic!("Invalid rule {:?}", member.as_rule()),
                }
            }
            members
        } else {
            HashMap::with_capacity(0)
        };

        match imports.entries.get(type_name_str) {
            Some(ImportEntry::Native(ty)) => {
                if let Some(rd) = self.registry.get(*ty).unwrap().data::<ReflectFromAst>() {
                    let obj = rd.from_ast(members, self.load_context);
                    return match obj {
                        Ok(obj) => Ok(obj),
                        Err(err) => {
                            return Err(anyhow::Error::new(
                                pest::error::Error::new_from_span(
                                    ErrorVariant::<()>::CustomError {
                                        message: err.to_string(),
                                    },
                                    obj_ast.as_span(),
                                )
                                .with_path(self.load_context.path().to_str().unwrap()),
                            ));
                        }
                    };
                } else {
                    return Err(anyhow::Error::new(
                        pest::error::Error::new_from_span(
                            ErrorVariant::<()>::CustomError {
                                message: String::from("Can't create object of type"),
                            },
                            type_name.as_span(),
                        )
                        .with_path(self.load_context.path().to_str().unwrap()),
                    ));
                }
            }
            Some(ImportEntry::Asset(path)) => Ok(Expr::Invoke((
                Box::new(Expr::Asset(self.load_context.load(path))),
                Arc::new(members),
            ))),
            None => {
                return Err(anyhow::Error::new(
                    pest::error::Error::new_from_span(
                        ErrorVariant::<()>::CustomError {
                            message: String::from("Unknown object type"),
                        },
                        type_name.as_span(),
                    )
                    .with_path(self.load_context.path().to_str().unwrap()),
                ));
            }
        }
    }

    fn visit_param_type<'b>(
        &self,
        pair: &'b Pair<'b, Rule>,
        imports: &'b mut Imports,
    ) -> Result<TypeId, anyhow::Error> {
        let type_expr = pair.clone().into_inner().next().unwrap();
        match type_expr.as_rule() {
            Rule::arg_list_type => {
                let element_type_id =
                    self.visit_param_type(&type_expr.into_inner().next().unwrap(), imports)?;
                let _element_type = self.registry.get(element_type_id).unwrap();
                Ok(element_type_id)
                // todo!("List type: {:?}", element_type);
            }
            Rule::qualified_name => {
                let qname = type_expr.as_str();
                if qname.find("::").is_none() {
                    if let Some(imp) = imports.entries.get(qname) {
                        match imp {
                            ImportEntry::Native(ty) => return Ok(*ty),
                            ImportEntry::Asset(_) => todo!(),
                        }
                    }
                }
                match self.registry.get_with_name(qname) {
                    Some(ty) => Ok(ty.type_id()),
                    None => Err(anyhow::Error::new(
                        pest::error::Error::new_from_span(
                            ErrorVariant::<()>::CustomError {
                                message: format!("Unknown type name: '{}'", qname),
                            },
                            pair.as_span(),
                        )
                        .with_path(self.load_context.path().to_str().unwrap()),
                    )),
                }
            }
            _ => panic!("Invalid rule {:?}", type_expr.as_rule()),
        }
    }

    fn visit_key<'b>(&self, pair: &'b Pair<'b, Rule>) -> &'b str {
        match pair.as_rule() {
            Rule::identifier => pair.as_str(),
            Rule::string => pair.as_str(),
            _ => unreachable!(),
        }
    }
}

enum ImportEntry {
    /// A native Rust type that has been reflected
    Native(TypeId),

    /// An imported asset reference
    Asset(AssetPath<'static>),
}

/// Helper class for managing use/import statements
struct Imports<'a> {
    /// Reference to enclosing scope
    next: Option<&'a Imports<'a>>,

    entries: HashMap<String, ImportEntry>,
}

impl<'a> Imports<'a> {
    fn new() -> Self {
        Self {
            next: None,
            entries: HashMap::new(),
        }
    }
}
