//! Aspect declarations: from their AST to the [`AspectDecl`]s a module exports.

use std::sync::{Arc, OnceLock};

use bevy::{
    ecs::entity::Entity,
    reflect::{
        Reflect, TypeInfo, Typed,
        attributes::{CustomAttributes, CustomAttributesBuilder},
    },
};
use smol_str::SmolStr;

use super::{
    build::ModuleExprs,
    decl::{Scope, ScopeType},
    expr::FunctionBody,
    resolve_types::resolve_types,
    types::ExprType,
};
use crate::{
    ScriptModule,
    compiler::CompilationError,
    host::{FieldKind, HostAttribute, HostState},
    scripted::{AspectDecl, AspectField, InteractionDecl},
    syntax::{
        ast::{ASTAttribute, ASTDecl, ASTInteraction, ASTNode, NodeKind},
        oper::UnaryOp,
    },
};

/// The aspect key of aspect `name` in the module at `path`: the path from the asset root, its
/// extension dropped, then the name, joined by `::`.
fn aspect_key(path: &str, name: &str) -> Arc<str> {
    let module = path.strip_suffix(".saga").unwrap_or(path);
    let mut key: String = module.split('/').collect::<Vec<_>>().join("::");
    key.push_str("::");
    key.push_str(name);
    key.into()
}

/// The Rust type a field of type `typ` holds, if a field may have it.
fn field_type_info(typ: &ExprType) -> Option<&'static TypeInfo> {
    Some(match typ {
        ExprType::Boolean => bool::type_info(),
        ExprType::I32 => i32::type_info(),
        ExprType::I64 => i64::type_info(),
        ExprType::F32 => f32::type_info(),
        ExprType::F64 => f64::type_info(),
        ExprType::String => String::type_info(),
        ExprType::Entity => Entity::type_info(),
        ExprType::Reflected(info) => info,
        _ => return None,
    })
}

/// A literal's value as a field of type `typ`: a boolean, number or string, a number negated.
fn literal(ast: &ASTNode, typ: &ExprType) -> Option<Box<dyn Reflect>> {
    let (value, negated) = match &ast.kind {
        NodeKind::UnaryExpr {
            op: UnaryOp::Neg,
            arg,
        } => (&arg.kind, true),
        kind => (kind, false),
    };
    let sign = if negated { -1.0 } else { 1.0 };
    Some(match (value, typ) {
        (NodeKind::LitBool(b), ExprType::Boolean) if !negated => Box::new(*b),
        (NodeKind::LitString(s), ExprType::String) if !negated => Box::new(s.to_string()),
        (NodeKind::LitInt(n, _), ExprType::I32) => {
            Box::new(i32::try_from(if negated { -n } else { *n }).ok()?)
        }
        (NodeKind::LitInt(n, _), ExprType::I64) => Box::new(if negated { -n } else { *n }),
        (NodeKind::LitInt(n, _), ExprType::F32) => Box::new((*n as f64 * sign) as f32),
        (NodeKind::LitInt(n, _), ExprType::F64) => Box::new(*n as f64 * sign),
        (NodeKind::LitFloat(n, _), ExprType::F32) => Box::new((n * sign) as f32),
        (NodeKind::LitFloat(n, _), ExprType::F64) => Box::new(n * sign),
        _ => return None,
    })
}

/// The custom attributes `attrs` give, each found in `table`, the host's, its arguments literals
/// of its parameters' types. `skip` names the attributes read elsewhere, which are left out.
fn attributes(
    table: &[HostAttribute],
    attrs: &[&ASTAttribute],
    skip: &[&str],
) -> Result<CustomAttributes, CompilationError> {
    let mut builder = CustomAttributesBuilder::new();
    let mut seen: Vec<&str> = Vec::with_capacity(attrs.len());
    for attr in attrs {
        if seen.contains(&attr.name) {
            return Err(CompilationError::NameRedefinition(
                attr.location,
                attr.name.into(),
            ));
        }
        seen.push(attr.name);
        if skip.contains(&attr.name) {
            continue;
        }
        let Some(host) = table.iter().find(|host| host.name == attr.name) else {
            return Err(CompilationError::UnknownAttribute(
                attr.location,
                attr.name.into(),
            ));
        };
        let args = (attr.args.len() == host.params.len())
            .then(|| {
                attr.args
                    .iter()
                    .zip(&host.params)
                    .map(|(arg, typ)| literal(arg, typ))
                    .collect::<Option<Vec<_>>>()
            })
            .flatten()
            .ok_or_else(|| CompilationError::Expected(attr.location, attribute_form(host)))?;
        builder = (host.build)(builder, &args);
    }
    Ok(builder.build())
}

