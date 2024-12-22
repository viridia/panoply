use core::result;

use bevy::{core::Name, scene::ron::de, utils::HashMap};
use wasm_encoder::{
    CodeSection, DataCountSection, DataSection, EntityType, ExportKind, ExportSection, FieldType,
    FuncType, Function, FunctionSection, GlobalType, HeapType, ImportSection, IndirectNameMap,
    Instruction, NameMap, NameSection, RefType, StorageType, TypeSection, ValType,
};
use wasmparser::Element;
use wasmtime::component::types::Field;

use crate::{
    ast::{ASTNode, NodeKind},
    compiler::{CompilationError, CompilationUnit},
    decl::{self, FunctionDecl, LocalDecl, ParamDecl, Scope},
    expr::{Expr, ExprKind},
    oper::BinaryOp,
    types::Type,
};

use super::{assign_types::assign_types, type_inference::TypeInference};

pub struct CodeGenerator {
    type_names: NameMap,
    local_names: IndirectNameMap,
    function_names: NameMap,
    types: TypeSection,
    functions: FunctionSection,
    imports: ImportSection,
    exports: ExportSection,
    codes: CodeSection,
    data: DataSection,
    // elements: ElementSection,
    next_type_index: u32,
    next_data_index: u32,
    params: Vec<ParamDecl>,
    locals: Vec<LocalDecl>,
    type_string: Option<u32>,
}

impl Default for CodeGenerator {
    fn default() -> Self {
        Self {
            type_names: NameMap::new(),
            local_names: IndirectNameMap::new(),
            function_names: NameMap::new(),
            types: TypeSection::new(),
            functions: FunctionSection::new(),
            imports: ImportSection::new(),
            exports: ExportSection::new(),
            codes: CodeSection::new(),
            data: DataSection::new(),
            // elements: ElementSection::new(),
            next_type_index: 0,
            next_data_index: 0,
            params: Vec::new(),
            locals: Vec::new(),
            type_string: None,
        }
    }
}

impl CodeGenerator {
    pub fn next_type_index(&mut self) -> u32 {
        let index = self.next_type_index;
        self.next_type_index += 1;
        index
    }

    pub fn next_data_index(&mut self) -> u32 {
        let index = self.next_data_index;
        self.next_data_index += 1;
        index
    }

    fn gen_val_types(&mut self, typ: &Type, out: &mut Vec<ValType>) {
        match &typ {
            Type::None | Type::Void => {}
            Type::IUnsized | Type::Infer(_) => unreachable!(),
            Type::Boolean => {
                out.push(ValType::I32);
            }
            Type::I32 => {
                out.push(ValType::I32);
            }
            Type::I64 => {
                out.push(ValType::I64);
            }
            Type::F32 => {
                out.push(ValType::F32);
            }
            Type::F64 => {
                out.push(ValType::F64);
            }
            Type::String => {
                out.push(ValType::Ref(RefType {
                    nullable: false,
                    heap_type: HeapType::Concrete(self.get_string_type()),
                }));
            }
            Type::Array(_element) => {
                todo!();
                // out.push(ValType::Ref(RefType {
                //     nullable: false,
                //     heap_type: HeapType::A,
                // }));
                // self.gen_val_types(element, out);
            }
            Type::Struct(stype) => {
                if stype.is_record {
                    out.push(ValType::I32);
                } else {
                    for field in &stype.fields {
                        self.gen_val_types(&field.typ, out);
                    }
                }
            }
            Type::TupleStruct(stype) => {
                if stype.is_record {
                    out.push(ValType::I32);
                } else {
                    for typ in &stype.fields {
                        self.gen_val_types(typ, out);
                    }
                }
            }
            Type::Tuple(members) => {
                for member in members.iter() {
                    self.gen_val_types(member, out);
                }
            }
            Type::Function(_ftype) => {
                todo!();
                // for param in &ftype.params {
                //     self.gen_val_types(param, out);
                // }
                // for ret in &ftype.ret {
                //     self.gen_val_types(ret, out);
                // }
            }
        }
    }

    pub fn get_string_type(&mut self) -> u32 {
        if let Some(index) = self.type_string {
            index
        } else {
            self.types.ty().array(&StorageType::I8, false);
            let s = self.next_type_index();
            self.type_names.append(s, "String");
            self.type_string = Some(s);
            s
        }
    }
}

