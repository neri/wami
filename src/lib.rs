// Wasm-O
#![no_std]
#![feature(try_reserve)]

pub mod opcode;
mod wasm;
pub use crate::wasm::*;
pub mod wasmintr;

extern crate alloc;
