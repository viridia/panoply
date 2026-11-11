//! String type and methods.

use std::sync::Arc;

use super::HostState;
use crate::{
    sema::types::ExprType,
    vm::{InvocationContext, StackValue, VMError},
};

/// string.len()
fn string_len(ctx: &mut InvocationContext) -> Result<StackValue, VMError> {
    let s: Arc<String> = ctx.argument(0)?;
    Ok(StackValue::I32(i32::try_from(s.len()).unwrap_or(i32::MAX)))
}

pub(crate) fn init_string_methods(host: &mut HostState) {
    if let Some(mut string_type) = host.get_host_type_mut::<String>() {
        string_type.add_method("len", string_len, Vec::new(), ExprType::I32);
    }
}
