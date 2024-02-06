//! WebAssembly Interpreter

#![cfg_attr(not(test), no_std)]
#![deny(unsafe_op_in_unsafe_fn)]
#![feature(slice_split_at_unchecked)]
#![feature(float_minimum_maximum)]
#![feature(negative_impls)]
#![feature(error_in_core)]
//
#![feature(core_intrinsics)]
#![allow(internal_features)]
#![feature(assert_matches)]

extern crate alloc;
extern crate libm;

mod wasm;
pub use crate::wasm::*;

pub mod cg;
pub mod leb128;
pub mod memory;
pub mod stack;
pub mod sync;

#[path = "_generated/opcode.rs"]
pub mod opcode;

#[cfg(test)]
mod tests;
