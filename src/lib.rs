//! WebAssembly Runtime Library

#![cfg_attr(not(test), no_std)]
#![deny(unsafe_op_in_unsafe_fn)]
#![feature(slice_split_at_unchecked)]

mod wasm;
pub use crate::wasm::*;

pub mod cg;
pub mod leb128;
// pub mod memory;
pub mod opcode;
pub mod stack;

#[cfg(test)]
mod tests;

extern crate alloc;