pub(crate) fn gen_module(unit: &mut CompilationUnit) -> Result<(), CompilationError> {
    let mut generator = CodeGenerator::default();

    for sd in unit.decls.structs.iter() {
        if sd.typ.is_record {
            // let name_str = unit.symbols.resolve(sd.name);
            // let mut fields = vec![];
            // for field in &sd.fields {
            //     fields.push(FieldType {
            //         typ: generator.gen_type(&field.typ),
            //         mutable: field.mutable,
            //     });
            // }
            // let type_index = generator.next_type_index();
            // generator
            //     .type_names
            //     .append(type_index, format!("{}.type", name_str).as_str());
            // generator.types.ty().struct_type(fields);
            // generator
            //     .imports
            //     .import("host", &name_str, EntityType::Struct(type_index));
        }
    }

    for fd in unit.decls.functions.iter() {
        let name_str = unit.symbols.resolve(fd.name);
        let mut ret_types: Vec<ValType> = Vec::new();
        if !fd.typ.ret.is_void() {
            generator.gen_val_types(&fd.typ.ret, &mut ret_types);
        };

        let mut param_types: Vec<ValType> = Vec::new();
        for param in &fd.typ.params {
            generator.gen_val_types(&param.typ, &mut param_types);
        }

        let type_index = generator.next_type_index();
        generator
            .function_names
            .append(fd.function_index as u32, &name_str);
        generator
            .type_names
            .append(type_index, format!("{}.type", name_str).as_str());
        generator.types.ty().function(param_types, ret_types);
        if fd.is_native {
            generator
                .imports
                .import("host", &name_str, EntityType::Function(type_index));
        } else {
            generator.functions.function(type_index);
            if fd.visibility == decl::DeclVisibility::Public {
                generator
                    .exports
                    .export(&name_str, ExportKind::Func, fd.function_index as u32);
            }
            let mut locals: Vec<ValType> = vec![];
            for local in &fd.locals {
                generator.gen_val_types(&local.typ, &mut locals);
            }
            let mut locals_compressed: Vec<(u32, ValType)> = Vec::new();
            if !locals.is_empty() {
                let mut current_type = &locals[0];
                let mut count = 1;
                for local in locals.iter().skip(1) {
                    if local == current_type {
                        count += 1;
                    } else {
                        locals_compressed.push((count, *current_type));
                        current_type = local;
                        count = 1;
                    }
                }
                locals_compressed.push((count, *current_type));
            }
            let mut f = Function::new(locals_compressed);
            generator.params.clone_from(&fd.typ.params);
            generator.locals.clone_from(&fd.locals);
            let mut local_offset = 0;
            for param in generator.params.iter_mut() {
                param.local_index = local_offset;
                local_offset += param.typ.value_count();
            }
            for local in generator.locals.iter_mut() {
                local.local_index = local_offset;
                local_offset += local.typ.value_count();
            }
            gen_expr(unit, &mut generator, &fd.body, &mut f)?;
            if !fd.body.typ.is_void() {
                f.instruction(&Instruction::Return);
            }
            f.instruction(&Instruction::End);
            generator.codes.function(&f);
        }
    }

    let mut names = NameSection::new();
    names.module(unit.filename());

    if !generator.type_names.is_empty() {
        names.types(&generator.type_names);
    }

    names.locals(&generator.local_names);

    if !generator.function_names.is_empty() {
        names.functions(&generator.function_names);
    }

    unit.module.section(&names);
    unit.module.section(&generator.types);
    if !generator.imports.is_empty() {
        unit.module.section(&generator.imports);
    }
    unit.module.section(&generator.functions);
    unit.module.section(&generator.exports);
    if generator.next_data_index > 0 {
        unit.module.section(&DataCountSection {
            count: generator.next_data_index,
        });
    }
    unit.module.section(&generator.codes);
    if generator.next_data_index > 0 {
        unit.module.section(&generator.data);
    }

    println!(
        "{}",
        wasmprinter::print_bytes(unit.module.as_slice()).unwrap()
    );
    wasmparser::validate(unit.module.as_slice()).unwrap();
    Ok(())
}

