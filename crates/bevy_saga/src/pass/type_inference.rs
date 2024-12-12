use std::sync::Arc;

use bevy::utils::HashMap;

use crate::{location::TokenLocation, types::TypeVarId, CompilationError, Type};

#[derive(Debug)]
struct Constraint {
    left: Type,
    right: Type,
    location: TokenLocation,
    // reason: ConstraintReason,
}

#[derive(Default)]
pub(crate) struct TypeInference {
    /// Equivalence constraints.
    pub(crate) constraints: Vec<Constraint>,

    /// Type variable substitutions.
    pub(crate) substitutions: HashMap<TypeVarId, Type>,
}

impl TypeInference {
    pub(crate) fn add_constraint(&mut self, left: Type, right: Type, location: TokenLocation) {
        self.constraints.push(Constraint {
            left,
            right,
            location,
        });
    }

    pub(crate) fn replace_type_vars(&self, ty: &mut Type) {
        *ty = self.substitute(ty);
    }

    pub(crate) fn substitute(&self, ty: &Type) -> Type {
        match ty {
            Type::Infer(id) => {
                if let Some(substituted_type) = self.substitutions.get(id) {
                    self.substitute(substituted_type)
                } else {
                    ty.clone()
                }
            }

            // Type::Error(_) => {
            //     // Do nothing
            //     ty.clone()
            // }
            Type::Tuple(types) => {
                // TODO: Return the original type if no substitutions are made.
                let mut new_types = Vec::with_capacity(types.len());
                for t in types.iter() {
                    new_types.push(self.substitute(t));
                }
                Type::Tuple(Arc::from(new_types))
            }

            // TODO: Return the original type if no substitutions are made.
            Type::Array(ty) => Type::Array(Arc::new(self.substitute(ty))),

            _ => ty.clone(),
        }
    }

    fn occurs_check(&self, var_id: TypeVarId, ty: &Type) -> bool {
        match ty {
            Type::Infer(id) => {
                if *id == var_id {
                    return true;
                }
                // Check if this type variable has a substitution
                if let Some(substituted_type) = self.substitutions.get(id) {
                    return self.occurs_check(var_id, substituted_type);
                }
                false
            }
            _ => false,
        }
    }

    fn unify(
        &mut self,
        t1: &Type,
        t2: &Type,
        location: TokenLocation,
    ) -> Result<(), CompilationError> {
        let t1 = self.substitute(t1);
        let t2 = self.substitute(t2);

        if t1 == t2 {
            return Ok(());
        }

        match (&t1, &t2) {
            // (Type::Error(err), _) | (_, Type::Error(err)) => Err(err.clone()),
            (Type::Infer(id), ty) | (ty, Type::Infer(id)) => {
                if self.occurs_check(*id, ty) {
                    return Err(CompilationError::RecursiveType(location, ty.clone()));
                } else {
                    self.substitutions.insert(*id, ty.clone());
                }
                Ok(())
            }

            (Type::I32, Type::IUnsized) | (Type::IUnsized, Type::I32) => Ok(()),
            (Type::I64, Type::IUnsized) | (Type::IUnsized, Type::I64) => Ok(()),

            (Type::Tuple(types1), Type::Tuple(types2)) => {
                if types1.len() != types2.len() {
                    return Err(CompilationError::MismatchedTypes(
                        location,
                        t1.clone(),
                        t2.clone(),
                    ));
                }
                for (t1, t2) in types1.iter().zip(types2.iter()) {
                    self.unify(t1, t2, location)?;
                }
                Ok(())
            }

            (Type::Array(t1), Type::Array(t2)) => self.unify(t1, t2, location),

            // (Type::Int, Type::Float) | (Type::Float, Type::Int) => {
            //     Ok(()) // Allow implicit conversion between int and float
            // }
            _ => Err(CompilationError::MismatchedTypes(
                location,
                t1.clone(),
                t2.clone(),
            )),
        }
    }

    pub(crate) fn solve_constraints(&mut self) -> Result<(), CompilationError> {
        while let Some(Constraint {
            left: t1,
            right: t2,
            location,
        }) = self.constraints.pop()
        {
            self.unify(&t1, &t2, location)?;
        }
        Ok(())
    }
}
