//! WebAssembly Interpreter

#![cfg_attr(not(test), no_std)]
#![deny(unsafe_op_in_unsafe_fn)]
#![feature(slice_split_at_unchecked)]
#![feature(float_minimum_maximum)]
#![feature(negative_impls)]
//
#![feature(core_intrinsics)]
#![allow(internal_features)]
#![feature(assert_matches)]

extern crate alloc;

mod wasm;
pub use crate::wasm::*;

pub mod cg;
pub mod leb128;
pub mod memory;
pub mod stack;
pub mod sync;

#[path = "_generated/bytecode.rs"]
pub mod bytecode;

#[cfg(test)]
mod tests;
