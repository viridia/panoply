//! The bytecode interpreter, built per evaluation.

mod slots;

pub use slots::{FromSlots, SlotStore, ToSlots};

use core::cell::Cell;
use std::sync::Arc;

use bevy::{
    ecs::{component::Component, entity::Entity, world::World},
    reflect::PartialReflect,
};
use thiserror::Error;

use crate::{
    EffectBuffer, ScriptModule,
    codegen::instr,
    host::{
        HostMember, HostState,
        layout::{Layout, Leaf, SlotSink, flatten},
    },
    reactive::MemoTable,
    scripted,
    sema::types::ExprType,
    tracking::TrackingScope,
};

/// One stack slot. A slot owns nothing: a string is an index into the evaluation's
/// [`SlotStore`].
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum StackValue {
    /// An uninitialized local.
    Void,
    Bool(bool),
    I32(i32),
    I64(i64),
    F32(f32),
    F64(f64),
    Entity(Entity),
    /// An index into the evaluation's strings.
    Str(u32),
}

#[non_exhaustive]
#[derive(Debug, Error, PartialEq)]
pub enum VMError {
    #[error("Invalid instruction")]
    InvalidInstruction,
    #[error("Stack underflow")]
    StackUnderflow,
    #[error("Mismatched types: {0} {1}")]
    MismatchedTypes(ExprType, ExprType),
    #[error("Mismatched types: {0} {1}")]
    MismatchedTypes2(&'static str, &'static str),
    #[error("Invalid type for operation: {0}")]
    InvalidType(ExprType),
    #[error("Invalid global index: {0}")]
    InvalidGlobalIndex(usize),
    #[error("Invalid native property: {0}")]
    InvalidNativeProp(usize),
    #[error("Invalid native method: {0}")]
    InvalidNativeMethod(usize),
    #[error("Invalid function index: {0}")]
    InvalidFunction(usize),
    #[error("Invalid local: {0}")]
    InvalidLocal(usize),
    #[error("Invalid string: {0}")]
    InvalidString(u32),
    #[error("Function not found: {0}")]
    MissingFunction(String),
    /// The entity lacks a component the script reads, named by its Rust type.
    #[error("No component {0}")]
    MissingComponent(&'static str),
    /// The entity lacks a scripted aspect the script reads, named by its key.
    #[error("No aspect {0}")]
    MissingAspect(Arc<str>),
    #[error("Function {0} expects {1} parameters, got {2}")]
    IncorrectParamCount(String, usize, usize),
    #[error("Index {0} out of bounds for an array of {1}")]
    IndexOutOfBounds(i32, usize),
    /// A native wrote a result unlike the type it was registered with.
    #[error("A native returned {1} slots where its type takes {0}")]
    NativeResultSlots(usize, usize),
    /// A memo was called with an argument other than a primitive or an entity.
    #[error("A memo's arguments must be primitives or entities")]
    MemoArguments,
    /// An evaluation's result is not the type it was read as.
    #[error("The result takes {1} slots where {0} were expected")]
    ResultSlots(usize, usize),
    #[error("Division by zero")]
    DivideByZero,
    #[error("Unsupported: {0}")]
    Unsupported(&'static str),
}

/// A caller's state, saved during a call.
struct CallStackEntry<'w> {
    module: Option<&'w ScriptModule>,
    frame_base: usize,
    iptr: *const u8,
}

/// What a native function sees: its arguments, where it writes its result, and the world.
pub struct InvocationContext<'vm, 'w> {
    world: &'w World,
    store: &'vm mut SlotStore<'w>,
    args: &'vm [StackValue],
    /// The slot of the next argument [`Self::arg`] reads.
    next_arg: Cell<usize>,
    out: &'vm mut Vec<StackValue>,
    tracking: &'vm mut TrackingScope,
    /// The memos of the cell evaluating, if the evaluation is a cell's.
    pub(crate) memos: Option<&'vm mut MemoTable>,
    /// Identifies the call: the address just past its instruction, the same on every evaluation
    /// of a module.
    pub(crate) site: usize,
}

impl<'w> InvocationContext<'_, 'w> {
    pub(crate) fn world(&self) -> &'w World {
        self.world
    }

    /// The arguments' slots, all of them.
    pub(crate) fn arg_slots(&self) -> &[StackValue] {
        self.args
    }

    /// The next argument, in the order the function's parameters are declared.
    pub fn arg<'a, T: FromSlots<'a>>(&'a self) -> Result<T, VMError> {
        let start = self.next_arg.get();
        let slots = self
            .args
            .get(start..start + T::SLOTS)
            .ok_or(VMError::StackUnderflow)?;
        self.next_arg.set(start + T::SLOTS);
        T::from_slots(slots, self.store)
    }

    /// Writes the function's result.
    pub fn ret(&mut self, value: impl ToSlots) -> Result<(), VMError> {
        value.to_slots(self.out, self.store)
    }

    /// Component `C` on `entity`, recorded as a read of the evaluation.
    pub fn component<C: Component>(&mut self, entity: Entity) -> Option<&'w C> {
        let comp = self.world.get_entity(entity).ok()?.get::<C>();
        self.tracking
            .track_component::<C>(entity, self.world, comp.is_some());
        comp
    }
}

/// A virtual machine for evaluating formulas.
pub struct ScriptVm<'w, 'p> {
    pub world: &'w World,

    /// The entity the script runs on.
    pub owner: Entity,

    /// Globals and entity members.
    pub host: &'w HostState,

    /// The module executing.
    module: Option<&'w ScriptModule>,

    /// The callers of the function executing.
    call_stack: Vec<CallStackEntry<'w>>,

    /// Where the executing function's parameters start on the stack; its locals follow them.
    frame_base: usize,

    /// What slots refer to.
    store: SlotStore<'w>,

    /// A native function's result, before it replaces the arguments.
    out: Vec<StackValue>,

    /// Every frame's parameters, locals and temporaries.
    stack: Vec<StackValue>,