fn gen_expr<'a>(
    unit: &'a CompilationUnit,
    generator: &mut CodeGenerator,
    expr: &'a Expr,
    out: &mut wasm_encoder::Function,
) -> Result<(), CompilationError> {
    match expr.kind {
        ExprKind::Empty => todo!(),
        ExprKind::ConstBool(value) => {
            match value {
                true => out.instruction(&Instruction::I32Const(1)),
                false => out.instruction(&Instruction::I32Const(0)),
            };
        }
        ExprKind::ConstInteger(value) => {
            match expr.typ {
                Type::I32 => out.instruction(&Instruction::I32Const(value as i32)),
                Type::I64 => out.instruction(&Instruction::I64Const(value)),
                _ => panic!("Invalid integer type: {:?}", expr.typ),
            };
        }
        ExprKind::ConstFloat(value) => {
            match expr.typ {
                Type::F32 => out.instruction(&Instruction::F32Const(value as f32)),
                Type::F64 => out.instruction(&Instruction::F64Const(value)),
                _ => panic!("Invalid float type: {:?}", expr.typ),
            };
        }
        ExprKind::ConstString(symbol) => {
            let string = unit.symbols.resolve(symbol);
            let bytes = string.as_bytes();
            let array_data_index = generator.next_data_index();
            generator.data.passive(bytes.iter().copied());
            out.instruction(&Instruction::I32Const(0));
            out.instruction(&Instruction::I32Const(bytes.len() as i32));
            out.instruction(&Instruction::ArrayNewData {
                array_type_index: generator.get_string_type(),
                array_data_index,
            });
        }
        ExprKind::FunctionRef(index) => {
            panic!("Cannot codegen function reference: {:?}", index);
        }
        ExprKind::ParamRef(index) => {
            let local = &generator.params[index];
            local_get(local.local_index, &local.typ, out);
        }
        ExprKind::LocalRef(index) => {
            let local = &generator.locals[index];
            local_get(local.local_index, &local.typ, out);
        }
        ExprKind::Field(ref base, field_index) => {
            // gen_expr(unit, generator, base, out)?;
            let field = match &base.typ {
                Type::Struct(stype) => &stype.fields[field_index],
                _ => panic!("Invalid field access: {:?}", base.typ),
            };
            match base.kind {
                ExprKind::LocalRef(index) => {
                    let local = &generator.locals[index];
                    local_get(local.local_index + field.index, &field.typ, out);
                }
                _ => todo!(),
            }
        }
        ExprKind::Index(ref base, _field_index) => {
            gen_expr(unit, generator, base, out)?;
            todo!();
        }
        ExprKind::GlobalRef(index) => {
            out.instruction(&Instruction::GlobalGet(index as u32));
        }
        ExprKind::LocalDecl(index, ref init) => {
            if let Some(init) = init {
                gen_expr(unit, generator, init, out)?;
                let local = &generator.locals[index];
                local_set(local.local_index, &local.typ, out);
            }
        }

        ExprKind::BinaryExpr {
            op,
            ref lhs,
            ref rhs,
        } => {
            gen_expr(unit, generator, lhs, out)?;
            gen_expr(unit, generator, rhs, out)?;
            gen_binop(&expr.typ, op, out);
        }

        ExprKind::Assign { ref lhs, ref rhs } => {
            gen_expr(unit, generator, rhs, out)?;
            gen_assign(generator, lhs, None, out)?;
        }

        ExprKind::AssignOp {
            op,
            ref lhs,
            ref rhs,
        } => {
            gen_expr(unit, generator, rhs, out)?;
            gen_assign(generator, lhs, Some(op), out)?;
        }

        ExprKind::UnaryExpr { op, ref arg } => {
            gen_expr(unit, generator, arg, out)?;
            match op {
                crate::oper::UnaryOp::Not => todo!(),
                crate::oper::UnaryOp::Neg => todo!(),
                crate::oper::UnaryOp::BitNot => todo!(),
            }
        }

        ExprKind::Cast(ref arg) => {
            gen_expr(unit, generator, arg, out)?;
            // Convert from arg.typ to expr.typ.
            match (&expr.typ, &arg.typ) {
                (Type::Boolean, Type::I32) => {
                    out.instruction(&Instruction::I32Eqz);
                }
                (Type::Boolean, Type::I64) => {
                    out.instruction(&Instruction::I64Eqz);
                    out.instruction(&Instruction::I32WrapI64);
                }
                (Type::I32, Type::Boolean) => {
                    // Same type, do nothing.
                }
                (Type::I32, Type::I64) => {
                    out.instruction(&Instruction::I32WrapI64);
                }
                (Type::I32, Type::F32) => {
                    out.instruction(&Instruction::I32TruncF32S);
                }
                (Type::I32, Type::F64) => {
                    out.instruction(&Instruction::I32TruncF64S);
                }
                (Type::I64, Type::Boolean) => {
                    out.instruction(&Instruction::I64ExtendI32U);
                }
                (Type::I64, Type::I32) => {
                    out.instruction(&Instruction::I64Extend32S);
                }
                (Type::I64, Type::F32) => {
                    out.instruction(&Instruction::I64TruncF32S);
                }
                (Type::I64, Type::F64) => {
                    out.instruction(&Instruction::I64TruncF64S);
                }
                (Type::F32, Type::I32) => {
                    out.instruction(&Instruction::F32ConvertI32S);
                }
                (Type::F32, Type::I64) => {
                    out.instruction(&Instruction::F32ConvertI64S);
                }
                (Type::F32, Type::F64) => {
                    out.instruction(&Instruction::F32DemoteF64);
                }
                (Type::F64, Type::I32) => {
                    out.instruction(&Instruction::F64ConvertI32S);
                }
                (Type::F64, Type::I64) => {
                    out.instruction(&Instruction::F64ConvertI64S);
                }
                (Type::F64, Type::F32) => {
                    out.instruction(&Instruction::F64PromoteF32);
                }
                _ => panic!("Invalid cast: {:?}", expr),
            }
        }

        ExprKind::Call(ref func, ref args) => {
            for arg in args {
                gen_expr(unit, generator, arg, out)?;
            }
            if let ExprKind::FunctionRef(index) = func.kind {
                // println!("Call function: {}", index);
                out.instruction(&Instruction::Call(index as u32));
            } else {
                panic!("Invalid function reference: {:?}", func);
            }
        }

        ExprKind::Block(ref vec, ref expr) => {
            for stmt in vec {
                gen_expr(unit, generator, stmt, out)?;
                match stmt.typ {
                    Type::Void | Type::None => {}
                    _ => {
                        out.instruction(&Instruction::Drop);
                    }
                }
            }

            if let Some(expr) = expr {
                gen_expr(unit, generator, expr, out)?;
            }
        }
    }

    Ok(())
}

