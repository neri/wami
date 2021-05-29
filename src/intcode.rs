//! Intermediate code for Webassembly runtime

/// Intermediate code for Webassembly runtime
#[non_exhaustive]
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum WasmIntMnemonic {
    /// Undefined
    Undefined,
    /// Unreachable
    Unreachable,
    /// No operation, this mnemonic will be removed during the compaction phase.
    Nop,

    /// branch
    Br,
    /// branch if true
    BrIf,
    /// Fused check and branch
    FusedI32BrZ,
    FusedI64BrZ,

    /// branch table
    BrTable,

    /// Block Marker, this mnemonic will be removed during the compaction phase.
    Block,
    /// End of block marker, this mnemonic will be removed during the compaction phase.
    End,

    /// return from function
    Return,
    /// call function
    Call,
    /// call indirect
    CallIndirect,
    /// select value
    Select,
    /// Get a value from a local variable
    LocalGet,
    /// Set a value to a local variable
    LocalSet,
    /// Duplicate a value to local variable
    LocalTee,
    /// Get a value from a global variable
    GlobalGet,
    /// Set a value to a global variable
    GlobalSet,

    I32Load,
    I32Load8S,
    I32Load8U,
    I32Load16S,
    I32Load16U,
    I32Store,
    I32Store8,
    I32Store16,
    I64Load,
    I64Load8S,
    I64Load8U,
    I64Load16S,
    I64Load16U,
    I64Load32S,
    I64Load32U,
    I64Store,
    I64Store8,
    I64Store16,
    I64Store32,
    MemorySize,
    MemoryGrow,

    I32Const,
    I64Const,

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
    I32DivS,
    I32DivU,
    I32RemS,
    I32RemU,
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
    I64DivS,
    I64DivU,
    I64RemS,
    I64RemU,
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
    FusedI32AddI,
    FusedI32SubI,
    FusedI64AddI,
    FusedI64SubI,
    FusedI32AndI,
    FusedI32OrI,
    FusedI32XorI,
    FusedI32ShlI,
    FusedI32ShrSI,
    FusedI32ShrUI,
}

impl WasmIntMnemonic {
    #[inline]
    pub fn is_branch(&self) -> bool {
        use WasmIntMnemonic::*;
        match *self {
            Br | BrIf | FusedI32BrZ | FusedI64BrZ => true,
            _ => false,
        }
    }
}
