//! Typed tree to basic blocks to bytecode.

use super::{
    bblock::{BasicBlock, BlockTerminator, StackOp, StackOpType},
    instr::{self, InstructionBuilder},
};
use crate::{
    CompiledFunction, ScriptModule,
    compiler::CompilationError,
    host::{
        HostState,
        layout::{Layout, has_opaque, slot_count},
    },
    sema::{
        build::ModuleExprs,
        expr::{Expr, ExprKind},
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

/// How many slots a value of `typ` takes.
fn slots_of(location: TokenLocation, typ: &ExprType) -> Result<usize, CompilationError> {
    slot_count(typ).ok_or(CompilationError::Internal(location, "a type with no slots"))
}

/// The offset of each of `types` in a frame starting at `start`, and where they end. A variable
/// of a type with no slots (a function) takes none: its value is refused where it is made.
fn offsets(types: &[ExprType], start: usize) -> (Vec<usize>, usize) {
    let mut offset = start;
    let mut result = Vec::with_capacity(types.len());
    for typ in types {
        result.push(offset);
        offset += slot_count(typ).unwrap_or(0);
    }
    (result, offset)
}

pub(crate) fn gen_code(
    module: &mut ScriptModule,
    exprs: &ModuleExprs,
    host: &HostState,
) -> Result<(), CompilationError> {
    module.functions.clear();
    for function in exprs.functions.iter() {
        let location = function.body.map(|b| b.location).unwrap_or_default();
        let (params, num_params) = offsets(&function.params, 0);
        let (locals, frame_end) = offsets(&function.locals, num_params);
        let ret_slots = imm16(
            location,
            slots_of(location, &function.ret)?,
            "results of more than 65535 slots",
        )?;
        let mut emitter = Emitter {
            types: &exprs.types,
            host,
            params,
            locals,
            out: BlockBuilder::default(),
        };
        emitter.out.current = emitter.out.new_block();
        if let Some(body) = function.body {
            emitter.visit_expr(body)?;
        }
        emitter.out.set_block_term(BlockTerminator::Return);
        module.functions.push(CompiledFunction {
            code: build_instructions(emitter.out, ret_slots)?.inner(),
            num_params,
            num_locals: frame_end - num_params,
        });
    }
    Ok(())
}

struct Emitter<'f, 'e> {
    types: &'f TypeTable,
    host: &'f HostState,
    /// Each parameter's slot in the frame.
    params: Vec<usize>,
    /// Each local's slot in the frame, after the parameters.
    locals: Vec<usize>,
    out: BlockBuilder<'e>,
}

/// Where a value can be read in place, without loading the value it is part of.
enum Place<'e> {
    /// A frame slot.
    Local(usize),
    /// A component of an entity: the entity's expression and the component's member.
    Component(&'e Expr<'e>, usize),
    /// A field of a scripted aspect on an entity: the entity's expression, the aspect's index in
    /// the module's scripted reads, and the field's index.
    Scripted(&'e Expr<'e>, usize, usize),
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

    fn slots(&self, expr: &Expr) -> Result<usize, CompilationError> {
        slots_of(expr.location, self.type_of(expr))
    }

    fn slots16(&self, expr: &Expr) -> Result<u16, CompilationError> {
        imm16(
            expr.location,
            self.slots(expr)?,
            "values of more than 65535 slots",
        )
    }

    /// The offset and slot count of field `index` of a value of `base`'s type.
    fn field(&self, base: &Expr, index: usize) -> Result<(usize, usize), CompilationError> {
        let ExprType::Reflected(info) = self.type_of(base) else {
            return Err(CompilationError::Internal(
                base.location,
                "field of a non-struct",
            ));
        };
        Layout::of(info)
            .field(index)
            .ok_or(CompilationError::Internal(base.location, "field index"))
    }

    /// Where `expr` can be read in place, and its offset there.
    fn place(&self, expr: &'e Expr<'e>) -> Result<Option<(Place<'e>, usize)>, CompilationError> {
        let internal = || CompilationError::Internal(expr.location, "variable index");
        Ok(match &expr.kind {
            ExprKind::LocalRef(index) => Some((
                Place::Local(*self.locals.get(*index).ok_or_else(internal)?),
                0,
            )),
            ExprKind::ParamRef(index) => Some((
                Place::Local(*self.params.get(*index).ok_or_else(internal)?),
                0,
            )),
            ExprKind::NativeProp(entity, member) if self.host.is_component(*member) => {
                Some((Place::Component(entity, *member), 0))
            }
            ExprKind::ScriptedField(entity, read, field) => {
                Some((Place::Scripted(entity, *read, *field), 0))
            }
            ExprKind::Field(base, index) => match self.place(base)? {
                Some((place, offset)) => Some((place, offset + self.field(base, *index)?.0)),
                None => None,
            },
            _ => None,
        })
    }

    /// Loads the `count` slots of `expr`, in place at `offset`.
    fn load_place(
        &mut self,
        expr: &Expr,
        place: Place<'e>,
        offset: usize,
    ) -> Result<(), CompilationError> {
        let loc = expr.location;
        if has_opaque(self.type_of(expr)) {
            return Err(CompilationError::Unsupported(
                loc,
                "reading a value scripts cannot see into, such as an enum or a list",
            ));
        }
        let count = self.slots16(expr)?;
        let offset = imm16(loc, offset, "more than 65535 slots")?;
        match place {
            Place::Local(slot) => {
                if count > 0 {
                    let slot = imm16(loc, slot, "more than 65535 locals")?;
                    self.out.add_op(StackOp::LoadLocal {
                        slot: slot + offset,
                        count,
                    });
                }
            }
            Place::Component(entity, member) => {
                self.visit_expr(entity)?;
                let member = imm16(loc, member, "more than 65535 host members")?;
                self.out.add_op(StackOp::LoadComponent {
                    member,
                    offset,
                    count,
                });
            }
            Place::Scripted(entity, read, field) => {
                self.visit_expr(entity)?;
                self.out.add_op(StackOp::LoadScripted {
                    read: imm16(loc, read, "more than 65535 scripted aspects")?,
                    field: imm16(loc, field, "more than 65535 fields")?,
                    offset,
                    count,
                });
            }
        }
        Ok(())
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
            ExprKind::ParamRef(_) | ExprKind::LocalRef(_) | ExprKind::Field(..) => {
                if let Some((place, offset)) = self.place(expr)? {
                    return self.load_place(expr, place, offset);
                }
                let ExprKind::Field(base, index) = &expr.kind else {
                    return Err(CompilationError::Internal(loc, "variable place"));
                };
                // A field of a temporary: load the whole value, then keep the field.
                self.visit_expr(base)?;
                let (offset, count) = self.field(base, *index)?;
                self.out.add_op(StackOp::Extract {
                    offset: imm16(loc, offset, "more than 65535 slots")?,
                    count: imm16(loc, count, "more than 65535 slots")?,
                    total: self.slots16(base)?,
                });
            }
            ExprKind::NativeProp(base, index) => {
                if let Some((place, offset)) = self.place(expr)? {
                    return self.load_place(expr, place, offset);
                }
                self.visit_expr(base)?;
                self.out.add_op(StackOp::CallHostMethod {
                    method: imm16(loc, *index, "more than 65535 host members")?,
                    num_args: self.slots16(base)?,
                    num_results: self.slots16(expr)?,
                });
            }
            ExprKind::ScriptedField(..) => {
                let (place, offset) = self
                    .place(expr)?
                    .ok_or(CompilationError::Internal(loc, "scripted field place"))?;
                self.load_place(expr, place, offset)?;
            }
            ExprKind::GlobalRef(index) => {
                let index = imm16(loc, *index, "more than 65535 host members")?;
                self.out.add_op(StackOp::LoadGlobal(index));
            }
            ExprKind::ArrayLit(elements) => {
                for element in elements.iter() {
                    self.visit_expr(element)?;
                }
            }
            ExprKind::Index(base, index) => {
                let ExprType::Array(_, len) = self.type_of(base) else {
                    return Err(CompilationError::Internal(loc, "index of a non-array"));
                };
                self.visit_expr(base)?;
                self.visit_expr(index)?;
                self.out.add_op(StackOp::Index {
                    size: self.slots16(expr)?,
                    len: imm16(loc, *len, "arrays longer than 65535")?,
                });
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
                let (index, base) = match func.kind {
                    ExprKind::HostFunctionRef(index) | ExprKind::ScriptFunctionRef(index) => {
                        (index, None)
                    }
                    ExprKind::HostMethodRef(base, index) => (index, Some(base)),
                    _ => return Err(CompilationError::NotCallable(loc)),
                };
                let mut num_args = 0;
                if let Some(base) = base {
                    self.visit_expr(base)?;
                    num_args += self.slots(base)?;
                }
                for arg in args.iter() {
                    self.visit_expr(arg)?;
                    num_args += self.slots(arg)?;
                }
                let method = imm16(loc, index, "more than 65535 functions")?;
                let num_args = imm16(loc, num_args, "arguments of more than 65535 slots")?;
                let num_results = self.slots16(expr)?;
                self.out.add_op(match func.kind {
                    ExprKind::HostFunctionRef(_) => StackOp::CallHostFunction {
                        method,
                        num_args,
                        num_results,
                    },
                    ExprKind::ScriptFunctionRef(_) => {
                        StackOp::CallScriptFunction { method, num_args }
                    }
                    _ => StackOp::CallHostMethod {
                        method,
                        num_args,
                        num_results,
                    },
                });
            }
            ExprKind::Block(stmts, result) => {
                for stmt in stmts.iter() {
                    self.visit_expr(stmt)?;
                    let count = slot_count(self.type_of(stmt)).unwrap_or(0);
                    if count > 0 {
                        self.out.add_op(StackOp::Drop(count));
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
        if let ExprKind::ScriptedField(entity, read, field) = lhs.kind {
            // The entity, then the value, which a compound assignment computes from the field's
            // value now: a write is buffered, so the field still holds it.
            let loc = lhs.location;
            self.visit_expr(entity)?;
            if op.is_some() {
                self.visit_expr(lhs)?;
            }
            self.visit_expr(rhs)?;
            if let Some(op) = op {
                let typ = op_type(loc, self.type_of(lhs))?;
                self.out.add_op(StackOp::BinaryOp(op, typ));
            }
            self.out.add_op(StackOp::StoreScripted {
                read: imm16(loc, read, "more than 65535 scripted aspects")?,
                field: imm16(loc, field, "more than 65535 fields")?,
            });
            return Ok(());
        }
        let ExprKind::LocalRef(index) = lhs.kind else {
            return Err(CompilationError::InvalidAssignmentTarget(lhs.location));
        };
        // As in `offsets`: a variable of a type with no slots takes none.
        let count = slot_count(self.type_of(lhs)).unwrap_or(0);
        let count = imm16(lhs.location, count, "values of more than 65535 slots")?;
        let slot = *self
            .locals
            .get(index)
            .ok_or(CompilationError::Internal(lhs.location, "variable index"))?;
        let slot = imm16(lhs.location, slot, "more than 65535 locals")?;
        if op.is_some() && count > 0 {
            self.out.add_op(StackOp::LoadLocal { slot, count });
        }
        self.visit_expr(rhs)?;
        if let Some(op) = op {
            let typ = op_type(lhs.location, self.type_of(lhs))?;
            self.out.add_op(StackOp::BinaryOp(op, typ));
        }
        if count > 0 {
            self.out.add_op(StackOp::StoreLocal { slot, count });
        }
        Ok(())
    }
}

fn build_instructions(
    builder: BlockBuilder,
    ret_slots: u16,
) -> Result<InstructionBuilder, CompilationError> {
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
            BlockTerminator::Return => {
                out.push_op(instr::OP_RET);
                out.push_immediate(ret_slots);
            }
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
        StackOp::LoadLocal { slot, count } => {
            out.push_op(instr::OP_LOAD_LOCAL);
            out.push_immediate(*slot);
            out.push_immediate(*count);
        }
        StackOp::LoadGlobal(index) => {
            out.push_op(instr::OP_LOAD_GLOBAL);
            out.push_immediate(*index);
        }
        StackOp::LoadComponent {
            member,
            offset,
            count,
        } => {
            out.push_op(instr::OP_LOAD_COMPONENT);
            out.push_immediate(*member);
            out.push_immediate(*offset);
            out.push_immediate(*count);
        }
        StackOp::LoadScripted {
            read,
            field,
            offset,
            count,
        } => {
            out.push_op(instr::OP_LOAD_SCRIPTED);
            out.push_immediate(*read);
            out.push_immediate(*field);
            out.push_immediate(*offset);
            out.push_immediate(*count);
        }
        StackOp::StoreScripted { read, field } => {
            out.push_op(instr::OP_STORE_SCRIPTED);
            out.push_immediate(*read);
            out.push_immediate(*field);
        }
        StackOp::StoreLocal { slot, count } => {
            out.push_op(instr::OP_STORE_LOCAL);
            out.push_immediate(*slot);
            out.push_immediate(*count);
        }
        StackOp::Extract {
            offset,
            count,
            total,
        } => {
            out.push_op(instr::OP_EXTRACT);
            out.push_immediate(*offset);
            out.push_immediate(*count);
            out.push_immediate(*total);
        }
        StackOp::Index { size, len } => {
            out.push_op(instr::OP_INDEX);
            out.push_immediate(*size);
            out.push_immediate(*len);
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
        StackOp::CallHostMethod {
            method,
            num_args,
            num_results,
        }
        | StackOp::CallHostFunction {
            method,
            num_args,
            num_results,
        } => {
            out.push_op(match op {
                StackOp::CallHostMethod { .. } => instr::OP_CALL_HOST_METHOD,
                _ => instr::OP_CALL_HOST_FUNCTION,
            });
            out.push_immediate(*method);
            out.push_immediate(*num_args);
            out.push_immediate(*num_results);
        }
        StackOp::CallScriptFunction { method, num_args } => {
            out.push_op(instr::OP_CALL);
            out.push_immediate(*method);
            out.push_immediate(*num_args);
        }
    }
    Ok(())
}
