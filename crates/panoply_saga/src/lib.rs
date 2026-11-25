//! SAGA, Panoply's scripting language: a compiler from source to bytecode, and the VM that
//! runs it against the world.
//!
//! # Terms
//!
//! - **Module:** a compiled `.saga` file, its functions and the aspects it declares.
//! - **Formula:** a module compiled from one expression embedded in an asset, such as an
//!   animation's `when`. It reads the world and returns a value; it never writes. See
//!   [`embed`].
//! - **Host:** what Panoply offers scripts ([`HostState`]): types, functions, global properties
//!   such as `self`, and components read through **accessors** (`self.container` reads
//!   `ContainerState`). Plugins register them while the app builds.
//! - **Owner:** the entity a formula runs on, `self` in its source.
//! - **Slot:** one value on the VM's stack ([`StackValue`]): a primitive, an entity, or an index
//!   into the evaluation's strings and reflected values. A struct such as `Vec3` is flattened
//!   into consecutive slots, one per field. Natives read their arguments and write their results
//!   through [`FromSlots`] and [`ToSlots`].
//! - **Native:** a function written in Rust that scripts call.
//! - **Tracking scope:** the list of components and resources one evaluation read
//!   ([`TrackingScope`]). It is how a reactive formula knows when to run again.
//! - **Cell:** a formula kept alive on an owner, holding its last value, what it read, and its
//!   memos ([`reactive`]). The reaction pass runs a cell again only when something it read has
//!   changed, and a cell's value counts as changed only when it differs from the last.
//! - **Memo:** a native that keeps state between evaluations and computes its value with its
//!   own tracking scope. `proximity(a, b, enter, leave)` reads two transforms that change every
//!   frame, but returns a boolean that rarely does, so the formula calling it runs again only
//!   when that boolean flips. A memo lives in its cell, one per call site: the same call in two
//!   places computes twice. It outlives its cell's evaluations, since its state (`proximity`'s
//!   last value) must, so a call compares its arguments with the last call's: the same, and the
//!   memo's current value is returned; different, and it computes anew. Like a cell, a memo
//!   compares only its own output; what reads it trusts that.
//! - **Reaction pass:** the system that, once a frame, brings every cell up to date
//!   ([`reactive::run_reactions`]).

pub mod asset;
mod codegen;
pub mod compiler;
pub mod embed;
pub mod host;
pub mod natives;
mod plugin;
pub mod reactive;
pub mod scripted;
mod sema;
mod syntax;
pub mod tracking;
pub mod vm;

pub use asset::{ScriptModuleAsset, ScriptModuleLoader};
pub use codegen::instr::disassemble;
pub use compiler::{
    CompilationError, ModuleResolver, NoImports, compile_formula, compile_module, format_error,
};
pub use embed::{Formula, FormulaQueue, FormulaRef, ReflectFormula};
pub use host::HostState;
pub use plugin::{SagaHost, SagaHostBuilder, SagaPlugin};
pub use reactive::{CellId, NativeMemo, ReactionPass, ReactionReads, Reactions};
pub use sema::types::{ExprType, HostParam};
pub use syntax::location::TokenLocation;
pub use tracking::TrackingScope;
pub use vm::{FromSlots, InvocationContext, ScriptVm, SlotStore, StackValue, ToSlots, VMError};

use std::{any::TypeId, sync::Arc};

use scripted::AspectDecl;
use sema::decl::{DeclKind, DeclTable};

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

    /// The components read from `self` through accessors (`self.lock`), once each.
    pub(crate) self_components: Vec<TypeId>,

    /// The aspects the module declares.
    pub(crate) aspects: Vec<Arc<AspectDecl>>,

    /// The scripted aspects the module's code reads, which `load scripted` names by index.
    pub(crate) scripted_reads: Vec<Arc<AspectDecl>>,

    /// The scripted aspects read from `self`, once each.
    pub(crate) self_aspects: Vec<Arc<AspectDecl>>,
}

impl ScriptModule {
    /// The name of the default export.
    pub const DEFAULT: &str = ".default";

    /// The components the module reads from `self` through accessors, which the entity it runs
    /// on must have. A read through a local (`let s = self; s.lock`) is not listed.
    pub fn self_components(&self) -> &[TypeId] {
        &self.self_components
    }

    /// The scripted aspects the module reads from `self`, which the entity it runs on must
    /// have, as [`Self::self_components`] lists native ones.
    pub fn self_aspects(&self) -> &[Arc<AspectDecl>] {
        &self.self_aspects
    }

    /// The aspects the module declares.
    pub fn aspects(&self) -> &[Arc<AspectDecl>] {
        &self.aspects
    }

    /// The index of the function `name` declares.
    pub(crate) fn function_index(&self, name: &str) -> Option<usize> {
        match self.module_decls.get(name)?.kind {
            DeclKind::Function { index } => Some(index),
            _ => None,
        }
    }

    /// The code of each function, disassembled, for debugging.
    pub fn disassemble(&self) -> String {
        self.functions
            .iter()
            .enumerate()
            .map(|(index, f)| format!("fn {index}:\n{}", disassemble(&f.code)))
            .collect()
    }
}

/// A function's bytecode and frame, counted in slots.
#[derive(Default, Debug)]
pub(crate) struct CompiledFunction {
    pub(crate) code: Vec<u8>,
    pub(crate) num_params: usize,
    pub(crate) num_locals: usize,
}
