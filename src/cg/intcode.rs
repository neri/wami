use super::{GlobalVarIndex, LocalVarIndex, StackLevel};
use crate::{bytecode::WasmMnemonic, WasmTypeIndex};
use alloc::{boxed::Box, vec::Vec};

/// Intermediate code for Webassembly interpreter
#[non_exhaustive]
#[derive(Debug, PartialEq)]
pub enum WasmIntMnemonic {
    /// Intermediate code that could not be converted
    Undefined(WasmMnemonic, ExceptionPosition),

    /// No operation marker, this mnemonic will be removed during the compaction phase.
    Nop,
    /// Block Marker, this mnemonic will be removed during the compaction phase.
    Block(usize),
    /// End of block marker, this mnemonic will be removed during the compaction phase.
    End(usize),

    /// `00 unreachable`
    Unreachable(ExceptionPosition),

    /// `0C br labelidx`
    Br(u32),
    /// `0D br_if labelidx`
    BrIf(u32),
    /// `0E br_table vec(labelidx) labelidx`
    BrTable(Box<[u32]>),

    /// return from function (-> nil)
    ReturnN,
    /// return from function (-> integer)
    ReturnI,
    /// return from function (-> float)
    ReturnF,

    /// `10 call funcidx`
    Call(usize, ExceptionPosition),
    /// `11 call_indirect typeidx 0x00`
    CallIndirect(WasmTypeIndex, ExceptionPosition),

    /// `1B select`
    SelectI,
    SelectF,

    LocalGetI(LocalVarIndex),
    LocalSetI(LocalVarIndex),
    LocalTeeI(LocalVarIndex),
    GlobalGetI(GlobalVarIndex),
    GlobalSetI(GlobalVarIndex),

    LocalGetF(LocalVarIndex),
    LocalSetF(LocalVarIndex),
    LocalTeeF(LocalVarIndex),
    GlobalGetF(GlobalVarIndex),
    GlobalSetF(GlobalVarIndex),

    I32Load(u32, ExceptionPosition),
    I64Load(u32, ExceptionPosition),
    I32Load8S(u32, ExceptionPosition),
    I32Load8U(u32, ExceptionPosition),
    I32Load16S(u32, ExceptionPosition),
    I32Load16U(u32, ExceptionPosition),
    I64Load8S(u32, ExceptionPosition),
    I64Load8U(u32, ExceptionPosition),
    I64Load16S(u32, ExceptionPosition),
    I64Load16U(u32, ExceptionPosition),
    I64Load32S(u32, ExceptionPosition),
    I64Load32U(u32, ExceptionPosition),
    F32Load(u32, ExceptionPosition),
    F64Load(u32, ExceptionPosition),

    I32Store(u32, ExceptionPosition),
    I64Store(u32, ExceptionPosition),
    I32Store8(u32, ExceptionPosition),
    I32Store16(u32, ExceptionPosition),
    I64Store8(u32, ExceptionPosition),
    I64Store16(u32, ExceptionPosition),
    I64Store32(u32, ExceptionPosition),
    F32Store(u32, ExceptionPosition),
    F64Store(u32, ExceptionPosition),

    /// `3F memory.size 0x00`
    MemorySize,
    /// `40 memory.grow 0x00`
    MemoryGrow,
    /// `FC 0A memory.copy memory_dst memory_src` (bulk_memory_operations)
    MemoryCopy(ExceptionPosition),
    /// `FC 0B memory.fill memory` (bulk_memory_operations)
    MemoryFill(ExceptionPosition),

    /// `41 i32.const n`
    I32Const(i32),
    /// `42 i64.const n`
    I64Const(i64),
    /// `43 f32.const z`
    F32Const(f32),
    /// `44 f64.const z`
    F64Const(f64),

    I32Eqz,
    I32Eq,
    I32Ne,
    I32LtS,
    I32LtU,
    I32GtS,
    I32GtU,
    I32LeS,
    I32LeU,
    I32GeS,
    I32GeU,
    I32Clz,
    I32Ctz,
    I32Popcnt,

