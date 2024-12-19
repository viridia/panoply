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
    decl::{self, FunctionDecl, Scope},
    expr::{Expr, ExprKind},
    types::Type,
};

use super::{
    assign_types::assign_types, resolve_types::resolve_types, type_inference::TypeInference,
};

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
    local_offset: u32,
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
            local_offset: 0,
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

    fn gen_type(&mut self, typ: &Type) -> ValType {
        match &typ {
            Type::Boolean => ValType::I32,
            Type::I32 => ValType::I32,
            Type::I64 => ValType::I64,
            Type::F32 => ValType::F32,
            Type::F64 => ValType::F64,
            Type::String => ValType::Ref(RefType {
                nullable: false,
                heap_type: HeapType::Concrete(self.get_string_type()),
            }),
            Type::Tuple(members) => todo!(),
            Type::Array(element) => todo!(),
            Type::Function(ftype) => todo!(),
            _ => panic!("Invalid type for code generation: {:?}", typ),
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

    for fd in unit.decls.functions.iter() {
        let name_str = unit.symbols.resolve(fd.name);
        let ret_types = if !fd.typ.ret.is_void() {
            vec![generator.gen_type(&fd.typ.ret)]
        } else {
            vec![]
        };

        let param_types = fd
            .typ
            .params
            .iter()
            .map(|p| generator.gen_type(&p.typ))
            .collect::<Vec<_>>();

        let type_index = generator.next_type_index();
        generator.function_names.append(fd.index as u32, &name_str);
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
                    .export(&name_str, ExportKind::Func, fd.index as u32);
            }
            let mut locals = vec![];
            for local in &fd.locals {
                locals.push((1, generator.gen_type(&local.typ)));
            }
            let mut f = Function::new(locals);
            generator.local_offset = fd.typ.params.len() as u32;
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
    match &expr.kind {
        ExprKind::Empty => todo!(),
        ExprKind::ConstBool(value) => {
            match value {
                true => out.instruction(&Instruction::I32Const(1)),
                false => out.instruction(&Instruction::I32Const(0)),
            };
        }
        ExprKind::ConstInteger(value) => {
            match expr.typ {
                Type::I32 => out.instruction(&Instruction::I32Const(*value as i32)),
                Type::I64 => out.instruction(&Instruction::I64Const(*value)),
                _ => panic!("Invalid integer type: {:?}", expr.typ),
            };
        }
        ExprKind::ConstFloat(value) => {
            match expr.typ {
                Type::F32 => out.instruction(&Instruction::F32Const(*value as f32)),
                Type::F64 => out.instruction(&Instruction::F64Const(*value)),
                _ => panic!("Invalid float type: {:?}", expr.typ),
            };
        }
        ExprKind::ConstString(symbol) => {
            let string = unit.symbols.resolve(*symbol);
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
            out.instruction(&Instruction::LocalGet(*index as u32));
        }
        ExprKind::LocalRef(index) => {
            out.instruction(&Instruction::LocalGet(
                *index as u32 + generator.local_offset,
            ));
        }
        ExprKind::LocalDecl(index, init) => {
            if let Some(init) = init {
                gen_expr(unit, generator, init, out)?;
                out.instruction(&Instruction::LocalSet(
                    *index as u32 + generator.local_offset,
                ));
            }
        }
        ExprKind::BinaryExpr { op, lhs, rhs } => {
            gen_expr(unit, generator, lhs, out)?;
            gen_expr(unit, generator, rhs, out)?;
            match op {
                crate::oper::BinaryOp::Add => {
                    match expr.typ {
                        Type::I32 => out.instruction(&Instruction::I32Add),
                        Type::I64 => out.instruction(&Instruction::I64Add),
                        Type::F32 => out.instruction(&Instruction::F32Add),
                        Type::F64 => out.instruction(&Instruction::F64Add),
                        _ => panic!("Invalid type for binary addition: {:?}", expr.typ),
                    };
                }
                crate::oper::BinaryOp::Sub => {
                    match expr.typ {
                        Type::I32 => out.instruction(&Instruction::I32Sub),
                        Type::I64 => out.instruction(&Instruction::I64Sub),
                        Type::F32 => out.instruction(&Instruction::F32Sub),
                        Type::F64 => out.instruction(&Instruction::F64Sub),
                        _ => panic!("Invalid type for binary subtraction: {:?}", expr.typ),
                    };
                }
                crate::oper::BinaryOp::Mul => {
                    match expr.typ {
                        Type::I32 => out.instruction(&Instruction::I32Mul),
                        Type::I64 => out.instruction(&Instruction::I64Mul),
                        Type::F32 => out.instruction(&Instruction::F32Mul),
                        Type::F64 => out.instruction(&Instruction::F64Mul),
                        _ => panic!("Invalid type for binary multiplication: {:?}", expr.typ),
                    };
                }
                crate::oper::BinaryOp::Div => {
                    match expr.typ {
                        Type::I32 => out.instruction(&Instruction::I32DivS),
                        Type::I64 => out.instruction(&Instruction::I64DivS),
                        Type::F32 => out.instruction(&Instruction::F32Div),
                        Type::F64 => out.instruction(&Instruction::F64Div),
                        _ => panic!("Invalid type for binary division: {:?}", expr.typ),
                    };
                }
                crate::oper::BinaryOp::Mod => {
                    match expr.typ {
                        Type::I32 => out.instruction(&Instruction::I32RemS),
                        Type::I64 => out.instruction(&Instruction::I64RemS),
                        _ => panic!("Invalid type for binary modulo: {:?}", expr.typ),
                    };
                }
                crate::oper::BinaryOp::LogAnd => {
                    // TODO: Short-circuiting.
                    match expr.typ {
                        Type::Boolean => out.instruction(&Instruction::I32And),
                        _ => panic!("Invalid type for logical AND: {:?}", expr.typ),
                    };
                }
                crate::oper::BinaryOp::LogOr => {
                    // TODO: Short-circuiting.
                    match expr.typ {
                        Type::Boolean => out.instruction(&Instruction::I32Or),
                        _ => panic!("Invalid type for logical OR: {:?}", expr.typ),
                    };
                }
                crate::oper::BinaryOp::BitAnd => {
                    match expr.typ {
                        Type::I32 => out.instruction(&Instruction::I32And),
                        Type::I64 => out.instruction(&Instruction::I64And),
                        _ => panic!("Invalid type for bitwise AND: {:?}", expr.typ),
                    };
                }
                crate::oper::BinaryOp::BitOr => {
                    match expr.typ {
                        Type::I32 => out.instruction(&Instruction::I32Or),
                        Type::I64 => out.instruction(&Instruction::I64Or),
                        _ => panic!("Invalid type for bitwise OR: {:?}", expr.typ),
                    };
                }
                crate::oper::BinaryOp::BitXor => {
                    match expr.typ {
                        Type::I32 => out.instruction(&Instruction::I32Xor),
                        Type::I64 => out.instruction(&Instruction::I64Xor),
                        _ => panic!("Invalid type for bitwise XOR: {:?}", expr.typ),
                    };
                }
                crate::oper::BinaryOp::Shl => {
                    match expr.typ {
                        Type::I32 => out.instruction(&Instruction::I32Shl),
                        Type::I64 => out.instruction(&Instruction::I64Shl),
                        _ => panic!("Invalid type for bitwise shift left: {:?}", expr.typ),
                    };
                }
                crate::oper::BinaryOp::Shr => {
                    match expr.typ {
                        Type::I32 => out.instruction(&Instruction::I32ShrS),
                        Type::I64 => out.instruction(&Instruction::I64ShrS),
                        _ => panic!("Invalid type for bitwise shift right: {:?}", expr.typ),
                    };
                }
                crate::oper::BinaryOp::Eq => {
                    match expr.typ {
                        Type::I32 => out.instruction(&Instruction::I32Eq),
                        Type::I64 => out.instruction(&Instruction::I64Eq),
                        Type::F32 => out.instruction(&Instruction::F32Eq),
                        Type::F64 => out.instruction(&Instruction::F64Eq),
                        _ => panic!("Invalid type for equality comparison: {:?}", expr.typ),
                    };
                }
                crate::oper::BinaryOp::Ne => {
                    match expr.typ {
                        Type::I32 => out.instruction(&Instruction::I32Ne),
                        Type::I64 => out.instruction(&Instruction::I64Ne),
                        Type::F32 => out.instruction(&Instruction::F32Ne),
                        Type::F64 => out.instruction(&Instruction::F64Ne),
                        _ => panic!("Invalid type for inequality comparison: {:?}", expr.typ),
                    };
                }
                crate::oper::BinaryOp::Lt => {
                    match expr.typ {
                        Type::I32 => out.instruction(&Instruction::I32LtS),
                        Type::I64 => out.instruction(&Instruction::I64LtS),
                        Type::F32 => out.instruction(&Instruction::F32Lt),
                        Type::F64 => out.instruction(&Instruction::F64Lt),
                        _ => panic!("Invalid type for less-than comparison: {:?}", expr.typ),
                    };
                }
                crate::oper::BinaryOp::Le => {
                    match expr.typ {
                        Type::I32 => out.instruction(&Instruction::I32LeS),
                        Type::I64 => out.instruction(&Instruction::I64LeS),
                        Type::F32 => out.instruction(&Instruction::F32Le),
                        Type::F64 => out.instruction(&Instruction::F64Le),
                        _ => panic!(
                            "Invalid type for less-than-or-equal comparison: {:?}",
                            expr.typ
                        ),
                    };
                }
                crate::oper::BinaryOp::Gt => {
                    match expr.typ {
                        Type::I32 => out.instruction(&Instruction::I32GtS),
                        Type::I64 => out.instruction(&Instruction::I64GtS),
                        Type::F32 => out.instruction(&Instruction::F32Gt),
                        Type::F64 => out.instruction(&Instruction::F64Gt),
                        _ => panic!("Invalid type for greater-than comparison: {:?}", expr.typ),
                    };
                }
                crate::oper::BinaryOp::Ge => {
                    match expr.typ {
                        Type::I32 => out.instruction(&Instruction::I32GeS),
                        Type::I64 => out.instruction(&Instruction::I64GeS),
                        Type::F32 => out.instruction(&Instruction::F32Ge),
                        Type::F64 => out.instruction(&Instruction::F64Ge),
                        _ => panic!(
                            "Invalid type for greater-than-or-equal comparison: {:?}",
                            expr.typ
                        ),
                    };
                }
            }
        }

        ExprKind::Cast(arg) => {
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

        ExprKind::Call(func, args) => {
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

        ExprKind::Block(vec, expr) => {
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
