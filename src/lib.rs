//! WebAssembly Interpreter

#![cfg_attr(not(test), no_std)]
//
#![feature(float_minimum_maximum)]
#![feature(negative_impls)]
#![feature(assert_matches)]
//
#![allow(internal_features)]
#![feature(core_intrinsics)]

extern crate alloc;
extern crate libm;

mod wasm;
pub use crate::wasm::*;

pub mod cg;
pub mod global;
pub mod leb128;
pub mod memory;
pub mod stack;
pub mod sync;

#[path = "_generated/opcode.rs"]
pub mod opcode;

#[cfg(test)]
mod tests;

pub mod prelude {
    pub use crate::cg::intr::WasmRuntimeError;
    pub use crate::global::*;
    pub use crate::memory::{WasmMemory, WasmPtr, WasmPtrMut};
    pub use crate::{
        WasmArgs, WasmCompileError, WasmCompileErrorKind, WasmDynResult, WasmEnv, WasmExports,
        WasmImportFuncResult, WasmInstance, WasmInvocation, WasmLinkError, WasmModule, WasmResult,
        WasmRuntimeErrorKind, WasmType, WasmValType, WasmValue, WebAssembly,
    };
    pub use wami_macro::*;
}

#[allow(unused_imports)]
pub use crate::_prelude_::*;

pub(crate) mod _prelude_ {
    pub use alloc::borrow::ToOwned;
    pub use alloc::boxed::Box;
    pub use alloc::collections::BTreeMap;
    pub use alloc::string::{String, ToString};
    pub use alloc::sync::Arc;
    pub use alloc::vec::Vec;
}
