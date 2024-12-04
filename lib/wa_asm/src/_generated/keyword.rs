//! WebAssembly Reserved Keywords

/* This file is generated automatically. DO NOT EDIT DIRECTLY. */

/// WebAssembly Reserved Keywords
#[non_exhaustive]
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Keyword {
    /// "anyfunc"
    Anyfunc,
    /// "block"
    Block,
    /// "br"
    Br,
    /// "br_if"
    BrIf,
    /// "br_table"
    BrTable,
    /// "call"
    Call,
    /// "call_indirect"
    CallIndirect,
    /// "data"
    Data,
    /// "drop"
    Drop,
    /// "elem"
    Elem,
    /// "else"
    Else,
    /// "end"
    End,
    /// "export"
    Export,
    /// "externref"
    Externref,
    /// "f32.abs"
    F32Abs,
    /// "f32.add"
    F32Add,
    /// "f32.ceil"
    F32Ceil,
    /// "f32.const"
    F32Const,
    /// "f32.convert_i32_s"
    F32ConvertI32S,
    /// "f32.convert_i32_u"
    F32ConvertI32U,
    /// "f32.convert_i64_s"
    F32ConvertI64S,
    /// "f32.convert_i64_u"
    F32ConvertI64U,
    /// "f32.copysign"
    F32Copysign,
    /// "f32.demote_f64"
    F32DemoteF64,
    /// "f32.div"
    F32Div,
    /// "f32.eq"
    F32Eq,
    /// "f32.floor"
    F32Floor,
    /// "f32.ge"
    F32Ge,
    /// "f32.gt"
    F32Gt,
    /// "f32.le"
    F32Le,
    /// "f32.load"
    F32Load,
    /// "f32.lt"
    F32Lt,
    /// "f32.max"
    F32Max,
    /// "f32.min"
    F32Min,
    /// "f32.mul"
    F32Mul,
    /// "f32.ne"
    F32Ne,
    /// "f32.nearest"
    F32Nearest,
    /// "f32.neg"
    F32Neg,
    /// "f32.reinterpret_i32"
    F32ReinterpretI32,
    /// "f32.sqrt"
    F32Sqrt,
    /// "f32.store"
    F32Store,
    /// "f32.sub"
    F32Sub,
    /// "f32.trunc"
    F32Trunc,
    /// "f64.abs"
    F64Abs,
    /// "f64.add"
    F64Add,
    /// "f64.ceil"
    F64Ceil,
    /// "f64.const"
    F64Const,
    /// "f64.convert_i32_s"
    F64ConvertI32S,
    /// "f64.convert_i32_u"
    F64ConvertI32U,
    /// "f64.convert_i64_s"
    F64ConvertI64S,
    /// "f64.convert_i64_u"
    F64ConvertI64U,
    /// "f64.copysign"
    F64Copysign,
    /// "f64.div"
    F64Div,
    /// "f64.eq"
    F64Eq,
    /// "f64.floor"
    F64Floor,
    /// "f64.ge"
    F64Ge,
    /// "f64.gt"
    F64Gt,
    /// "f64.le"
    F64Le,
    /// "f64.load"
    F64Load,
    /// "f64.lt"
    F64Lt,
    /// "f64.max"
    F64Max,
    /// "f64.min"
    F64Min,
    /// "f64.mul"
    F64Mul,
    /// "f64.ne"
    F64Ne,
    /// "f64.nearest"
    F64Nearest,
    /// "f64.neg"
    F64Neg,
    /// "f64.promote_f32"
    F64PromoteF32,
    /// "f64.reinterpret_i64"
    F64ReinterpretI64,
    /// "f64.sqrt"
    F64Sqrt,
    /// "f64.store"
    F64Store,
    /// "f64.sub"
    F64Sub,
    /// "f64.trunc"
    F64Trunc,
    /// "func"
    Func,
    /// "funcref"
    Funcref,
    /// "global"
    Global,
    /// "global.get"
    GlobalGet,
    /// "global.set"
    GlobalSet,
    /// "i32.add"
    I32Add,
    /// "i32.and"
    I32And,
    /// "i32.clz"
    I32Clz,
    /// "i32.const"
    I32Const,
    /// "i32.ctz"
    I32Ctz,
    /// "i32.div_s"
    I32DivS,
    /// "i32.div_u"
    I32DivU,
    /// "i32.eq"
    I32Eq,
    /// "i32.eqz"
    I32Eqz,
    /// "i32.extend16_s"
    I32Extend16S,
    /// "i32.extend8_s"
    I32Extend8S,
    /// "i32.ge_s"
    I32GeS,
    /// "i32.ge_u"
    I32GeU,
    /// "i32.gt_s"
    I32GtS,
    /// "i32.gt_u"
    I32GtU,
    /// "i32.le_s"
    I32LeS,
    /// "i32.le_u"
    I32LeU,
    /// "i32.load"
    I32Load,
    /// "i32.load16_s"
    I32Load16S,
    /// "i32.load16_u"
    I32Load16U,
    /// "i32.load8_s"
    I32Load8S,
    /// "i32.load8_u"
    I32Load8U,
    /// "i32.lt_s"
    I32LtS,
    /// "i32.lt_u"
    I32LtU,
    /// "i32.mul"
    I32Mul,
    /// "i32.ne"
    I32Ne,
    /// "i32.or"
    I32Or,
    /// "i32.popcnt"
    I32Popcnt,
    /// "i32.reinterpret_f32"
    I32ReinterpretF32,
    /// "i32.rem_s"
    I32RemS,
    /// "i32.rem_u"
    I32RemU,
    /// "i32.rotl"
    I32Rotl,
    /// "i32.rotr"
    I32Rotr,
    /// "i32.shl"
    I32Shl,
    /// "i32.shr_s"
    I32ShrS,
    /// "i32.shr_u"
    I32ShrU,
    /// "i32.store"
    I32Store,
    /// "i32.store16"
    I32Store16,
    /// "i32.store8"
    I32Store8,
    /// "i32.sub"
    I32Sub,
    /// "i32.trunc_f32_s"
    I32TruncF32S,
    /// "i32.trunc_f32_u"
    I32TruncF32U,
    /// "i32.trunc_f64_s"
    I32TruncF64S,
    /// "i32.trunc_f64_u"
    I32TruncF64U,
    /// "i32.trunc_sat_f32_s"
    I32TruncSatF32S,
    /// "i32.trunc_sat_f32_u"
    I32TruncSatF32U,
    /// "i32.trunc_sat_f64_s"
    I32TruncSatF64S,
    /// "i32.trunc_sat_f64_u"
    I32TruncSatF64U,
    /// "i32.wrap_i64"
    I32WrapI64,
    /// "i32.xor"
    I32Xor,
    /// "i64.add"
    I64Add,
    /// "i64.and"
    I64And,
    /// "i64.clz"
    I64Clz,
    /// "i64.const"
    I64Const,
    /// "i64.ctz"
    I64Ctz,
    /// "i64.div_s"
    I64DivS,
    /// "i64.div_u"
    I64DivU,
    /// "i64.eq"
    I64Eq,
    /// "i64.eqz"
    I64Eqz,
    /// "i64.extend16_s"
    I64Extend16S,
    /// "i64.extend32_s"
    I64Extend32S,
    /// "i64.extend8_s"
    I64Extend8S,
    /// "i64.extend_i32_s"
    I64ExtendI32S,
    /// "i64.extend_i32_u"
    I64ExtendI32U,
    /// "i64.ge_s"
    I64GeS,
    /// "i64.ge_u"
    I64GeU,
    /// "i64.gt_s"
    I64GtS,
    /// "i64.gt_u"
    I64GtU,
    /// "i64.le_s"
    I64LeS,
    /// "i64.le_u"
    I64LeU,
    /// "i64.load"
    I64Load,
    /// "i64.load16_s"
    I64Load16S,
    /// "i64.load16_u"
    I64Load16U,
    /// "i64.load32_s"
    I64Load32S,
    /// "i64.load32_u"
    I64Load32U,
    /// "i64.load8_s"
    I64Load8S,
    /// "i64.load8_u"
    I64Load8U,
    /// "i64.lt_s"
    I64LtS,
    /// "i64.lt_u"
    I64LtU,
    /// "i64.mul"
    I64Mul,
    /// "i64.ne"
    I64Ne,
    /// "i64.or"
    I64Or,
    /// "i64.popcnt"
    I64Popcnt,
    /// "i64.reinterpret_f64"
    I64ReinterpretF64,
    /// "i64.rem_s"
    I64RemS,
    /// "i64.rem_u"
    I64RemU,
    /// "i64.rotl"
    I64Rotl,
    /// "i64.rotr"
    I64Rotr,
    /// "i64.shl"
    I64Shl,
    /// "i64.shr_s"
    I64ShrS,
    /// "i64.shr_u"
    I64ShrU,
    /// "i64.store"
    I64Store,
    /// "i64.store16"
    I64Store16,
    /// "i64.store32"
    I64Store32,
    /// "i64.store8"
    I64Store8,
    /// "i64.sub"
    I64Sub,
    /// "i64.trunc_f32_s"
    I64TruncF32S,
    /// "i64.trunc_f32_u"
    I64TruncF32U,
    /// "i64.trunc_f64_s"
    I64TruncF64S,
    /// "i64.trunc_f64_u"
    I64TruncF64U,
    /// "i64.trunc_sat_f32_s"
    I64TruncSatF32S,
    /// "i64.trunc_sat_f32_u"
    I64TruncSatF32U,
    /// "i64.trunc_sat_f64_s"
    I64TruncSatF64S,
    /// "i64.trunc_sat_f64_u"
    I64TruncSatF64U,
    /// "i64.xor"
    I64Xor,
    /// "if"
    If,
    /// "import"
    Import,
    /// "item"
    Item,
    /// "local"
    Local,
    /// "local.get"
    LocalGet,
    /// "local.set"
    LocalSet,
    /// "local.tee"
    LocalTee,
    /// "loop"
    Loop,
    /// "memory"
    Memory,
    /// "memory.copy"
    MemoryCopy,
    /// "memory.fill"
    MemoryFill,
    /// "memory.grow"
    MemoryGrow,
    /// "memory.size"
    MemorySize,
    /// "module"
    Module,
    /// "mut"
    Mut,
    /// "nop"
    Nop,
    /// "offset"
    Offset,
    /// "param"
    Param,
    /// "result"
    Result,
    /// "return"
    Return,
    /// "select"
    Select,
    /// "start"
    Start,
    /// "table"
    Table,
    /// "type"
    Type,
    /// "unreachable"
    Unreachable,
}

