#![no_std]

use core::default;

use alloc::vec::Vec;
use wasm_encoder::Instruction;
extern crate alloc;
pub struct Tags<'a>{
    pub tags: &'a mut [Tag]
}
#[derive(Default)]
#[non_exhaustive]
pub struct Tag{
    pub target: Vec<u32>,
    pub mtarget: u32,
}
impl<'a> Tags<'a>{
    pub fn assign_targets(&mut self, ops: &[Instruction<'_>]){
        let mut stack = Vec::new();
        let scan_block = |begin: usize| -> usize{
            let mut depth = 1;
            for i in begin..{
                let Some(inst) = ops.get(i) else { break };
                match inst{
                    Instruction::Loop(_) | Instruction::Block(_) | Instruction::If(_) |Instruction::TryTable(_, _)=> {
                        depth += 1;
                    }
                    Instruction::End => {
                        depth -= 1;
                        if depth == 0{
                            return i;
                        }
                    }
                    _ => {}
                }
            }
            unreachable!()
        };
        for i in 0..{
            let Some(inst) = ops.get(i) else { break };
            let Some(tag) = self.tags.get_mut(i) else { break };
            match inst{
                Instruction::Loop(_) => {
                    stack.push(i as u32);
                    tag.mtarget = i as u32;
                }
                Instruction::Block(_) | Instruction::If(_) |Instruction::TryTable(_, _)=> {
                    let end = scan_block(i);
                    stack.push(end as u32);
                    tag.mtarget = end as u32;
                }
                Instruction::End => {
                    stack.pop();
                }
                Instruction::Br(depth) | Instruction::BrIf(depth) => {
                    let target = stack[stack.len() - 1 - (*depth as usize)];
                    tag.target = [target].into_iter().collect()
                }
                Instruction::BrTable(targets, default) => {
                    let mut tag_targets = Vec::new();
                    for target in targets.iter().chain(core::iter::once(default)){
                        let target = stack[stack.len() - 1 - (*target as usize)];
                        tag_targets.push(target);
                    }
                    tag.target = tag_targets;
                }
                _ => {}
            }
        }
    }
}