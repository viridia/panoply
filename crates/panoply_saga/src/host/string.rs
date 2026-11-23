//! String type and methods.

use super::HostState;
use crate::{
    sema::types::ExprType,
    vm::{InvocationContext, VMError},
};

/// string.len()
fn string_len(ctx: &mut InvocationContext) -> Result<(), VMError> {
    let len = ctx.arg::<&str>()?.len();
    ctx.ret(i32::try_from(len).unwrap_or(i32::MAX))
}

pub(crate) fn init_string_methods(host: &mut HostState) {
    if let Some(mut string_type) = host.get_host_type_mut::<String>() {
        string_type.add_method("len", string_len, Vec::new(), ExprType::I32);
    }
}
