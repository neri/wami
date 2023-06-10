use super::{GlobalVarIndex, LocalVarIndex, StackLevel};
use crate::opcode::{WasmOpcode, WasmSingleOpcode};
use alloc::{boxed::Box, vec::Vec};

/// Intermediate code for Webassembly runtime
#[non_exhaustive]
#[derive(Debug, PartialEq)]
pub enum WasmIntMnemonic {
    /// Intermediate code that could not be converted
    Undefined(WasmOpcode, ExceptionPosition),

    /// No operation marker, this mnemonic will be removed during the compaction phase.
    Nop,
    /// Block Marker, this mnemonic will be removed during the compaction phase.
    Block(usize),
    /// End of block marker, this mnemonic will be removed during the compaction phase.
    End(usize),

    /// `00 unreachable`
    Unreachable(ExceptionPosition),

    /// `0C br labelidx`
    Br(usize),
    /// `0D br_if labelidx`
    BrIf(usize),
    /// `0E br_table vec(labelidx) labelidx`
    BrTable(Box<[usize]>),

    /// return from function
    ReturnV,
    /// return from function (integer)
    ReturnI,
    /// return from function (float)
    ReturnF,

    /// `10 call funcidx`
    Call(usize, ExceptionPosition),
    /// `11 call_indirect typeidx 0x00`
    CallIndirect(usize, ExceptionPosition),

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
    I32Load8S(u32, ExceptionPosition),
    I32Load8U(u32, ExceptionPosition),
    I32Load16S(u32, ExceptionPosition),
    I32Load16U(u32, ExceptionPosition),
    I32Store(u32, ExceptionPosition),
    I32Store8(u32, ExceptionPosition),
    I32Store16(u32, ExceptionPosition),
    I64Load(u32, ExceptionPosition),
    I64Load8S(u32, ExceptionPosition),
    I64Load8U(u32, ExceptionPosition),
    I64Load16S(u32, ExceptionPosition),
    I64Load16U(u32, ExceptionPosition),
    I64Load32S(u32, ExceptionPosition),
    I64Load32U(u32, ExceptionPosition),
    I64Store(u32, ExceptionPosition),
    I64Store8(u32, ExceptionPosition),
    I64Store16(u32, ExceptionPosition),
    I64Store32(u32, ExceptionPosition),

    #[cfg(feature = "float")]
    F32Load(u32, ExceptionPosition),
    #[cfg(feature = "float")]
    F32Store(u32, ExceptionPosition),
    #[cfg(feature = "float")]
    F64Load(u32, ExceptionPosition),
    #[cfg(feature = "float")]
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
    #[cfg(feature = "float")]
    /// `43 f32.const z`
    F32Const(f32),
    #[cfg(feature = "float")]
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

    I64Extend8S,
    I64Extend16S,
    I64Extend32S,
    I64ExtendI32S,
    I64ExtendI32U,
    I32WrapI64,
    I32Extend8S,
    I32Extend16S,

    // Fused Instructions
    FusedI32SetConst(LocalVarIndex, i32),
    FusedI32AddI(i32),
    FusedI32SubI(i32),
    FusedI32AndI(i32),
    FusedI32OrI(i32),
    FusedI32XorI(i32),
    FusedI32ShlI(i32),
    FusedI32ShrSI(i32),
    FusedI32ShrUI(i32),

    FusedI64SetConst(LocalVarIndex, i64),
    FusedI64AddI(i64),
    FusedI64SubI(i64),

    FusedI32BrZ(usize),
    FusedI32BrEq(usize),
    FusedI32BrNe(usize),
    FusedI32BrLtS(usize),
    FusedI32BrLtU(usize),
    FusedI32BrGtS(usize),
    FusedI32BrGtU(usize),
    FusedI32BrLeS(usize),
    FusedI32BrLeU(usize),
    FusedI32BrGeS(usize),
    FusedI32BrGeU(usize),

    FusedI64BrZ(usize),
    FusedI64BrEq(usize),
    FusedI64BrNe(usize),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ExceptionPosition {
    pub position: u32,
}

impl ExceptionPosition {
    pub const UNKNOWN: Self = Self::new(0);

    #[inline]
    pub const fn new(position: usize) -> Self {
        Self {
            position: position as u32,
        }
    }

    #[inline]
    pub const fn position(&self) -> usize {
        self.position as usize
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
    pub const fn mnemonic_mut(&mut self) -> &mut WasmIntMnemonic {
        &mut self.mnemonic
    }

    #[inline]
    pub const fn base_stack_level(&self) -> StackLevel {
        self.stack_level
    }

    pub fn adjust_branch_target<F, E>(&mut self, mut f: F) -> Result<(), E>
    where
        F: FnMut(WasmOpcode, usize) -> Result<usize, E>,
    {
        use WasmIntMnemonic::*;
        match self.mnemonic_mut() {
            Br(target) => {
                *target = f(WasmSingleOpcode::Br.into(), *target)?;
            }
            BrIf(target) => {
                *target = f(WasmSingleOpcode::BrIf.into(), *target)?;
            }

            FusedI32BrZ(target) => {
                *target = f(WasmSingleOpcode::BrIf.into(), *target)?;
            }
            FusedI32BrEq(target) => {
                *target = f(WasmSingleOpcode::BrIf.into(), *target)?;
            }
            FusedI32BrNe(target) => {
                *target = f(WasmSingleOpcode::BrIf.into(), *target)?;
            }
            FusedI32BrLtS(target) => {
                *target = f(WasmSingleOpcode::BrIf.into(), *target)?;
            }
            FusedI32BrLtU(target) => {
                *target = f(WasmSingleOpcode::BrIf.into(), *target)?;
            }
            FusedI32BrGtS(target) => {
                *target = f(WasmSingleOpcode::BrIf.into(), *target)?;
            }
            FusedI32BrGtU(target) => {
                *target = f(WasmSingleOpcode::BrIf.into(), *target)?;
            }
            FusedI32BrLeS(target) => {
                *target = f(WasmSingleOpcode::BrIf.into(), *target)?;
            }
            FusedI32BrLeU(target) => {
                *target = f(WasmSingleOpcode::BrIf.into(), *target)?;
            }
            FusedI32BrGeS(target) => {
                *target = f(WasmSingleOpcode::BrIf.into(), *target)?;
            }
            FusedI32BrGeU(target) => {
                *target = f(WasmSingleOpcode::BrIf.into(), *target)?;
            }

            FusedI64BrZ(target) => {
                *target = f(WasmSingleOpcode::BrIf.into(), *target)?;
            }
            FusedI64BrEq(target) => {
                *target = f(WasmSingleOpcode::BrIf.into(), *target)?;
            }
            FusedI64BrNe(target) => {
                *target = f(WasmSingleOpcode::BrIf.into(), *target)?;
            }

            BrTable(table) => {
                let mut vec = Vec::with_capacity(table.len());
                for target in table.iter() {
                    vec.push(f(WasmSingleOpcode::BrTable.into(), *target)?);
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
