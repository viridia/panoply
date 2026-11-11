//! The bytecode interpreter, built per evaluation.

use std::{
    any::{Any, TypeId},
    sync::Arc,
};

use bevy::{
    ecs::{component::Component, entity::Entity, world::World},
    reflect::{PartialReflect, ReflectRef, TypePath},
};
use thiserror::Error;

use crate::{
    ScriptModule,
    codegen::instr,
    host::{HostMember, HostState},
    sema::{decl::DeclKind, types::ExprType},
    tracking::TrackingScope,
};

/// Values on the stack
#[derive(Debug, PartialEq, Clone)]
pub enum StackValue {
    Void,
    Bool(bool),
    I32(i32),
    I64(i64),
    F32(f32),
    F64(f64),
    Entity(Entity),
    String(Arc<String>),
    /// An index into the table of world references, contains `&'world dyn PartialReflect`.
    WorldRef(usize),
    /// An index into the VM's temporary heap, contains `Box<dyn PartialReflect>`.
    HeapRef(u16),
}

impl StackValue {
    pub fn is_void(&self) -> bool {
        *self == StackValue::Void
    }

    pub fn type_path(
        &self,
        world_refs: &[&dyn PartialReflect],
        heap_refs: &[Box<dyn PartialReflect>],
    ) -> &'static str {
        let reflected = match self {
            StackValue::Void => return "void",
            StackValue::Bool(_) => return "bool",
            StackValue::I32(_) => return "i32",
            StackValue::I64(_) => return "i64",
            StackValue::F32(_) => return "f32",
            StackValue::F64(_) => return "f64",
            StackValue::Entity(_) => return "Entity",
            StackValue::String(_) => return "String",
            StackValue::WorldRef(index) => world_refs.get(*index).copied(),
            StackValue::HeapRef(index) => heap_refs.get(*index as usize).map(|r| r.as_ref()),
        };
        reflected
            .and_then(|r| r.get_represented_type_info())
            .map_or("none", |info| info.type_path())
    }
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
    #[error("Invalid field: {0}")]
    InvalidField(usize),
    #[error("Function not found: {0}")]
    MissingFunction(String),
    #[error("Missing Component: {0:?}")]
    MissingComponent(TypeId),
    #[error("Function {0} expects {1} parameters, got {2}")]
    IncorrectParamCount(String, usize, usize),
    #[error("Division by zero")]
    DivideByZero,
    #[error("Unsupported: {0}")]
    Unsupported(&'static str),
}

/// Saved state of VM for nested function calls
struct CallStackEntry {
    /// Module we are executing
    module: *const ScriptModule,

    /// Value stack
    stack: Vec<StackValue>,

    /// Instruction pointer
    iptr: *const u8,
}

/// A value's inner `T`, cloned out: a primitive's own, or a reflected one's by downcast.
fn extract<T: TypePath + Clone + 'static>(
    value: &StackValue,
    world_refs: &[&dyn PartialReflect],
    heap_refs: &[Box<dyn PartialReflect>],
) -> Result<T, VMError> {
    let primitive: Option<&dyn Any> = match value {
        StackValue::Bool(b) => Some(b),
        StackValue::I32(n) => Some(n),
        StackValue::I64(n) => Some(n),
        StackValue::F32(n) => Some(n),
        StackValue::F64(n) => Some(n),
        StackValue::Entity(e) => Some(e),
        StackValue::String(s) => Some(s),
        _ => None,
    };
    if let Some(t) = primitive.and_then(|p| p.downcast_ref::<T>()) {
        return Ok(t.clone());
    }
    let reflected = match value {
        StackValue::WorldRef(index) => world_refs.get(*index).copied(),
        StackValue::HeapRef(index) => heap_refs.get(*index as usize).map(|r| r.as_ref()),
        _ => None,
    };
    reflected
        .and_then(|r| r.try_downcast_ref::<T>().cloned())
        .ok_or_else(|| {
            VMError::MismatchedTypes2(value.type_path(world_refs, heap_refs), T::short_type_path())
        })
}