fn local_get(mut local_index: usize, typ: &Type, out: &mut wasm_encoder::Function) {
    match typ {
        Type::None | Type::Void => {}
        Type::IUnsized | Type::Infer(_) => unreachable!(),
        Type::Boolean | Type::I32 | Type::I64 | Type::F32 | Type::F64 => {
            out.instruction(&Instruction::LocalGet(local_index as u32));
        }
        Type::String => {
            out.instruction(&Instruction::LocalGet(local_index as u32));
        }
        Type::Array(_element) => {
            todo!();
        }
        Type::Struct(stype) => {
            if stype.is_record {
                out.instruction(&Instruction::LocalGet(local_index as u32));
            } else {
                for field in &stype.fields {
                    local_get(local_index, &field.typ, out);
                    local_index += field.typ.value_count();
                }
            }
        }
        Type::TupleStruct(stype) => {
            if stype.is_record {
                out.instruction(&Instruction::LocalGet(local_index as u32));
            } else {
                for typ in &stype.fields {
                    local_get(local_index, typ, out);
                    local_index += typ.value_count();
                }
            }
        }
        Type::Tuple(members) => {
            for member in members.iter() {
                local_get(local_index, member, out);
                local_index += member.value_count();
            }
        }
        Type::Function(_ftype) => {
            todo!();
        }
    }
}

