//! Opcodes, the bytecode writer and the disassembler. Immediates follow their opcode unaligned.

use core::fmt::Write;

// Constants
pub const OP_CONST_VOID: u8 = 2; // Takes up a stack slot but represents no value.
pub const OP_CONST_TRUE: u8 = 3;
pub const OP_CONST_FALSE: u8 = 4;
pub const OP_CONST_I32: u8 = 5; // (imm i32)
pub const OP_CONST_I64: u8 = 6; // (imm i64)
pub const OP_CONST_F32: u8 = 7; // (imm f32)
pub const OP_CONST_F64: u8 = 8; // (imm f64)
pub const OP_CONST_STR: u8 = 9; // (imm u16 length, [bytes])

// Stack
pub const OP_DROP: u8 = 10;
pub const OP_DROP_N: u8 = 11; // (imm u32)

// Loads and stores
pub const OP_LOAD_PARAM: u8 = 20; // (imm u16)
pub const OP_LOAD_LOCAL: u8 = 21; // (imm u16)
pub const OP_LOAD_GLOBAL: u8 = 22; // (imm u16)
pub const OP_LOAD_NATIVE_PROP: u8 = 24; // (imm u16, consumes TOS)
pub const OP_LOAD_FIELD: u8 = 25; // (imm u16, consumes TOS)
pub const OP_STORE_LOCAL: u8 = 26; // (imm u16)

// Unary operators: consume TOS and push the result.
pub const OP_NOT: u8 = 50;
pub const OP_NEG_I32: u8 = 51;
pub const OP_NEG_I64: u8 = 52;
pub const OP_NEG_F32: u8 = 53;
pub const OP_NEG_F64: u8 = 54;
pub const OP_BIT_NOT_I32: u8 = 55;
pub const OP_BIT_NOT_I64: u8 = 56;
/// Converts TOS between primitive types. (imm u8 from, u8 to: `TYPE_*`)
pub const OP_CONVERT: u8 = 57;

// Control flow
pub const OP_BRANCH: u8 = 62; // (imm i32 relative offset)
pub const OP_BRANCH_IF_TRUE: u8 = 63; // (imm i32 relative offset, consumes TOS)
pub const OP_BRANCH_IF_FALSE: u8 = 64; // (imm i32 relative offset, consumes TOS)

// Calls
pub const OP_CALL: u8 = 70; // Call script function (imm u16 index, u16 arguments)
pub const OP_CALL_HOST_METHOD: u8 = 71; // Call method on native type (imm u16, u16)
pub const OP_CALL_HOST_FUNCTION: u8 = 72; // Call host function (imm u16, u16)
pub const OP_RET: u8 = 74; // transfers value from child stack to parent

// Binary operators: consume two values and push the result.
pub const OP_ADD_I32: u8 = 100;
pub const OP_ADD_I64: u8 = 101;
pub const OP_ADD_F32: u8 = 102;
pub const OP_ADD_F64: u8 = 103;

pub const OP_SUB_I32: u8 = 104;
pub const OP_SUB_I64: u8 = 105;
pub const OP_SUB_F32: u8 = 106;
pub const OP_SUB_F64: u8 = 107;

pub const OP_MUL_I32: u8 = 108;
pub const OP_MUL_I64: u8 = 109;
pub const OP_MUL_F32: u8 = 110;
pub const OP_MUL_F64: u8 = 111;

pub const OP_DIV_I32: u8 = 112;
pub const OP_DIV_I64: u8 = 113;
pub const OP_DIV_F32: u8 = 114;
pub const OP_DIV_F64: u8 = 115;

pub const OP_REM_I32: u8 = 116;
pub const OP_REM_I64: u8 = 117;
pub const OP_REM_F32: u8 = 118;
pub const OP_REM_F64: u8 = 119;

pub const OP_BIT_AND_I32: u8 = 122;
pub const OP_BIT_AND_I64: u8 = 123;
pub const OP_BIT_OR_I32: u8 = 124;
pub const OP_BIT_OR_I64: u8 = 125;
pub const OP_BIT_XOR_I32: u8 = 126;
pub const OP_BIT_XOR_I64: u8 = 127;
pub const OP_SHL_I32: u8 = 128;
pub const OP_SHL_I64: u8 = 129;
pub const OP_SHR_I32: u8 = 130;
pub const OP_SHR_I64: u8 = 131;

