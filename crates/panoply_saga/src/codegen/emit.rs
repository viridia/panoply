//! Typed tree to basic blocks to bytecode.

use super::{
    bblock::{BasicBlock, BlockTerminator, StackOp, StackOpType},
    instr::{self, InstructionBuilder},
};
use crate::{
    CompiledFunction, ScriptModule,
    compiler::CompilationError,
    sema::{
        build::ModuleExprs,
        expr::{Expr, ExprKind, FunctionBody},
        types::{ExprType, TypeTable},
    },
    syntax::{
        location::TokenLocation,
        oper::{BinaryOp, UnaryOp},
    },
};

#[derive(Default)]
struct BlockBuilder<'e> {
    blocks: Vec<BasicBlock<'e>>,
    current: usize,
}

impl<'e> BlockBuilder<'e> {
    fn new_block(&mut self) -> usize {
        let id = self.blocks.len();
        self.blocks.push(BasicBlock::default());
        id
    }

    fn add_op(&mut self, op: StackOp<'e>) {
        self.blocks[self.current].ops.push(op);
    }

    fn set_block_term(&mut self, term: BlockTerminator) {
        self.blocks[self.current].term = term;
    }
}

/// An index or count as a 16-bit immediate.
fn imm16(location: TokenLocation, n: usize, what: &'static str) -> Result<u16, CompilationError> {
    u16::try_from(n).map_err(|_| CompilationError::Unsupported(location, what))
}

pub(crate) fn gen_code(
    module: &mut ScriptModule,
    exprs: &ModuleExprs,
) -> Result<(), CompilationError> {
    module.functions.clear();
    for function in exprs.functions.iter() {
        let mut emitter = Emitter {
            function,
            types: &exprs.types,
            out: BlockBuilder::default(),
        };
        emitter.out.current = emitter.out.new_block();
        if let Some(body) = function.body {
            emitter.visit_expr(body)?;
            if exprs.types.get(body.typ).is_void() {
                emitter.out.add_op(StackOp::ConstVoid);
            }
        } else {
            emitter.out.add_op(StackOp::ConstVoid);
        }
        emitter.out.set_block_term(BlockTerminator::Return);
        module.functions.push(CompiledFunction {
            code: build_instructions(emitter.out)?.inner(),
            num_locals: function.locals.len(),
        });
    }
    Ok(())
}

struct Emitter<'f, 'e> {
    function: &'f FunctionBody<'e>,
    types: &'f TypeTable,
    out: BlockBuilder<'e>,
}

/// The primitive type of a value, for typed operators.
fn op_type(location: TokenLocation, typ: &ExprType) -> Result<StackOpType, CompilationError> {
    match typ {
        ExprType::Boolean => Ok(StackOpType::Bool),
        ExprType::I32 => Ok(StackOpType::I32),
        ExprType::I64 => Ok(StackOpType::I64),
        ExprType::F32 => Ok(StackOpType::F32),
        ExprType::F64 => Ok(StackOpType::F64),
        _ => Err(CompilationError::Internal(
            location,
            "operator on a non-primitive",
        )),
    }
}

