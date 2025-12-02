//! # wax-core
//!
//! A `no_std` compatible library for WebAssembly instruction manipulation, code transformation,
//! and analysis. This crate provides utilities for working with WASM bytecode at a low level.
//!
//! ## Features
//!
//! - **Instruction Building**: Flexible sink/source patterns for constructing WASM instructions
//! - **Code Rewriting**: Type and function index rewriting with shimming support
//! - **Code Analysis**: Dead code elimination and structured control flow analysis
//! - **Lowering Transformations**: Various code transformation passes
//!
//! ## Modules
//!
//! - [`build`]: Instruction and operator sink/source abstractions
//! - [`rewrite`]: Index rewriting and shimming utilities
//! - [`analysis`]: Code analysis passes (DCE, SCF)
//! - [`lowering`]: Code transformation passes (tail calls, return cleanup, globalization)
//!
//! ## Example
//!
//! ```rust,no_run
//! use wax_core::build::InstructionSink;
//! use wasm_encoder::{Function, Instruction};
//!
//! let mut func = Function::new([]);
//! func.instruction(&Instruction::I32Const(42));
//! ```

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
pub mod analysis;

/// Converts a borrowed WASM instruction to a static lifetime version.
///
/// This function takes an `Instruction` with any lifetime and converts it to one with a
/// `'static` lifetime. For most instructions, this is done via unsafe transmutation.
/// For instructions that contain borrowed data (like `BrTable`, `TryTable`, and
/// `TypedSelectMulti`), the data is cloned into owned variants.
///
/// # Safety
///
/// This function uses unsafe transmutation for most instruction types. It is safe because
/// `Instruction` variants that don't contain borrowed data can be safely transmuted to
/// static lifetime. The variants with borrowed data are explicitly handled.
///
/// # Examples
///
/// ```rust,no_run
/// use wasm_encoder::Instruction;
/// use wax_core::r#static;
///
/// let inst = Instruction::Nop;
/// let static_inst: Instruction<'static> = r#static(inst);
/// ```
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
