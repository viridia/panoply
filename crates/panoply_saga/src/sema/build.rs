//! AST to typed tree: name resolution and type constraints.

use std::{any::TypeId, sync::Arc};

use bevy::{ecs::entity::Entity, reflect::TypeInfo};
use bumpalo::Bump;
use smol_str::SmolStr;

use super::{
    check::check_types,
    decl::{Decl, DeclKind, DeclTable, DeclVisibility, FunctionParam, Scope, ScopeType},
    expr::{Expr, ExprKind, FunctionBody},
    infer::TypeInference,
    resolve_types::resolve_types,
    types::{ExprType, FunctionType, TypeTable},
};
use crate::{
    ScriptModule,
    compiler::CompilationError,
    host::{HostState, HostType},
    syntax::{
        ast::{ASTDecl, ASTNode, FloatSuffix, IntegerSuffix, NodeKind},
        location::TokenLocation,
        oper::{BinaryOp, UnaryOp},
    },
};

/// The typed trees of a module's functions, and the types they refer to.
#[derive(Default)]
pub(crate) struct ModuleExprs<'e> {
    pub(crate) functions: Vec<FunctionBody<'e>>,
    pub(crate) types: TypeTable,
}

pub(crate) fn build_module_exprs<'e>(
    host: &HostState,
    module: &mut ScriptModule,
    ast: &ASTNode,
    arena: &'e Bump,
) -> Result<ModuleExprs<'e>, CompilationError> {
    let NodeKind::Module(ast_decls) = &ast.kind else {
        return Err(CompilationError::Internal(ast.location, "module root"));
    };
    let mut exprs = ModuleExprs::default();
    create_decls(module, &mut exprs, ast_decls)?;
    resolve_decl_types(host, module, ast_decls)?;
    build_function_bodies(host, module, ast_decls, &mut exprs, arena)?;
    Ok(exprs)
}

pub(crate) fn build_formula_exprs<'e>(
    host: &HostState,
    module: &mut ScriptModule,
    ast: &ASTNode,
    ret_type: ExprType,
    arena: &'e Bump,
) -> Result<ModuleExprs<'e>, CompilationError> {
    // The `formula` rule produces a block.
    let NodeKind::Block(stmts, result) = &ast.kind else {
        return Err(CompilationError::Internal(ast.location, "formula root"));
    };
    let mut exprs = ModuleExprs::default();

    let host_scope = Scope {
        parent: None,
        decls: &host.decls,
        scope_type: ScopeType::Host,
    };
    let module_scope = Scope {
        parent: Some(&host_scope),
        decls: &module.module_decls,
        scope_type: ScopeType::Module,
    };

    let mut builder = FunctionBuilder::new(host, arena, &mut exprs.types);
    let body = builder.build_block(ast, &module_scope, stmts, result)?;
    builder.take_self_components(module);
    let function = builder.finish(body, ret_type.clone(), 0)?;
    exprs.functions.push(function);

    module.module_decls.insert(
        ScriptModule::DEFAULT.into(),
        Decl {
            location: ast.location,
            visibility: DeclVisibility::Private,
            typ: ExprType::Function(Arc::new(FunctionType {
                params: Vec::new(),
                ret: ret_type,
            })),
            kind: DeclKind::Function { index: 0 },
        },
    );
    Ok(exprs)
}