impl<'f, 'e> Emitter<'f, 'e> {
    fn type_of(&self, expr: &Expr) -> &'f ExprType {
        self.types.get(expr.typ)
    }

    /// The stack slot of a local: after the parameters.
    fn local_slot(&self, location: TokenLocation, index: usize) -> Result<u16, CompilationError> {
        imm16(
            location,
            self.function.num_params + index,
            "more than 65535 locals",
        )
    }

    fn visit_expr(&mut self, expr: &'e Expr<'e>) -> Result<(), CompilationError> {
        let loc = expr.location;
        match &expr.kind {
            ExprKind::Empty => {}
            ExprKind::ConstBool(b) => self.out.add_op(StackOp::ConstBool(*b)),
            ExprKind::ConstInteger(n) => match self.type_of(expr) {
                ExprType::I32 => {
                    let n = i32::try_from(*n)
                        .map_err(|_| CompilationError::Internal(loc, "i32 constant range"))?;
                    self.out.add_op(StackOp::ConstI32(n));
                }
                ExprType::I64 => self.out.add_op(StackOp::ConstI64(*n)),
                _ => return Err(CompilationError::Internal(loc, "integer constant type")),
            },
            ExprKind::ConstFloat(n) => match self.type_of(expr) {
                ExprType::F32 => self.out.add_op(StackOp::ConstF32(*n as f32)),
                ExprType::F64 => self.out.add_op(StackOp::ConstF64(*n)),
                _ => return Err(CompilationError::Internal(loc, "float constant type")),
            },
            ExprKind::ConstString(s) => {
                imm16(loc, s.len(), "string literals longer than 65535 bytes")?;
                self.out.add_op(StackOp::ConstStr(s));
            }
            ExprKind::ParamRef(index) => {
                let index = imm16(loc, *index, "more than 65535 parameters")?;
                self.out.add_op(StackOp::LoadParam(index));
            }
            ExprKind::LocalRef(index) => {
                // A void local holds nothing.
                if !self.type_of(expr).is_void() {
                    let slot = self.local_slot(loc, *index)?;
                    self.out.add_op(StackOp::LoadLocal(slot));
                }
            }
            ExprKind::Field(base, field_index) => {
                self.visit_expr(base)?;
                let field_index = imm16(loc, *field_index, "more than 65535 fields")?;
                self.out.add_op(StackOp::LoadField(field_index));
            }
            ExprKind::NativeProp(base, index) => {
                self.visit_expr(base)?;
                let index = imm16(loc, *index, "more than 65535 host members")?;
                self.out.add_op(StackOp::LoadNativeProp(index));
            }
            ExprKind::GlobalRef(index) => {
                let index = imm16(loc, *index, "more than 65535 host members")?;
                self.out.add_op(StackOp::LoadGlobal(index));
            }
            ExprKind::BinaryExpr { op, lhs, rhs } => match op {
                BinaryOp::LogAnd | BinaryOp::LogOr => self.visit_logical(*op, lhs, rhs)?,
                _ => {
                    self.visit_expr(lhs)?;
                    self.visit_expr(rhs)?;
                    let typ = op_type(loc, self.type_of(lhs))?;
                    self.out.add_op(StackOp::BinaryOp(*op, typ));
                }
            },
            ExprKind::Assign { lhs, rhs } => self.visit_assign(lhs, rhs, None)?,
            ExprKind::AssignOp { op, lhs, rhs } => self.visit_assign(lhs, rhs, Some(*op))?,
            ExprKind::UnaryExpr { op, arg } => {
                self.visit_expr(arg)?;
                let typ = op_type(loc, self.type_of(arg))?;
                self.out.add_op(StackOp::UnaryOp(*op, typ));
            }
            ExprKind::Cast(arg) => {
                self.visit_expr(arg)?;
                let from = self.type_of(arg);
                let to = self.type_of(expr);
                if from != to {
                    let from = op_type(loc, from)?;
                    let to = op_type(loc, to)?;
                    self.out.add_op(StackOp::Convert { from, to });
                }
            }
            ExprKind::Call(func, args) => {
                let (num_args, make_op): (usize, fn(u16, u16) -> StackOp<'e>) = match &func.kind {
                    ExprKind::HostFunctionRef(_) => (args.len(), |method, num_args| {
                        StackOp::CallHostFunction { method, num_args }
                    }),
                    ExprKind::ScriptFunctionRef(_) => (args.len(), |method, num_args| {
                        StackOp::CallScriptFunction { method, num_args }
                    }),
                    ExprKind::HostMethodRef(base, _) => {
                        self.visit_expr(base)?;
                        (args.len() + 1, |method, num_args| StackOp::CallHostMethod {
                            method,
                            num_args,
                        })
                    }
                    _ => return Err(CompilationError::NotCallable(loc)),
                };
                let (ExprKind::HostFunctionRef(index)
                | ExprKind::ScriptFunctionRef(index)
                | ExprKind::HostMethodRef(_, index)) = func.kind
                else {
                    return Err(CompilationError::Internal(loc, "callee"));
                };
                for arg in args.iter() {
                    self.visit_expr(arg)?;
                }
                let method = imm16(loc, index, "more than 65535 functions")?;
                let num_args = imm16(loc, num_args, "more than 65535 arguments")?;
                self.out.add_op(make_op(method, num_args));
            }
            ExprKind::Block(stmts, result) => {
                for stmt in stmts.iter() {
                    self.visit_expr(stmt)?;
                    if !matches!(self.type_of(stmt), ExprType::Void | ExprType::None) {
                        self.out.add_op(StackOp::Drop(1));
                    }
                }
                if let Some(result) = result {
                    self.visit_expr(result)?;
                }
            }
            ExprKind::If {
                condition,
                then_branch,
                else_branch,
            } => {
                self.visit_expr(condition)?;
                let then_block = self.out.new_block();
                match else_branch {
                    Some(else_branch) => {
                        let else_block = self.out.new_block();
                        let next_block = self.out.new_block();
                        self.out.set_block_term(BlockTerminator::Branch {
                            then_target: then_block,
                            else_target: else_block,
                        });
                        self.out.current = then_block;
                        self.visit_expr(then_branch)?;
                        self.out
                            .set_block_term(BlockTerminator::Jump { target: next_block });
                        self.out.current = else_block;
                        self.visit_expr(else_branch)?;
                        self.out
                            .set_block_term(BlockTerminator::Jump { target: next_block });
                        self.out.current = next_block;
                    }
                    None => {
                        let next_block = self.out.new_block();
                        self.out.set_block_term(BlockTerminator::Branch {
                            then_target: then_block,
                            else_target: next_block,
                        });
                        self.out.current = then_block;
                        self.visit_expr(then_branch)?;
                        self.out
                            .set_block_term(BlockTerminator::Jump { target: next_block });
                        self.out.current = next_block;
                    }
                }
            }
            ExprKind::ScriptFunctionRef(_)
            | ExprKind::HostFunctionRef(_)
            | ExprKind::HostMethodRef(..) => {
                // TODO(X92): functions as values, for closures.
                return Err(CompilationError::Unsupported(loc, "functions as values"));
            }
        }
        Ok(())
    }

    /// `&&` and `||`, which evaluate their right side only when it decides the result.
    fn visit_logical(
        &mut self,
        op: BinaryOp,
        lhs: &'e Expr<'e>,
        rhs: &'e Expr<'e>,
    ) -> Result<(), CompilationError> {
        self.visit_expr(lhs)?;
        let true_block = self.out.new_block();
        let false_block = self.out.new_block();
        let next_block = self.out.new_block();
        self.out.set_block_term(BlockTerminator::Branch {
            then_target: true_block,
            else_target: false_block,
        });
        let (rhs_block, constant_block, constant) = if op == BinaryOp::LogAnd {
            (true_block, false_block, false)
        } else {
            (false_block, true_block, true)
        };
        self.out.current = rhs_block;
        self.visit_expr(rhs)?;
        self.out
            .set_block_term(BlockTerminator::Jump { target: next_block });
        self.out.current = constant_block;
        self.out.add_op(StackOp::ConstBool(constant));
        self.out
            .set_block_term(BlockTerminator::Jump { target: next_block });
        self.out.current = next_block;
        Ok(())
    }

    fn visit_assign(
        &mut self,
        lhs: &'e Expr<'e>,
        rhs: &'e Expr<'e>,
        op: Option<BinaryOp>,
    ) -> Result<(), CompilationError> {
        let ExprKind::LocalRef(index) = lhs.kind else {
            return Err(CompilationError::InvalidAssignmentTarget(lhs.location));
        };
        let void = self.type_of(lhs).is_void();
        let slot = self.local_slot(lhs.location, index)?;
        if op.is_some() && !void {
            self.out.add_op(StackOp::LoadLocal(slot));
        }
        self.visit_expr(rhs)?;
        if let Some(op) = op {
            let typ = op_type(lhs.location, self.type_of(lhs))?;
            self.out.add_op(StackOp::BinaryOp(op, typ));
        }
        if !void {
            self.out.add_op(StackOp::StoreLocal(slot));
        }
        Ok(())
    }
}