    I32Add,
    I32Sub,
    I32Mul,
    I32DivS(ExceptionPosition),
    I32DivU(ExceptionPosition),
    I32RemS(ExceptionPosition),
    I32RemU(ExceptionPosition),
    I32And,
    I32Or,
    I32Xor,
    I32Shl,
    I32ShrS,
    I32ShrU,
    I32Rotl,
    I32Rotr,

    I64Eqz,
    I64Eq,
    I64Ne,
    I64LtS,
    I64LtU,
    I64GtS,
    I64GtU,
    I64LeS,
    I64LeU,
    I64GeS,
    I64GeU,
    I64Clz,
    I64Ctz,
    I64Popcnt,

    I64Add,
    I64Sub,
    I64Mul,
    I64DivS(ExceptionPosition),
    I64DivU(ExceptionPosition),
    I64RemS(ExceptionPosition),
    I64RemU(ExceptionPosition),
    I64And,
    I64Or,
    I64Xor,
    I64Shl,
    I64ShrS,
    I64ShrU,
    I64Rotl,
    I64Rotr,

    F32Eq,
    F32Ne,
    F32Lt,
    F32Gt,
    F32Le,
    F32Ge,

    F32Abs,
    F32Neg,
    F32Ceil,
    F32Floor,
    F32Trunc,
    F32Nearest,
    F32Sqrt,
    F32Add,
    F32Sub,
    F32Mul,
    F32Div,
    F32Min,
    F32Max,
    F32Copysign,

    F64Eq,
    F64Ne,
    F64Lt,
    F64Gt,
    F64Le,
    F64Ge,

    F64Abs,
    F64Neg,
    F64Ceil,
    F64Floor,
    F64Trunc,
    F64Nearest,
    F64Sqrt,
    F64Add,
    F64Sub,
    F64Mul,
    F64Div,
    F64Min,
    F64Max,
    F64Copysign,

    I64Extend8S,
    I64Extend16S,
    I64Extend32S,
    I64ExtendI32S,
    I64ExtendI32U,
    I32WrapI64,
    I32Extend8S,
    I32Extend16S,

    I32TruncF32S,
    I32TruncF32U,
    I32TruncF64S,
    I32TruncF64U,
    I64TruncF32S,
    I64TruncF32U,
    I64TruncF64S,
    I64TruncF64U,
    F32ConvertI32S,
    F32ConvertI32U,
    F32ConvertI64S,
    F32ConvertI64U,
    F32DemoteF64,
    F64ConvertI32S,
    F64ConvertI32U,
    F64ConvertI64S,
    F64ConvertI64U,
    F64PromoteF32,
    I32ReinterpretF32,
    I64ReinterpretF64,
    F32ReinterpretI32,
    F64ReinterpretI64,

    I32TruncSatF32S,
    I32TruncSatF32U,
    I32TruncSatF64S,
    I32TruncSatF64U,
    I64TruncSatF32S,
    I64TruncSatF32U,
    I64TruncSatF64S,
    I64TruncSatF64U,

    //
    // Fused Instructions
    //
    FusedI32SetConst(LocalVarIndex, i32),
    FusedI64SetConst(LocalVarIndex, i64),

    FusedI32AddI(i32),
    FusedI32SubI(i32),
    FusedI32AndI(u32),
    FusedI32OrI(u32),
    FusedI32XorI(u32),
    FusedI32ShlI(u32),
    FusedI32ShrSI(u32),
    FusedI32ShrUI(u32),

    FusedI64AddI(i64),
    FusedI64SubI(i64),
    FusedI64AndI(u64),
    FusedI64OrI(u64),
    FusedI64XorI(u64),
    FusedI64ShlI(u32),
    FusedI64ShrSI(u32),
    FusedI64ShrUI(u32),

    FusedI32BrZ(u32),
    FusedI32BrEq(u32),
    FusedI32BrNe(u32),
    FusedI32BrLtS(u32),
    FusedI32BrLtU(u32),
    FusedI32BrGtS(u32),
    FusedI32BrGtU(u32),
    FusedI32BrLeS(u32),
    FusedI32BrLeU(u32),
    FusedI32BrGeS(u32),
    FusedI32BrGeU(u32),

    FusedI64BrZ(u32),
    FusedI64BrEq(u32),
    FusedI64BrNe(u32),
}

