#![no_std]

use core::{mem::transmute, ops::Range};

use alloc::{borrow::Cow, boxed::Box, vec::Vec};
use wasm_encoder::{BlockType, Function, Instruction, reencode::Reencode};
use wasmparser::Operator;

use crate::build::InstructionSink;
extern crate alloc;
pub mod build;
pub mod lowering;
pub mod rewrite;
pub fn r#static(a: Instruction<'_>) -> Instruction<'static> {
    match a {
        Instruction::BrTable(a, b) => Instruction::BrTable(Cow::Owned(a.into_owned()), b),
        Instruction::TryTable(a, b) => Instruction::TryTable(a, Cow::Owned(b.into_owned())),
        Instruction::TypedSelectMulti(a) => {
            Instruction::TypedSelectMulti(Cow::Owned(a.into_owned()))
        }
        a => unsafe { transmute(a) },
    }
}
