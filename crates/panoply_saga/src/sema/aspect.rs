//! Aspect declarations: from their AST to the [`AspectDecl`]s a module exports.

use std::sync::{Arc, OnceLock};

use bevy::{
    ecs::entity::Entity,
    reflect::{Reflect, TypeInfo, Typed},
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
    host::{FieldKind, HostState},
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

/// The accessor `attrs` give an aspect. `accessor` is the only attribute an aspect takes.
fn accessor(attrs: &[&ASTAttribute]) -> Result<Option<SmolStr>, CompilationError> {
    let mut accessor = None;
    for attr in attrs {
        if attr.name != "accessor" {
            return Err(CompilationError::UnknownAttribute(
                attr.location,
                attr.name.into(),
            ));
        }
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
        if accessor.replace(SmolStr::from(*name)).is_some() {
            return Err(CompilationError::NameRedefinition(
                attr.location,
                "accessor".into(),
            ));
        }
    }
    Ok(accessor)
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
            });
        }
        let mut fns = Vec::new();
        let interactions = build_interactions(host, exprs, ast_interactions, &mut fns)?;
        let aspect = Arc::new(AspectDecl {
            key: aspect_key(&module.path, name),
            name: (*name).into(),
            accessor: accessor(attrs)?,
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