/// A simplified interface for calling native functions.
pub struct InvocationContext<'vm, 'world, 'p> {
    /// Bevy World
    world: &'world World,

    /// Reflected references to the world.
    world_refs: &'vm mut Vec<&'world dyn PartialReflect>,

    /// Reflected temporary objects
    heap_refs: &'vm mut Vec<Box<dyn PartialReflect>>,

    /// Calling arguments (used for function calls but not property accessors).
    args: &'vm [StackValue],

    /// What the evaluation reads, for reactions.
    tracking: &'p mut TrackingScope,
}

impl<'world> InvocationContext<'_, 'world, '_> {
    /// Return the number of arguments to the function.
    pub fn num_arguments(&self) -> usize {
        self.args.len()
    }

    /// Get the nth calling argument and unwrap it, returning a clone of the inner value.
    pub fn argument<T: TypePath + Clone + 'static>(&self, arg_index: usize) -> Result<T, VMError> {
        let value = self.args.get(arg_index).ok_or(VMError::StackUnderflow)?;
        extract(value, self.world_refs, self.heap_refs)
    }

    /// Construct a `Value` holding a temporary `PartialReflect`, freed when the evaluation ends.
    pub fn create_heap_ref<T: PartialReflect>(&mut self, value: T) -> Result<StackValue, VMError> {
        let index = u16::try_from(self.heap_refs.len())
            .map_err(|_| VMError::Unsupported("more than 65535 temporaries"))?;
        self.heap_refs.push(Box::new(value).into_partial_reflect());
        Ok(StackValue::HeapRef(index))
    }

    /// Construct a `Value` referring to a `PartialReflect` in the Bevy world.
    pub fn create_world_ref(&mut self, reflected: &'world dyn PartialReflect) -> StackValue {
        let index = self.world_refs.len();
        self.world_refs.push(reflected);
        StackValue::WorldRef(index)
    }

    /// Return a reference to the Component `C` on the given entity. Calling this function
    /// adds the component as a dependency of the current tracking scope.
    pub fn component<C: Component>(&mut self, entity: Entity) -> Option<&'world C> {
        let comp = self.world.get_entity(entity).ok()?.get::<C>();
        self.tracking
            .track_component::<C>(entity, self.world, comp.is_some());
        comp
    }
}

/// A virtual machine for evaluating formulas.
pub struct ScriptVm<'world, 'host, 'p> {
    /// Bevy World
    pub world: &'world World,

    /// Entity upon which this script is attached.
    pub owner: Entity,

    /// Globals and entity members
    pub host: &'host HostState,

    /// Module we are executing
    module: *const ScriptModule,

    /// Execution stack
    call_stack: Vec<CallStackEntry>,

    /// Reflected values.
    world_refs: Vec<&'world dyn PartialReflect>,

    /// Temporary non-primitive values created during execution.
    heap_refs: Vec<Box<dyn PartialReflect>>,

    /// Value stack: consists of [Value; num_params + num_locals + temporaries]
    stack: Vec<StackValue>,

    /// Instruction pointer
    iptr: *const u8,

    /// What the evaluation reads, for reactions.
    pub tracking: &'p mut TrackingScope,
}

type InstrHandler = fn(&mut ScriptVm) -> Result<(), VMError>;

impl<'world, 'host, 'p> ScriptVm<'world, 'host, 'p> {
    const JUMP_TABLE: [InstrHandler; 256] = build_jump_table();

    // Initialize a new virtual machine.
    pub fn new(
        world: &'world World,
        host: &'host HostState,
        tracking: &'p mut TrackingScope,
    ) -> Self {
        Self {
            world,
            owner: Entity::PLACEHOLDER,
            host,
            module: core::ptr::null(),
            call_stack: Vec::new(),
            world_refs: Vec::new(),
            heap_refs: Vec::new(),
            stack: Vec::new(),
            iptr: core::ptr::null(),
            tracking,
        }
    }

