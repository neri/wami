//! TotAssembly Types

#[path = "../_generated/valtype.rs"]
mod _valtype;
pub use _valtype::*;

impl ValType {
    #[inline]
    pub fn as_bytecode(&self) -> isize {
        match self {
            ValType::I32 => -1,
            ValType::I64 => -2,
            ValType::F32 => -3,
            ValType::F64 => -4,
        }
    }

    #[inline]
    pub fn signature(&self) -> &str {
        match self {
            ValType::I32 => "i",
            ValType::I64 => "l",
            ValType::F32 => "f",
            ValType::F64 => "d",
        }
    }
}
