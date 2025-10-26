#![no_std]

use core::ops::Range;

use alloc::boxed::Box;
use wasm_encoder::{Instruction, reencode::Reencode};
use wasmparser::Operator;
extern crate alloc;
pub mod build;