fn build_instructions(builder: BlockBuilder) -> Result<InstructionBuilder, CompilationError> {
    struct BranchPatch {
        from_offset: usize,
        to_block: usize,
    }
    let mut blocks = builder.blocks;
    let mut patches: Vec<BranchPatch> = Vec::with_capacity(blocks.len() * 2);
    let mut out = InstructionBuilder::default();
    let num_blocks = blocks.len();
    for (block_id, block) in blocks.iter_mut().enumerate() {
        block.start_offset = out.position();
        for op in block.ops.iter() {
            visit_stack_op(op, &mut out)?;
        }

        let mut branch = |out: &mut InstructionBuilder, op: u8, to_block: usize| {
            out.push_op(op);
            patches.push(BranchPatch {
                from_offset: out.reserve_immediate::<i32>(),
                to_block,
            });
        };
        let next = block_id + 1;
        match block.term {
            BlockTerminator::Invalid => {
                return Err(CompilationError::Internal(
                    TokenLocation::default(),
                    "basic block with no terminator",
                ));
            }
            BlockTerminator::Return => out.push_op(instr::OP_RET),
            BlockTerminator::Jump { target } => {
                if target != next {
                    branch(&mut out, instr::OP_BRANCH, target);
                }
            }
            BlockTerminator::Branch {
                then_target,
                else_target,
            } => {
                if then_target == next {
                    // Fall through to the then block when true.
                    branch(&mut out, instr::OP_BRANCH_IF_FALSE, else_target);
                } else if else_target == next {
                    // Fall through to the else block when false.
                    branch(&mut out, instr::OP_BRANCH_IF_TRUE, then_target);
                } else {
                    branch(&mut out, instr::OP_BRANCH_IF_TRUE, then_target);
                    branch(&mut out, instr::OP_BRANCH, else_target);
                }
            }
        }
    }
    for patch in patches.iter() {
        if patch.to_block >= num_blocks {
            return Err(CompilationError::Internal(
                TokenLocation::default(),
                "branch target",
            ));
        }
        out.patch_branch_target(patch.from_offset, blocks[patch.to_block].start_offset);
    }
    Ok(out)
}

