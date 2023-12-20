use super::WasmProposalType;

/// Multi Bytes Opcodes (FC)
#[non_exhaustive]
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum WasmOpcodeFC {
    /// `FC 00 i32.trunc_sat_f32_s` (nontrapping_float_to_int_conversion)
    I32TruncSatF32S = 0x00,
    /// `FC 01 i32.trunc_sat_f32_u` (nontrapping_float_to_int_conversion)
    I32TruncSatF32U = 0x01,
    /// `FC 02 i32.trunc_sat_f64_s` (nontrapping_float_to_int_conversion)
    I32TruncSatF64S = 0x02,
    /// `FC 03 i32.trunc_sat_f64_u` (nontrapping_float_to_int_conversion)
    I32TruncSatF64U = 0x03,
    /// `FC 04 i64.trunc_sat_f32_s` (nontrapping_float_to_int_conversion)
    I64TruncSatF32S = 0x04,
    /// `FC 05 i64.trunc_sat_f32_u` (nontrapping_float_to_int_conversion)
    I64TruncSatF32U = 0x05,
    /// `FC 06 i64.trunc_sat_f64_s` (nontrapping_float_to_int_conversion)
    I64TruncSatF64S = 0x06,
    /// `FC 07 i64.trunc_sat_f64_u` (nontrapping_float_to_int_conversion)
    I64TruncSatF64U = 0x07,
    /// `FC 08 memory.init segment memory` (bulk_memory_operations)
    MemoryInit = 0x08,
    /// `FC 09 data.drop segment` (bulk_memory_operations)
    DataDrop = 0x09,
    /// `FC 0A memory.copy memory_dst memory_src` (bulk_memory_operations)
    MemoryCopy = 0x0A,
    /// `FC 0B memory.fill memory` (bulk_memory_operations)
    MemoryFill = 0x0B,
    /// `FC 0C table.init segment table` (bulk_memory_operations)
    TableInit = 0x0C,
    /// `FC 0D elem.drop segment` (bulk_memory_operations)
    ElemDrop = 0x0D,
    /// `FC 0E table.copy table_dst table_src` (bulk_memory_operations)
    TableCopy = 0x0E,
}

impl WasmOpcodeFC {
    pub const fn new(value: u32) -> Option<Self> {
        match value {
            0x00 => Some(Self::I32TruncSatF32S),
            0x01 => Some(Self::I32TruncSatF32U),
            0x02 => Some(Self::I32TruncSatF64S),
            0x03 => Some(Self::I32TruncSatF64U),
            0x04 => Some(Self::I64TruncSatF32S),
            0x05 => Some(Self::I64TruncSatF32U),
            0x06 => Some(Self::I64TruncSatF64S),
            0x07 => Some(Self::I64TruncSatF64U),
            0x08 => Some(Self::MemoryInit),
            0x09 => Some(Self::DataDrop),
            0x0A => Some(Self::MemoryCopy),
            0x0B => Some(Self::MemoryFill),
            0x0C => Some(Self::TableInit),
            0x0D => Some(Self::ElemDrop),
            0x0E => Some(Self::TableCopy),
            _ => None,
        }
    }

    pub const fn to_str(&self) -> &str {
        match *self {
            Self::I32TruncSatF32S => "i32.trunc_sat_f32_s",
            Self::I32TruncSatF32U => "i32.trunc_sat_f32_u",
            Self::I32TruncSatF64S => "i32.trunc_sat_f64_s",
            Self::I32TruncSatF64U => "i32.trunc_sat_f64_u",
            Self::I64TruncSatF32S => "i64.trunc_sat_f32_s",
            Self::I64TruncSatF32U => "i64.trunc_sat_f32_u",
            Self::I64TruncSatF64S => "i64.trunc_sat_f64_s",
            Self::I64TruncSatF64U => "i64.trunc_sat_f64_u",
            Self::MemoryInit => "memory.init",
            Self::DataDrop => "data.drop",
            Self::MemoryCopy => "memory.copy",
            Self::MemoryFill => "memory.fill",
            Self::TableInit => "table.init",
            Self::ElemDrop => "elem.drop",
            Self::TableCopy => "table.copy",
        }
    }

    pub const fn proposal_type(&self) -> WasmProposalType {
        match *self {
            Self::I32TruncSatF32S
            | Self::I32TruncSatF32U
            | Self::I32TruncSatF64S
            | Self::I32TruncSatF64U
            | Self::I64TruncSatF32S
            | Self::I64TruncSatF32U
            | Self::I64TruncSatF64S
            | Self::I64TruncSatF64U => WasmProposalType::NontrappingFloatToIntConversion,
            Self::MemoryInit
            | Self::DataDrop
            | Self::MemoryCopy
            | Self::MemoryFill
            | Self::TableInit
            | Self::ElemDrop
            | Self::TableCopy => WasmProposalType::BulkMemoryOperations,
        }
    }
}
