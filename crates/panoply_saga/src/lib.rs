//! SAGA, Panoply's scripting language: a compiler from source to bytecode, and the VM that
//! runs it against the world.

pub mod asset;
mod codegen;
pub mod compiler;
pub mod embed;
pub mod host;
mod plugin;
mod sema;
mod syntax;
pub mod tracking;
pub mod vm;

pub use asset::{ScriptModuleAsset, ScriptModuleLoader};
pub use codegen::instr::disassemble;
pub use compiler::{
    CompilationError, ModuleResolver, NoImports, compile_formula, compile_module, format_error,
};
pub use embed::{Formula, FormulaQueue, ReflectFormula};
pub use host::HostState;
pub use plugin::{SagaHost, SagaHostBuilder, SagaPlugin};
pub use sema::types::{ExprType, HostParam};
pub use syntax::location::TokenLocation;
pub use tracking::TrackingScope;
pub use vm::{InvocationContext, ScriptVm, StackValue, VMError};

use sema::decl::DeclTable;

/// The output of a compilation, either a module or a formula.
/// - a "module" is a standalone asset containing declarations (functions and variables)
/// - a "formula" is a module that is intended to be embedded within another asset. It's similar
///   to a module, however it can have executable statements at the root level which are compiled
///   into a special ".default" function that takes no arguments.
#[derive(Default, Debug)]
pub struct ScriptModule {
    /// Name of the asset or file containing the source of this module.
    pub path: String,

    /// Top-level declarations in this module.
    pub(crate) module_decls: DeclTable,

    /// Compiled functions, including both top-level and nested.
    pub(crate) functions: Vec<CompiledFunction>,
}

impl ScriptModule {
    /// The name of the default export.
    pub const DEFAULT: &str = ".default";

    /// The code of each function, disassembled, for debugging.
    pub fn disassemble(&self) -> String {
        self.functions
            .iter()
            .enumerate()
            .map(|(index, f)| format!("fn {index}:\n{}", disassemble(&f.code)))
            .collect()
    }
}

/// A function's bytecode and frame.
#[derive(Default, Debug)]
pub(crate) struct CompiledFunction {
    pub(crate) code: Vec<u8>,
    pub(crate) num_locals: usize,
}