impl WasmIntMnemonic {
    #[inline]
    pub fn normalized(self) -> Self {
        // TODO:
        self
    }
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ExceptionPosition(u32);

impl ExceptionPosition {
    pub const UNKNOWN: Self = Self::new(0);

    #[inline]
    pub const fn new(position: usize) -> Self {
        Self(position as u32)
    }

    #[inline]
    pub const fn position(&self) -> usize {
        self.0 as usize
    }
}

/// Wasm Intermediate Code
#[derive(Debug)]
pub struct WasmImc {
    pub mnemonic: WasmIntMnemonic,
    pub stack_level: StackLevel,
}

impl WasmImc {
    #[inline]
    pub const fn from_mnemonic(mnemonic: WasmIntMnemonic) -> Self {
        Self {
            mnemonic,
            stack_level: StackLevel::zero(),
        }
    }

    #[inline]
    pub const fn new(mnemonic: WasmIntMnemonic, stack_level: StackLevel) -> Self {
        Self {
            mnemonic,
            stack_level,
        }
    }

    #[inline]
    pub const fn mnemonic(&self) -> &WasmIntMnemonic {
        &self.mnemonic
    }

    #[inline]
    pub fn mnemonic_mut(&mut self) -> &mut WasmIntMnemonic {
        &mut self.mnemonic
    }

    #[inline]
    pub const fn base_stack_level(&self) -> StackLevel {
        self.stack_level
    }

    pub fn adjust_branch_target<F, E>(&mut self, mut f: F) -> Result<(), E>
    where
        F: FnMut(WasmMnemonic, usize) -> Result<usize, E>,
    {
        use WasmIntMnemonic::*;
        match self.mnemonic_mut() {
            Br(target) => {
                *target = f(WasmMnemonic::Br, *target as usize)? as u32;
            }
            BrIf(target) => {
                *target = f(WasmMnemonic::BrIf, *target as usize)? as u32;
            }

            FusedI32BrZ(target) => {
                *target = f(WasmMnemonic::BrIf, *target as usize)? as u32;
            }
            FusedI32BrEq(target) => {
                *target = f(WasmMnemonic::BrIf, *target as usize)? as u32;
            }
            FusedI32BrNe(target) => {
                *target = f(WasmMnemonic::BrIf, *target as usize)? as u32;
            }
            FusedI32BrLtS(target) => {
                *target = f(WasmMnemonic::BrIf, *target as usize)? as u32;
            }
            FusedI32BrLtU(target) => {
                *target = f(WasmMnemonic::BrIf, *target as usize)? as u32;
            }
            FusedI32BrGtS(target) => {
                *target = f(WasmMnemonic::BrIf, *target as usize)? as u32;
            }
            FusedI32BrGtU(target) => {
                *target = f(WasmMnemonic::BrIf, *target as usize)? as u32;
            }
            FusedI32BrLeS(target) => {
                *target = f(WasmMnemonic::BrIf, *target as usize)? as u32;
            }
            FusedI32BrLeU(target) => {
                *target = f(WasmMnemonic::BrIf, *target as usize)? as u32;
            }
            FusedI32BrGeS(target) => {
                *target = f(WasmMnemonic::BrIf, *target as usize)? as u32;
            }
            FusedI32BrGeU(target) => {
                *target = f(WasmMnemonic::BrIf, *target as usize)? as u32;
            }

            FusedI64BrZ(target) => {
                *target = f(WasmMnemonic::BrIf, *target as usize)? as u32;
            }
            FusedI64BrEq(target) => {
                *target = f(WasmMnemonic::BrIf, *target as usize)? as u32;
            }
            FusedI64BrNe(target) => {
                *target = f(WasmMnemonic::BrIf, *target as usize)? as u32;
            }

            BrTable(table) => {
                let mut vec = Vec::with_capacity(table.len());
                for target in table.iter() {
                    vec.push(f(WasmMnemonic::BrTable, *target as usize)? as u32);
                }
                *table = vec.into_boxed_slice();
            }
            _ => (),
        }
        Ok(())
    }
}

impl From<WasmIntMnemonic> for WasmImc {
    #[inline]
    fn from(val: WasmIntMnemonic) -> Self {
        Self::from_mnemonic(val)
    }
}
