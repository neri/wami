//! WebAssembly Value Types

/* This file is @generated automatically. DO NOT EDIT DIRECTLY. */

/// WebAssembly Value Types
#[non_exhaustive]
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum ValType {
    /// "f32"
    F32,
    /// "f64"
    F64,
    /// "i32"
    I32,
    /// "i64"
    I64,
}

impl ValType {
    pub fn all_values() -> &'static [Self] {
        &[
            Self::F32,
            Self::F64,
            Self::I32,
            Self::I64,
        ]
    }

    pub fn from_str(v: &str) -> Option<Self> {
        match v {
            "f32" => Some(Self::F32),
            "f64" => Some(Self::F64),
            "i32" => Some(Self::I32),
            "i64" => Some(Self::I64),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::F32 => "f32",
            Self::F64 => "f64",
            Self::I32 => "i32",
            Self::I64 => "i64",
        }
    }
}

impl core::fmt::Display for ValType {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl core::fmt::Debug for ValType {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:?}", self.as_str())
    }
}
