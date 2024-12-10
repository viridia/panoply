/// Set of possible binary operators.
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum BinaryOp {
    MemberAccess,
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    LogAnd,
    LogOr,
    BitAnd,
    BitOr,
    BitXor,
    Shl,
    Shr,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

/// Set of possible unary operators.
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum UnaryOp {
    Not,
    Neg,
    BitNot,
}