/// Populate the module scope with all of the declarations.
fn create_decls(
    module: &mut ScriptModule,
    exprs: &mut ModuleExprs,
    ast_decls: &[&ASTNode],
) -> Result<(), CompilationError> {
    for ast_decl in ast_decls {
        match &ast_decl.kind {
            // TODO(PAN71): resolve the import through the module resolver.
            NodeKind::Import(..) => {
                return Err(CompilationError::Unsupported(ast_decl.location, "imports"));
            }
            NodeKind::Decl(ASTDecl::Function {
                name, visibility, ..
            }) => {
                if module.module_decls.contains_key(*name) {
                    return Err(CompilationError::FunctionRedefinition(
                        ast_decl.location,
                        name.to_string(),
                    ));
                }
                // The parameter and return types are filled in once every declaration exists.
                let index = exprs.functions.len();
                exprs.functions.push(FunctionBody {
                    body: None,
                    locals: Vec::new(),
                    num_params: 0,
                });
                module.module_decls.insert(
                    (*name).into(),
                    Decl {
                        location: ast_decl.location,
                        visibility: *visibility,
                        typ: ExprType::None,
                        kind: DeclKind::Function { index },
                    },
                );
            }
            // TODO(X91): module-level constants.
            NodeKind::Decl(ASTDecl::Let { .. }) => {
                return Err(CompilationError::Unsupported(
                    ast_decl.location,
                    "module-level variables",
                ));
            }
            // TODO(X87): script-declared structs.
            NodeKind::Decl(ASTDecl::Struct) => {
                return Err(CompilationError::Unsupported(
                    ast_decl.location,
                    "struct declarations",
                ));
            }
            _ => return Err(CompilationError::ExpectDeclaration(ast_decl.location)),
        }
    }
    Ok(())
}

/// Assign the parameter and return types of each function.
fn resolve_decl_types(
    host: &HostState,
    module: &mut ScriptModule,
    ast_decls: &[&ASTNode],
) -> Result<(), CompilationError> {
    let host_scope = Scope {
        parent: None,
        decls: &host.decls,
        scope_type: ScopeType::Host,
    };

    for ast_decl in ast_decls {
        let NodeKind::Decl(ASTDecl::Function {
            name, params, ret, ..
        }) = &ast_decl.kind
        else {
            continue;
        };
        let module_scope = Scope {
            parent: Some(&host_scope),
            decls: &module.module_decls,
            scope_type: ScopeType::Module,
        };
        let ret = match ret {
            None => ExprType::Void,
            Some(ret) => resolve_types(&module_scope, ret)?,
        };
        let mut mapped = Vec::with_capacity(params.len());
        for (index, p) in params.iter().enumerate() {
            if mapped.iter().any(|m: &FunctionParam| m.name == p.name) {
                return Err(CompilationError::NameRedefinition(
                    p.location,
                    p.name.to_string(),
                ));
            }
            mapped.push(FunctionParam {
                location: p.location,
                name: p.name.into(),
                typ: resolve_types(&module_scope, p.typ)?,
                index,
            });
        }
        let Some(decl) = module.module_decls.get_mut(*name) else {
            return Err(CompilationError::Internal(
                ast_decl.location,
                "function decl",
            ));
        };
        decl.typ = ExprType::Function(Arc::new(FunctionType {
            params: mapped,
            ret,
        }));
    }
    Ok(())
}

fn build_function_bodies<'e>(
    host: &HostState,
    module: &mut ScriptModule,
    ast_decls: &[&ASTNode],
    exprs: &mut ModuleExprs<'e>,
    arena: &'e Bump,
) -> Result<(), CompilationError> {
    let host_scope = Scope {
        parent: None,
        decls: &host.decls,
        scope_type: ScopeType::Host,
    };

    for ast_decl in ast_decls {
        let NodeKind::Decl(ASTDecl::Function { name, body, .. }) = &ast_decl.kind else {
            continue;
        };
        let Some(Decl {
            kind: DeclKind::Function { index },
            typ: ExprType::Function(ftype),
            ..
        }) = module.module_decls.get(*name)
        else {
            return Err(CompilationError::Internal(
                ast_decl.location,
                "function decl",
            ));
        };
        let index = *index;
        let ftype = ftype.clone();

        let mut param_decls = DeclTable::default();
        for param in ftype.params.iter() {
            param_decls.insert(
                param.name.clone(),
                Decl {
                    location: param.location,
                    visibility: DeclVisibility::Public,
                    typ: param.typ.clone(),
                    kind: DeclKind::Param { index: param.index },
                },
            );
        }
        let module_scope = Scope {
            parent: Some(&host_scope),
            decls: &module.module_decls,
            scope_type: ScopeType::Module,
        };
        let param_scope = Scope {
            parent: Some(&module_scope),
            decls: &param_decls,
            scope_type: ScopeType::Param,
        };

        let mut builder = FunctionBuilder::new(host, arena, &mut exprs.types);
        let body_expr = builder.build_expr(body, &param_scope)?;
        builder.take_self_components(module);
        exprs.functions[index] =
            builder.finish(body_expr, ftype.ret.clone(), ftype.params.len())?;
    }
    Ok(())
}

