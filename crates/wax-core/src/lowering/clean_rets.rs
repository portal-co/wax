use core::mem::replace;

use alloc::collections::btree_map::BTreeMap;
use wasm_encoder::{FuncType, GlobalType, ValType};
// use wasm_encoder::Global;

use crate::rewrite::{Shimmer, Tracker};

use super::*;
pub struct RetCleaner {
    types: Vec<Vec<ValType>>,
    block_types: Vec<u32>,
    globals: Vec<Vec<u32>>,
    func_types: Vec<u32>,
}
impl RetCleaner {
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
