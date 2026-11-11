use std::fmt::Display;

/// Set of possible binary operators.
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum BinaryOp {
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

impl Display for BinaryOp {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            BinaryOp::Add => write!(f, "+"),
            BinaryOp::Sub => write!(f, "-"),
            BinaryOp::Mul => write!(f, "*"),
            BinaryOp::Div => write!(f, "/"),
            BinaryOp::Mod => write!(f, "%"),
            BinaryOp::LogAnd => write!(f, "&&"),
            BinaryOp::LogOr => write!(f, "||"),
            BinaryOp::BitAnd => write!(f, "&"),
            BinaryOp::BitOr => write!(f, "|"),
            BinaryOp::BitXor => write!(f, "^"),
            BinaryOp::Shl => write!(f, "<<"),
            BinaryOp::Shr => write!(f, ">>"),
            BinaryOp::Eq => write!(f, "=="),
            BinaryOp::Ne => write!(f, "!="),
            BinaryOp::Lt => write!(f, "<"),
            BinaryOp::Le => write!(f, "<="),
            BinaryOp::Gt => write!(f, ">"),
            BinaryOp::Ge => write!(f, ">="),
        }
    }
}

/// Set of possible unary operators.
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum UnaryOp {
    Not,
    Neg,
    BitNot,
}