fn local_set(mut local_index: usize, typ: &Type, out: &mut wasm_encoder::Function) {
    match typ {
        Type::None | Type::Void => {}
        Type::IUnsized | Type::Infer(_) => unreachable!(),
        Type::Boolean | Type::I32 | Type::I64 | Type::F32 | Type::F64 => {
            out.instruction(&Instruction::LocalSet(local_index as u32));
        }
        Type::String => {
            out.instruction(&Instruction::LocalSet(local_index as u32));
        }
        Type::Array(_element) => {
            todo!();
        }
        Type::Struct(stype) => {
            if stype.is_record {
                out.instruction(&Instruction::LocalSet(local_index as u32));
            } else {
                for field in stype.fields.iter().rev() {
                    local_set(local_index, &field.typ, out);
                    local_index += field.typ.value_count();
                }
            }
        }
        Type::TupleStruct(stype) => {
            if stype.is_record {
                out.instruction(&Instruction::LocalSet(local_index as u32));
            } else {
                for typ in stype.fields.iter().rev() {
                    local_set(local_index, typ, out);
                    local_index += typ.value_count();
                }
            }
        }
        Type::Tuple(members) => {
            for member in members.iter().rev() {
                local_set(local_index, member, out);
                local_index += member.value_count();
            }
        }
        Type::Function(_ftype) => {
            todo!();
        }
    }
}

fn gen_assign(
    generator: &mut CodeGenerator,
    expr: &Expr,
    op: Option<BinaryOp>,
    out: &mut wasm_encoder::Function,
) -> Result<(), CompilationError> {
    match expr.kind {
        ExprKind::LocalRef(index) => {
            let local = &generator.locals[index];
            if let Some(bop) = op {
                local_get(local.local_index, &local.typ, out);
                gen_binop(&local.typ, bop, out);
            }
            local_set(local.local_index, &local.typ, out);
        }
        ExprKind::GlobalRef(_index) => todo!(),
        ExprKind::Field(ref _base, _field_index) => todo!(),
        ExprKind::Index(ref _expr, _) => todo!(),
        _ => return Err(CompilationError::InvalidAssignmentTarget(expr.location)),
    }

    Ok(())
}