    /// Set the entity which owns the script being executed.
    pub fn set_owner(&mut self, owner: Entity) -> &mut Self {
        self.owner = owner;
        self
    }

    /// A value's inner `T`, cloned out.
    pub fn extract_value<T: TypePath + Clone + 'static>(
        &self,
        value: StackValue,
    ) -> Result<T, VMError> {
        extract(&value, &self.world_refs, &self.heap_refs)
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
    fn pop(&mut self) -> Result<StackValue, VMError> {
        self.stack.pop().ok_or(VMError::StackUnderflow)
    }

    /// Run at the instruction pointer until the outermost function returns.
    fn start(&mut self) -> Result<StackValue, VMError> {
        loop {
            // SAFETY: as `read_immediate`.
            let op = unsafe { *self.iptr };
            if op == instr::OP_RET {
                let ret_val = self.pop()?;
                if self.call_stack.is_empty() {
                    return Ok(ret_val);
                }
                self.pop_call_stack()?;
                if !ret_val.is_void() {
                    self.stack.push(ret_val);
                }
                continue;
            }
            // SAFETY: as `read_immediate`.
            self.iptr = unsafe { self.iptr.add(1) };
            ScriptVm::JUMP_TABLE[op as usize](self)?;
        }
    }

    /// Run a compiled function that takes no parameters, by name.
    pub fn run(
        &mut self,
        module: &ScriptModule,
        function_name: &str,
    ) -> Result<StackValue, VMError> {
        self.run_with(module, function_name, &[])
    }

    /// Run a compiled function by name with arguments.
    pub fn run_with(
        &mut self,
        module: &ScriptModule,
        function_name: &str,
        params: &[StackValue],
    ) -> Result<StackValue, VMError> {
        let missing = || VMError::MissingFunction(function_name.to_string());
        let entry_fn = module.module_decls.get(function_name).ok_or_else(missing)?;
        let (DeclKind::Function { index }, ExprType::Function(ftype)) =
            (&entry_fn.kind, &entry_fn.typ)
        else {
            return Err(missing());
        };
        if ftype.params.len() != params.len() {
            return Err(VMError::IncorrectParamCount(
                function_name.to_string(),
                ftype.params.len(),
                params.len(),
            ));
        }
        let function = module
            .functions
            .get(*index)
            .ok_or(VMError::InvalidFunction(*index))?;
        self.module = module;
        self.call_stack.clear();
        self.stack.clear();
        self.stack.extend_from_slice(params);
        self.stack
            .resize(self.stack.len() + function.num_locals, StackValue::Void);
        self.iptr = function.code.as_ptr();
        self.start()
    }

    fn push_call_stack(&mut self, new_stack: Vec<StackValue>) {
        self.call_stack.push(CallStackEntry {
            module: self.module,
            stack: core::mem::replace(&mut self.stack, new_stack),
            iptr: self.iptr,
        });
    }

    fn pop_call_stack(&mut self) -> Result<(), VMError> {
        let tos = self.call_stack.pop().ok_or(VMError::StackUnderflow)?;
        self.module = tos.module;
        self.stack = tos.stack;
        self.iptr = tos.iptr;
        Ok(())
    }

    /// Determine the type of this value, if possible.
    pub(crate) fn value_type(&self, value: &StackValue) -> ExprType {
        let reflected = match value {
            StackValue::Void => return ExprType::Void,
            StackValue::Bool(_) => return ExprType::Boolean,
            StackValue::I32(_) => return ExprType::I32,
            StackValue::I64(_) => return ExprType::I64,
            StackValue::F32(_) => return ExprType::F32,
            StackValue::F64(_) => return ExprType::F64,
            StackValue::Entity(_) => return ExprType::Entity,
            StackValue::String(_) => return ExprType::String,
            StackValue::WorldRef(index) => self.world_refs.get(*index).copied(),
            StackValue::HeapRef(index) => self.heap_refs.get(*index as usize).map(|r| r.as_ref()),
        };
        reflected
            .and_then(|r| r.get_represented_type_info())
            .map_or(ExprType::None, ExprType::Reflected)
    }

    fn invocation_context<'vm>(
        &'vm mut self,
        args_start: usize,
    ) -> InvocationContext<'vm, 'world, 'vm> {
        InvocationContext {
            world: self.world,
            world_refs: &mut self.world_refs,
            heap_refs: &mut self.heap_refs,
            args: &self.stack[args_start..],
            tracking: self.tracking,
        }
    }
}