impl Keyword {
    pub fn all_values() -> &'static [Self] {
        &[
            Self::Anyfunc,
            Self::Block,
            Self::Br,
            Self::BrIf,
            Self::BrTable,
            Self::Call,
            Self::CallIndirect,
            Self::Data,
            Self::Drop,
            Self::Elem,
            Self::Else,
            Self::End,
            Self::Export,
            Self::Externref,
            Self::F32Abs,
            Self::F32Add,
            Self::F32Ceil,
            Self::F32Const,
            Self::F32ConvertI32S,
            Self::F32ConvertI32U,
            Self::F32ConvertI64S,
            Self::F32ConvertI64U,
            Self::F32Copysign,
            Self::F32DemoteF64,
            Self::F32Div,
            Self::F32Eq,
            Self::F32Floor,
            Self::F32Ge,
            Self::F32Gt,
            Self::F32Le,
            Self::F32Load,
            Self::F32Lt,
            Self::F32Max,
            Self::F32Min,
            Self::F32Mul,
            Self::F32Ne,
            Self::F32Nearest,
            Self::F32Neg,
            Self::F32ReinterpretI32,
            Self::F32Sqrt,
            Self::F32Store,
            Self::F32Sub,
            Self::F32Trunc,
            Self::F64Abs,
            Self::F64Add,
            Self::F64Ceil,
            Self::F64Const,
            Self::F64ConvertI32S,
            Self::F64ConvertI32U,
            Self::F64ConvertI64S,
            Self::F64ConvertI64U,
            Self::F64Copysign,
            Self::F64Div,
            Self::F64Eq,
            Self::F64Floor,
            Self::F64Ge,
            Self::F64Gt,
            Self::F64Le,
            Self::F64Load,
            Self::F64Lt,
            Self::F64Max,
            Self::F64Min,
            Self::F64Mul,
            Self::F64Ne,
            Self::F64Nearest,
            Self::F64Neg,
            Self::F64PromoteF32,
            Self::F64ReinterpretI64,
            Self::F64Sqrt,
            Self::F64Store,
            Self::F64Sub,
            Self::F64Trunc,
            Self::Func,
            Self::Funcref,
            Self::Global,
            Self::GlobalGet,
            Self::GlobalSet,
            Self::I32Add,
            Self::I32And,
            Self::I32Clz,
            Self::I32Const,
            Self::I32Ctz,
            Self::I32DivS,
            Self::I32DivU,
            Self::I32Eq,
            Self::I32Eqz,
            Self::I32Extend16S,
            Self::I32Extend8S,
            Self::I32GeS,
            Self::I32GeU,
            Self::I32GtS,
            Self::I32GtU,
            Self::I32LeS,
            Self::I32LeU,
            Self::I32Load,
            Self::I32Load16S,
            Self::I32Load16U,
            Self::I32Load8S,
            Self::I32Load8U,
            Self::I32LtS,
            Self::I32LtU,
            Self::I32Mul,
            Self::I32Ne,
            Self::I32Or,
            Self::I32Popcnt,
            Self::I32ReinterpretF32,
            Self::I32RemS,
            Self::I32RemU,
            Self::I32Rotl,
            Self::I32Rotr,
            Self::I32Shl,
            Self::I32ShrS,
            Self::I32ShrU,
            Self::I32Store,
            Self::I32Store16,
            Self::I32Store8,
            Self::I32Sub,
            Self::I32TruncF32S,
            Self::I32TruncF32U,
            Self::I32TruncF64S,
            Self::I32TruncF64U,
            Self::I32TruncSatF32S,
            Self::I32TruncSatF32U,
            Self::I32TruncSatF64S,
            Self::I32TruncSatF64U,
            Self::I32WrapI64,
            Self::I32Xor,
            Self::I64Add,
            Self::I64And,
            Self::I64Clz,
            Self::I64Const,
            Self::I64Ctz,
            Self::I64DivS,
            Self::I64DivU,
            Self::I64Eq,
            Self::I64Eqz,
            Self::I64Extend16S,
            Self::I64Extend32S,
            Self::I64Extend8S,
            Self::I64ExtendI32S,
            Self::I64ExtendI32U,
            Self::I64GeS,
            Self::I64GeU,
            Self::I64GtS,
            Self::I64GtU,
            Self::I64LeS,
            Self::I64LeU,
            Self::I64Load,
            Self::I64Load16S,
            Self::I64Load16U,
            Self::I64Load32S,
            Self::I64Load32U,
            Self::I64Load8S,
            Self::I64Load8U,
            Self::I64LtS,
            Self::I64LtU,
            Self::I64Mul,
            Self::I64Ne,
            Self::I64Or,
            Self::I64Popcnt,
            Self::I64ReinterpretF64,
            Self::I64RemS,
            Self::I64RemU,
            Self::I64Rotl,
            Self::I64Rotr,
            Self::I64Shl,
            Self::I64ShrS,
            Self::I64ShrU,
            Self::I64Store,
            Self::I64Store16,
            Self::I64Store32,
            Self::I64Store8,
            Self::I64Sub,
            Self::I64TruncF32S,
            Self::I64TruncF32U,
            Self::I64TruncF64S,
            Self::I64TruncF64U,
            Self::I64TruncSatF32S,
            Self::I64TruncSatF32U,
            Self::I64TruncSatF64S,
            Self::I64TruncSatF64U,
            Self::I64Xor,
            Self::If,
            Self::Import,
            Self::Item,
            Self::Local,
            Self::LocalGet,
            Self::LocalSet,
            Self::LocalTee,
            Self::Loop,
            Self::Memory,
            Self::MemoryCopy,
            Self::MemoryFill,
            Self::MemoryGrow,
            Self::MemorySize,
            Self::Module,
            Self::Mut,
            Self::Nop,
            Self::Offset,
            Self::Param,
            Self::Result,
            Self::Return,
            Self::Select,
            Self::Start,
            Self::Table,
            Self::Type,
            Self::Unreachable,
        ]
    }

    pub fn from_str(v: &str) -> Option<Self> {
        match v {
            "anyfunc" => Some(Self::Anyfunc),
            "block" => Some(Self::Block),
            "br" => Some(Self::Br),
            "br_if" => Some(Self::BrIf),
            "br_table" => Some(Self::BrTable),
            "call" => Some(Self::Call),
            "call_indirect" => Some(Self::CallIndirect),
            "data" => Some(Self::Data),
            "drop" => Some(Self::Drop),
            "elem" => Some(Self::Elem),
            "else" => Some(Self::Else),
            "end" => Some(Self::End),
            "export" => Some(Self::Export),
            "externref" => Some(Self::Externref),
            "f32.abs" => Some(Self::F32Abs),
            "f32.add" => Some(Self::F32Add),
            "f32.ceil" => Some(Self::F32Ceil),
            "f32.const" => Some(Self::F32Const),
            "f32.convert_i32_s" => Some(Self::F32ConvertI32S),
            "f32.convert_i32_u" => Some(Self::F32ConvertI32U),
            "f32.convert_i64_s" => Some(Self::F32ConvertI64S),
            "f32.convert_i64_u" => Some(Self::F32ConvertI64U),
            "f32.copysign" => Some(Self::F32Copysign),
            "f32.demote_f64" => Some(Self::F32DemoteF64),
            "f32.div" => Some(Self::F32Div),
            "f32.eq" => Some(Self::F32Eq),
            "f32.floor" => Some(Self::F32Floor),
            "f32.ge" => Some(Self::F32Ge),
            "f32.gt" => Some(Self::F32Gt),
            "f32.le" => Some(Self::F32Le),
            "f32.load" => Some(Self::F32Load),
            "f32.lt" => Some(Self::F32Lt),
            "f32.max" => Some(Self::F32Max),
            "f32.min" => Some(Self::F32Min),
            "f32.mul" => Some(Self::F32Mul),
            "f32.ne" => Some(Self::F32Ne),
            "f32.nearest" => Some(Self::F32Nearest),
            "f32.neg" => Some(Self::F32Neg),
            "f32.reinterpret_i32" => Some(Self::F32ReinterpretI32),
            "f32.sqrt" => Some(Self::F32Sqrt),
            "f32.store" => Some(Self::F32Store),
            "f32.sub" => Some(Self::F32Sub),
            "f32.trunc" => Some(Self::F32Trunc),
            "f64.abs" => Some(Self::F64Abs),
            "f64.add" => Some(Self::F64Add),
            "f64.ceil" => Some(Self::F64Ceil),
            "f64.const" => Some(Self::F64Const),
            "f64.convert_i32_s" => Some(Self::F64ConvertI32S),
            "f64.convert_i32_u" => Some(Self::F64ConvertI32U),
            "f64.convert_i64_s" => Some(Self::F64ConvertI64S),
            "f64.convert_i64_u" => Some(Self::F64ConvertI64U),
            "f64.copysign" => Some(Self::F64Copysign),
            "f64.div" => Some(Self::F64Div),
            "f64.eq" => Some(Self::F64Eq),
            "f64.floor" => Some(Self::F64Floor),
            "f64.ge" => Some(Self::F64Ge),
            "f64.gt" => Some(Self::F64Gt),
            "f64.le" => Some(Self::F64Le),
            "f64.load" => Some(Self::F64Load),
            "f64.lt" => Some(Self::F64Lt),
            "f64.max" => Some(Self::F64Max),
            "f64.min" => Some(Self::F64Min),
            "f64.mul" => Some(Self::F64Mul),
            "f64.ne" => Some(Self::F64Ne),
            "f64.nearest" => Some(Self::F64Nearest),
            "f64.neg" => Some(Self::F64Neg),
            "f64.promote_f32" => Some(Self::F64PromoteF32),
            "f64.reinterpret_i64" => Some(Self::F64ReinterpretI64),
            "f64.sqrt" => Some(Self::F64Sqrt),
            "f64.store" => Some(Self::F64Store),
            "f64.sub" => Some(Self::F64Sub),
            "f64.trunc" => Some(Self::F64Trunc),
            "func" => Some(Self::Func),
            "funcref" => Some(Self::Funcref),
            "global" => Some(Self::Global),
            "global.get" => Some(Self::GlobalGet),
            "global.set" => Some(Self::GlobalSet),
            "i32.add" => Some(Self::I32Add),
            "i32.and" => Some(Self::I32And),
            "i32.clz" => Some(Self::I32Clz),
            "i32.const" => Some(Self::I32Const),
            "i32.ctz" => Some(Self::I32Ctz),
            "i32.div_s" => Some(Self::I32DivS),
            "i32.div_u" => Some(Self::I32DivU),
            "i32.eq" => Some(Self::I32Eq),
            "i32.eqz" => Some(Self::I32Eqz),
            "i32.extend16_s" => Some(Self::I32Extend16S),
            "i32.extend8_s" => Some(Self::I32Extend8S),
            "i32.ge_s" => Some(Self::I32GeS),
            "i32.ge_u" => Some(Self::I32GeU),
            "i32.gt_s" => Some(Self::I32GtS),
            "i32.gt_u" => Some(Self::I32GtU),
            "i32.le_s" => Some(Self::I32LeS),
            "i32.le_u" => Some(Self::I32LeU),
            "i32.load" => Some(Self::I32Load),
            "i32.load16_s" => Some(Self::I32Load16S),
            "i32.load16_u" => Some(Self::I32Load16U),
            "i32.load8_s" => Some(Self::I32Load8S),
            "i32.load8_u" => Some(Self::I32Load8U),
            "i32.lt_s" => Some(Self::I32LtS),
            "i32.lt_u" => Some(Self::I32LtU),
            "i32.mul" => Some(Self::I32Mul),
            "i32.ne" => Some(Self::I32Ne),
            "i32.or" => Some(Self::I32Or),
            "i32.popcnt" => Some(Self::I32Popcnt),
            "i32.reinterpret_f32" => Some(Self::I32ReinterpretF32),
            "i32.rem_s" => Some(Self::I32RemS),
            "i32.rem_u" => Some(Self::I32RemU),
            "i32.rotl" => Some(Self::I32Rotl),
            "i32.rotr" => Some(Self::I32Rotr),
            "i32.shl" => Some(Self::I32Shl),
            "i32.shr_s" => Some(Self::I32ShrS),
            "i32.shr_u" => Some(Self::I32ShrU),
            "i32.store" => Some(Self::I32Store),
            "i32.store16" => Some(Self::I32Store16),
            "i32.store8" => Some(Self::I32Store8),
            "i32.sub" => Some(Self::I32Sub),
            "i32.trunc_f32_s" => Some(Self::I32TruncF32S),
            "i32.trunc_f32_u" => Some(Self::I32TruncF32U),
            "i32.trunc_f64_s" => Some(Self::I32TruncF64S),
            "i32.trunc_f64_u" => Some(Self::I32TruncF64U),
            "i32.trunc_sat_f32_s" => Some(Self::I32TruncSatF32S),
            "i32.trunc_sat_f32_u" => Some(Self::I32TruncSatF32U),
            "i32.trunc_sat_f64_s" => Some(Self::I32TruncSatF64S),
            "i32.trunc_sat_f64_u" => Some(Self::I32TruncSatF64U),
            "i32.wrap_i64" => Some(Self::I32WrapI64),
            "i32.xor" => Some(Self::I32Xor),
            "i64.add" => Some(Self::I64Add),
            "i64.and" => Some(Self::I64And),
            "i64.clz" => Some(Self::I64Clz),
            "i64.const" => Some(Self::I64Const),
            "i64.ctz" => Some(Self::I64Ctz),
            "i64.div_s" => Some(Self::I64DivS),
            "i64.div_u" => Some(Self::I64DivU),
            "i64.eq" => Some(Self::I64Eq),
            "i64.eqz" => Some(Self::I64Eqz),
            "i64.extend16_s" => Some(Self::I64Extend16S),
            "i64.extend32_s" => Some(Self::I64Extend32S),
            "i64.extend8_s" => Some(Self::I64Extend8S),
            "i64.extend_i32_s" => Some(Self::I64ExtendI32S),
            "i64.extend_i32_u" => Some(Self::I64ExtendI32U),
            "i64.ge_s" => Some(Self::I64GeS),
            "i64.ge_u" => Some(Self::I64GeU),
            "i64.gt_s" => Some(Self::I64GtS),
            "i64.gt_u" => Some(Self::I64GtU),
            "i64.le_s" => Some(Self::I64LeS),
            "i64.le_u" => Some(Self::I64LeU),
            "i64.load" => Some(Self::I64Load),
            "i64.load16_s" => Some(Self::I64Load16S),
            "i64.load16_u" => Some(Self::I64Load16U),
            "i64.load32_s" => Some(Self::I64Load32S),
            "i64.load32_u" => Some(Self::I64Load32U),
            "i64.load8_s" => Some(Self::I64Load8S),
            "i64.load8_u" => Some(Self::I64Load8U),
            "i64.lt_s" => Some(Self::I64LtS),
            "i64.lt_u" => Some(Self::I64LtU),
            "i64.mul" => Some(Self::I64Mul),
            "i64.ne" => Some(Self::I64Ne),
            "i64.or" => Some(Self::I64Or),
            "i64.popcnt" => Some(Self::I64Popcnt),
            "i64.reinterpret_f64" => Some(Self::I64ReinterpretF64),
            "i64.rem_s" => Some(Self::I64RemS),
            "i64.rem_u" => Some(Self::I64RemU),
            "i64.rotl" => Some(Self::I64Rotl),
            "i64.rotr" => Some(Self::I64Rotr),
            "i64.shl" => Some(Self::I64Shl),
            "i64.shr_s" => Some(Self::I64ShrS),
            "i64.shr_u" => Some(Self::I64ShrU),
            "i64.store" => Some(Self::I64Store),
            "i64.store16" => Some(Self::I64Store16),
            "i64.store32" => Some(Self::I64Store32),
            "i64.store8" => Some(Self::I64Store8),
            "i64.sub" => Some(Self::I64Sub),
            "i64.trunc_f32_s" => Some(Self::I64TruncF32S),
            "i64.trunc_f32_u" => Some(Self::I64TruncF32U),
            "i64.trunc_f64_s" => Some(Self::I64TruncF64S),
            "i64.trunc_f64_u" => Some(Self::I64TruncF64U),
            "i64.trunc_sat_f32_s" => Some(Self::I64TruncSatF32S),
            "i64.trunc_sat_f32_u" => Some(Self::I64TruncSatF32U),
            "i64.trunc_sat_f64_s" => Some(Self::I64TruncSatF64S),
            "i64.trunc_sat_f64_u" => Some(Self::I64TruncSatF64U),
            "i64.xor" => Some(Self::I64Xor),
            "if" => Some(Self::If),
            "import" => Some(Self::Import),
            "item" => Some(Self::Item),
            "local" => Some(Self::Local),
            "local.get" => Some(Self::LocalGet),
            "local.set" => Some(Self::LocalSet),
            "local.tee" => Some(Self::LocalTee),
            "loop" => Some(Self::Loop),
            "memory" => Some(Self::Memory),
            "memory.copy" => Some(Self::MemoryCopy),
            "memory.fill" => Some(Self::MemoryFill),
            "memory.grow" => Some(Self::MemoryGrow),
            "memory.size" => Some(Self::MemorySize),
            "module" => Some(Self::Module),
            "mut" => Some(Self::Mut),
            "nop" => Some(Self::Nop),
            "offset" => Some(Self::Offset),
            "param" => Some(Self::Param),
            "result" => Some(Self::Result),
            "return" => Some(Self::Return),
            "select" => Some(Self::Select),
            "start" => Some(Self::Start),
            "table" => Some(Self::Table),
            "type" => Some(Self::Type),
            "unreachable" => Some(Self::Unreachable),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Anyfunc => "anyfunc",
            Self::Block => "block",
            Self::Br => "br",
            Self::BrIf => "br_if",
            Self::BrTable => "br_table",
            Self::Call => "call",
            Self::CallIndirect => "call_indirect",
            Self::Data => "data",
            Self::Drop => "drop",
            Self::Elem => "elem",
            Self::Else => "else",
            Self::End => "end",
            Self::Export => "export",
            Self::Externref => "externref",
            Self::F32Abs => "f32.abs",
            Self::F32Add => "f32.add",
            Self::F32Ceil => "f32.ceil",
            Self::F32Const => "f32.const",
            Self::F32ConvertI32S => "f32.convert_i32_s",
            Self::F32ConvertI32U => "f32.convert_i32_u",
            Self::F32ConvertI64S => "f32.convert_i64_s",
            Self::F32ConvertI64U => "f32.convert_i64_u",
            Self::F32Copysign => "f32.copysign",
            Self::F32DemoteF64 => "f32.demote_f64",
            Self::F32Div => "f32.div",
            Self::F32Eq => "f32.eq",
            Self::F32Floor => "f32.floor",
            Self::F32Ge => "f32.ge",
            Self::F32Gt => "f32.gt",
            Self::F32Le => "f32.le",
            Self::F32Load => "f32.load",
            Self::F32Lt => "f32.lt",
            Self::F32Max => "f32.max",
            Self::F32Min => "f32.min",
            Self::F32Mul => "f32.mul",
            Self::F32Ne => "f32.ne",
            Self::F32Nearest => "f32.nearest",
            Self::F32Neg => "f32.neg",
            Self::F32ReinterpretI32 => "f32.reinterpret_i32",
            Self::F32Sqrt => "f32.sqrt",
            Self::F32Store => "f32.store",
            Self::F32Sub => "f32.sub",
            Self::F32Trunc => "f32.trunc",
            Self::F64Abs => "f64.abs",
            Self::F64Add => "f64.add",
            Self::F64Ceil => "f64.ceil",
            Self::F64Const => "f64.const",
            Self::F64ConvertI32S => "f64.convert_i32_s",
            Self::F64ConvertI32U => "f64.convert_i32_u",
            Self::F64ConvertI64S => "f64.convert_i64_s",
            Self::F64ConvertI64U => "f64.convert_i64_u",
            Self::F64Copysign => "f64.copysign",
            Self::F64Div => "f64.div",
            Self::F64Eq => "f64.eq",
            Self::F64Floor => "f64.floor",
            Self::F64Ge => "f64.ge",
            Self::F64Gt => "f64.gt",
            Self::F64Le => "f64.le",
            Self::F64Load => "f64.load",
            Self::F64Lt => "f64.lt",
            Self::F64Max => "f64.max",
            Self::F64Min => "f64.min",
            Self::F64Mul => "f64.mul",
            Self::F64Ne => "f64.ne",
            Self::F64Nearest => "f64.nearest",
            Self::F64Neg => "f64.neg",
            Self::F64PromoteF32 => "f64.promote_f32",
            Self::F64ReinterpretI64 => "f64.reinterpret_i64",
            Self::F64Sqrt => "f64.sqrt",
            Self::F64Store => "f64.store",
            Self::F64Sub => "f64.sub",
            Self::F64Trunc => "f64.trunc",
            Self::Func => "func",
            Self::Funcref => "funcref",
            Self::Global => "global",
            Self::GlobalGet => "global.get",
            Self::GlobalSet => "global.set",
            Self::I32Add => "i32.add",
            Self::I32And => "i32.and",
            Self::I32Clz => "i32.clz",
            Self::I32Const => "i32.const",
            Self::I32Ctz => "i32.ctz",
            Self::I32DivS => "i32.div_s",
            Self::I32DivU => "i32.div_u",
            Self::I32Eq => "i32.eq",
            Self::I32Eqz => "i32.eqz",
            Self::I32Extend16S => "i32.extend16_s",
            Self::I32Extend8S => "i32.extend8_s",
            Self::I32GeS => "i32.ge_s",
            Self::I32GeU => "i32.ge_u",
            Self::I32GtS => "i32.gt_s",
            Self::I32GtU => "i32.gt_u",
            Self::I32LeS => "i32.le_s",
            Self::I32LeU => "i32.le_u",
            Self::I32Load => "i32.load",
            Self::I32Load16S => "i32.load16_s",
            Self::I32Load16U => "i32.load16_u",
            Self::I32Load8S => "i32.load8_s",
            Self::I32Load8U => "i32.load8_u",
            Self::I32LtS => "i32.lt_s",
            Self::I32LtU => "i32.lt_u",
            Self::I32Mul => "i32.mul",
            Self::I32Ne => "i32.ne",
            Self::I32Or => "i32.or",
            Self::I32Popcnt => "i32.popcnt",
            Self::I32ReinterpretF32 => "i32.reinterpret_f32",
            Self::I32RemS => "i32.rem_s",
            Self::I32RemU => "i32.rem_u",
            Self::I32Rotl => "i32.rotl",
            Self::I32Rotr => "i32.rotr",
            Self::I32Shl => "i32.shl",
            Self::I32ShrS => "i32.shr_s",
            Self::I32ShrU => "i32.shr_u",
            Self::I32Store => "i32.store",
            Self::I32Store16 => "i32.store16",
            Self::I32Store8 => "i32.store8",
            Self::I32Sub => "i32.sub",
            Self::I32TruncF32S => "i32.trunc_f32_s",
            Self::I32TruncF32U => "i32.trunc_f32_u",
            Self::I32TruncF64S => "i32.trunc_f64_s",
            Self::I32TruncF64U => "i32.trunc_f64_u",
            Self::I32TruncSatF32S => "i32.trunc_sat_f32_s",
            Self::I32TruncSatF32U => "i32.trunc_sat_f32_u",
            Self::I32TruncSatF64S => "i32.trunc_sat_f64_s",
            Self::I32TruncSatF64U => "i32.trunc_sat_f64_u",
            Self::I32WrapI64 => "i32.wrap_i64",
            Self::I32Xor => "i32.xor",
            Self::I64Add => "i64.add",
            Self::I64And => "i64.and",
            Self::I64Clz => "i64.clz",
            Self::I64Const => "i64.const",
            Self::I64Ctz => "i64.ctz",
            Self::I64DivS => "i64.div_s",
            Self::I64DivU => "i64.div_u",
            Self::I64Eq => "i64.eq",
            Self::I64Eqz => "i64.eqz",
            Self::I64Extend16S => "i64.extend16_s",
            Self::I64Extend32S => "i64.extend32_s",
            Self::I64Extend8S => "i64.extend8_s",
            Self::I64ExtendI32S => "i64.extend_i32_s",
            Self::I64ExtendI32U => "i64.extend_i32_u",
            Self::I64GeS => "i64.ge_s",
            Self::I64GeU => "i64.ge_u",
            Self::I64GtS => "i64.gt_s",
            Self::I64GtU => "i64.gt_u",
            Self::I64LeS => "i64.le_s",
            Self::I64LeU => "i64.le_u",
            Self::I64Load => "i64.load",
            Self::I64Load16S => "i64.load16_s",
            Self::I64Load16U => "i64.load16_u",
            Self::I64Load32S => "i64.load32_s",
            Self::I64Load32U => "i64.load32_u",
            Self::I64Load8S => "i64.load8_s",
            Self::I64Load8U => "i64.load8_u",
            Self::I64LtS => "i64.lt_s",
            Self::I64LtU => "i64.lt_u",
            Self::I64Mul => "i64.mul",
            Self::I64Ne => "i64.ne",
            Self::I64Or => "i64.or",
            Self::I64Popcnt => "i64.popcnt",
            Self::I64ReinterpretF64 => "i64.reinterpret_f64",
            Self::I64RemS => "i64.rem_s",
            Self::I64RemU => "i64.rem_u",
            Self::I64Rotl => "i64.rotl",
            Self::I64Rotr => "i64.rotr",
            Self::I64Shl => "i64.shl",
            Self::I64ShrS => "i64.shr_s",
            Self::I64ShrU => "i64.shr_u",
            Self::I64Store => "i64.store",
            Self::I64Store16 => "i64.store16",
            Self::I64Store32 => "i64.store32",
            Self::I64Store8 => "i64.store8",
            Self::I64Sub => "i64.sub",
            Self::I64TruncF32S => "i64.trunc_f32_s",
            Self::I64TruncF32U => "i64.trunc_f32_u",
            Self::I64TruncF64S => "i64.trunc_f64_s",
            Self::I64TruncF64U => "i64.trunc_f64_u",
            Self::I64TruncSatF32S => "i64.trunc_sat_f32_s",
            Self::I64TruncSatF32U => "i64.trunc_sat_f32_u",
            Self::I64TruncSatF64S => "i64.trunc_sat_f64_s",
            Self::I64TruncSatF64U => "i64.trunc_sat_f64_u",
            Self::I64Xor => "i64.xor",
            Self::If => "if",
            Self::Import => "import",
            Self::Item => "item",
            Self::Local => "local",
            Self::LocalGet => "local.get",
            Self::LocalSet => "local.set",
            Self::LocalTee => "local.tee",
            Self::Loop => "loop",
            Self::Memory => "memory",
            Self::MemoryCopy => "memory.copy",
            Self::MemoryFill => "memory.fill",
            Self::MemoryGrow => "memory.grow",
            Self::MemorySize => "memory.size",
            Self::Module => "module",
            Self::Mut => "mut",
            Self::Nop => "nop",
            Self::Offset => "offset",
            Self::Param => "param",
            Self::Result => "result",
            Self::Return => "return",
            Self::Select => "select",
            Self::Start => "start",
            Self::Table => "table",
            Self::Type => "type",
            Self::Unreachable => "unreachable",
        }
    }
}

impl core::fmt::Display for Keyword {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl core::fmt::Debug for Keyword {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:?}", self.as_str())
    }
}
