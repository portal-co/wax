use wasm_encoder::{FuncType, GlobalType};

use crate::rewrite::Shimmer;

use super::*;
pub struct Globalize {
    num_globals: u32,
}
impl Globalize {
    pub fn new(a: &mut [FuncType], g: &[GlobalType]) -> Self {
        let globals: Vec<_> = g.iter().map(|a| a.val_type.clone()).collect();
        for a in a.iter_mut() {
            *a = FuncType::new(
                a.params().iter().cloned().chain(globals.iter().cloned()),
                a.results().iter().cloned().chain(globals.iter().cloned()),
            );
        }
        Self {
            num_globals: g.len() as u32,
        }
    }
    pub fn inst<E>(
        &self,
        num_params: u32,
        instruction: &Instruction<'_>,
        wrapped: &mut (dyn InstructionSink<E> + '_),
    ) -> Result<(), E> {
        match instruction {
            Instruction::LocalGet(a) => {
                wrapped.instruction(&Instruction::LocalGet(if *a >= num_params {
                    *a + self.num_globals
                } else {
                    *a
                }))
            }
            Instruction::LocalSet(a) => {
                wrapped.instruction(&Instruction::LocalSet(if *a >= num_params {
                    *a + self.num_globals
                } else {
                    *a
                }))
            }
            Instruction::LocalTee(a) => {
                wrapped.instruction(&Instruction::LocalTee(if *a >= num_params {
                    *a + self.num_globals
                } else {
                    *a
                }))
            }
            Instruction::GlobalGet(a) => {
                wrapped.instruction(&Instruction::LocalGet(*a + num_params))
            }
            Instruction::GlobalSet(a) => {
                wrapped.instruction(&Instruction::LocalSet(*a + num_params))
            }
            i @ (Instruction::Call(_)
            | Instruction::CallIndirect { .. }
            | Instruction::CallRef(_)) => {
                for g in 0..self.num_globals {
                    wrapped.instruction(&Instruction::LocalGet(g + num_params))?;
                }
                wrapped.instruction(&i)?;
                for g in (0..self.num_globals).rev() {
                    wrapped.instruction(&Instruction::LocalSet(g + num_params))?;
                }
                Ok(())
            }
            i @ (Instruction::ReturnCall(_)
            | Instruction::ReturnCallIndirect { .. }
            | Instruction::ReturnCallRef(_)
            | Instruction::Return) => {
                for g in 0..self.num_globals {
                    wrapped.instruction(&Instruction::LocalGet(g + num_params))?;
                }
                wrapped.instruction(&i)?;
                Ok(())
            }
            instruction => wrapped.instruction(instruction),
        }
    }
}
impl<E> Shimmer<E> for Globalize {
    fn shim(
        &self,
        old: u32,
        func_types: &[u32],
        types: &[FuncType],
        kind: rewrite::ShimKind,
        sink: &mut (dyn InstructionSink<E> + '_),
    ) -> Result<(), E> {
        for p in 0..(types[func_types[old as usize] as usize].params().len() as u32){
            sink.instruction(&Instruction::LocalGet(p))?;
        }
        match kind {
            rewrite::ShimKind::Import => {
                for n in 0..self.num_globals {
                    sink.instruction(&Instruction::GlobalSet(n))?;
                }
                sink.instruction(&Instruction::Call(old))?;
                for n in 0..self.num_globals {
                    sink.instruction(&Instruction::GlobalGet(n))?;
                }
                sink.instruction(&Instruction::Return)?;
            }
            rewrite::ShimKind::Export => {
                for n in 0..self.num_globals {
                    sink.instruction(&Instruction::GlobalGet(n))?;
                }
                sink.instruction(&Instruction::Call(old))?;
                for n in 0..self.num_globals {
                    sink.instruction(&Instruction::GlobalSet(n))?;
                }
                sink.instruction(&Instruction::Return)?;
            }
        };
        Ok(())
    }
}