/// How `attr` is written, for an error: `#[range(f64, f64)]`, or `#[read_only]`.
fn attribute_form(attr: &HostAttribute) -> SmolStr {
    if attr.params.is_empty() {
        return format!("#[{}]", attr.name).into();
    }
    let params: Vec<_> = attr.params.iter().map(ToString::to_string).collect();
    format!("#[{}({})]", attr.name, params.join(", ")).into()
}

/// The accessor `attrs` give an aspect, from `#[accessor(name)]`.
fn accessor(attrs: &[&ASTAttribute]) -> Result<Option<SmolStr>, CompilationError> {
    let Some(attr) = attrs.iter().find(|attr| attr.name == "accessor") else {
        return Ok(None);
    };
    let [
        ASTNode {
            kind: NodeKind::Ident(name),
            ..
        },
    ] = attr.args
    else {
        return Err(CompilationError::Expected(
            attr.location,
            "#[accessor(name)]".into(),
        ));
    };
    Ok(Some(SmolStr::from(*name)))
}

/// The keys of the aspects `attrs` require, from `#[require(A, B)]`, in the module at `path`,
/// which declares the aspects `declared`. A name it declares is that aspect's key; any other is
/// a native aspect's, which only an instance spawning checks.
fn requires(
    attrs: &[&ASTAttribute],
    path: &str,
    declared: &[&str],
) -> Result<Vec<Arc<str>>, CompilationError> {
    let Some(attr) = attrs.iter().find(|attr| attr.name == "require") else {
        return Ok(Vec::new());
    };
    let expected = || CompilationError::Expected(attr.location, "#[require(Name, ...)]".into());
    if attr.args.is_empty() {
        return Err(expected());
    }
    attr.args
        .iter()
        .map(|arg| match arg.kind {
            NodeKind::Ident(name) if declared.contains(&name) => Ok(aspect_key(path, name)),
            NodeKind::Ident(name) => Ok(name.into()),
            _ => Err(expected()),
        })
        .collect()
}

/// A field an interaction sets, whose function [`build_aspects`] reserves and the builder then
/// fills.
pub(crate) struct InteractionFieldFn<'a> {
    pub(crate) aspect: Arc<AspectDecl>,
    /// The function's index in the module.
    pub(crate) index: usize,
    pub(crate) value: &'a ASTNode<'a>,
    pub(crate) ret: ExprType,
    pub(crate) effect: bool,
}

/// The interactions `ast` declares, each field checked against the host's interaction type and
/// given a function of the module, which `fields` lists for the builder.
fn build_interactions<'a>(
    host: &HostState,
    exprs: &mut ModuleExprs,
    ast: &[&'a ASTInteraction<'a>],
    fields: &mut Vec<(usize, &'a ASTNode<'a>, ExprType, bool)>,
) -> Result<Vec<InteractionDecl>, CompilationError> {
    let mut interactions: Vec<InteractionDecl> = Vec::with_capacity(ast.len());
    for interaction in ast {
        if interactions.iter().any(|i| i.name == interaction.name) {
            return Err(CompilationError::NameRedefinition(
                interaction.location,
                interaction.name.to_string(),
            ));
        }
        let mut decl = InteractionDecl {
            name: interaction.name.into(),
            constants: Vec::new(),
            fields: Vec::with_capacity(interaction.fields.len()),
        };
        for field in interaction.fields {
            let Some(host_field) = host.interaction_field(field.name) else {
                return Err(CompilationError::UnknownInteractionField(
                    field.location,
                    field.name.into(),
                ));
            };
            if decl.sets(field.name) {
                return Err(CompilationError::NameRedefinition(
                    field.location,
                    field.name.to_string(),
                ));
            }
            if let FieldKind::Constant(typ) = &host_field.kind {
                let value = literal(field.value, typ).ok_or_else(|| {
                    CompilationError::NotALiteral(
                        field.value.location,
                        field.name.into(),
                        typ.clone(),
                    )
                })?;
                decl.constants.push((field.name.into(), value));
                continue;
            }
            // Its parameter and return types are the field's; the builder fills in its body.
            let index = exprs.functions.len();
            exprs.functions.push(FunctionBody {
                body: None,
                params: Vec::new(),
                locals: Vec::new(),
                ret: ExprType::Void,
            });
            decl.fields.push((field.name.into(), index));
            let effect = host_field.kind == FieldKind::Effect;
            fields.push((index, field.value, host_field.typ(), effect));
        }
        if let Some(missing) = host
            .interaction_fields
            .iter()
            .find(|f| f.required && !decl.sets(&f.name))
        {
            return Err(CompilationError::MissingInteractionField(
                interaction.location,
                interaction.name.into(),
                missing.name.clone(),
            ));
        }
        interactions.push(decl);
    }
    Ok(interactions)
}