    iptr: *const u8,

    /// What the evaluation reads, for reactions.
    pub tracking: &'p mut TrackingScope,

    /// The memos of the cell evaluating, if the evaluation is a cell's.
    memos: Option<&'p mut MemoTable>,

    /// Where an effect's writes go; `None` while a formula runs, which cannot write.
    effects: Option<&'p mut EffectBuffer>,
}

type InstrHandler = fn(&mut ScriptVm) -> Result<(), VMError>;

impl<'w, 'p> ScriptVm<'w, 'p> {
    const JUMP_TABLE: [InstrHandler; 256] = build_jump_table();

    pub fn new(world: &'w World, host: &'w HostState, tracking: &'p mut TrackingScope) -> Self {
        Self {
            world,
            owner: Entity::PLACEHOLDER,
            host,
            module: None,
            call_stack: Vec::new(),
            frame_base: 0,
            store: SlotStore::default(),
            out: Vec::new(),
            stack: Vec::new(),
            iptr: core::ptr::null(),
            tracking,
            memos: None,
            effects: None,
        }
    }

    /// Run as a cell's evaluation, its memos kept in `memos` between evaluations.
    pub(crate) fn set_memos(&mut self, memos: &'p mut MemoTable) -> &mut Self {
        self.memos = Some(memos);
        self
    }

    /// Run as an effect, its writes buffered in `effects`.
    pub(crate) fn set_effects(&mut self, effects: &'p mut EffectBuffer) -> &mut Self {
        self.effects = Some(effects);
        self
    }

    /// Set the entity the script runs on.
    pub fn set_owner(&mut self, owner: Entity) -> &mut Self {
        self.owner = owner;
        self
    }