pub const OP_EQ_I32: u8 = 150;
pub const OP_EQ_I64: u8 = 151;
pub const OP_EQ_F32: u8 = 152;
pub const OP_EQ_F64: u8 = 153;

pub const OP_NE_I32: u8 = 154;
pub const OP_NE_I64: u8 = 155;
pub const OP_NE_F32: u8 = 156;
pub const OP_NE_F64: u8 = 157;

pub const OP_LT_I32: u8 = 158;
pub const OP_LT_I64: u8 = 159;
pub const OP_LT_F32: u8 = 160;
pub const OP_LT_F64: u8 = 161;

pub const OP_LE_I32: u8 = 162;
pub const OP_LE_I64: u8 = 163;
pub const OP_LE_F32: u8 = 164;
pub const OP_LE_F64: u8 = 165;

pub const OP_GT_I32: u8 = 166;
pub const OP_GT_I64: u8 = 167;
pub const OP_GT_F32: u8 = 168;
pub const OP_GT_F64: u8 = 169;

pub const OP_GE_I32: u8 = 170;
pub const OP_GE_I64: u8 = 171;
pub const OP_GE_F32: u8 = 172;
pub const OP_GE_F64: u8 = 173;

pub const OP_EQ_BOOL: u8 = 174;
pub const OP_NE_BOOL: u8 = 175;

// Type tags for `OP_CONVERT`.
pub const TYPE_BOOL: u8 = 0;
pub const TYPE_I32: u8 = 1;
pub const TYPE_I64: u8 = 2;
pub const TYPE_F32: u8 = 3;
pub const TYPE_F64: u8 = 4;

/// The mnemonic of an opcode that takes no immediates.
fn simple_name(op: u8) -> Option<&'static str> {
    Some(match op {
        OP_CONST_VOID => "void",
        OP_CONST_TRUE => "const true",
        OP_CONST_FALSE => "const false",
        OP_DROP => "drop",
        OP_NOT => "not",
        OP_NEG_I32 => "neg.i32",
        OP_NEG_I64 => "neg.i64",
        OP_NEG_F32 => "neg.f32",
        OP_NEG_F64 => "neg.f64",
        OP_BIT_NOT_I32 => "bit_not.i32",
        OP_BIT_NOT_I64 => "bit_not.i64",
        OP_RET => "ret",
        OP_ADD_I32 => "add.i32",
        OP_ADD_I64 => "add.i64",
        OP_ADD_F32 => "add.f32",
        OP_ADD_F64 => "add.f64",
        OP_SUB_I32 => "sub.i32",
        OP_SUB_I64 => "sub.i64",
        OP_SUB_F32 => "sub.f32",
        OP_SUB_F64 => "sub.f64",
        OP_MUL_I32 => "mul.i32",
        OP_MUL_I64 => "mul.i64",
        OP_MUL_F32 => "mul.f32",
        OP_MUL_F64 => "mul.f64",
        OP_DIV_I32 => "div.i32",
        OP_DIV_I64 => "div.i64",
        OP_DIV_F32 => "div.f32",
        OP_DIV_F64 => "div.f64",
        OP_REM_I32 => "rem.i32",
        OP_REM_I64 => "rem.i64",
        OP_REM_F32 => "rem.f32",
        OP_REM_F64 => "rem.f64",
        OP_BIT_AND_I32 => "bit_and.i32",
        OP_BIT_AND_I64 => "bit_and.i64",
        OP_BIT_OR_I32 => "bit_or.i32",
        OP_BIT_OR_I64 => "bit_or.i64",
        OP_BIT_XOR_I32 => "bit_xor.i32",
        OP_BIT_XOR_I64 => "bit_xor.i64",
        OP_SHL_I32 => "shl.i32",
        OP_SHL_I64 => "shl.i64",
        OP_SHR_I32 => "shr.i32",
        OP_SHR_I64 => "shr.i64",
        OP_EQ_I32 => "eq.i32",
        OP_EQ_I64 => "eq.i64",
        OP_EQ_F32 => "eq.f32",
        OP_EQ_F64 => "eq.f64",
        OP_EQ_BOOL => "eq.bool",
        OP_NE_I32 => "ne.i32",
        OP_NE_I64 => "ne.i64",
        OP_NE_F32 => "ne.f32",
        OP_NE_F64 => "ne.f64",
        OP_NE_BOOL => "ne.bool",
        OP_LT_I32 => "lt.i32",
        OP_LT_I64 => "lt.i64",
        OP_LT_F32 => "lt.f32",
        OP_LT_F64 => "lt.f64",
        OP_LE_I32 => "le.i32",
        OP_LE_I64 => "le.i64",
        OP_LE_F32 => "le.f32",
        OP_LE_F64 => "le.f64",
        OP_GT_I32 => "gt.i32",
        OP_GT_I64 => "gt.i64",
        OP_GT_F32 => "gt.f32",
        OP_GT_F64 => "gt.f64",
        OP_GE_I32 => "ge.i32",
        OP_GE_I64 => "ge.i64",
        OP_GE_F32 => "ge.f32",
        OP_GE_F64 => "ge.f64",
        _ => return None,
    })
}