/// Builds one function's typed tree.
struct FunctionBuilder<'h, 'e, 't> {
    host: &'h HostState,
    arena: &'e Bump,
    types: &'t mut TypeTable,
    /// Where this function's types start in the table: its inference variables are its own.
    first_type: usize,
    inference: TypeInference,
    locals: Vec<ExprType>,
    /// How deeply `build_expr` has recursed: the tree's depth, which the passes after it recurse
    /// to as well.
    depth: usize,
    /// The components the function reads from `self`, for its module's
    /// [`ScriptModule::self_components`].
    self_components: Vec<TypeId>,
}

/// How deep a typed tree may be. A long chain of binary operators is as deep as it is long.
const MAX_DEPTH: usize = 256;

impl<'h, 'e, 't> FunctionBuilder<'h, 'e, 't> {
    fn new(host: &'h HostState, arena: &'e Bump, types: &'t mut TypeTable) -> Self {
        let first_type = types.len();
        Self {
            host,
            arena,
            types,
            first_type,
            inference: TypeInference::default(),
            locals: Vec::new(),
            depth: 0,
            self_components: Vec::new(),
        }
    }

    /// Adds the components this function reads from `self` to `module`'s.
    fn take_self_components(&mut self, module: &mut ScriptModule) {
        for id in self.self_components.drain(..) {
            if !module.self_components.contains(&id) {
                module.self_components.push(id);
            }
        }
    }

