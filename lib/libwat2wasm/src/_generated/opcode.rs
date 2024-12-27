//! WebAssembly opcodes

/* This file is generated automatically. DO NOT EDIT DIRECTLY. */

use crate::types::ValType;

/// WebAssembly opcodes
#[non_exhaustive]
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum WasmOpcode {
    /// 0x00 `unreachable`
    Unreachable,
    /// 0x01 `nop`
    Nop,
    /// 0x02 `block`
    Block,
    /// 0x03 `loop`
    Loop,
    /// 0x04 `if`
    If,
    /// 0x05 `else`
    Else,
    /// 0x0B `end`
    End,
    /// 0x0C `br`
    Br,
    /// 0x0D `br_if`
    BrIf,
    /// 0x0E `br_table`
    BrTable,
    /// 0x0F `return`
    Return,
    /// 0x10 `call`
    Call,
    /// 0x11 `call_indirect`
    CallIndirect,
    /// 0x1A `drop`
    Drop,
    /// 0x1B `select`
    Select,
    /// 0x20 `local.get`
    LocalGet,
    /// 0x21 `local.set`
    LocalSet,
    /// 0x22 `local.tee`
    LocalTee,
    /// 0x23 `global.get`
    GlobalGet,
    /// 0x24 `global.set`
    GlobalSet,
    /// 0x28 `i32.load`
    I32Load,
    /// 0x29 `i64.load`
    I64Load,
    /// 0x2A `f32.load`
    F32Load,
    /// 0x2B `f64.load`
    F64Load,
    /// 0x2C `i32.load8_s`
    I32Load8S,
    /// 0x2D `i32.load8_u`
    I32Load8U,
    /// 0x2E `i32.load16_s`
    I32Load16S,
    /// 0x2F `i32.load16_u`
    I32Load16U,
    /// 0x30 `i64.load8_s`
    I64Load8S,
    /// 0x31 `i64.load8_u`
    I64Load8U,
    /// 0x32 `i64.load16_s`
    I64Load16S,
    /// 0x33 `i64.load16_u`
    I64Load16U,
    /// 0x34 `i64.load32_s`
    I64Load32S,
    /// 0x35 `i64.load32_u`
    I64Load32U,
    /// 0x36 `i32.store`
    I32Store,
    /// 0x37 `i64.store`
    I64Store,
    /// 0x38 `f32.store`
    F32Store,
    /// 0x39 `f64.store`
    F64Store,
    /// 0x3A `i32.store8`
    I32Store8,
    /// 0x3B `i32.store16`
    I32Store16,
    /// 0x3C `i64.store8`
    I64Store8,
    /// 0x3D `i64.store16`
    I64Store16,
    /// 0x3E `i64.store32`
    I64Store32,
    /// 0x3F `memory.size`
    MemorySize,
    /// 0x40 `memory.grow`
    MemoryGrow,
    /// 0x41 `i32.const`
    I32Const,
    /// 0x42 `i64.const`
    I64Const,
    /// 0x43 `f32.const`
    F32Const,
    /// 0x44 `f64.const`
    F64Const,
    /// 0x45 `i32.eqz`
    I32Eqz,
    /// 0x46 `i32.eq`
    I32Eq,
    /// 0x47 `i32.ne`
    I32Ne,
    /// 0x48 `i32.lt_s`
    I32LtS,
    /// 0x49 `i32.lt_u`
    I32LtU,
    /// 0x4A `i32.gt_s`
    I32GtS,
    /// 0x4B `i32.gt_u`
    I32GtU,
    /// 0x4C `i32.le_s`
    I32LeS,
    /// 0x4D `i32.le_u`
    I32LeU,
    /// 0x4E `i32.ge_s`
    I32GeS,
    /// 0x4F `i32.ge_u`
    I32GeU,
    /// 0x50 `i64.eqz`
    I64Eqz,
    /// 0x51 `i64.eq`
    I64Eq,
    /// 0x52 `i64.ne`
    I64Ne,
    /// 0x53 `i64.lt_s`
    I64LtS,
    /// 0x54 `i64.lt_u`
    I64LtU,
    /// 0x55 `i64.gt_s`
    I64GtS,
    /// 0x56 `i64.gt_u`
    I64GtU,
    /// 0x57 `i64.le_s`
    I64LeS,
    /// 0x58 `i64.le_u`
    I64LeU,
    /// 0x59 `i64.ge_s`
    I64GeS,
    /// 0x5A `i64.ge_u`
    I64GeU,
    /// 0x5B `f32.eq`
    F32Eq,
    /// 0x5C `f32.ne`
    F32Ne,
    /// 0x5D `f32.lt`
    F32Lt,
    /// 0x5E `f32.gt`
    F32Gt,
    /// 0x5F `f32.le`
    F32Le,
    /// 0x60 `f32.ge`
    F32Ge,
    /// 0x61 `f64.eq`
    F64Eq,
    /// 0x62 `f64.ne`
    F64Ne,
    /// 0x63 `f64.lt`
    F64Lt,
    /// 0x64 `f64.gt`
    F64Gt,
    /// 0x65 `f64.le`
    F64Le,
    /// 0x66 `f64.ge`
    F64Ge,
    /// 0x67 `i32.clz`
    I32Clz,
    /// 0x68 `i32.ctz`
    I32Ctz,
    /// 0x69 `i32.popcnt`
    I32Popcnt,
    /// 0x6A `i32.add`
    I32Add,
    /// 0x6B `i32.sub`
    I32Sub,
    /// 0x6C `i32.mul`
    I32Mul,
    /// 0x6D `i32.div_s`
    I32DivS,
    /// 0x6E `i32.div_u`
    I32DivU,
    /// 0x6F `i32.rem_s`
    I32RemS,
    /// 0x70 `i32.rem_u`
    I32RemU,
    /// 0x71 `i32.and`
    I32And,
    /// 0x72 `i32.or`
    I32Or,
    /// 0x73 `i32.xor`
    I32Xor,
    /// 0x74 `i32.shl`
    I32Shl,
    /// 0x75 `i32.shr_s`
    I32ShrS,
    /// 0x76 `i32.shr_u`
    I32ShrU,
    /// 0x77 `i32.rotl`
    I32Rotl,
    /// 0x78 `i32.rotr`
    I32Rotr,
    /// 0x79 `i64.clz`
    I64Clz,
    /// 0x7A `i64.ctz`
    I64Ctz,
    /// 0x7B `i64.popcnt`
    I64Popcnt,
    /// 0x7C `i64.add`
    I64Add,
    /// 0x7D `i64.sub`
    I64Sub,
    /// 0x7E `i64.mul`
    I64Mul,
    /// 0x7F `i64.div_s`
    I64DivS,
    /// 0x80 `i64.div_u`
    I64DivU,
    /// 0x81 `i64.rem_s`
    I64RemS,
    /// 0x82 `i64.rem_u`
    I64RemU,
    /// 0x83 `i64.and`
    I64And,
    /// 0x84 `i64.or`
    I64Or,
    /// 0x85 `i64.xor`
    I64Xor,
    /// 0x86 `i64.shl`
    I64Shl,
    /// 0x87 `i64.shr_s`
    I64ShrS,
    /// 0x88 `i64.shr_u`
    I64ShrU,
    /// 0x89 `i64.rotl`
    I64Rotl,
    /// 0x8A `i64.rotr`
    I64Rotr,
    /// 0x8B `f32.abs`
    F32Abs,
    /// 0x8C `f32.neg`
    F32Neg,
    /// 0x8D `f32.ceil`
    F32Ceil,
    /// 0x8E `f32.floor`
    F32Floor,
    /// 0x8F `f32.trunc`
    F32Trunc,
    /// 0x90 `f32.nearest`
    F32Nearest,
    /// 0x91 `f32.sqrt`
    F32Sqrt,
    /// 0x92 `f32.add`
    F32Add,
    /// 0x93 `f32.sub`
    F32Sub,
    /// 0x94 `f32.mul`
    F32Mul,
    /// 0x95 `f32.div`
    F32Div,
    /// 0x96 `f32.min`
    F32Min,
    /// 0x97 `f32.max`
    F32Max,
    /// 0x98 `f32.copysign`
    F32Copysign,
    /// 0x99 `f64.abs`
    F64Abs,
    /// 0x9A `f64.neg`
    F64Neg,
    /// 0x9B `f64.ceil`
    F64Ceil,
    /// 0x9C `f64.floor`
    F64Floor,
    /// 0x9D `f64.trunc`
    F64Trunc,
    /// 0x9E `f64.nearest`
    F64Nearest,
    /// 0x9F `f64.sqrt`
    F64Sqrt,
    /// 0xA0 `f64.add`
    F64Add,
    /// 0xA1 `f64.sub`
    F64Sub,
    /// 0xA2 `f64.mul`
    F64Mul,
    /// 0xA3 `f64.div`
    F64Div,
    /// 0xA4 `f64.min`
    F64Min,
    /// 0xA5 `f64.max`
    F64Max,
    /// 0xA6 `f64.copysign`
    F64Copysign,
    /// 0xA7 `i32.wrap_i64`
    I32WrapI64,
    /// 0xA8 `i32.trunc_f32_s`
    I32TruncF32S,
    /// 0xA9 `i32.trunc_f32_u`
    I32TruncF32U,
    /// 0xAA `i32.trunc_f64_s`
    I32TruncF64S,
    /// 0xAB `i32.trunc_f64_u`
    I32TruncF64U,
    /// 0xAC `i64.extend_i32_s`
    I64ExtendI32S,
    /// 0xAD `i64.extend_i32_u`
    I64ExtendI32U,
    /// 0xAE `i64.trunc_f32_s`
    I64TruncF32S,
    /// 0xAF `i64.trunc_f32_u`
    I64TruncF32U,
    /// 0xB0 `i64.trunc_f64_s`
    I64TruncF64S,
    /// 0xB1 `i64.trunc_f64_u`
    I64TruncF64U,
    /// 0xB2 `f32.convert_i32_s`
    F32ConvertI32S,
    /// 0xB3 `f32.convert_i32_u`
    F32ConvertI32U,
    /// 0xB4 `f32.convert_i64_s`
    F32ConvertI64S,
    /// 0xB5 `f32.convert_i64_u`
    F32ConvertI64U,
    /// 0xB6 `f32.demote_f64`
    F32DemoteF64,
    /// 0xB7 `f64.convert_i32_s`
    F64ConvertI32S,
    /// 0xB8 `f64.convert_i32_u`
    F64ConvertI32U,
    /// 0xB9 `f64.convert_i64_s`
    F64ConvertI64S,
    /// 0xBA `f64.convert_i64_u`
    F64ConvertI64U,
    /// 0xBB `f64.promote_f32`
    F64PromoteF32,
    /// 0xBC `i32.reinterpret_f32`
    I32ReinterpretF32,
    /// 0xBD `i64.reinterpret_f64`
    I64ReinterpretF64,
    /// 0xBE `f32.reinterpret_i32`
    F32ReinterpretI32,
    /// 0xBF `f64.reinterpret_i64`
    F64ReinterpretI64,
    /// 0xC0 `i32.extend8_s`
    I32Extend8S,
    /// 0xC1 `i32.extend16_s`
    I32Extend16S,
    /// 0xC2 `i64.extend8_s`
    I64Extend8S,
    /// 0xC3 `i64.extend16_s`
    I64Extend16S,
    /// 0xC4 `i64.extend32_s`
    I64Extend32S,
    /// 0xFC 0x00 `i32.trunc_sat_f32_s`
    I32TruncSatF32S,
    /// 0xFC 0x01 `i32.trunc_sat_f32_u`
    I32TruncSatF32U,
    /// 0xFC 0x02 `i32.trunc_sat_f64_s`
    I32TruncSatF64S,
    /// 0xFC 0x03 `i32.trunc_sat_f64_u`
    I32TruncSatF64U,
    /// 0xFC 0x04 `i64.trunc_sat_f32_s`
    I64TruncSatF32S,
    /// 0xFC 0x05 `i64.trunc_sat_f32_u`
    I64TruncSatF32U,
    /// 0xFC 0x06 `i64.trunc_sat_f64_s`
    I64TruncSatF64S,
    /// 0xFC 0x07 `i64.trunc_sat_f64_u`
    I64TruncSatF64U,
    /// 0xFC 0x0A `memory.copy`
    MemoryCopy,
    /// 0xFC 0x0B `memory.fill`
    MemoryFill,
}

