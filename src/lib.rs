//! WebAssembly Runtime Library

#![cfg_attr(not(test), no_std)]
#![deny(unsafe_op_in_unsafe_fn)]
#![feature(slice_split_at_unchecked)]
#![feature(float_minimum_maximum)]
#![feature(negative_impls)]
//
#![feature(core_intrinsics)]
#![allow(internal_features)]

mod wasm;
pub use crate::wasm::*;

pub mod cg;
pub mod leb128;
pub mod opcode;
pub mod stack;
pub mod sync;

#[cfg(test)]
mod tests;

extern crate alloc;
