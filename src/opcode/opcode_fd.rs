use super::WasmProposalType;

/// Multi Bytes Opcodes (FD-SIMD)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WasmOpcodeFD {
    // TODO:
    /// `FD 00 v128.load` (simd)
    V128Load = 0x00,
}

impl WasmOpcodeFD {
    pub const fn new(_value: u32) -> Option<Self> {
        // TODO:
        None
    }

    pub const fn to_str(&self) -> &str {
        "(simd)"
    }

    pub const fn proposal_type(&self) -> WasmProposalType {
        WasmProposalType::Simd
    }
}
