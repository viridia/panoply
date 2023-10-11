use super::{coerce::Coerce, expr::Expr};

#[derive(Debug, Clone)]
pub enum TypedExpr<T> {
    Constant(T),
    Expr(Expr),
}

impl<T> TypedExpr<T>
where
    Expr: Coerce<T>,
{
    pub fn from_expr(expr: &Expr) -> Self {
        match expr.coerce() {
            Some(v) => Self::Constant(v),
            None => Self::Expr(expr.clone()),
        }
    }

    pub fn eval(&self) -> Result<&T, anyhow::Error> {
        match self {
            TypedExpr::Constant(val) => Ok(val),
            TypedExpr::Expr(_) => todo!(),
        }
    }
}