// Build the jump table
const fn build_jump_table() -> [InstrHandler; 256] {
    use instr::*;
    let mut table: [InstrHandler; 256] = [invalid; 256];
    table[OP_CONST_VOID as usize] = const_void;
    table[OP_CONST_TRUE as usize] = const_true;
    table[OP_CONST_FALSE as usize] = const_false;
    table[OP_CONST_I32 as usize] = const_i32;
    table[OP_CONST_I64 as usize] = const_i64;
    table[OP_CONST_F32 as usize] = const_f32;
    table[OP_CONST_F64 as usize] = const_f64;
    table[OP_CONST_STR as usize] = const_str;
    table[OP_DROP as usize] = drop1;
    table[OP_DROP_N as usize] = drop_n;
    table[OP_LOAD_PARAM as usize] = load_local;
    table[OP_LOAD_LOCAL as usize] = load_local;
    table[OP_LOAD_GLOBAL as usize] = load_global;
    table[OP_LOAD_NATIVE_PROP as usize] = load_native_prop;
    table[OP_LOAD_FIELD as usize] = load_field;
    table[OP_STORE_LOCAL as usize] = store_local;

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

fn const_void(vm: &mut ScriptVm) -> Result<(), VMError> {
    vm.stack.push(StackValue::Void);
    Ok(())
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
    let len = vm.read_immediate::<u16>() as usize;
    // SAFETY: the compiler writes the string's `len` bytes after its length, from a `&str`.
    let s = unsafe {
        let bytes = core::slice::from_raw_parts(vm.iptr, len);
        vm.iptr = vm.iptr.add(len);
        core::str::from_utf8_unchecked(bytes)
    };
    vm.stack.push(StackValue::String(Arc::new(s.to_string())));
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
    let n = vm.read_immediate::<u16>() as usize;
    let val = vm.stack.get(n).ok_or(VMError::InvalidLocal(n))?.clone();
    vm.stack.push(val);
    Ok(())
}

fn store_local(vm: &mut ScriptVm) -> Result<(), VMError> {
    let n = vm.read_immediate::<u16>() as usize;
    let arg = vm.pop()?;
    *vm.stack.get_mut(n).ok_or(VMError::InvalidLocal(n))? = arg;
    Ok(())
}

fn load_global(vm: &mut ScriptVm) -> Result<(), VMError> {
    let n = vm.read_immediate::<u16>() as usize;
    let val = match vm.host.members.get(n) {
        Some(HostMember::Const(val)) => val.clone(),
        Some(HostMember::GlobalProperty(accessor)) => accessor(vm)?,
        Some(_) => return Err(VMError::InvalidNativeProp(n)),
        None => return Err(VMError::InvalidGlobalIndex(n)),
    };
    vm.stack.push(val);
    Ok(())
}

fn load_native_prop(vm: &mut ScriptVm) -> Result<(), VMError> {
    let arg = vm.pop()?;
    let prop_index = vm.read_immediate::<u16>() as usize;
    let Some(HostMember::InstanceProperty(accessor)) = vm.host.members.get(prop_index) else {
        return Err(VMError::InvalidNativeProp(prop_index));
    };
    let accessor = *accessor;
    let args_start = vm.stack.len();
    let val = accessor(&mut vm.invocation_context(args_start), arg)?;
    vm.stack.push(val);
    Ok(())
}

/// The value of a reflected field that is a primitive.
fn primitive_field(field: &dyn PartialReflect) -> Option<StackValue> {
    if let Some(val) = field.try_downcast_ref::<bool>() {
        Some(StackValue::Bool(*val))
    } else if let Some(val) = field.try_downcast_ref::<i32>() {
        Some(StackValue::I32(*val))
    } else if let Some(val) = field.try_downcast_ref::<i64>() {
        Some(StackValue::I64(*val))
    } else if let Some(val) = field.try_downcast_ref::<f32>() {
        Some(StackValue::F32(*val))
    } else if let Some(val) = field.try_downcast_ref::<f64>() {
        Some(StackValue::F64(*val))
    } else {
        field
            .try_downcast_ref::<Entity>()
            .map(|val| StackValue::Entity(*val))
    }
}

/// The field at `index` of a reflected struct.
fn struct_field(
    reflect: &dyn PartialReflect,
    index: usize,
) -> Result<&dyn PartialReflect, VMError> {
    let ReflectRef::Struct(rstruct) = reflect.reflect_ref() else {
        return Err(VMError::Unsupported("fields of a non-struct"));
    };
    rstruct.field_at(index).ok_or(VMError::InvalidField(index))
}

fn load_field(vm: &mut ScriptVm) -> Result<(), VMError> {
    let arg = vm.pop()?;
    let field_index = vm.read_immediate::<u16>() as usize;
    let val = match arg {
        StackValue::WorldRef(world_index) => {
            let reflect = *vm
                .world_refs
                .get(world_index)
                .ok_or(VMError::InvalidType(ExprType::None))?;
            let field = struct_field(reflect, field_index)?;
            match primitive_field(field) {
                Some(val) => val,
                None => {
                    let index = vm.world_refs.len();
                    vm.world_refs.push(field);
                    StackValue::WorldRef(index)
                }
            }
        }
        StackValue::HeapRef(heap_index) => {
            let reflect = vm
                .heap_refs
                .get(heap_index as usize)
                .ok_or(VMError::InvalidType(ExprType::None))?;
            let field = struct_field(reflect.as_ref(), field_index)?;
            // TODO(X7): composite fields of temporaries, flattened into slots.
            primitive_field(field).ok_or(VMError::Unsupported("composite fields of temporaries"))?
        }
        _ => return Err(VMError::InvalidType(vm.value_type(&arg))),
    };
    vm.stack.push(val);
    Ok(())
}

/// A binary operator on two values of one variant; `$body` gives the result.
macro_rules! binop {
    ($name:ident, $variant:ident, |$l:ident, $r:ident| $body:expr) => {
        fn $name(vm: &mut ScriptVm) -> Result<(), VMError> {
            let rhs = vm.pop()?;
            let lhs = vm.pop()?;
            let (StackValue::$variant($l), StackValue::$variant($r)) = (&lhs, &rhs) else {
                return Err(VMError::MismatchedTypes(
                    vm.value_type(&lhs),
                    vm.value_type(&rhs),
                ));
            };
            let ($l, $r) = (*$l, *$r);
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
                return Err(VMError::InvalidType(vm.value_type(&arg)));
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
    let val = match (&arg, to) {
        (Bool(b), instr::TYPE_I32) => I32(*b as i32),
        (Bool(b), instr::TYPE_I64) => I64(*b as i64),
        (I32(n), instr::TYPE_I32) => I32(*n),
        (I32(n), instr::TYPE_I64) => I64(*n as i64),
        (I32(n), instr::TYPE_F32) => F32(*n as f32),
        (I32(n), instr::TYPE_F64) => F64(*n as f64),
        (I64(n), instr::TYPE_I32) => I32(*n as i32),
        (I64(n), instr::TYPE_I64) => I64(*n),
        (I64(n), instr::TYPE_F32) => F32(*n as f32),
        (I64(n), instr::TYPE_F64) => F64(*n as f64),
        (F32(n), instr::TYPE_I32) => I32(*n as i32),
        (F32(n), instr::TYPE_I64) => I64(*n as i64),
        (F32(n), instr::TYPE_F32) => F32(*n),
        (F32(n), instr::TYPE_F64) => F64(*n as f64),
        (F64(n), instr::TYPE_I32) => I32(*n as i32),
        (F64(n), instr::TYPE_I64) => I64(*n as i64),
        (F64(n), instr::TYPE_F32) => F32(*n as f32),
        (F64(n), instr::TYPE_F64) => F64(*n),
        _ => return Err(VMError::InvalidType(vm.value_type(&arg))),
    };
    vm.stack.push(val);
    Ok(())
}

fn call(vm: &mut ScriptVm) -> Result<(), VMError> {
    let fn_index = vm.read_immediate::<u16>() as usize;
    let num_params = vm.read_immediate::<u16>() as usize;
    let stack_len = vm.stack.len();
    if num_params > stack_len {
        return Err(VMError::StackUnderflow);
    }
    // SAFETY: `run_with` set `module` from a reference that outlives this evaluation.
    let module = unsafe { vm.module.as_ref() }.ok_or(VMError::InvalidFunction(fn_index))?;
    let func = module
        .functions
        .get(fn_index)
        .ok_or(VMError::InvalidFunction(fn_index))?;
    let iptr = func.code.as_ptr();
    let num_locals = func.num_locals;
    let mut new_stack = vm.stack.split_off(stack_len - num_params);
    new_stack.resize(new_stack.len() + num_locals, StackValue::Void);
    vm.push_call_stack(new_stack);
    vm.iptr = iptr;
    Ok(())
}

fn call_host_method(vm: &mut ScriptVm) -> Result<(), VMError> {
    let method_index = vm.read_immediate::<u16>() as usize;
    let num_params = vm.read_immediate::<u16>() as usize;
    let Some(HostMember::InstanceMethod(method)) = vm.host.members.get(method_index) else {
        return Err(VMError::InvalidNativeMethod(method_index));
    };
    call_native(vm, *method, num_params)
}

fn call_host_function(vm: &mut ScriptVm) -> Result<(), VMError> {
    let function_index = vm.read_immediate::<u16>() as usize;
    let num_params = vm.read_immediate::<u16>() as usize;
    let Some(HostMember::Function(f)) = vm.host.members.get(function_index) else {
        return Err(VMError::InvalidGlobalIndex(function_index));
    };
    call_native(vm, *f, num_params)
}

/// Calls a native function on the top `num_params` values, which it replaces with its result.
fn call_native(
    vm: &mut ScriptVm,
    f: crate::host::HostFunction,
    num_params: usize,
) -> Result<(), VMError> {
    let stack_len = vm.stack.len();
    let args_start = stack_len
        .checked_sub(num_params)
        .ok_or(VMError::StackUnderflow)?;
    let val = f(&mut vm.invocation_context(args_start))?;
    vm.stack.truncate(args_start);
    if !val.is_void() {
        vm.stack.push(val);
    }
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
            vm.value_type(&test),
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

    /// Runs `build`'s code, with a `ret` appended.
    fn run_code(build: impl FnOnce(&mut InstructionBuilder)) -> Result<StackValue, VMError> {
        let world = World::new();
        let host = HostState::default();
        let mut tracking = TrackingScope::default();
        let mut builder = InstructionBuilder::default();
        build(&mut builder);
        builder.push_op(instr::OP_RET);
        let code = builder.inner();
        let mut vm = ScriptVm::new(&world, &host, &mut tracking);
        vm.iptr = code.as_ptr();
        vm.start()
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
        let code = builder.inner();
        let mut vm = ScriptVm::new(&world, &host, &mut tracking);
        vm.owner = actor_id;
        vm.iptr = code.as_ptr();
        let result = vm.start().unwrap();
        assert_eq!(result, StackValue::Entity(actor_id));
    }

    fn get_self(vm: &ScriptVm) -> Result<StackValue, VMError> {
        Ok(StackValue::Entity(vm.owner))
    }

    #[test]
    fn test_value_size() {
        assert_eq!(size_of::<StackValue>(), 16);
    }
}