fn gen_binop(typ: &Type, op: BinaryOp, out: &mut wasm_encoder::Function) {
    match op {
        crate::oper::BinaryOp::Add => {
            match typ {
                Type::I32 => out.instruction(&Instruction::I32Add),
                Type::I64 => out.instruction(&Instruction::I64Add),
                Type::F32 => out.instruction(&Instruction::F32Add),
                Type::F64 => out.instruction(&Instruction::F64Add),
                _ => panic!("Invalid type for binary addition: {:?}", typ),
            };
        }
        crate::oper::BinaryOp::Sub => {
            match typ {
                Type::I32 => out.instruction(&Instruction::I32Sub),
                Type::I64 => out.instruction(&Instruction::I64Sub),
                Type::F32 => out.instruction(&Instruction::F32Sub),
                Type::F64 => out.instruction(&Instruction::F64Sub),
                _ => panic!("Invalid type for binary subtraction: {:?}", typ),
            };
        }
        crate::oper::BinaryOp::Mul => {
            match typ {
                Type::I32 => out.instruction(&Instruction::I32Mul),
                Type::I64 => out.instruction(&Instruction::I64Mul),
                Type::F32 => out.instruction(&Instruction::F32Mul),
                Type::F64 => out.instruction(&Instruction::F64Mul),
                _ => panic!("Invalid type for binary multiplication: {:?}", typ),
            };
        }
        crate::oper::BinaryOp::Div => {
            match typ {
                Type::I32 => out.instruction(&Instruction::I32DivS),
                Type::I64 => out.instruction(&Instruction::I64DivS),
                Type::F32 => out.instruction(&Instruction::F32Div),
                Type::F64 => out.instruction(&Instruction::F64Div),
                _ => panic!("Invalid type for binary division: {:?}", typ),
            };
        }
        crate::oper::BinaryOp::Mod => {
            match typ {
                Type::I32 => out.instruction(&Instruction::I32RemS),
                Type::I64 => out.instruction(&Instruction::I64RemS),
                _ => panic!("Invalid type for binary modulo: {:?}", typ),
            };
        }
        crate::oper::BinaryOp::LogAnd => {
            // TODO: Short-circuiting.
            match typ {
                Type::Boolean => out.instruction(&Instruction::I32And),
                _ => panic!("Invalid type for logical AND: {:?}", typ),
            };
        }
        crate::oper::BinaryOp::LogOr => {
            // TODO: Short-circuiting.
            match typ {
                Type::Boolean => out.instruction(&Instruction::I32Or),
                _ => panic!("Invalid type for logical OR: {:?}", typ),
            };
        }
        crate::oper::BinaryOp::BitAnd => {
            match typ {
                Type::I32 => out.instruction(&Instruction::I32And),
                Type::I64 => out.instruction(&Instruction::I64And),
                _ => panic!("Invalid type for bitwise AND: {:?}", typ),
            };
        }
        crate::oper::BinaryOp::BitOr => {
            match typ {
                Type::I32 => out.instruction(&Instruction::I32Or),
                Type::I64 => out.instruction(&Instruction::I64Or),
                _ => panic!("Invalid type for bitwise OR: {:?}", typ),
            };
        }
        crate::oper::BinaryOp::BitXor => {
            match typ {
                Type::I32 => out.instruction(&Instruction::I32Xor),
                Type::I64 => out.instruction(&Instruction::I64Xor),
                _ => panic!("Invalid type for bitwise XOR: {:?}", typ),
            };
        }
        crate::oper::BinaryOp::Shl => {
            match typ {
                Type::I32 => out.instruction(&Instruction::I32Shl),
                Type::I64 => out.instruction(&Instruction::I64Shl),
                _ => panic!("Invalid type for bitwise shift left: {:?}", typ),
            };
        }
        crate::oper::BinaryOp::Shr => {
            match typ {
                Type::I32 => out.instruction(&Instruction::I32ShrS),
                Type::I64 => out.instruction(&Instruction::I64ShrS),
                _ => panic!("Invalid type for bitwise shift right: {:?}", typ),
            };
        }
        crate::oper::BinaryOp::Eq => {
            match typ {
                Type::I32 => out.instruction(&Instruction::I32Eq),
                Type::I64 => out.instruction(&Instruction::I64Eq),
                Type::F32 => out.instruction(&Instruction::F32Eq),
                Type::F64 => out.instruction(&Instruction::F64Eq),
                _ => panic!("Invalid type for equality comparison: {:?}", typ),
            };
        }
        crate::oper::BinaryOp::Ne => {
            match typ {
                Type::I32 => out.instruction(&Instruction::I32Ne),
                Type::I64 => out.instruction(&Instruction::I64Ne),
                Type::F32 => out.instruction(&Instruction::F32Ne),
                Type::F64 => out.instruction(&Instruction::F64Ne),
                _ => panic!("Invalid type for inequality comparison: {:?}", typ),
            };
        }
        crate::oper::BinaryOp::Lt => {
            match typ {
                Type::I32 => out.instruction(&Instruction::I32LtS),
                Type::I64 => out.instruction(&Instruction::I64LtS),
                Type::F32 => out.instruction(&Instruction::F32Lt),
                Type::F64 => out.instruction(&Instruction::F64Lt),
                _ => panic!("Invalid type for less-than comparison: {:?}", typ),
            };
        }
        crate::oper::BinaryOp::Le => {
            match typ {
                Type::I32 => out.instruction(&Instruction::I32LeS),
                Type::I64 => out.instruction(&Instruction::I64LeS),
                Type::F32 => out.instruction(&Instruction::F32Le),
                Type::F64 => out.instruction(&Instruction::F64Le),
                _ => panic!("Invalid type for less-than-or-equal comparison: {:?}", typ),
            };
        }
        crate::oper::BinaryOp::Gt => {
            match typ {
                Type::I32 => out.instruction(&Instruction::I32GtS),
                Type::I64 => out.instruction(&Instruction::I64GtS),
                Type::F32 => out.instruction(&Instruction::F32Gt),
                Type::F64 => out.instruction(&Instruction::F64Gt),
                _ => panic!("Invalid type for greater-than comparison: {:?}", typ),
            };
        }
        crate::oper::BinaryOp::Ge => {
            match typ {
                Type::I32 => out.instruction(&Instruction::I32GeS),
                Type::I64 => out.instruction(&Instruction::I64GeS),
                Type::F32 => out.instruction(&Instruction::F32Ge),
                Type::F64 => out.instruction(&Instruction::F64Ge),
                _ => panic!(
                    "Invalid type for greater-than-or-equal comparison: {:?}",
                    typ
                ),
            };
        }
    }
}