impl WasmOpcode {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Block => "block",
            Self::Br => "br",
            Self::BrIf => "br_if",
            Self::BrTable => "br_table",
            Self::Call => "call",
            Self::CallIndirect => "call_indirect",
            Self::Drop => "drop",
            Self::Else => "else",
            Self::End => "end",
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
            Self::LocalGet => "local.get",
            Self::LocalSet => "local.set",
            Self::LocalTee => "local.tee",
            Self::Loop => "loop",
            Self::MemoryCopy => "memory.copy",
            Self::MemoryFill => "memory.fill",
            Self::MemoryGrow => "memory.grow",
            Self::MemorySize => "memory.size",
            Self::Nop => "nop",
            Self::Return => "return",
            Self::Select => "select",
            Self::Unreachable => "unreachable",
        }
    }

    pub fn from_str(v: &str) -> Option<Self> {
        match v {
            "block" => Some(Self::Block),
            "br" => Some(Self::Br),
            "br_if" => Some(Self::BrIf),
            "br_table" => Some(Self::BrTable),
            "call" => Some(Self::Call),
            "call_indirect" => Some(Self::CallIndirect),
            "drop" => Some(Self::Drop),
            "else" => Some(Self::Else),
            "end" => Some(Self::End),
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
            "local.get" => Some(Self::LocalGet),
            "local.set" => Some(Self::LocalSet),
            "local.tee" => Some(Self::LocalTee),
            "loop" => Some(Self::Loop),
            "memory.copy" => Some(Self::MemoryCopy),
            "memory.fill" => Some(Self::MemoryFill),
            "memory.grow" => Some(Self::MemoryGrow),
            "memory.size" => Some(Self::MemorySize),
            "nop" => Some(Self::Nop),
            "return" => Some(Self::Return),
            "select" => Some(Self::Select),
            "unreachable" => Some(Self::Unreachable),
           _=> None,
        }
    }

    pub const fn leading_byte(&self) -> u8 {
        match self {
            Self::Block => 2,
            Self::Br => 12,
            Self::BrIf => 13,
            Self::BrTable => 14,
            Self::Call => 16,
            Self::CallIndirect => 17,
            Self::Drop => 26,
            Self::Else => 5,
            Self::End => 11,
            Self::F32Abs => 139,
            Self::F32Add => 146,
            Self::F32Ceil => 141,
            Self::F32Const => 67,
            Self::F32ConvertI32S => 178,
            Self::F32ConvertI32U => 179,
            Self::F32ConvertI64S => 180,
            Self::F32ConvertI64U => 181,
            Self::F32Copysign => 152,
            Self::F32DemoteF64 => 182,
            Self::F32Div => 149,
            Self::F32Eq => 91,
            Self::F32Floor => 142,
            Self::F32Ge => 96,
            Self::F32Gt => 94,
            Self::F32Le => 95,
            Self::F32Load => 42,
            Self::F32Lt => 93,
            Self::F32Max => 151,
            Self::F32Min => 150,
            Self::F32Mul => 148,
            Self::F32Ne => 92,
            Self::F32Nearest => 144,
            Self::F32Neg => 140,
            Self::F32ReinterpretI32 => 190,
            Self::F32Sqrt => 145,
            Self::F32Store => 56,
            Self::F32Sub => 147,
            Self::F32Trunc => 143,
            Self::F64Abs => 153,
            Self::F64Add => 160,
            Self::F64Ceil => 155,
            Self::F64Const => 68,
            Self::F64ConvertI32S => 183,
            Self::F64ConvertI32U => 184,
            Self::F64ConvertI64S => 185,
            Self::F64ConvertI64U => 186,
            Self::F64Copysign => 166,
            Self::F64Div => 163,
            Self::F64Eq => 97,
            Self::F64Floor => 156,
            Self::F64Ge => 102,
            Self::F64Gt => 100,
            Self::F64Le => 101,
            Self::F64Load => 43,
            Self::F64Lt => 99,
            Self::F64Max => 165,
            Self::F64Min => 164,
            Self::F64Mul => 162,
            Self::F64Ne => 98,
            Self::F64Nearest => 158,
            Self::F64Neg => 154,
            Self::F64PromoteF32 => 187,
            Self::F64ReinterpretI64 => 191,
            Self::F64Sqrt => 159,
            Self::F64Store => 57,
            Self::F64Sub => 161,
            Self::F64Trunc => 157,
            Self::GlobalGet => 35,
            Self::GlobalSet => 36,
            Self::I32Add => 106,
            Self::I32And => 113,
            Self::I32Clz => 103,
            Self::I32Const => 65,
            Self::I32Ctz => 104,
            Self::I32DivS => 109,
            Self::I32DivU => 110,
            Self::I32Eq => 70,
            Self::I32Eqz => 69,
            Self::I32Extend16S => 193,
            Self::I32Extend8S => 192,
            Self::I32GeS => 78,
            Self::I32GeU => 79,
            Self::I32GtS => 74,
            Self::I32GtU => 75,
            Self::I32LeS => 76,
            Self::I32LeU => 77,
            Self::I32Load => 40,
            Self::I32Load16S => 46,
            Self::I32Load16U => 47,
            Self::I32Load8S => 44,
            Self::I32Load8U => 45,
            Self::I32LtS => 72,
            Self::I32LtU => 73,
            Self::I32Mul => 108,
            Self::I32Ne => 71,
            Self::I32Or => 114,
            Self::I32Popcnt => 105,
            Self::I32ReinterpretF32 => 188,
            Self::I32RemS => 111,
            Self::I32RemU => 112,
            Self::I32Rotl => 119,
            Self::I32Rotr => 120,
            Self::I32Shl => 116,
            Self::I32ShrS => 117,
            Self::I32ShrU => 118,
            Self::I32Store => 54,
            Self::I32Store16 => 59,
            Self::I32Store8 => 58,
            Self::I32Sub => 107,
            Self::I32TruncF32S => 168,
            Self::I32TruncF32U => 169,
            Self::I32TruncF64S => 170,
            Self::I32TruncF64U => 171,
            Self::I32TruncSatF32S => 252,
            Self::I32TruncSatF32U => 252,
            Self::I32TruncSatF64S => 252,
            Self::I32TruncSatF64U => 252,
            Self::I32WrapI64 => 167,
            Self::I32Xor => 115,
            Self::I64Add => 124,
            Self::I64And => 131,
            Self::I64Clz => 121,
            Self::I64Const => 66,
            Self::I64Ctz => 122,
            Self::I64DivS => 127,
            Self::I64DivU => 128,
            Self::I64Eq => 81,
            Self::I64Eqz => 80,
            Self::I64Extend16S => 195,
            Self::I64Extend32S => 196,
            Self::I64Extend8S => 194,
            Self::I64ExtendI32S => 172,
            Self::I64ExtendI32U => 173,
            Self::I64GeS => 89,
            Self::I64GeU => 90,
            Self::I64GtS => 85,
            Self::I64GtU => 86,
            Self::I64LeS => 87,
            Self::I64LeU => 88,
            Self::I64Load => 41,
            Self::I64Load16S => 50,
            Self::I64Load16U => 51,
            Self::I64Load32S => 52,
            Self::I64Load32U => 53,
            Self::I64Load8S => 48,
            Self::I64Load8U => 49,
            Self::I64LtS => 83,
            Self::I64LtU => 84,
            Self::I64Mul => 126,
            Self::I64Ne => 82,
            Self::I64Or => 132,
            Self::I64Popcnt => 123,
            Self::I64ReinterpretF64 => 189,
            Self::I64RemS => 129,
            Self::I64RemU => 130,
            Self::I64Rotl => 137,
            Self::I64Rotr => 138,
            Self::I64Shl => 134,
            Self::I64ShrS => 135,
            Self::I64ShrU => 136,
            Self::I64Store => 55,
            Self::I64Store16 => 61,
            Self::I64Store32 => 62,
            Self::I64Store8 => 60,
            Self::I64Sub => 125,
            Self::I64TruncF32S => 174,
            Self::I64TruncF32U => 175,
            Self::I64TruncF64S => 176,
            Self::I64TruncF64U => 177,
            Self::I64TruncSatF32S => 252,
            Self::I64TruncSatF32U => 252,
            Self::I64TruncSatF64S => 252,
            Self::I64TruncSatF64U => 252,
            Self::I64Xor => 133,
            Self::If => 4,
            Self::LocalGet => 32,
            Self::LocalSet => 33,
            Self::LocalTee => 34,
            Self::Loop => 3,
            Self::MemoryCopy => 252,
            Self::MemoryFill => 252,
            Self::MemoryGrow => 64,
            Self::MemorySize => 63,
            Self::Nop => 1,
            Self::Return => 15,
            Self::Select => 27,
            Self::Unreachable => 0,
        }
    }

    pub const fn trailing_word(&self) -> Option<u32> {
        match self {
            Self::I32TruncSatF32S => Some(0),
            Self::I32TruncSatF32U => Some(1),
            Self::I32TruncSatF64S => Some(2),
            Self::I32TruncSatF64U => Some(3),
            Self::I64TruncSatF32S => Some(4),
            Self::I64TruncSatF32U => Some(5),
            Self::I64TruncSatF64S => Some(6),
            Self::I64TruncSatF64U => Some(7),
            Self::MemoryCopy => Some(10),
            Self::MemoryFill => Some(11),
            _ => None
        }
    }

    /// Returns stack I/O definitions for general data instructions that can define context-independent stack I/O.
    pub fn stack_io(&self) -> Option<(&[ValType], &[ValType])> {
        use ValType::*;
        match self {
            Self::I32Load => Some((&[I32], &[I32])),
            Self::I64Load => Some((&[I32], &[I64])),
            Self::F32Load => Some((&[I32], &[F32])),
            Self::F64Load => Some((&[I32], &[F64])),
            Self::I32Load8S => Some((&[I32], &[I32])),
            Self::I32Load8U => Some((&[I32], &[I32])),
            Self::I32Load16S => Some((&[I32], &[I32])),
            Self::I32Load16U => Some((&[I32], &[I32])),
            Self::I64Load8S => Some((&[I32], &[I64])),
            Self::I64Load8U => Some((&[I32], &[I64])),
            Self::I64Load16S => Some((&[I32], &[I64])),
            Self::I64Load16U => Some((&[I32], &[I64])),
            Self::I64Load32S => Some((&[I32], &[I64])),
            Self::I64Load32U => Some((&[I32], &[I64])),
            Self::I32Store => Some((&[I32, I32], &[])),
            Self::I64Store => Some((&[I32, I64], &[])),
            Self::F32Store => Some((&[I32, F32], &[])),
            Self::F64Store => Some((&[I32, F64], &[])),
            Self::I32Store8 => Some((&[I32, I32], &[])),
            Self::I32Store16 => Some((&[I32, I32], &[])),
            Self::I64Store8 => Some((&[I32, I64], &[])),
            Self::I64Store16 => Some((&[I32, I64], &[])),
            Self::I64Store32 => Some((&[I32, I64], &[])),
            Self::I32Const => Some((&[], &[I32])),
            Self::I64Const => Some((&[], &[I64])),
            Self::F32Const => Some((&[], &[F32])),
            Self::F64Const => Some((&[], &[F64])),
            Self::I32Eqz => Some((&[I32], &[I32])),
            Self::I32Eq => Some((&[I32, I32], &[I32])),
            Self::I32Ne => Some((&[I32, I32], &[I32])),
            Self::I32LtS => Some((&[I32, I32], &[I32])),
            Self::I32LtU => Some((&[I32, I32], &[I32])),
            Self::I32GtS => Some((&[I32, I32], &[I32])),
            Self::I32GtU => Some((&[I32, I32], &[I32])),
            Self::I32LeS => Some((&[I32, I32], &[I32])),
            Self::I32LeU => Some((&[I32, I32], &[I32])),
            Self::I32GeS => Some((&[I32, I32], &[I32])),
            Self::I32GeU => Some((&[I32, I32], &[I32])),
            Self::I64Eqz => Some((&[I64], &[I32])),
            Self::I64Eq => Some((&[I64, I64], &[I32])),
            Self::I64Ne => Some((&[I64, I64], &[I32])),
            Self::I64LtS => Some((&[I64, I64], &[I32])),
            Self::I64LtU => Some((&[I64, I64], &[I32])),
            Self::I64GtS => Some((&[I64, I64], &[I32])),
            Self::I64GtU => Some((&[I64, I64], &[I32])),
            Self::I64LeS => Some((&[I64, I64], &[I32])),
            Self::I64LeU => Some((&[I64, I64], &[I32])),
            Self::I64GeS => Some((&[I64, I64], &[I32])),
            Self::I64GeU => Some((&[I64, I64], &[I32])),
            Self::F32Eq => Some((&[F32, F32], &[I32])),
            Self::F32Ne => Some((&[F32, F32], &[I32])),
            Self::F32Lt => Some((&[F32, F32], &[I32])),
            Self::F32Gt => Some((&[F32, F32], &[I32])),
            Self::F32Le => Some((&[F32, F32], &[I32])),
            Self::F32Ge => Some((&[F32, F32], &[I32])),
            Self::F64Eq => Some((&[F64, F64], &[I32])),
            Self::F64Ne => Some((&[F64, F64], &[I32])),
            Self::F64Lt => Some((&[F64, F64], &[I32])),
            Self::F64Gt => Some((&[F64, F64], &[I32])),
            Self::F64Le => Some((&[F64, F64], &[I32])),
            Self::F64Ge => Some((&[F64, F64], &[I32])),
            Self::I32Clz => Some((&[I32], &[I32])),
            Self::I32Ctz => Some((&[I32], &[I32])),
            Self::I32Popcnt => Some((&[I32], &[I32])),
            Self::I32Add => Some((&[I32, I32], &[I32])),
            Self::I32Sub => Some((&[I32, I32], &[I32])),
            Self::I32Mul => Some((&[I32, I32], &[I32])),
            Self::I32DivS => Some((&[I32, I32], &[I32])),
            Self::I32DivU => Some((&[I32, I32], &[I32])),
            Self::I32RemS => Some((&[I32, I32], &[I32])),
            Self::I32RemU => Some((&[I32, I32], &[I32])),
            Self::I32And => Some((&[I32, I32], &[I32])),
            Self::I32Or => Some((&[I32, I32], &[I32])),
            Self::I32Xor => Some((&[I32, I32], &[I32])),
            Self::I32Shl => Some((&[I32, I32], &[I32])),
            Self::I32ShrS => Some((&[I32, I32], &[I32])),
            Self::I32ShrU => Some((&[I32, I32], &[I32])),
            Self::I32Rotl => Some((&[I32, I32], &[I32])),
            Self::I32Rotr => Some((&[I32, I32], &[I32])),
            Self::I64Clz => Some((&[I64], &[I64])),
            Self::I64Ctz => Some((&[I64], &[I64])),
            Self::I64Popcnt => Some((&[I64], &[I64])),
            Self::I64Add => Some((&[I64, I64], &[I64])),
            Self::I64Sub => Some((&[I64, I64], &[I64])),
            Self::I64Mul => Some((&[I64, I64], &[I64])),
            Self::I64DivS => Some((&[I64, I64], &[I64])),
            Self::I64DivU => Some((&[I64, I64], &[I64])),
            Self::I64RemS => Some((&[I64, I64], &[I64])),
            Self::I64RemU => Some((&[I64, I64], &[I64])),
            Self::I64And => Some((&[I64, I64], &[I64])),
            Self::I64Or => Some((&[I64, I64], &[I64])),
            Self::I64Xor => Some((&[I64, I64], &[I64])),
            Self::I64Shl => Some((&[I64, I64], &[I64])),
            Self::I64ShrS => Some((&[I64, I64], &[I64])),
            Self::I64ShrU => Some((&[I64, I64], &[I64])),
            Self::I64Rotl => Some((&[I64, I64], &[I64])),
            Self::I64Rotr => Some((&[I64, I64], &[I64])),
            Self::F32Abs => Some((&[F32], &[F32])),
            Self::F32Neg => Some((&[F32], &[F32])),
            Self::F32Ceil => Some((&[F32], &[F32])),
            Self::F32Floor => Some((&[F32], &[F32])),
            Self::F32Trunc => Some((&[F32], &[F32])),
            Self::F32Nearest => Some((&[F32], &[F32])),
            Self::F32Sqrt => Some((&[F32], &[F32])),
            Self::F32Add => Some((&[F32, F32], &[F32])),
            Self::F32Sub => Some((&[F32, F32], &[F32])),
            Self::F32Mul => Some((&[F32, F32], &[F32])),
            Self::F32Div => Some((&[F32, F32], &[F32])),
            Self::F32Min => Some((&[F32, F32], &[F32])),
            Self::F32Max => Some((&[F32, F32], &[F32])),
            Self::F32Copysign => Some((&[F32, F32], &[F32])),
            Self::F64Abs => Some((&[F64], &[F64])),
            Self::F64Neg => Some((&[F64], &[F64])),
            Self::F64Ceil => Some((&[F64], &[F64])),
            Self::F64Floor => Some((&[F64], &[F64])),
            Self::F64Trunc => Some((&[F64], &[F64])),
            Self::F64Nearest => Some((&[F64], &[F64])),
            Self::F64Sqrt => Some((&[F64], &[F64])),
            Self::F64Add => Some((&[F64, F64], &[F64])),
            Self::F64Sub => Some((&[F64, F64], &[F64])),
            Self::F64Mul => Some((&[F64, F64], &[F64])),
            Self::F64Div => Some((&[F64, F64], &[F64])),
            Self::F64Min => Some((&[F64, F64], &[F64])),
            Self::F64Max => Some((&[F64, F64], &[F64])),
            Self::F64Copysign => Some((&[F64, F64], &[F64])),
            Self::I32WrapI64 => Some((&[I64], &[I32])),
            Self::I32TruncF32S => Some((&[F32], &[I32])),
            Self::I32TruncF32U => Some((&[F32], &[I32])),
            Self::I32TruncF64S => Some((&[F64], &[I32])),
            Self::I32TruncF64U => Some((&[F64], &[I32])),
            Self::I64ExtendI32S => Some((&[I32], &[I64])),
            Self::I64ExtendI32U => Some((&[I32], &[I64])),
            Self::I64TruncF32S => Some((&[F32], &[I64])),
            Self::I64TruncF32U => Some((&[F32], &[I64])),
            Self::I64TruncF64S => Some((&[F64], &[I64])),
            Self::I64TruncF64U => Some((&[F64], &[I64])),
            Self::F32ConvertI32S => Some((&[I32], &[F32])),
            Self::F32ConvertI32U => Some((&[I32], &[F32])),
            Self::F32ConvertI64S => Some((&[I64], &[F32])),
            Self::F32ConvertI64U => Some((&[I64], &[F32])),
            Self::F32DemoteF64 => Some((&[F64], &[F32])),
            Self::F64ConvertI32S => Some((&[I32], &[F64])),
            Self::F64ConvertI32U => Some((&[I32], &[F64])),
            Self::F64ConvertI64S => Some((&[I64], &[F64])),
            Self::F64ConvertI64U => Some((&[I64], &[F64])),
            Self::F64PromoteF32 => Some((&[F32], &[F64])),
            Self::I32ReinterpretF32 => Some((&[F32], &[I32])),
            Self::I64ReinterpretF64 => Some((&[F64], &[I64])),
            Self::F32ReinterpretI32 => Some((&[I32], &[F32])),
            Self::F64ReinterpretI64 => Some((&[I64], &[F64])),
            Self::I32Extend8S => Some((&[I32], &[I32])),
            Self::I32Extend16S => Some((&[I32], &[I32])),
            Self::I64Extend8S => Some((&[I64], &[I64])),
            Self::I64Extend16S => Some((&[I64], &[I64])),
            Self::I64Extend32S => Some((&[I64], &[I64])),
            Self::I32TruncSatF32S => Some((&[F32], &[I32])),
            Self::I32TruncSatF32U => Some((&[F32], &[I32])),
            Self::I32TruncSatF64S => Some((&[F64], &[I32])),
            Self::I32TruncSatF64U => Some((&[F64], &[I32])),
            Self::I64TruncSatF32S => Some((&[F32], &[I64])),
            Self::I64TruncSatF32U => Some((&[F32], &[I64])),
            Self::I64TruncSatF64S => Some((&[F64], &[I64])),
            Self::I64TruncSatF64U => Some((&[F64], &[I64])),
            _ => None
        }
    }

}

impl core::fmt::Debug for WasmOpcode {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl core::fmt::Display for WasmOpcode {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(self.as_str())
    }
}
