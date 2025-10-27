use core::mem::replace;

use alloc::collections::btree_map::BTreeMap;
use wasm_encoder::{FuncType, GlobalType, ValType};
// use wasm_encoder::Global;

use crate::rewrite::Tracker;

use super::*;
pub struct RetCleaner {
    types: Vec<Vec<ValType>>,
    block_types: Vec<u32>,
    globals: Vec<Vec<u32>>,
    func_types: BTreeMap<u32, u32>,
}
impl RetCleaner {
    pub fn new(
        f: &mut [FuncType],
        func_types: BTreeMap<u32, u32>,
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
            func_types,
        }
    }
    pub fn inst(
        &self,
        cur_func: u32,
        stash: u32,
        i: &Instruction<'_>,
        f: &mut (dyn InstructionSink + '_),
        trap: &mut (dyn FnMut(&mut (dyn InstructionSink + '_), u32) + '_),
    ) {
        match i {
            Instruction::Return => {
                let g = &self.globals[self.func_types.get(&cur_func).cloned().unwrap() as usize];
                for g in g.iter().rev().cloned() {
                    f.instruction(&Instruction::GlobalSet(g))
                }
                f.instruction(&Instruction::I32Const(-1));
                f.instruction(&Instruction::Return)
            }
            Instruction::Call(a) => {
                f.instruction(&Instruction::Call(*a));
                f.instruction(&Instruction::LocalTee(stash));
                f.instruction(&Instruction::I32Const(-1));
                f.instruction(&Instruction::I32Ne);
                let ft = self.func_types.get(a).cloned().unwrap();
                f.instruction(&Instruction::If(BlockType::FunctionType(self.block_types[ft as usize])));
                trap(f, stash);
                f.instruction(&Instruction::Else);
                let g = &self.globals[ft as usize];
                for g in g.iter().cloned() {
                    f.instruction(&Instruction::GlobalGet(g))
                }
                f.instruction(&Instruction::End);
            }
            Instruction::CallRef(a) => {
                f.instruction(&Instruction::CallRef(*a));
                f.instruction(&Instruction::LocalTee(stash));
                f.instruction(&Instruction::I32Const(-1));
                f.instruction(&Instruction::I32Ne);
                let ft = *a;
                f.instruction(&Instruction::If(BlockType::FunctionType(self.block_types[ft as usize])));
                trap(f, stash);
                f.instruction(&Instruction::Else);
                let g = &self.globals[ft as usize];
                for g in g.iter().cloned() {
                    f.instruction(&Instruction::GlobalGet(g))
                }
                f.instruction(&Instruction::End);
            }
            Instruction::CallIndirect {
                type_index,
                table_index,
            } => {
                f.instruction(&Instruction::CallIndirect {
                    type_index: *type_index,
                    table_index: *table_index,
                });
                f.instruction(&Instruction::LocalTee(stash));
                f.instruction(&Instruction::I32Const(-1));
                f.instruction(&Instruction::I32Ne);
                let ft = *type_index;
                f.instruction(&Instruction::If(BlockType::FunctionType(self.block_types[ft as usize])));
                trap(f, stash);
                f.instruction(&Instruction::Else);
                let g = &self.globals[ft as usize];
                for g in g.iter().cloned() {
                    f.instruction(&Instruction::GlobalGet(g))
                }
                f.instruction(&Instruction::End);
            }
            i => f.instruction(i),
        }
    }
}