/// Declares the module's aspects, in order, and returns the fields their interactions set.
pub(crate) fn build_aspects<'a>(
    host: &HostState,
    module: &mut ScriptModule,
    exprs: &mut ModuleExprs,
    ast_decls: &[&'a ASTNode<'a>],
) -> Result<Vec<InteractionFieldFn<'a>>, CompilationError> {
    let scope = Scope {
        parent: None,
        decls: &host.decls,
        scope_type: ScopeType::Host,
    };
    let mut interaction_fields = Vec::new();
    // Every aspect the module declares, so that one can require another declared after it.
    let declared: Vec<&str> = ast_decls
        .iter()
        .filter_map(|ast_decl| match &ast_decl.kind {
            NodeKind::Decl(ASTDecl::Aspect { name, .. }) => Some(*name),
            _ => None,
        })
        .collect();
    for ast_decl in ast_decls {
        let NodeKind::Decl(ASTDecl::Aspect {
            attrs,
            name,
            kinds,
            fields: ast_fields,
            interactions: ast_interactions,
        }) = &ast_decl.kind
        else {
            continue;
        };
        if module.aspects.iter().any(|aspect| aspect.name == *name) {
            return Err(CompilationError::NameRedefinition(
                ast_decl.location,
                name.to_string(),
            ));
        }
        let mut fields: Vec<AspectField> = Vec::with_capacity(ast_fields.len());
        for field in ast_fields.iter() {
            if fields.iter().any(|f| f.name == field.name) {
                return Err(CompilationError::NameRedefinition(
                    field.location,
                    field.name.to_string(),
                ));
            }
            let typ = resolve_types(&scope, field.typ)?;
            let Some(info) = field_type_info(&typ) else {
                return Err(CompilationError::Unsupported(
                    field.typ.location,
                    "fields of this type",
                ));
            };
            let default = match field.default {
                None => None,
                Some(default) => Some(literal(default, &typ).ok_or_else(|| {
                    CompilationError::InvalidDefault(default.location, field.name.into(), typ)
                })?),
            };
            fields.push(AspectField {
                name: field.name.into(),
                is_state: field.is_state,
                info,
                default,
                attributes: attributes(&host.field_attributes, field.attrs, &[])?,
            });
        }
        let mut fns = Vec::new();
        let interactions = build_interactions(host, exprs, ast_interactions, &mut fns)?;
        let aspect = Arc::new(AspectDecl {
            key: aspect_key(&module.path, name),
            name: (*name).into(),
            accessor: accessor(attrs)?,
            attributes: attributes(&host.aspect_attributes, attrs, &["accessor", "require"])?,
            requires: requires(attrs, &module.path, &declared)?,
            kinds: kinds.iter().map(|kind| (*kind).into()).collect(),
            fields,
            interactions,
            module: OnceLock::new(),
        });
        interaction_fields.extend(fns.into_iter().map(|(index, value, ret, effect)| {
            InteractionFieldFn {
                aspect: aspect.clone(),
                index,
                value,
                ret,
                effect,
            }
        }));
        module.aspects.push(aspect);
    }
    Ok(interaction_fields)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_join_the_path_and_the_name() {
        assert_eq!(
            &*aspect_key("scripts/toggle_switch.saga", "ToggleSwitch"),
            "scripts::toggle_switch::ToggleSwitch"
        );
    }
}
