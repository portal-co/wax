use alloc::vec::Vec;
use wasm_encoder::{Instruction, ValType, reencode::Reencode};
use wasmparser::Operator;
#[derive(Default)]
pub struct SCF {
    blocks: Vec<Option<Vec<ValType>>>,
}
impl SCF {
    pub fn val_types(&self, mut br_target: u32) -> Option<&[ValType]> {
        let mut i = self.blocks.len();
        loop {
            i = i.wrapping_sub(1);
            let Some(a) = self.blocks.get(i)? else {
                continue;
            };
            if br_target == 0 {
                return Some(a);
            }
            br_target -= 1;
        }
    }
    pub fn on_op<E>(
        &mut self,
        op: &Operator,
        funcs: &[wasm_encoder::FuncType],
        rewriter: &mut (dyn Reencode<Error = E> + '_),
    ) -> Result<(), wasm_encoder::reencode::Error<E>> {
        match op {
            Operator::End => {
                self.blocks.pop();
            }
            Operator::If { blockty } => {
                self.blocks.push(None);
            }
            Operator::Block { blockty } => {
                self.blocks.push(Some(match blockty {
                    wasmparser::BlockType::Empty => Default::default(),
                    wasmparser::BlockType::Type(val_type) => {
                        [rewriter.val_type(*val_type)?].into_iter().collect()
                    }
                    wasmparser::BlockType::FuncType(f) => funcs[*f as usize]
                        .results()
                        .iter()
                        .cloned()
                        // .map(|a| rewriter.val_type(*a))
                        // .collect::<Result<Vec<_>, _>>()?,
                        .collect(),
                }));
            }
            Operator::Loop { blockty } => {
                self.blocks.push(Some(match blockty {
                    wasmparser::BlockType::Empty => Default::default(),
                    wasmparser::BlockType::Type(val_type) => Default::default(),
                    wasmparser::BlockType::FuncType(f) => funcs[*f as usize]
                        .params()
                        .iter()
                        .cloned()
                        // .map(|a| rewriter.val_type(*a))
                        // .collect::<Result<Vec<_>, _>>()?,
                        .collect(),
                }));
            }
            _ => {}
        }
        Ok(())
    }
    pub fn on_inst(&mut self, op: &Instruction, funcs: &[wasm_encoder::FuncType]) {
        match op {
            Instruction::End => {
                self.blocks.pop();
            }
            Instruction::If(blockty) => {
                self.blocks.push(None);
            }
            Instruction::Block(blockty) => {
                self.blocks.push(Some(match blockty {
                    wasm_encoder::BlockType::Empty => Default::default(),
                    wasm_encoder::BlockType::Result(t) => [*t].into_iter().collect(),
                    wasm_encoder::BlockType::FunctionType(f) => {
                        funcs[*f as usize].results().iter().cloned().collect()
                    }
                }));
            }
            Instruction::Loop(blockty) => {
                self.blocks.push(Some(match blockty {
                    wasm_encoder::BlockType::Empty => Default::default(),
                    wasm_encoder::BlockType::Result(t) => Default::default(),
                    wasm_encoder::BlockType::FunctionType(f) => {
                        funcs[*f as usize].params().iter().cloned().collect()
                    }
                }));
            }
            _ => {}
        }
    }
}
