use crate::syntax::oper::{BinaryOp, UnaryOp};

#[derive(Debug, Default)]
pub(crate) enum BlockTerminator {
    /// Termination hasn't been specified yet
    #[default]
    Invalid,
    /// Return from function
    Return,
    /// Unconditional transfer to another block
    Jump { target: usize },
    /// Conditional branch on TOS to one of two blocks
    Branch {
        then_target: usize,
        else_target: usize,
    },
}

/// Basic block for control flow.
#[derive(Debug, Default)]
pub(crate) struct BasicBlock<'e> {
    /// Entry point of this block within the generated code buffer.
    pub(crate) start_offset: usize,
    /// List of instructions within the body of this block.
    pub(crate) ops: Vec<StackOp<'e>>,
    /// Block terminator
    pub(crate) term: BlockTerminator,
}

/// The primitive type an operator works on.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StackOpType {
    Bool,
    I32,
    I64,
    F32,
    F64,
}

#[derive(Debug, PartialEq)]
pub(crate) enum StackOp<'e> {
    ConstVoid,
    ConstBool(bool),
    ConstI32(i32),
    ConstI64(i64),
    ConstF32(f32),
    ConstF64(f64),
    ConstStr(&'e str),
    Drop(usize),
    LoadParam(u16),
    LoadLocal(u16),
    LoadGlobal(u16),
    LoadNativeProp(u16),
    LoadField(u16),
    StoreLocal(u16),
    UnaryOp(UnaryOp, StackOpType),
    BinaryOp(BinaryOp, StackOpType),
    Convert { from: StackOpType, to: StackOpType },
    CallHostMethod { method: u16, num_args: u16 },
    CallHostFunction { method: u16, num_args: u16 },
    CallScriptFunction { method: u16, num_args: u16 },
}