    /// What the result's slots refer to.
    pub fn store(&self) -> &SlotStore<'w> {
        &self.store
    }

    /// Read an immediate value following the current instruction.
    #[inline(always)]
    fn read_immediate<T: Copy>(&mut self) -> T {
        // SAFETY: the compiler writes each opcode's immediates after it, unaligned, and the
        // function's code ends with `ret`.
        unsafe {
            let val = core::ptr::read_unaligned(self.iptr as *const T);
            self.iptr = self.iptr.add(size_of::<T>());
            val
        }
    }

    #[inline(always)]
    fn read_u16(&mut self) -> usize {
        self.read_immediate::<u16>() as usize
    }

    #[inline(always)]
    fn pop(&mut self) -> Result<StackValue, VMError> {
        self.stack.pop().ok_or(VMError::StackUnderflow)
    }

    /// The start of the top `n` slots.
    fn top(&self, n: usize) -> Result<usize, VMError> {
        self.stack
            .len()
            .checked_sub(n)
            .ok_or(VMError::StackUnderflow)
    }

    /// Run at the instruction pointer until the outermost function returns; its result is the
    /// slots it leaves at the bottom of the stack, and this is their count.
    fn start(&mut self) -> Result<usize, VMError> {
        loop {
            // SAFETY: as `read_immediate`.
            let op = unsafe { *self.iptr };
            // SAFETY: as `read_immediate`.
            self.iptr = unsafe { self.iptr.add(1) };
            if op == instr::OP_RET {
                let n = self.read_u16();
                let start = self.top(n)?;
                if start < self.frame_base {
                    return Err(VMError::StackUnderflow);
                }
                self.stack.copy_within(start.., self.frame_base);
                self.stack.truncate(self.frame_base + n);
                let Some(caller) = self.call_stack.pop() else {
                    return Ok(n);
                };
                self.module = caller.module;
                self.frame_base = caller.frame_base;
                self.iptr = caller.iptr;
                continue;
            }
            ScriptVm::JUMP_TABLE[op as usize](self)?;
        }
    }

    /// Run a compiled function by name with arguments, returning its result's slots.
    pub fn run_with(
        &mut self,
        module: &'w ScriptModule,
        function_name: &str,
        args: &[StackValue],
    ) -> Result<&[StackValue], VMError> {
        let missing = || VMError::MissingFunction(function_name.to_string());
        let index = module.function_index(function_name).ok_or_else(missing)?;
        self.run_index(module, index, args)
    }

    /// Run function `index` of `module` with arguments, returning its result's slots.
    pub fn run_index(
        &mut self,
        module: &'w ScriptModule,
        index: usize,
        args: &[StackValue],
    ) -> Result<&[StackValue], VMError> {
        let function = module
            .functions
            .get(index)
            .ok_or(VMError::InvalidFunction(index))?;
        if function.num_params != args.len() {
            return Err(VMError::IncorrectParamCount(
                format!("#{index}"),
                function.num_params,
                args.len(),
            ));
        }
        self.module = Some(module);
        self.call_stack.clear();
        self.store.clear();
        self.frame_base = 0;
        self.stack.clear();
        self.stack.extend_from_slice(args);
        self.stack
            .resize(self.stack.len() + function.num_locals, StackValue::Void);
        self.iptr = function.code.as_ptr();
        let n = self.start()?;
        Ok(&self.stack[..n])
    }

    /// Run a compiled function that takes no parameters, by name, returning its result's slots.
    pub fn run(
        &mut self,
        module: &'w ScriptModule,
        function_name: &str,
    ) -> Result<&[StackValue], VMError> {
        self.run_with(module, function_name, &[])
    }

    /// Run a compiled function that takes no parameters, by name, returning its result as `T`.
    pub fn run_as<T>(&mut self, module: &'w ScriptModule, function_name: &str) -> Result<T, VMError>
    where
        T: for<'a> FromSlots<'a>,
    {
        let n = self.run(module, function_name)?.len();
        read_result(&self.stack[..n], &self.store)
    }

    /// Run function `index`, which takes no parameters, returning its result as `T`.
    pub fn run_index_as<T>(&mut self, module: &'w ScriptModule, index: usize) -> Result<T, VMError>
    where
        T: for<'a> FromSlots<'a>,
    {
        let n = self.run_index(module, index, &[])?.len();
        read_result(&self.stack[..n], &self.store)
    }
}

fn read_result<'a, T: FromSlots<'a>>(
    slots: &[StackValue],
    store: &'a SlotStore<'_>,
) -> Result<T, VMError> {
    if slots.len() != T::SLOTS {
        return Err(VMError::ResultSlots(T::SLOTS, slots.len()));
    }
    T::from_slots(slots, store)
}

/// The type of what a slot holds, for errors.
fn value_type(value: &StackValue) -> ExprType {
    match value {
        StackValue::Void => ExprType::Void,
        StackValue::Bool(_) => ExprType::Boolean,
        StackValue::I32(_) => ExprType::I32,
        StackValue::I64(_) => ExprType::I64,
        StackValue::F32(_) => ExprType::F32,
        StackValue::F64(_) => ExprType::F64,
        StackValue::Entity(_) => ExprType::Entity,
        StackValue::Str(_) => ExprType::String,
    }
}

// Build the jump table
const fn build_jump_table() -> [InstrHandler; 256] {
    use instr::*;
    let mut table: [InstrHandler; 256] = [invalid; 256];
    table[OP_CONST_TRUE as usize] = const_true;
    table[OP_CONST_FALSE as usize] = const_false;
    table[OP_CONST_I32 as usize] = const_i32;
    table[OP_CONST_I64 as usize] = const_i64;
    table[OP_CONST_F32 as usize] = const_f32;
    table[OP_CONST_F64 as usize] = const_f64;
    table[OP_CONST_STR as usize] = const_str;
    table[OP_DROP as usize] = drop1;
    table[OP_DROP_N as usize] = drop_n;
    table[OP_LOAD_LOCAL as usize] = load_local;
    table[OP_LOAD_GLOBAL as usize] = load_global;
    table[OP_LOAD_COMPONENT as usize] = load_component;
    table[OP_LOAD_SCRIPTED as usize] = load_scripted;
    table[OP_STORE_SCRIPTED as usize] = store_scripted;
    table[OP_STORE_LOCAL as usize] = store_local;
    table[OP_EXTRACT as usize] = extract;
    table[OP_INDEX as usize] = index;

    table[OP_NOT as usize] = not;
    table[OP_NEG_I32 as usize] = neg_i32;
    table[OP_NEG_I64 as usize] = neg_i64;
    table[OP_NEG_F32 as usize] = neg_f32;
    table[OP_NEG_F64 as usize] = neg_f64;
    table[OP_BIT_NOT_I32 as usize] = bit_not_i32;
    table[OP_BIT_NOT_I64 as usize] = bit_not_i64;
    table[OP_CONVERT as usize] = convert;

    table[OP_RET as usize] = ret;
    table[OP_BRANCH as usize] = branch;
    table[OP_BRANCH_IF_TRUE as usize] = branch_if_true;
    table[OP_BRANCH_IF_FALSE as usize] = branch_if_false;

    table[OP_CALL as usize] = call;
    table[OP_CALL_HOST_METHOD as usize] = call_host_method;
    table[OP_CALL_HOST_FUNCTION as usize] = call_host_function;

    table[OP_ADD_I32 as usize] = add_i32;
    table[OP_ADD_I64 as usize] = add_i64;
    table[OP_ADD_F32 as usize] = add_f32;
    table[OP_ADD_F64 as usize] = add_f64;

    table[OP_SUB_I32 as usize] = sub_i32;
    table[OP_SUB_I64 as usize] = sub_i64;
    table[OP_SUB_F32 as usize] = sub_f32;
    table[OP_SUB_F64 as usize] = sub_f64;

    table[OP_MUL_I32 as usize] = mul_i32;
    table[OP_MUL_I64 as usize] = mul_i64;
    table[OP_MUL_F32 as usize] = mul_f32;
    table[OP_MUL_F64 as usize] = mul_f64;

    table[OP_DIV_I32 as usize] = div_i32;
    table[OP_DIV_I64 as usize] = div_i64;
    table[OP_DIV_F32 as usize] = div_f32;
    table[OP_DIV_F64 as usize] = div_f64;

    table[OP_REM_I32 as usize] = rem_i32;
    table[OP_REM_I64 as usize] = rem_i64;
    table[OP_REM_F32 as usize] = rem_f32;
    table[OP_REM_F64 as usize] = rem_f64;

    table[OP_BIT_AND_I32 as usize] = bit_and_i32;
    table[OP_BIT_AND_I64 as usize] = bit_and_i64;
    table[OP_BIT_OR_I32 as usize] = bit_or_i32;
    table[OP_BIT_OR_I64 as usize] = bit_or_i64;
    table[OP_BIT_XOR_I32 as usize] = bit_xor_i32;
    table[OP_BIT_XOR_I64 as usize] = bit_xor_i64;
    table[OP_SHL_I32 as usize] = shl_i32;
    table[OP_SHL_I64 as usize] = shl_i64;
    table[OP_SHR_I32 as usize] = shr_i32;
    table[OP_SHR_I64 as usize] = shr_i64;

    table[OP_EQ_BOOL as usize] = eq_bool;
    table[OP_EQ_I32 as usize] = eq_i32;
    table[OP_EQ_I64 as usize] = eq_i64;
    table[OP_EQ_F32 as usize] = eq_f32;
    table[OP_EQ_F64 as usize] = eq_f64;

    table[OP_NE_BOOL as usize] = ne_bool;
    table[OP_NE_I32 as usize] = ne_i32;
    table[OP_NE_I64 as usize] = ne_i64;
    table[OP_NE_F32 as usize] = ne_f32;
    table[OP_NE_F64 as usize] = ne_f64;

    table[OP_LT_I32 as usize] = lt_i32;
    table[OP_LT_I64 as usize] = lt_i64;
    table[OP_LT_F32 as usize] = lt_f32;
    table[OP_LT_F64 as usize] = lt_f64;

    table[OP_LE_I32 as usize] = le_i32;
    table[OP_LE_I64 as usize] = le_i64;
    table[OP_LE_F32 as usize] = le_f32;
    table[OP_LE_F64 as usize] = le_f64;

    table[OP_GT_I32 as usize] = gt_i32;
    table[OP_GT_I64 as usize] = gt_i64;
    table[OP_GT_F32 as usize] = gt_f32;
    table[OP_GT_F64 as usize] = gt_f64;

    table[OP_GE_I32 as usize] = ge_i32;
    table[OP_GE_I64 as usize] = ge_i64;
    table[OP_GE_F32 as usize] = ge_f32;
    table[OP_GE_F64 as usize] = ge_f64;

    table
}

fn invalid(_vm: &mut ScriptVm) -> Result<(), VMError> {
    Err(VMError::InvalidInstruction)
}

fn const_true(vm: &mut ScriptVm) -> Result<(), VMError> {
    vm.stack.push(StackValue::Bool(true));
    Ok(())
}

fn const_false(vm: &mut ScriptVm) -> Result<(), VMError> {
    vm.stack.push(StackValue::Bool(false));
    Ok(())
}

fn const_i32(vm: &mut ScriptVm) -> Result<(), VMError> {
    let val = vm.read_immediate::<i32>();
    vm.stack.push(StackValue::I32(val));
    Ok(())
}

fn const_i64(vm: &mut ScriptVm) -> Result<(), VMError> {
    let val = vm.read_immediate::<i64>();
    vm.stack.push(StackValue::I64(val));
    Ok(())
}

fn const_f32(vm: &mut ScriptVm) -> Result<(), VMError> {
    let val = vm.read_immediate::<f32>();
    vm.stack.push(StackValue::F32(val));
    Ok(())
}

fn const_f64(vm: &mut ScriptVm) -> Result<(), VMError> {
    let val = vm.read_immediate::<f64>();
    vm.stack.push(StackValue::F64(val));
    Ok(())
}

fn const_str(vm: &mut ScriptVm) -> Result<(), VMError> {
    let len = vm.read_u16();
    // SAFETY: the compiler writes the string's `len` bytes after its length, from a `&str`, in
    // the code of a module borrowed for the evaluation.
    let s = unsafe {
        let bytes = core::slice::from_raw_parts(vm.iptr, len);
        vm.iptr = vm.iptr.add(len);
        core::str::from_utf8_unchecked(bytes)
    };
    let slot = vm.store.borrowed_str(s)?;
    vm.stack.push(slot);
    Ok(())
}

fn drop1(vm: &mut ScriptVm) -> Result<(), VMError> {
    vm.pop()?;
    Ok(())
}

fn drop_n(vm: &mut ScriptVm) -> Result<(), VMError> {
    let n = vm.read_immediate::<u32>() as usize;
    let final_length = vm.stack.len().saturating_sub(n);
    vm.stack.truncate(final_length);
    Ok(())
}

/// Parameters and locals share the frame: parameters first.
fn load_local(vm: &mut ScriptVm) -> Result<(), VMError> {
    let slot = vm.read_u16();
    let n = vm.read_u16();
    let start = vm.frame_base + slot;
    if start + n > vm.stack.len() {
        return Err(VMError::InvalidLocal(slot));
    }
    vm.stack.extend_from_within(start..start + n);
    Ok(())
}

fn store_local(vm: &mut ScriptVm) -> Result<(), VMError> {
    let slot = vm.read_u16();
    let n = vm.read_u16();
    let source = vm.top(n)?;
    let target = vm.frame_base + slot;
    if target + n > source {
        return Err(VMError::InvalidLocal(slot));
    }
    vm.stack.copy_within(source.., target);
    vm.stack.truncate(source);
    Ok(())
}

fn load_global(vm: &mut ScriptVm) -> Result<(), VMError> {
    let n = vm.read_u16();
    let host = vm.host;
    let val = match host.members.get(n) {
        Some(HostMember::Const(val)) => *val,
        Some(HostMember::GlobalProperty(accessor)) => accessor(vm)?,
        Some(_) => return Err(VMError::InvalidNativeProp(n)),
        None => return Err(VMError::InvalidGlobalIndex(n)),
    };
    vm.stack.push(val);
    Ok(())
}

/// Pushes a component's leaves.
struct StackSink<'s, 'w> {
    stack: &'s mut Vec<StackValue>,
    store: &'s mut SlotStore<'w>,
}

impl<'w> SlotSink<'w> for StackSink<'_, 'w> {
    fn leaf(&mut self, leaf: Leaf, value: &'w dyn PartialReflect) -> Result<(), VMError> {
        let mismatch = || VMError::Unsupported("a value unlike its type's layout");
        let slot = match leaf {
            Leaf::Bool => StackValue::Bool(*value.try_downcast_ref().ok_or_else(mismatch)?),
            Leaf::I32 => StackValue::I32(*value.try_downcast_ref().ok_or_else(mismatch)?),
            Leaf::I64 => StackValue::I64(*value.try_downcast_ref().ok_or_else(mismatch)?),
            Leaf::F32 => StackValue::F32(*value.try_downcast_ref().ok_or_else(mismatch)?),
            Leaf::F64 => StackValue::F64(*value.try_downcast_ref().ok_or_else(mismatch)?),
            Leaf::Entity => StackValue::Entity(*value.try_downcast_ref().ok_or_else(mismatch)?),
            Leaf::Str => {
                let s: &'w String = value.try_downcast_ref().ok_or_else(mismatch)?;
                self.store.borrowed_str(s)?
            }
            // Refused when the formula compiles.
            Leaf::Opaque => return Err(VMError::Unsupported("a value scripts cannot see into")),
        };
        self.stack.push(slot);
        Ok(())
    }
}

/// Replaces an entity with `n` slots of one of its components, from slot `offset`.
fn load_component(vm: &mut ScriptVm) -> Result<(), VMError> {
    let member = vm.read_u16();
    let offset = vm.read_u16();
    let n = vm.read_u16();
    let StackValue::Entity(entity) = vm.pop()? else {
        return Err(VMError::InvalidType(ExprType::Entity));
    };
    let Some(HostMember::Component(component)) = vm.host.members.get(member) else {
        return Err(VMError::InvalidNativeProp(member));
    };
    let value = (component.read)(vm.world, entity, vm.tracking)?;
    let mut sink = StackSink {
        stack: &mut vm.stack,
        store: &mut vm.store,
    };
    flatten(value, &component.layout, offset, offset + n, &mut sink)
}

/// Replaces an entity with `n` slots of a field of one of its scripted aspects, from slot
/// `offset`. The field is found by name in the component, whose aspect may have been declared
/// by an older load of its module, and must still be of the type the formula was compiled for.
fn load_scripted(vm: &mut ScriptVm) -> Result<(), VMError> {
    let read = vm.read_u16();
    let field = vm.read_u16();
    let offset = vm.read_u16();
    let n = vm.read_u16();
    let StackValue::Entity(entity) = vm.pop()? else {
        return Err(VMError::InvalidType(ExprType::Entity));
    };
    let invalid = || VMError::Unsupported("a scripted field the module does not read");
    let aspect = vm
        .module
        .and_then(|module| module.scripted_reads.get(read))
        .ok_or_else(invalid)?;
    let field = aspect.fields.get(field).ok_or_else(invalid)?;
    let (id, fields) = scripted::get(vm.world, entity, &aspect.key, field.is_state);
    if let Some(id) = id {
        vm.tracking.track_component_id(entity, id, fields.is_some());
    }
    let missing = || VMError::MissingAspect(aspect.key.clone());
    let value = fields
        .and_then(|fields| fields.get(&field.name))
        .ok_or_else(missing)?;
    if value.as_any().type_id() != field.info.type_id() {
        return Err(VMError::Unsupported("a scripted field whose type changed"));
    }
    let mut sink = StackSink {
        stack: &mut vm.stack,
        store: &mut vm.store,
    };
    flatten(
        value.as_partial_reflect(),
        &Layout::of(field.info),
        offset,
        offset + n,
        &mut sink,
    )
}

/// Pops a value of one slot and the entity below it, and buffers a write of the value to a
/// state field of one of the entity's scripted aspects. Only an effect writes.
fn store_scripted(vm: &mut ScriptVm) -> Result<(), VMError> {
    let read = vm.read_u16();
    let field = vm.read_u16();
    let value = vm.pop()?;
    let StackValue::Entity(entity) = vm.pop()? else {
        return Err(VMError::InvalidType(ExprType::Entity));
    };
    let invalid = || VMError::Unsupported("a scripted field the module does not read");
    let aspect = vm
        .module
        .and_then(|module| module.scripted_reads.get(read))
        .ok_or_else(invalid)?;
    let field = aspect.fields.get(field).ok_or_else(invalid)?;
    let mismatch =
        || VMError::MismatchedTypes(ExprType::from_type_info(field.info), value_type(&value));
    let reflected: Box<dyn PartialReflect> = match (ExprType::from_type_info(field.info), value) {
        (ExprType::Boolean, StackValue::Bool(v)) => Box::new(v),
        (ExprType::I32, StackValue::I32(v)) => Box::new(v),
        (ExprType::I64, StackValue::I64(v)) => Box::new(v),
        (ExprType::F32, StackValue::F32(v)) => Box::new(v),
        (ExprType::F64, StackValue::F64(v)) => Box::new(v),
        (ExprType::Entity, StackValue::Entity(v)) => Box::new(v),
        (ExprType::String, StackValue::Str(index)) => Box::new(
            vm.store
                .str(index)
                .ok_or(VMError::InvalidString(index))?
                .to_string(),
        ),
        _ => return Err(mismatch()),
    };
    let effects = vm
        .effects
        .as_deref_mut()
        .ok_or(VMError::Unsupported("a write outside an effect"))?;
    effects.write_scripted(entity, aspect.key.clone(), field.name.clone(), reflected);
    Ok(())
}

/// Keeps `n` slots from `offset` of the top `total`: a field of a value.
fn extract(vm: &mut ScriptVm) -> Result<(), VMError> {
    let offset = vm.read_u16();
    let n = vm.read_u16();
    let total = vm.read_u16();
    let start = vm.top(total)?;
    if offset + n > total {
        return Err(VMError::StackUnderflow);
    }
    vm.stack
        .copy_within(start + offset..start + offset + n, start);
    vm.stack.truncate(start + n);
    Ok(())
}

/// Replaces an array of `len` elements of `size` slots, and an index above it, with the
/// element.
fn index(vm: &mut ScriptVm) -> Result<(), VMError> {
    let size = vm.read_u16();
    let len = vm.read_u16();
    let StackValue::I32(i) = vm.pop()? else {
        return Err(VMError::InvalidType(ExprType::I32));
    };
    let element = usize::try_from(i)
        .ok()
        .filter(|e| *e < len)
        .ok_or(VMError::IndexOutOfBounds(i, len))?;
    let start = vm.top(size * len)?;
    let from = start + element * size;
    vm.stack.copy_within(from..from + size, start);
    vm.stack.truncate(start + size);
    Ok(())
}

/// A binary operator on two values of one variant; `$body` gives the result.
macro_rules! binop {
    ($name:ident, $variant:ident, |$l:ident, $r:ident| $body:expr) => {
        fn $name(vm: &mut ScriptVm) -> Result<(), VMError> {
            let rhs = vm.pop()?;
            let lhs = vm.pop()?;
            let (StackValue::$variant($l), StackValue::$variant($r)) = (lhs, rhs) else {
                return Err(VMError::MismatchedTypes(value_type(&lhs), value_type(&rhs)));
            };
            let result: Result<StackValue, VMError> = $body;
            vm.stack.push(result?);
            Ok(())
        }
    };
}

/// A unary operator on one variant; `$body` gives the result.
macro_rules! unop {
    ($name:ident, $variant:ident, |$a:ident| $body:expr) => {
        fn $name(vm: &mut ScriptVm) -> Result<(), VMError> {
            let arg = vm.pop()?;
            let StackValue::$variant($a) = arg else {
                return Err(VMError::InvalidType(value_type(&arg)));
            };
            vm.stack.push($body);
            Ok(())
        }
    };
}

use StackValue::{Bool, F32, F64, I32, I64};

// Integer arithmetic wraps; division by zero is an error.
binop!(add_i32, I32, |l, r| Ok(I32(l.wrapping_add(r))));
binop!(add_i64, I64, |l, r| Ok(I64(l.wrapping_add(r))));
binop!(add_f32, F32, |l, r| Ok(F32(l + r)));
binop!(add_f64, F64, |l, r| Ok(F64(l + r)));

binop!(sub_i32, I32, |l, r| Ok(I32(l.wrapping_sub(r))));
binop!(sub_i64, I64, |l, r| Ok(I64(l.wrapping_sub(r))));
binop!(sub_f32, F32, |l, r| Ok(F32(l - r)));
binop!(sub_f64, F64, |l, r| Ok(F64(l - r)));

binop!(mul_i32, I32, |l, r| Ok(I32(l.wrapping_mul(r))));
binop!(mul_i64, I64, |l, r| Ok(I64(l.wrapping_mul(r))));
binop!(mul_f32, F32, |l, r| Ok(F32(l * r)));
binop!(mul_f64, F64, |l, r| Ok(F64(l * r)));

binop!(div_i32, I32, |l, r| if r == 0 {
    Err(VMError::DivideByZero)
} else {
    Ok(I32(l.wrapping_div(r)))
});
binop!(div_i64, I64, |l, r| if r == 0 {
    Err(VMError::DivideByZero)
} else {
    Ok(I64(l.wrapping_div(r)))
});
binop!(div_f32, F32, |l, r| Ok(F32(l / r)));
binop!(div_f64, F64, |l, r| Ok(F64(l / r)));

binop!(rem_i32, I32, |l, r| if r == 0 {
    Err(VMError::DivideByZero)
} else {
    Ok(I32(l.wrapping_rem(r)))
});
binop!(rem_i64, I64, |l, r| if r == 0 {
    Err(VMError::DivideByZero)
} else {
    Ok(I64(l.wrapping_rem(r)))
});
binop!(rem_f32, F32, |l, r| Ok(F32(l % r)));
binop!(rem_f64, F64, |l, r| Ok(F64(l % r)));

binop!(bit_and_i32, I32, |l, r| Ok(I32(l & r)));
binop!(bit_and_i64, I64, |l, r| Ok(I64(l & r)));
binop!(bit_or_i32, I32, |l, r| Ok(I32(l | r)));
binop!(bit_or_i64, I64, |l, r| Ok(I64(l | r)));
binop!(bit_xor_i32, I32, |l, r| Ok(I32(l ^ r)));
binop!(bit_xor_i64, I64, |l, r| Ok(I64(l ^ r)));
binop!(shl_i32, I32, |l, r| Ok(I32(l.wrapping_shl(r as u32))));
binop!(shl_i64, I64, |l, r| Ok(I64(l.wrapping_shl(r as u32))));
binop!(shr_i32, I32, |l, r| Ok(I32(l.wrapping_shr(r as u32))));
binop!(shr_i64, I64, |l, r| Ok(I64(l.wrapping_shr(r as u32))));

binop!(eq_bool, Bool, |l, r| Ok(Bool(l == r)));
binop!(eq_i32, I32, |l, r| Ok(Bool(l == r)));
binop!(eq_i64, I64, |l, r| Ok(Bool(l == r)));
binop!(eq_f32, F32, |l, r| Ok(Bool(l == r)));
binop!(eq_f64, F64, |l, r| Ok(Bool(l == r)));

binop!(ne_bool, Bool, |l, r| Ok(Bool(l != r)));
binop!(ne_i32, I32, |l, r| Ok(Bool(l != r)));
binop!(ne_i64, I64, |l, r| Ok(Bool(l != r)));
binop!(ne_f32, F32, |l, r| Ok(Bool(l != r)));
binop!(ne_f64, F64, |l, r| Ok(Bool(l != r)));

binop!(lt_i32, I32, |l, r| Ok(Bool(l < r)));
binop!(lt_i64, I64, |l, r| Ok(Bool(l < r)));
binop!(lt_f32, F32, |l, r| Ok(Bool(l < r)));
binop!(lt_f64, F64, |l, r| Ok(Bool(l < r)));

binop!(le_i32, I32, |l, r| Ok(Bool(l <= r)));
binop!(le_i64, I64, |l, r| Ok(Bool(l <= r)));
binop!(le_f32, F32, |l, r| Ok(Bool(l <= r)));
binop!(le_f64, F64, |l, r| Ok(Bool(l <= r)));

binop!(gt_i32, I32, |l, r| Ok(Bool(l > r)));
binop!(gt_i64, I64, |l, r| Ok(Bool(l > r)));
binop!(gt_f32, F32, |l, r| Ok(Bool(l > r)));
binop!(gt_f64, F64, |l, r| Ok(Bool(l > r)));

binop!(ge_i32, I32, |l, r| Ok(Bool(l >= r)));
binop!(ge_i64, I64, |l, r| Ok(Bool(l >= r)));
binop!(ge_f32, F32, |l, r| Ok(Bool(l >= r)));
binop!(ge_f64, F64, |l, r| Ok(Bool(l >= r)));

unop!(not, Bool, |a| Bool(!a));
unop!(neg_i32, I32, |a| I32(a.wrapping_neg()));
unop!(neg_i64, I64, |a| I64(a.wrapping_neg()));
unop!(neg_f32, F32, |a| F32(-a));
unop!(neg_f64, F64, |a| F64(-a));
unop!(bit_not_i32, I32, |a| I32(!a));
unop!(bit_not_i64, I64, |a| I64(!a));

/// Primitive conversions, as Rust's `as`: floats to integers saturate.
fn convert(vm: &mut ScriptVm) -> Result<(), VMError> {
    let _from = vm.read_immediate::<u8>();
    let to = vm.read_immediate::<u8>();
    let arg = vm.pop()?;
    let val = match (arg, to) {
        (Bool(b), instr::TYPE_I32) => I32(b as i32),
        (Bool(b), instr::TYPE_I64) => I64(b as i64),
        (I32(n), instr::TYPE_I32) => I32(n),
        (I32(n), instr::TYPE_I64) => I64(n as i64),
        (I32(n), instr::TYPE_F32) => F32(n as f32),
        (I32(n), instr::TYPE_F64) => F64(n as f64),
        (I64(n), instr::TYPE_I32) => I32(n as i32),
        (I64(n), instr::TYPE_I64) => I64(n),
        (I64(n), instr::TYPE_F32) => F32(n as f32),
        (I64(n), instr::TYPE_F64) => F64(n as f64),
        (F32(n), instr::TYPE_I32) => I32(n as i32),
        (F32(n), instr::TYPE_I64) => I64(n as i64),
        (F32(n), instr::TYPE_F32) => F32(n),
        (F32(n), instr::TYPE_F64) => F64(n as f64),
        (F64(n), instr::TYPE_I32) => I32(n as i32),
        (F64(n), instr::TYPE_I64) => I64(n as i64),
        (F64(n), instr::TYPE_F32) => F32(n as f32),
        (F64(n), instr::TYPE_F64) => F64(n),
        _ => return Err(VMError::InvalidType(value_type(&arg))),
    };
    vm.stack.push(val);
    Ok(())
}

/// Calls a script function: its arguments, the top `n` slots, become its frame's first.
fn call(vm: &mut ScriptVm) -> Result<(), VMError> {
    let fn_index = vm.read_u16();
    let n = vm.read_u16();
    let base = vm.top(n)?;
    let module = vm.module.ok_or(VMError::InvalidFunction(fn_index))?;
    let func = module
        .functions
        .get(fn_index)
        .ok_or(VMError::InvalidFunction(fn_index))?;
    vm.call_stack.push(CallStackEntry {
        module: vm.module,
        frame_base: vm.frame_base,
        iptr: vm.iptr,
    });
    vm.frame_base = base;
    vm.stack
        .resize(vm.stack.len() + func.num_locals, StackValue::Void);
    vm.iptr = func.code.as_ptr();
    Ok(())
}

fn call_host_method(vm: &mut ScriptVm) -> Result<(), VMError> {
    let method_index = vm.read_u16();
    let f = match vm.host.members.get(method_index) {
        Some(HostMember::InstanceMethod(f) | HostMember::InstanceProperty(f)) => *f,
        _ => return Err(VMError::InvalidNativeMethod(method_index)),
    };
    call_native(vm, f)
}

fn call_host_function(vm: &mut ScriptVm) -> Result<(), VMError> {
    let function_index = vm.read_u16();
    let Some(HostMember::Function(f)) = vm.host.members.get(function_index) else {
        return Err(VMError::InvalidGlobalIndex(function_index));
    };
    call_native(vm, *f)
}

/// Calls a native function on the top `n` slots, which it replaces with its result of
/// `results` slots.
fn call_native(vm: &mut ScriptVm, f: crate::host::HostFunction) -> Result<(), VMError> {
    let n = vm.read_u16();
    let results = vm.read_u16();
    let args_start = vm.top(n)?;
    vm.out.clear();
    let mut ctx = InvocationContext {
        world: vm.world,
        store: &mut vm.store,
        args: &vm.stack[args_start..],
        next_arg: Cell::new(0),
        out: &mut vm.out,
        tracking: vm.tracking,
        memos: vm.memos.as_deref_mut(),
        site: vm.iptr as usize,
    };
    f(&mut ctx)?;
    if vm.out.len() != results {
        return Err(VMError::NativeResultSlots(results, vm.out.len()));
    }
    vm.stack.truncate(args_start);
    vm.stack.extend_from_slice(&vm.out);
    Ok(())
}

fn jump(vm: &mut ScriptVm, offset: i32) {
    // SAFETY: the compiler patches each branch to the start of a block of the same function.
    vm.iptr = unsafe { vm.iptr.offset(offset as isize) };
}

fn branch(vm: &mut ScriptVm) -> Result<(), VMError> {
    let offset = vm.read_immediate::<i32>();
    jump(vm, offset);
    Ok(())
}

fn branch_if(vm: &mut ScriptVm, when: bool) -> Result<(), VMError> {
    let test = vm.pop()?;
    let offset = vm.read_immediate::<i32>();
    match test {
        Bool(b) => {
            if b == when {
                jump(vm, offset);
            }
            Ok(())
        }
        _ => Err(VMError::MismatchedTypes(
            value_type(&test),
            ExprType::Boolean,
        )),
    }
}

fn branch_if_false(vm: &mut ScriptVm) -> Result<(), VMError> {
    branch_if(vm, false)
}

fn branch_if_true(vm: &mut ScriptVm) -> Result<(), VMError> {
    branch_if(vm, true)
}

fn ret(_vm: &mut ScriptVm) -> Result<(), VMError> {
    // The interpreter loop handles `ret` before dispatch.
    Err(VMError::InvalidInstruction)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codegen::instr::InstructionBuilder;

    /// Runs `build`'s code, with a `ret` of one slot appended.
    fn run_code(build: impl FnOnce(&mut InstructionBuilder)) -> Result<StackValue, VMError> {
        let world = World::new();
        let host = HostState::default();
        let mut tracking = TrackingScope::default();
        let mut builder = InstructionBuilder::default();
        build(&mut builder);
        builder.push_op(instr::OP_RET);
        builder.push_immediate::<u16>(1);
        let code = builder.inner();
        let mut vm = ScriptVm::new(&world, &host, &mut tracking);
        vm.iptr = code.as_ptr();
        vm.start()?;
        vm.stack.first().copied().ok_or(VMError::StackUnderflow)
    }

    fn push_i32(b: &mut InstructionBuilder, n: i32) {
        b.push_op(instr::OP_CONST_I32);
        b.push_immediate(n);
    }

    #[test]
    fn test_invalid_instr() {
        let error = run_code(|b| b.push_op(0)).unwrap_err();
        assert_eq!(error, VMError::InvalidInstruction);
    }

    #[test]
    fn test_const_bool() {
        let result = run_code(|b| b.push_op(instr::OP_CONST_TRUE)).unwrap();
        assert_eq!(result, StackValue::Bool(true));
    }

    #[test]
    fn test_const_i32() {
        assert_eq!(run_code(|b| push_i32(b, 3)).unwrap(), StackValue::I32(3));
    }

    #[test]
    fn test_add_i32() {
        let result = run_code(|b| {
            push_i32(b, 5);
            push_i32(b, 1);
            b.push_op(instr::OP_ADD_I32);
        });
        assert_eq!(result.unwrap(), StackValue::I32(6));
    }

    #[test]
    fn test_add_f32() {
        let result = run_code(|b| {
            b.push_op(instr::OP_CONST_F32);
            b.push_immediate::<f32>(5.0);
            b.push_op(instr::OP_CONST_F32);
            b.push_immediate::<f32>(1.0);
            b.push_op(instr::OP_ADD_F32);
        });
        assert_eq!(result.unwrap(), StackValue::F32(6.0));
    }

    #[test]
    fn test_add_f64() {
        let result = run_code(|b| {
            b.push_op(instr::OP_CONST_F64);
            b.push_immediate::<f64>(5.0);
            b.push_op(instr::OP_CONST_F64);
            b.push_immediate::<f64>(1.0);
            b.push_op(instr::OP_ADD_F64);
        });
        assert_eq!(result.unwrap(), StackValue::F64(6.0));
    }

    #[test]
    fn test_add_mismatched() {
        let error = run_code(|b| {
            push_i32(b, 5);
            b.push_op(instr::OP_CONST_F32);
            b.push_immediate::<f32>(1.0);
            b.push_op(instr::OP_ADD_I32);
        })
        .unwrap_err();
        assert_eq!(
            error,
            VMError::MismatchedTypes(ExprType::I32, ExprType::F32)
        );
    }

    #[test]
    fn test_add_wraps() {
        let result = run_code(|b| {
            push_i32(b, i32::MAX);
            push_i32(b, 1);
            b.push_op(instr::OP_ADD_I32);
        });
        assert_eq!(result.unwrap(), StackValue::I32(i32::MIN));
    }

    #[test]
    fn test_divide_by_zero() {
        let error = run_code(|b| {
            push_i32(b, 5);
            push_i32(b, 0);
            b.push_op(instr::OP_DIV_I32);
        })
        .unwrap_err();
        assert_eq!(error, VMError::DivideByZero);
    }

    #[test]
    fn test_sub_i32() {
        let result = run_code(|b| {
            push_i32(b, 5);
            push_i32(b, 1);
            b.push_op(instr::OP_SUB_I32);
        });
        assert_eq!(result.unwrap(), StackValue::I32(4));
    }

    #[test]
    fn test_bit_and_i32() {
        let result = run_code(|b| {
            push_i32(b, 5);
            push_i32(b, 1);
            b.push_op(instr::OP_BIT_AND_I32);
        });
        assert_eq!(result.unwrap(), StackValue::I32(1));
    }

    #[test]
    fn test_bit_or_i32() {
        let result = run_code(|b| {
            push_i32(b, 4);
            push_i32(b, 1);
            b.push_op(instr::OP_BIT_OR_I32);
        });
        assert_eq!(result.unwrap(), StackValue::I32(5));
    }

    #[test]
    fn test_branch_if_true_falls_through_on_false() {
        let result = run_code(|b| {
            b.push_op(instr::OP_CONST_FALSE);
            b.push_op(instr::OP_BRANCH_IF_TRUE);
            // Skips the next instruction (5 bytes) when taken.
            b.push_immediate::<i32>(5);
            push_i32(b, 1);
        });
        assert_eq!(result.unwrap(), StackValue::I32(1));
    }

    #[test]
    fn test_extract_keeps_a_field() {
        let result = run_code(|b| {
            for n in 1..=3 {
                push_i32(b, n);
            }
            b.push_op(instr::OP_EXTRACT);
            b.push_immediate::<u16>(1);
            b.push_immediate::<u16>(1);
            b.push_immediate::<u16>(3);
        });
        assert_eq!(result.unwrap(), StackValue::I32(2));
    }

    #[test]
    fn test_global() {
        let mut world = World::new();
        let actor_id = world.spawn(()).id();
        let mut host = HostState::default();
        let self_id = host.add_global_prop("self", get_self, ExprType::Entity);
        let mut tracking = TrackingScope::default();
        let mut builder = InstructionBuilder::default();
        builder.push_op(instr::OP_LOAD_GLOBAL);
        builder.push_immediate::<u16>(self_id as u16);
        builder.push_op(instr::OP_RET);
        builder.push_immediate::<u16>(1);
        let code = builder.inner();
        let mut vm = ScriptVm::new(&world, &host, &mut tracking);
        vm.owner = actor_id;
        vm.iptr = code.as_ptr();
        assert_eq!(vm.start(), Ok(1));
        assert_eq!(vm.stack[0], StackValue::Entity(actor_id));
    }

    fn get_self(vm: &mut ScriptVm) -> Result<StackValue, VMError> {
        Ok(StackValue::Entity(vm.owner))
    }

    #[test]
    fn test_value_size() {
        assert_eq!(size_of::<StackValue>(), 16);
    }
}