    /// Solves the function's types, checks its tree, and returns it.
    fn finish(
        mut self,
        body: &'e Expr<'e>,
        ret: ExprType,
        num_params: usize,
    ) -> Result<FunctionBody<'e>, CompilationError> {
        let body_type = self.types.get(body.typ).clone();
        self.inference.add_constraint(ret, body_type, body.location);
        self.inference.solve_constraints()?;
        let inference = &self.inference;
        self.types
            .map_from(self.first_type, |t| inference.substitute(t));
        for local in self.locals.iter_mut() {
            *local = inference.substitute(local);
        }
        check_types(body, self.types)?;
        Ok(FunctionBody {
            body: Some(body),
            num_params,
            locals: self.locals,
        })
    }

    fn expr(&mut self, location: TokenLocation, kind: ExprKind<'e>, typ: ExprType) -> &'e Expr<'e> {
        let typ = self.types.add(typ);
        self.arena.alloc(Expr {
            location,
            kind,
            typ,
        })
    }

    fn type_of(&self, expr: &Expr) -> ExprType {
        self.types.get(expr.typ).clone()
    }

    fn constrain(&mut self, left: ExprType, right: &Expr) {
        let right_type = self.type_of(right);
        self.inference
            .add_constraint(left, right_type, right.location);
    }

    fn build_expr(
        &mut self,
        ast: &ASTNode,
        scope: &Scope,
    ) -> Result<&'e Expr<'e>, CompilationError> {
        if self.depth >= MAX_DEPTH {
            return Err(CompilationError::Unsupported(
                ast.location,
                "expressions nested this deep",
            ));
        }
        self.depth += 1;
        let result = self.build_expr_inner(ast, scope);
        self.depth -= 1;
        result
    }

    fn build_expr_inner(
        &mut self,
        ast: &ASTNode,
        scope: &Scope,
    ) -> Result<&'e Expr<'e>, CompilationError> {
        let loc = ast.location;
        match &ast.kind {
            NodeKind::LitInt(value, suffix) => {
                let typ = match suffix {
                    IntegerSuffix::Unsized if i32::try_from(*value).is_err() => ExprType::I64,
                    IntegerSuffix::Unsized | IntegerSuffix::I32 => ExprType::I32,
                    IntegerSuffix::I64 => ExprType::I64,
                };
                if typ == ExprType::I32 && i32::try_from(*value).is_err() {
                    return Err(CompilationError::Expected(loc, "an i32 in range".into()));
                }
                Ok(self.expr(loc, ExprKind::ConstInteger(*value), typ))
            }

            NodeKind::LitFloat(value, suffix) => {
                let typ = match suffix {
                    FloatSuffix::F32 => ExprType::F32,
                    FloatSuffix::F64 => ExprType::F64,
                };
                Ok(self.expr(loc, ExprKind::ConstFloat(*value), typ))
            }

            NodeKind::LitString(value) => {
                let value = self.arena.alloc_str(value);
                Ok(self.expr(loc, ExprKind::ConstString(value), ExprType::String))
            }

            NodeKind::LitBool(value) => {
                Ok(self.expr(loc, ExprKind::ConstBool(*value), ExprType::Boolean))
            }

            NodeKind::Ident(name) => self.build_ident(loc, name, scope),

            NodeKind::QName(parts) => self.build_qname(loc, parts, scope),

            NodeKind::BinaryExpr { op, lhs, rhs } => {
                let lhs = self.build_expr(lhs, scope)?;
                let rhs = self.build_expr(rhs, scope)?;
                let typ = match op {
                    BinaryOp::LogAnd | BinaryOp::LogOr => {
                        self.constrain(ExprType::Boolean, lhs);
                        self.constrain(ExprType::Boolean, rhs);
                        ExprType::Boolean
                    }
                    BinaryOp::Eq
                    | BinaryOp::Ne
                    | BinaryOp::Lt
                    | BinaryOp::Le
                    | BinaryOp::Gt
                    | BinaryOp::Ge => {
                        // Both sides the same type; the result is a boolean.
                        let operand = self.inference.fresh_typevar();
                        self.constrain(operand.clone(), lhs);
                        self.constrain(operand, rhs);
                        ExprType::Boolean
                    }
                    _ => {
                        // Both sides the same type, which is also the result's.
                        let typ = self.inference.fresh_typevar();
                        self.constrain(typ.clone(), lhs);
                        self.constrain(typ.clone(), rhs);
                        typ
                    }
                };
                Ok(self.expr(loc, ExprKind::BinaryExpr { op: *op, lhs, rhs }, typ))
            }

            NodeKind::UnaryExpr { op, arg } => {
                let arg = self.build_expr(arg, scope)?;
                let typ = match op {
                    UnaryOp::Not => {
                        self.constrain(ExprType::Boolean, arg);
                        ExprType::Boolean
                    }
                    UnaryOp::Neg | UnaryOp::BitNot => self.type_of(arg),
                };
                Ok(self.expr(loc, ExprKind::UnaryExpr { op: *op, arg }, typ))
            }

            NodeKind::Assign { lhs, rhs } => {
                let lhs = self.build_lvalue(lhs, scope)?;
                let rhs = self.build_expr(rhs, scope)?;
                self.constrain(self.type_of(lhs), rhs);
                // Assignments do not chain.
                Ok(self.expr(loc, ExprKind::Assign { lhs, rhs }, ExprType::Void))
            }

            NodeKind::AssignOp { op, lhs, rhs } => {
                let lhs = self.build_lvalue(lhs, scope)?;
                let rhs = self.build_expr(rhs, scope)?;
                self.constrain(self.type_of(lhs), rhs);
                Ok(self.expr(
                    loc,
                    ExprKind::AssignOp { op: *op, lhs, rhs },
                    ExprType::Void,
                ))
            }

            NodeKind::FieldName(base, name) => {
                let base = self.build_expr(base, scope)?;
                self.build_member(loc, base, name)
            }

            // TODO(X7): tuple fields, laid out in slots.
            NodeKind::FieldIndex => Err(CompilationError::Unsupported(loc, "tuple fields")),

            NodeKind::Empty => Ok(self.expr(loc, ExprKind::Empty, ExprType::Void)),

            NodeKind::Block(stmts, result) => self.build_block(ast, scope, stmts, result),

            NodeKind::Cast { arg, typ } => {
                let to = resolve_types(scope, typ)?;
                let arg = self.build_expr(arg, scope)?;
                // Whether the conversion is allowed is checked once the argument's type is
                // solved.
                Ok(self.expr(loc, ExprKind::Cast(arg), to))
            }

            NodeKind::Call(func, args) => {
                let func = self.build_expr(func, scope)?;
                let ExprType::Function(fty) = self.type_of(func) else {
                    return Err(CompilationError::NotCallable(loc));
                };
                if fty.params.len() != args.len() {
                    return Err(CompilationError::IncorrectNumberOfArguments(
                        loc,
                        fty.params.len(),
                        args.len(),
                    ));
                }
                let mut arg_exprs = Vec::with_capacity(args.len());
                for (param, arg) in fty.params.iter().zip(args.iter()) {
                    let arg = self.build_expr(arg, scope)?;
                    self.constrain(param.typ.clone(), arg);
                    arg_exprs.push(arg);
                }
                let args = self.arena.alloc_slice_copy(&arg_exprs);
                Ok(self.expr(loc, ExprKind::Call(func, args), fty.ret.clone()))
            }

            NodeKind::If {
                condition,
                then_block,
                else_block,
            } => {
                let condition = self.build_expr(condition, scope)?;
                self.constrain(ExprType::Boolean, condition);
                let then_branch = self.build_expr(then_block, scope)?;
                match else_block {
                    Some(else_block) => {
                        let else_branch = self.build_expr(else_block, scope)?;
                        // Both branches have the same type, which is also the result's.
                        let typ = self.inference.fresh_typevar();
                        self.constrain(typ.clone(), then_branch);
                        self.constrain(typ.clone(), else_branch);
                        Ok(self.expr(
                            loc,
                            ExprKind::If {
                                condition,
                                then_branch,
                                else_branch: Some(else_branch),
                            },
                            typ,
                        ))
                    }
                    None => {
                        // With no else branch, the then branch has no value.
                        self.constrain(ExprType::Void, then_branch);
                        Ok(self.expr(
                            loc,
                            ExprKind::If {
                                condition,
                                then_branch,
                                else_branch: None,
                            },
                            ExprType::Void,
                        ))
                    }
                }
            }

            NodeKind::Decl(_) => Err(CompilationError::ExpectExpression(loc)),

            NodeKind::Module(_) | NodeKind::Import(..) | NodeKind::ArrayType => {
                Err(CompilationError::ExpectExpression(loc))
            }
        }
    }

    fn build_ident(
        &mut self,
        loc: TokenLocation,
        name: &str,
        scope: &Scope,
    ) -> Result<&'e Expr<'e>, CompilationError> {
        let Some((scope_type, decl)) = scope.lookup(name) else {
            return Err(CompilationError::UnknownSymbol(loc, name.to_string()));
        };
        let kind = match (&decl.kind, scope_type) {
            (DeclKind::Function { index }, ScopeType::Host) => ExprKind::HostFunctionRef(*index),
            (DeclKind::Function { index }, ScopeType::Module) => {
                ExprKind::ScriptFunctionRef(*index)
            }
            (DeclKind::Function { .. }, _) => {
                return Err(CompilationError::Internal(loc, "function in a local scope"));
            }
            (DeclKind::Local { index, .. }, _) => ExprKind::LocalRef(*index),
            (DeclKind::Global { index, .. }, _) => ExprKind::GlobalRef(*index),
            (DeclKind::Param { index }, _) => ExprKind::ParamRef(*index),
            (DeclKind::NativeType { .. } | DeclKind::TypeAlias, _) => {
                return Err(CompilationError::TypeAsValue(loc, name.into()));
            }
        };
        Ok(self.expr(loc, kind, decl.typ.clone()))
    }

    fn build_qname(
        &mut self,
        loc: TokenLocation,
        parts: &[&ASTNode],
        scope: &Scope,
    ) -> Result<&'e Expr<'e>, CompilationError> {
        let [head, member] = parts else {
            // TODO(PAN71): names qualified by an imported module.
            return Err(CompilationError::Unsupported(
                loc,
                "names with more than two parts",
            ));
        };
        let (NodeKind::Ident(head_name), NodeKind::Ident(member_name)) = (&head.kind, &member.kind)
        else {
            return Err(CompilationError::Internal(loc, "qualified name parts"));
        };
        let Some((_, decl)) = scope.lookup(head_name) else {
            return Err(CompilationError::UnknownSymbol(
                head.location,
                head_name.to_string(),
            ));
        };
        let DeclKind::NativeType { id } = decl.kind else {
            return Err(CompilationError::NoMembers(
                head.location,
                (*head_name).into(),
            ));
        };
        let member_decl = self
            .host
            .get_host_type_by_id(id)
            .and_then(|host_type| host_type.get(member_name))
            .ok_or_else(|| {
                CompilationError::UnknownField(loc, (*head_name).into(), (*member_name).into())
            })?;
        match member_decl.kind {
            DeclKind::Function { index } => Ok(self.expr(
                loc,
                ExprKind::HostFunctionRef(index),
                member_decl.typ.clone(),
            )),
            _ => Err(CompilationError::UnknownField(
                loc,
                (*head_name).into(),
                (*member_name).into(),
            )),
        }
    }

    /// A member of a value: a host type's property or method, or a reflected struct's field.
    fn build_member(
        &mut self,
        loc: TokenLocation,
        base: &'e Expr<'e>,
        name: &str,
    ) -> Result<&'e Expr<'e>, CompilationError> {
        let base_type = self.type_of(base);
        let host_type: Option<&HostType> = match &base_type {
            ExprType::Entity => self.host.get_host_type::<Entity>(),
            ExprType::String => self.host.get_host_type::<String>(),
            ExprType::Reflected(type_info) => self.host.get_host_type_by_id(type_info.type_id()),
            _ => return Err(CompilationError::NoFields(loc, base_type)),
        };
        if let Some(member) = host_type.and_then(|host_type| host_type.get(name)) {
            let kind = match member.kind {
                DeclKind::Global { index, .. } => {
                    if let ExprKind::GlobalRef(base_index) = &base.kind
                        && Some(*base_index) == self.host.self_index
                        && let Some(&id) = self.host.component_members.get(&index)
                        && !self.self_components.contains(&id)
                    {
                        self.self_components.push(id);
                    }
                    ExprKind::NativeProp(base, index)
                }
                DeclKind::Function { index } => ExprKind::HostMethodRef(base, index),
                _ => return Err(CompilationError::Internal(loc, "host type member")),
            };
            return Ok(self.expr(loc, kind, member.typ.clone()));
        }

        let unknown =
            || CompilationError::UnknownField(loc, base_type.to_string().into(), name.into());
        match &base_type {
            ExprType::Reflected(TypeInfo::Struct(info)) => {
                let (Some(field), Some(index)) = (info.field(name), info.index_of(name)) else {
                    return Err(unknown());
                };
                let Some(field_info) = field.type_info() else {
                    return Err(CompilationError::Unsupported(loc, "fields of this type"));
                };
                Ok(self.expr(
                    loc,
                    ExprKind::Field(base, index),
                    ExprType::from_type_info(field_info),
                ))
            }
            // TODO(X7): tuple structs, tuples and enums, through value slots.
            ExprType::Reflected(_) => Err(CompilationError::Unsupported(
                loc,
                "fields of this kind of type",
            )),
            _ => Err(unknown()),
        }
    }

    /// The target of an assignment: a mutable local.
    fn build_lvalue(
        &mut self,
        ast: &ASTNode,
        scope: &Scope,
    ) -> Result<&'e Expr<'e>, CompilationError> {
        if let NodeKind::Ident(name) = &ast.kind
            && let Some((_, decl)) = scope.lookup(name)
            && let DeclKind::Local {
                is_const: false, ..
            } = decl.kind
        {
            return self.build_ident(ast.location, name, scope);
        }
        Err(CompilationError::InvalidAssignmentTarget(ast.location))
    }

    fn build_block(
        &mut self,
        ast: &ASTNode,
        scope: &Scope,
        stmts: &[&ASTNode],
        result: &Option<&ASTNode>,
    ) -> Result<&'e Expr<'e>, CompilationError> {
        let mut block_decls = DeclTable::default();
        let mut stmt_exprs = Vec::with_capacity(stmts.len());
        for stmt in stmts {
            stmt_exprs.push(self.build_stmt(stmt, scope, &mut block_decls)?);
        }
        let result_expr = match result {
            Some(result) => Some(self.build_stmt(result, scope, &mut block_decls)?),
            None => None,
        };
        let (typ, location) = match result_expr {
            Some(expr) => (self.type_of(expr), expr.location),
            None => (ExprType::Void, ast.location),
        };
        let stmts = self.arena.alloc_slice_copy(&stmt_exprs);
        Ok(self.expr(location, ExprKind::Block(stmts, result_expr), typ))
    }

    fn build_stmt(
        &mut self,
        ast: &ASTNode,
        parent_scope: &Scope,
        block_decls: &mut DeclTable,
    ) -> Result<&'e Expr<'e>, CompilationError> {
        let loc = ast.location;
        let NodeKind::Decl(decl) = &ast.kind else {
            let block_scope = Scope {
                parent: Some(parent_scope),
                scope_type: ScopeType::Local,
                decls: block_decls,
            };
            return self.build_expr(ast, &block_scope);
        };
        let ASTDecl::Let {
            name,
            typ,
            value,
            is_const,
            visibility,
        } = decl
        else {
            return Err(CompilationError::Unsupported(
                loc,
                "declarations inside a block",
            ));
        };
        let Some(value) = value else {
            return Err(CompilationError::Expected(loc, "an initial value".into()));
        };

        // The initializer sees the block's earlier locals, not the one it declares.
        let (typ, init) = {
            let block_scope = Scope {
                parent: Some(parent_scope),
                scope_type: ScopeType::Local,
                decls: block_decls,
            };
            let init = self.build_expr(value, &block_scope)?;
            let typ = match typ {
                Some(typ) => {
                    let typ = resolve_types(&block_scope, typ)?;
                    self.constrain(typ.clone(), init);
                    typ
                }
                None => self.type_of(init),
            };
            (typ, init)
        };

        let local_index = self.locals.len();
        self.locals.push(typ.clone());
        block_decls.insert(
            SmolStr::new(name),
            Decl {
                location: loc,
                visibility: *visibility,
                typ: typ.clone(),
                kind: DeclKind::Local {
                    is_const: *is_const,
                    index: local_index,
                },
            },
        );
        let lhs = self.expr(loc, ExprKind::LocalRef(local_index), typ);
        Ok(self.expr(loc, ExprKind::Assign { lhs, rhs: init }, ExprType::Void))
    }
}