fn binary_opcode(op: BinaryOp, typ: StackOpType) -> Option<u8> {
    use StackOpType::*;
    use instr::*;
    Some(match (op, typ) {
        (BinaryOp::Add, I32) => OP_ADD_I32,
        (BinaryOp::Add, I64) => OP_ADD_I64,
        (BinaryOp::Add, F32) => OP_ADD_F32,
        (BinaryOp::Add, F64) => OP_ADD_F64,
        (BinaryOp::Sub, I32) => OP_SUB_I32,
        (BinaryOp::Sub, I64) => OP_SUB_I64,
        (BinaryOp::Sub, F32) => OP_SUB_F32,
        (BinaryOp::Sub, F64) => OP_SUB_F64,
        (BinaryOp::Mul, I32) => OP_MUL_I32,
        (BinaryOp::Mul, I64) => OP_MUL_I64,
        (BinaryOp::Mul, F32) => OP_MUL_F32,
        (BinaryOp::Mul, F64) => OP_MUL_F64,
        (BinaryOp::Div, I32) => OP_DIV_I32,
        (BinaryOp::Div, I64) => OP_DIV_I64,
        (BinaryOp::Div, F32) => OP_DIV_F32,
        (BinaryOp::Div, F64) => OP_DIV_F64,
        (BinaryOp::Mod, I32) => OP_REM_I32,
        (BinaryOp::Mod, I64) => OP_REM_I64,
        (BinaryOp::Mod, F32) => OP_REM_F32,
        (BinaryOp::Mod, F64) => OP_REM_F64,
        (BinaryOp::BitAnd, I32) => OP_BIT_AND_I32,
        (BinaryOp::BitAnd, I64) => OP_BIT_AND_I64,
        (BinaryOp::BitOr, I32) => OP_BIT_OR_I32,
        (BinaryOp::BitOr, I64) => OP_BIT_OR_I64,
        (BinaryOp::BitXor, I32) => OP_BIT_XOR_I32,
        (BinaryOp::BitXor, I64) => OP_BIT_XOR_I64,
        (BinaryOp::Shl, I32) => OP_SHL_I32,
        (BinaryOp::Shl, I64) => OP_SHL_I64,
        (BinaryOp::Shr, I32) => OP_SHR_I32,
        (BinaryOp::Shr, I64) => OP_SHR_I64,
        (BinaryOp::Eq, Bool) => OP_EQ_BOOL,
        (BinaryOp::Eq, I32) => OP_EQ_I32,
        (BinaryOp::Eq, I64) => OP_EQ_I64,
        (BinaryOp::Eq, F32) => OP_EQ_F32,
        (BinaryOp::Eq, F64) => OP_EQ_F64,
        (BinaryOp::Ne, Bool) => OP_NE_BOOL,
        (BinaryOp::Ne, I32) => OP_NE_I32,
        (BinaryOp::Ne, I64) => OP_NE_I64,
        (BinaryOp::Ne, F32) => OP_NE_F32,
        (BinaryOp::Ne, F64) => OP_NE_F64,
        (BinaryOp::Lt, I32) => OP_LT_I32,
        (BinaryOp::Lt, I64) => OP_LT_I64,
        (BinaryOp::Lt, F32) => OP_LT_F32,
        (BinaryOp::Lt, F64) => OP_LT_F64,
        (BinaryOp::Le, I32) => OP_LE_I32,
        (BinaryOp::Le, I64) => OP_LE_I64,
        (BinaryOp::Le, F32) => OP_LE_F32,
        (BinaryOp::Le, F64) => OP_LE_F64,
        (BinaryOp::Gt, I32) => OP_GT_I32,
        (BinaryOp::Gt, I64) => OP_GT_I64,
        (BinaryOp::Gt, F32) => OP_GT_F32,
        (BinaryOp::Gt, F64) => OP_GT_F64,
        (BinaryOp::Ge, I32) => OP_GE_I32,
        (BinaryOp::Ge, I64) => OP_GE_I64,
        (BinaryOp::Ge, F32) => OP_GE_F32,
        (BinaryOp::Ge, F64) => OP_GE_F64,
        _ => return None,
    })
}