/// Writes bytecode.
#[derive(Default)]
pub struct InstructionBuilder {
    code: Vec<u8>,
}

impl InstructionBuilder {
    /// Write an instruction opcode (8 bits)
    pub fn push_op(&mut self, op: u8) {
        self.code.push(op);
    }

    /// Write an immediate operand.
    pub fn push_immediate<T: Copy>(&mut self, val: T) {
        let offset = self.code.len();
        self.code.resize(offset + size_of::<T>(), 0);
        // SAFETY: the buffer was just extended to hold `T` at `offset`, and the write is
        // unaligned.
        unsafe {
            let dst = self.code.as_mut_ptr().add(offset) as *mut T;
            core::ptr::write_unaligned(dst, val);
        }
    }

    /// Write a string's bytes as an immediate value.
    pub fn push_immediate_string(&mut self, str: &str) {
        self.code.extend_from_slice(str.as_bytes());
    }

    /// Reserve space for an immediate operand, to be patched later; returns its offset.
    pub fn reserve_immediate<T>(&mut self) -> usize {
        let offset = self.code.len();
        self.code.resize(offset + size_of::<T>(), 0);
        offset
    }

    /// The offset of the next instruction.
    pub fn position(&self) -> usize {
        self.code.len()
    }

    /// Writes a branch's target into the immediate reserved at `from`, as an offset relative to
    /// the end of that immediate: the branch reads its immediate before jumping.
    pub fn patch_branch_target(&mut self, from: usize, to: usize) {
        let end = from + size_of::<i32>();
        let offset = (to as i64 - end as i64) as i32;
        self.code[from..end].copy_from_slice(&offset.to_ne_bytes());
    }

    /// Return the generated opcode stream, consuming self.
    pub fn inner(self) -> Vec<u8> {
        self.code
    }
}

/// Reads bytecode, for the disassembler.
struct InstructionReader<'a> {
    code: &'a [u8],
    offset: usize,
}

impl<'a> InstructionReader<'a> {
    fn at_end(&self) -> bool {
        self.offset >= self.code.len()
    }

    /// The next `n` bytes, or `None` past the end.
    fn take(&mut self, n: usize) -> Option<&'a [u8]> {
        let bytes = self.code.get(self.offset..self.offset + n)?;
        self.offset += n;
        Some(bytes)
    }

    fn read<const N: usize>(&mut self) -> Option<[u8; N]> {
        self.take(N)?.try_into().ok()
    }

    fn u8(&mut self) -> Option<u8> {
        Some(self.read::<1>()?[0])
    }
    fn u16(&mut self) -> Option<u16> {
        Some(u16::from_ne_bytes(self.read()?))
    }
    fn u32(&mut self) -> Option<u32> {
        Some(u32::from_ne_bytes(self.read()?))
    }
    fn i32(&mut self) -> Option<i32> {
        Some(i32::from_ne_bytes(self.read()?))
    }
    fn i64(&mut self) -> Option<i64> {
        Some(i64::from_ne_bytes(self.read()?))
    }
    fn f32(&mut self) -> Option<f32> {
        Some(f32::from_ne_bytes(self.read()?))
    }
    fn f64(&mut self) -> Option<f64> {
        Some(f64::from_ne_bytes(self.read()?))
    }
}

