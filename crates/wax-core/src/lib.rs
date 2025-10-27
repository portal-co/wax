#![no_std]

use core::ops::Range;

use alloc::boxed::Box;
use wasm_encoder::{Function, Instruction, reencode::Reencode};
use wasmparser::Operator;

use crate::build::InstructionSink;
extern crate alloc;
pub mod build;
pub mod lowering;