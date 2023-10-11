use bevy::{
    asset::{io::Reader, AssetLoader, LoadContext},
    prelude::*,
    reflect::{TypePath, TypeRegistration, TypeRegistryArc, TypeRegistryInternal},
    utils::{BoxedFuture, HashMap},
};
use futures_lite::AsyncReadExt;
use pest::{
    error::ErrorVariant,
    iterators::{Pair, Pairs},
    Parser,
};

use crate::guise::from_ast::ReflectFromAst;

use super::{
    expr::{Expr, Template},
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
                        imports
                            .entries
                            .insert(short_name.to_string(), ImportEntry::Builtin(&ty.unwrap()));
                        // println!("USE {} as {}", qname, short_name);
                    } else {
                        error!("Not found {}", qname);
                        // for t in registry.iter() {
                        //     info!("Type: {}", t.type_name());
                        // }
                    }
                }
                Rule::decl => {
                    self.visit_decl(&mut decl.into_inner(), "", &imports)?;
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
        imports: &'b Imports,
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
                    self.visit_decl(&mut decl.into_inner(), &label, &imports)?
                }
            }

            Rule::object => self.visit_asset(&label, args, &mut value, base, &imports)?,

            _ => panic!("Invalid rule {:?}", value.as_rule()),
        }
        Ok(())
    }

    fn visit_asset<'b>(
        &mut self,
        label: &str,
        args: Option<Pair<'b, Rule>>,
        pair: &'b mut Pair<'b, Rule>,
        base: &str,
        imports: &'b Imports,
    ) -> Result<(), anyhow::Error> {
        self.load_context.begin_labeled_asset();
        let has_args = args.is_some();
        let expr = self.visit_expr(label, args, pair, base, imports)?;
        if has_args {
            self.load_context.add_labeled_asset(
                label.to_string(),
                GuiseAsset(Expr::Template(Box::new(Template {
                    params: HashMap::new(),
                    expr,
                }))),
            );
        } else {
            self.load_context
                .add_labeled_asset(label.to_string(), GuiseAsset(expr));
        }
        Ok(())
    }

    fn visit_expr<'b>(
        &mut self,
        label: &str,
        args: Option<Pair<'b, Rule>>,
        expr: &'b mut Pair<'b, Rule>,
        base: &str,
        imports: &'b Imports,
    ) -> Result<Expr, anyhow::Error> {
        match expr.as_rule() {
            Rule::object => self.visit_object(label, args, expr, base, imports),
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
                    elts.push(self.visit_expr(label, args.clone(), &mut element, base, imports)?);
                }
                Ok(Expr::List(elts.into_boxed_slice()))
            }
            _ => panic!("Invalid rule {:?}", expr.as_rule()),
        }
    }

    fn visit_object<'b>(
        &mut self,
        label: &str,
        args: Option<Pair<'b, Rule>>,
        obj_ast: &'b mut Pair<'b, Rule>,
        base: &str,
        imports: &'b Imports,
    ) -> Result<Expr, anyhow::Error> {
        let mut pairs = obj_ast.clone().into_inner();
        let type_name = pairs.next().unwrap(); // qualified name, string, etc.
        let type_name_str = type_name.as_str(); // qualified name, string, etc.
        let body = pairs.next().unwrap().into_inner();
        println!("OBJECT {} -> {}", label, type_name_str);
        let mut members: HashMap<String, Expr> = HashMap::with_capacity(body.len());
        for member in body.into_iter() {
            match member.as_rule() {
                Rule::key_value => {
                    let mut member_pairs = member.into_inner();
                    let id = member_pairs.next().unwrap();
                    let key = self.visit_key(&id);
                    let mut value = member_pairs.next().unwrap();
                    // println!("  MEMBER {}", key);
                    let expr = self.visit_expr(key, args.clone(), &mut value, base, imports)?;
                    members.insert(key.to_string(), expr);
                }

                _ => panic!("Invalid rule {:?}", member.as_rule()),
            }
            // println!("  MEMBER {:?}", member);
            // let mut member_pairs = member.into_inner();
        }

        match imports.entries.get(type_name_str) {
            Some(ImportEntry::Builtin(ty)) => {
                if let Some(rd) = ty.data::<ReflectFromAst>() {
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
        };
    }

    fn visit_key<'b>(&self, pair: &'b Pair<'b, Rule>) -> &'b str {
        match pair.as_rule() {
            Rule::identifier => pair.as_str(),
            Rule::string => pair.as_str(),
            _ => unreachable!(),
        }
    }
}

enum ImportEntry<'a> {
    Builtin(&'a TypeRegistration),
}

/// Helper class for managing use/import statements
struct Imports<'a> {
    /// Reference to enclosing scope
    next: Option<&'a Imports<'a>>,

    entries: HashMap<String, ImportEntry<'a>>,
}

impl<'a> Imports<'a> {
    fn new() -> Self {
        Self {
            next: None,
            entries: HashMap::new(),
        }
    }
}
