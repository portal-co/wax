//! Dead code elimination analysis.
//!
//! This module provides utilities for identifying unreachable code in WASM functions.
//! It tracks control flow through branches, returns, and other control flow instructions
//! to determine which instructions can never be executed.

use super::*;

/// Stack-based tracker for dead code elimination analysis.
///
/// This structure maintains a stack of boolean flags indicating whether code
/// is unreachable within nested control flow blocks. When a terminating instruction
/// (like `return`, `br`, or `unreachable`) is encountered, subsequent instructions
/// in that block are marked as dead until the block ends or an `else` is encountered.
#[derive(Default)]
pub struct DceStack(Vec<bool>);
impl DceStack {
    /// Creates a new empty DCE stack.
    pub fn new() -> Self {
        return Default::default();
    }
}

/// Analyzes a WASM operator for dead code elimination.
///
/// This function determines whether an operator is unreachable (dead code) based
/// on the control flow context maintained by the `DceStack`. It updates the stack
/// state as it encounters control flow instructions.
///
/// # Arguments
///
/// * `stack` - The DCE analysis stack tracking control flow context
/// * `o` - The operator to analyze
///
/// # Returns
///
/// `true` if the operator is dead code and can be eliminated, `false` otherwise
pub fn dce(DceStack(stack): &mut DceStack, o: &Operator<'_>) -> bool {
    match o {
        Operator::Else => {
            if let Some(a) = stack.last_mut() {
                *a = false
            }
        }
        Operator::If { .. }
        | Operator::Block { .. }
        | Operator::Loop { .. }
        | Operator::TryTable { .. } => {
            stack.push(false);
        }
        Operator::End => {
            stack.pop();
        }
        Operator::Br { .. }
        | Operator::BrTable { .. }
        | Operator::Return
        | Operator::ReturnCall { .. }
        | Operator::ReturnCallIndirect { .. }
        | Operator::ReturnCallRef { .. }
        | Operator::Unreachable
        | Operator::Throw { .. }
        | Operator::ThrowRef => {
            if let Some(a) = stack.last_mut() {
                *a = true
            }
        }
        o => {
            if stack.iter().any(|a| *a) {
                return true;
            } else {
            }
        }
    };
    return false;
}
/// Analyzes a WASM instruction for dead code elimination.
///
/// This is the instruction variant of the DCE analysis, working with
/// `wasm_encoder::Instruction` instead of `wasmparser::Operator`.
///
/// # Arguments
///
/// * `stack` - The DCE analysis stack tracking control flow context
/// * `o` - The instruction to analyze
///
/// # Returns
///
/// `true` if the instruction is dead code and can be eliminated, `false` otherwise
pub fn dce_instr(DceStack(stack): &mut DceStack, o: &Instruction<'_>) -> bool {
    match o {
        Instruction::Else => {
            if let Some(a) = stack.last_mut() {
                *a = false
            }
        }
        Instruction::If { .. }
        | Instruction::Block { .. }
        | Instruction::Loop { .. }
        | Instruction::TryTable(..) => {
            stack.push(false);
        }
        Instruction::End => {
            stack.pop();
        }
        Instruction::Br { .. }
        | Instruction::BrTable { .. }
        | Instruction::Return
        | Instruction::ReturnCall { .. }
        | Instruction::ReturnCallIndirect { .. }
        | Instruction::ReturnCallRef { .. }
        | Instruction::Unreachable
        | Instruction::Throw(..)
        | Instruction::ThrowRef => {
            if let Some(a) = stack.last_mut() {
                *a = true
            }
        }
        o => {
            if stack.iter().any(|a| *a) {
                return true;
            } else {
            }
        }
    };
    return false;
}