/// A disassembly of `code`, one instruction a line, for debugging.
pub fn disassemble(code: &[u8]) -> String {
    let mut out = String::new();
    let mut reader = InstructionReader { code, offset: 0 };
    while !reader.at_end() {
        let offset = reader.offset;
        if disassemble_one(&mut reader, offset, &mut out).is_none() {
            let _ = writeln!(out, "{offset:>4}: truncated instruction");
            break;
        }
    }
    out
}

fn disassemble_one(reader: &mut InstructionReader, offset: usize, out: &mut String) -> Option<()> {
    let op = reader.u8()?;
    let line = if let Some(name) = simple_name(op) {
        name.to_string()
    } else {
        match op {
            OP_CONST_I32 => format!("const i32 {}", reader.i32()?),
            OP_CONST_I64 => format!("const i64 {}", reader.i64()?),
            OP_CONST_F32 => format!("const f32 {}", reader.f32()?),
            OP_CONST_F64 => format!("const f64 {}", reader.f64()?),
            OP_CONST_STR => {
                let len = reader.u16()? as usize;
                let bytes = reader.take(len)?;
                format!("const str {:?}", String::from_utf8_lossy(bytes))
            }
            OP_DROP_N => format!("drop {}", reader.u32()?),
            OP_LOAD_PARAM => format!("load param {}", reader.u16()?),
            OP_LOAD_LOCAL => format!("load local {}", reader.u16()?),
            OP_LOAD_GLOBAL => format!("load global {}", reader.u16()?),
            OP_LOAD_NATIVE_PROP => format!("load native prop {}", reader.u16()?),
            OP_LOAD_FIELD => format!("load field {}", reader.u16()?),
            OP_STORE_LOCAL => format!("store local {}", reader.u16()?),
            OP_CONVERT => format!("convert {} -> {}", reader.u8()?, reader.u8()?),
            OP_BRANCH | OP_BRANCH_IF_TRUE | OP_BRANCH_IF_FALSE => {
                let name = match op {
                    OP_BRANCH => "branch",
                    OP_BRANCH_IF_TRUE => "branch_if_true",
                    _ => "branch_if_false",
                };
                let relative = reader.i32()?;
                let target = reader.offset as i64 + relative as i64;
                format!("{name} {target} [{relative:+}]")
            }
            OP_CALL | OP_CALL_HOST_METHOD | OP_CALL_HOST_FUNCTION => {
                let name = match op {
                    OP_CALL => "call module",
                    OP_CALL_HOST_METHOD => "call host method",
                    _ => "call host function",
                };
                format!("{name}[{}] {}", reader.u16()?, reader.u16()?)
            }
            _ => format!("invalid instruction {op}"),
        }
    };
    let _ = writeln!(out, "{offset:>4}: {line}");
    Some(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_instrs() {
        let mut builder = InstructionBuilder::default();
        builder.push_op(OP_CONST_TRUE);
        builder.push_op(OP_CONST_FALSE);
        builder.push_op(OP_ADD_I32);
        assert_eq!(
            builder.code,
            vec![OP_CONST_TRUE, OP_CONST_FALSE, OP_ADD_I32]
        );
    }

    #[test]
    fn test_build_imm() {
        let mut builder = InstructionBuilder::default();
        builder.push_op(OP_CONST_I32);
        builder.push_immediate(10i32);
        assert_eq!(builder.code.len(), 5);
        assert_eq!(disassemble(&builder.inner()), "   0: const i32 10\n");
    }

    #[test]
    fn test_disassemble_truncated() {
        let text = disassemble(&[OP_CONST_I64, 1, 2]);
        assert!(text.contains("truncated"));
    }
}