fn unary_opcode(op: UnaryOp, typ: StackOpType) -> Option<u8> {
    use StackOpType::*;
    use instr::*;
    Some(match (op, typ) {
        (UnaryOp::Not, Bool) => OP_NOT,
        (UnaryOp::Neg, I32) => OP_NEG_I32,
        (UnaryOp::Neg, I64) => OP_NEG_I64,
        (UnaryOp::Neg, F32) => OP_NEG_F32,
        (UnaryOp::Neg, F64) => OP_NEG_F64,
        (UnaryOp::BitNot, I32) => OP_BIT_NOT_I32,
        (UnaryOp::BitNot, I64) => OP_BIT_NOT_I64,
        _ => return None,
    })
}

fn type_tag(typ: StackOpType) -> u8 {
    match typ {
        StackOpType::Bool => instr::TYPE_BOOL,
        StackOpType::I32 => instr::TYPE_I32,
        StackOpType::I64 => instr::TYPE_I64,
        StackOpType::F32 => instr::TYPE_F32,
        StackOpType::F64 => instr::TYPE_F64,
    }
}

fn visit_stack_op(op: &StackOp, out: &mut InstructionBuilder) -> Result<(), CompilationError> {
    let internal = |what| CompilationError::Internal(TokenLocation::default(), what);
    match op {
        StackOp::ConstVoid => out.push_op(instr::OP_CONST_VOID),
        StackOp::ConstBool(true) => out.push_op(instr::OP_CONST_TRUE),
        StackOp::ConstBool(false) => out.push_op(instr::OP_CONST_FALSE),
        StackOp::ConstI32(value) => {
            out.push_op(instr::OP_CONST_I32);
            out.push_immediate(*value);
        }
        StackOp::ConstI64(value) => {
            out.push_op(instr::OP_CONST_I64);
            out.push_immediate(*value);
        }
        StackOp::ConstF32(value) => {
            out.push_op(instr::OP_CONST_F32);
            out.push_immediate(*value);
        }
        StackOp::ConstF64(value) => {
            out.push_op(instr::OP_CONST_F64);
            out.push_immediate(*value);
        }
        StackOp::ConstStr(value) => {
            let len = u16::try_from(value.len()).map_err(|_| internal("string length"))?;
            out.push_op(instr::OP_CONST_STR);
            out.push_immediate(len);
            out.push_immediate_string(value);
        }
        StackOp::Drop(1) => out.push_op(instr::OP_DROP),
        StackOp::Drop(n) => {
            let n = u32::try_from(*n).map_err(|_| internal("drop count"))?;
            out.push_op(instr::OP_DROP_N);
            out.push_immediate(n);
        }
        StackOp::LoadParam(index) => {
            out.push_op(instr::OP_LOAD_PARAM);
            out.push_immediate(*index);
        }
        StackOp::LoadLocal(index) => {
            out.push_op(instr::OP_LOAD_LOCAL);
            out.push_immediate(*index);
        }
        StackOp::LoadGlobal(index) => {
            out.push_op(instr::OP_LOAD_GLOBAL);
            out.push_immediate(*index);
        }
        StackOp::LoadNativeProp(index) => {
            out.push_op(instr::OP_LOAD_NATIVE_PROP);
            out.push_immediate(*index);
        }
        StackOp::LoadField(index) => {
            out.push_op(instr::OP_LOAD_FIELD);
            out.push_immediate(*index);
        }
        StackOp::StoreLocal(index) => {
            out.push_op(instr::OP_STORE_LOCAL);
            out.push_immediate(*index);
        }
        StackOp::UnaryOp(op, typ) => {
            out.push_op(unary_opcode(*op, *typ).ok_or_else(|| internal("unary operator"))?);
        }
        StackOp::BinaryOp(op, typ) => {
            out.push_op(binary_opcode(*op, *typ).ok_or_else(|| internal("binary operator"))?);
        }
        StackOp::Convert { from, to } => {
            out.push_op(instr::OP_CONVERT);
            out.push_immediate(type_tag(*from));
            out.push_immediate(type_tag(*to));
        }
        StackOp::CallHostMethod { method, num_args } => {
            out.push_op(instr::OP_CALL_HOST_METHOD);
            out.push_immediate(*method);
            out.push_immediate(*num_args);
        }
        StackOp::CallHostFunction { method, num_args } => {
            out.push_op(instr::OP_CALL_HOST_FUNCTION);
            out.push_immediate(*method);
            out.push_immediate(*num_args);
        }
        StackOp::CallScriptFunction { method, num_args } => {
            out.push_op(instr::OP_CALL);
            out.push_immediate(*method);
            out.push_immediate(*num_args);
        }
    }
    Ok(())
}
