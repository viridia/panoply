use std::sync::{Arc, Mutex};

use bevy::prelude::*;
use thiserror::Error;
use wasmer::{imports, CompileError, Instance, Module, Store, Value};

#[non_exhaustive]
#[derive(Debug, Error)]
pub enum SagaError {
    #[error("Compiler error: {0}")]
    CompileError(#[from] wasmer::CompileError),
    #[error("Import error: {0}")]
    InstantiationError(#[from] wasmer::InstantiationError),
    #[error("Export error: {0}")]
    ExportError(#[from] wasmer::ExportError),
    #[error("Runtime error: {0}")]
    Runtime(#[from] wasmer::RuntimeError),
}

#[derive(Resource)]
struct SagaEngine {
    store: Store,
    instance: Module,
}

fn vm_test() -> Result<(), SagaError> {
    let module_wat = r#"
    (module
    (type $t0 (func (param i32) (result i32)))
    (func $add_one (export "add_one") (type $t0) (param $p0 i32) (result i32)
        local.get $p0
        i32.const 1
        i32.add))
    "#;

    let mut store = Store::default();
    let module = Module::new(&store, module_wat)?;
    // The module doesn't import anything, so we create an empty import object.
    let import_object = imports! {};
    let instance = Instance::new(&mut store, &module, &import_object)?;

    let add_one = instance.exports.get_function("add_one")?;
    let result = add_one.call(&mut store, &[Value::I32(42)])?;
    assert_eq!(result[0], Value::I32(43));

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::vm::vm_test;

    #[test]
    fn run_vm() {
        vm_test().unwrap();
    }
}
