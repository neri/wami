//! WebAssembly Runtime Library

#![cfg_attr(not(test), no_std)]
#![deny(unsafe_op_in_unsafe_fn)]
#![feature(const_mut_refs)]
#![feature(const_option_ext)]
#![feature(const_trait_impl)]
#![feature(slice_split_at_unchecked)]
#![feature(let_chains)]

mod wasm;
pub use crate::wasm::*;

pub mod cg;
// pub mod memory;
pub mod opcode;
pub mod stack;

#[cfg(test)]
mod tests;

extern crate alloc;
