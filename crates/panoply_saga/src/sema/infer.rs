use bevy::platform::collections::HashMap;

use super::types::{ExprType, TypeVarId};
use crate::{compiler::CompilationError, syntax::location::TokenLocation};

#[derive(Debug)]
struct Constraint {
    left: ExprType,
    right: ExprType,
    location: TokenLocation,
}

/// Equality constraints between types, solved by unification.
#[derive(Default)]
pub(crate) struct TypeInference {
    constraints: Vec<Constraint>,
    next_typevar_id: usize,
    substitutions: HashMap<TypeVarId, ExprType>,
}

impl TypeInference {
    pub(crate) fn fresh_typevar(&mut self) -> ExprType {
        let id = self.next_typevar_id;
        self.next_typevar_id += 1;
        ExprType::Infer(TypeVarId(id))
    }

    pub(crate) fn add_constraint(
        &mut self,
        left: ExprType,
        right: ExprType,
        location: TokenLocation,
    ) {
        self.constraints.push(Constraint {
            left,
            right,
            location,
        });
    }

    pub(crate) fn substitute(&self, ty: &ExprType) -> ExprType {
        match ty {
            ExprType::Infer(id) => match self.substitutions.get(id) {
                Some(substituted) => self.substitute(substituted),
                None => ty.clone(),
            },
            _ => ty.clone(),
        }
    }

    fn occurs_check(&self, var_id: TypeVarId, ty: &ExprType) -> bool {
        match ty {
            ExprType::Infer(id) => {
                *id == var_id
                    || self
                        .substitutions
                        .get(id)
                        .is_some_and(|substituted| self.occurs_check(var_id, substituted))
            }
            _ => false,
        }
    }

    fn unify(
        &mut self,
        t1: &ExprType,
        t2: &ExprType,
        location: TokenLocation,
    ) -> Result<(), CompilationError> {
        let t1 = self.substitute(t1);
        let t2 = self.substitute(t2);

        if t1 == t2 {
            return Ok(());
        }

        match (&t1, &t2) {
            (ExprType::Infer(id), ty) | (ty, ExprType::Infer(id)) => {
                if self.occurs_check(*id, ty) {
                    return Err(CompilationError::RecursiveType(location, ty.clone()));
                }
                self.substitutions.insert(*id, ty.clone());
                Ok(())
            }
            _ => Err(CompilationError::MismatchedTypes(location, t1, t2)),
        }
    }

    /// Solves the constraints in the order they were added.
    pub(crate) fn solve_constraints(&mut self) -> Result<(), CompilationError> {
        let constraints = core::mem::take(&mut self.constraints);
        for Constraint {
            left,
            right,
            location,
        } in constraints
        {
            self.unify(&left, &right, location)?;
        }
        Ok(())
    }
}
