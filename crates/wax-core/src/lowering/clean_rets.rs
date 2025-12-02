//! Return statement cleanup and transformation.
//!
//! This module transforms function returns to use a sentinel value and global variables
//! for return values. This allows for more flexible control flow and exception handling.
//!
//! The transformation works by:
//! 1. Changing all function signatures to return a single i32 status code
//! 2. Using global variables to store actual return values
//! 3. Converting return statements to store values in globals and return -1
//! 4. Converting calls to check the status code and extract values from globals

use core::mem::replace;

use alloc::collections::btree_map::BTreeMap;
use wasm_encoder::{FuncType, GlobalType, ValType};
// use wasm_encoder::Global;

use crate::rewrite::{Shimmer, Tracker};

use super::*;

/// A transformer that cleans up return statements by replacing them with globals.
///
/// This structure maintains the mapping between original function types and the
/// global variables used to store their return values.
pub struct RetCleaner {
    types: Vec<Vec<ValType>>,
    block_types: Vec<u32>,
    globals: Vec<Vec<u32>>,
    func_types: Vec<u32>,
}
impl RetCleaner {
    /// Creates a new RetCleaner and prepares function types and globals.
    ///
    /// This method:
    /// 1. Transforms all function types to return i32 instead of their original return types
    /// 2. Allocates global variables for each return value of each function type
    /// 3. Creates new block types for extracting return values
    ///
    /// # Arguments
    ///
    /// * `f` - The function types to transform (modified in place)
    /// * `func_types` - Mapping from function index to type index
    /// * `globals` - Tracker for allocating new global variables
    /// * `new_types` - Tracker for allocating new block types
    pub fn new(
        f: &mut [FuncType],
        func_types: &[u32],
        globals: &mut Tracker<GlobalType>,
        new_types: &mut Tracker<FuncType>,
    ) -> Self {
        let types: Vec<Vec<ValType>> = f
            .iter_mut()
            .map(|a| {
                let b = replace(a, FuncType::new(a.params().iter().cloned(), [ValType::I32]));
                b.results().iter().cloned().collect()
            })
            .collect();
        let globals: Vec<Vec<u32>> = types
            .iter()
            .map(|a| {
                a.iter()
                    .map(|a| {
                        globals.push(GlobalType {
                            val_type: a.clone(),
                            mutable: true,
                            shared: false,
                        })
                    })
                    .collect()
            })
            .collect();
        let block_types: Vec<u32> = types
            .iter()
            .map(|a| new_types.push(FuncType::new([], a.iter().cloned())))
            .collect();
        Self {
            types,
            globals,
            block_types,
            func_types: func_types.iter().cloned().collect(),
        }
    }
    /// Transforms an instruction according to the return cleanup strategy.
    ///
    /// This method rewrites return and call instructions to use the global variable
    /// based return mechanism.
    ///
    /// # Arguments
    ///
    /// * `cur_func` - The index of the current function being transformed
    /// * `stash` - A local variable index for temporarily storing return values
    /// * `i` - The instruction to transform
    /// * `f` - The sink to emit transformed instructions to
    /// * `trap` - A callback for generating trap handling code
    ///
    /// # Errors
    ///
    /// Returns an error if instruction emission fails.
    pub fn inst<E>(
        &self,
        cur_func: u32,
        stash: u32,
        i: &Instruction<'_>,
        f: &mut (dyn InstructionSink<E> + '_),
        trap: &mut (dyn FnMut(&mut (dyn InstructionSink<E> + '_), u32) -> Result<(), E> + '_),
    ) -> Result<(), E> {
        match i {
            Instruction::Return => {
                let g = &self.globals[self.func_types[cur_func as usize] as usize];
                for g in g.iter().rev().cloned() {
                    f.instruction(&Instruction::GlobalSet(g))?;
                }
                f.instruction(&Instruction::I32Const(-1))?;
                f.instruction(&Instruction::Return)
            }
            Instruction::Call(a) => {
                f.instruction(&Instruction::Call(*a))?;
                f.instruction(&Instruction::LocalTee(stash))?;
                f.instruction(&Instruction::I32Const(-1))?;
                f.instruction(&Instruction::I32Ne)?;
                let ft = self.func_types[*a as usize];
                f.instruction(&Instruction::If(BlockType::FunctionType(
                    self.block_types[ft as usize],
                )))?;
                trap(f, stash)?;
                f.instruction(&Instruction::Else)?;
                let g = &self.globals[ft as usize];
                for g in g.iter().cloned() {
                    f.instruction(&Instruction::GlobalGet(g))?;
                }
                f.instruction(&Instruction::End)?;
                Ok(())
            }
            Instruction::CallRef(a) => {
                f.instruction(&Instruction::CallRef(*a))?;
                f.instruction(&Instruction::LocalTee(stash))?;
                f.instruction(&Instruction::I32Const(-1))?;
                f.instruction(&Instruction::I32Ne)?;
                let ft = *a;
                f.instruction(&Instruction::If(BlockType::FunctionType(
                    self.block_types[ft as usize],
                )))?;
                trap(f, stash)?;
                f.instruction(&Instruction::Else)?;
                let g = &self.globals[ft as usize];
                for g in g.iter().cloned() {
                    f.instruction(&Instruction::GlobalGet(g))?;
                }
                f.instruction(&Instruction::End)?;
                Ok(())
            }
            Instruction::CallIndirect {
                type_index,
                table_index,
            } => {
                f.instruction(&Instruction::CallIndirect {
                    type_index: *type_index,
                    table_index: *table_index,
                })?;
                f.instruction(&Instruction::LocalTee(stash))?;
                f.instruction(&Instruction::I32Const(-1))?;
                f.instruction(&Instruction::I32Ne)?;
                let ft = *type_index;
                f.instruction(&Instruction::If(BlockType::FunctionType(
                    self.block_types[ft as usize],
                )))?;
                trap(f, stash)?;
                f.instruction(&Instruction::Else)?;
                let g = &self.globals[ft as usize];
                for g in g.iter().cloned() {
                    f.instruction(&Instruction::GlobalGet(g))?;
                }
                f.instruction(&Instruction::End)?;
                Ok(())
            }
            i => f.instruction(i),
        }
    }
}
impl<E> Shimmer<E> for RetCleaner {
    fn shim(
        &self,
        old: u32,
        func_types: &[u32],
        types: &[FuncType],
        kind: rewrite::ShimKind,
        sink: &mut (dyn InstructionSink<E> + '_),
    ) -> Result<(), E> {
        let t = func_types[old as usize];
        for p in 0..(types[t as usize].params().len()) {
            sink.instruction(&Instruction::LocalGet(p as u32))?;
        }
        sink.instruction(&Instruction::Call(old))?;
        match kind {
            rewrite::ShimKind::Import => {
                for r in self.globals[t as usize].iter().cloned() {
                    sink.instruction(&Instruction::GlobalSet(r))?;
                }
                sink.instruction(&Instruction::I32Const(-1))?;
                sink.instruction(&Instruction::Return)
            }
            rewrite::ShimKind::Export => {
                let g = &self.globals[t as usize];
                for g in g.iter().cloned() {
                    sink.instruction(&Instruction::GlobalGet(g))?;
                }
                sink.instruction(&Instruction::Return)
            }
        }
    }
}
