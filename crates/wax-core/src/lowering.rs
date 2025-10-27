use super::*;
pub mod tail_calls;
pub struct Globalizer<T> {
    pub wrapped: T,
    pub num_globals: u32,
    pub num_params: u32,
}
impl<T: InstructionSink> InstructionSink for Globalizer<T> {
    fn instruction(&mut self, instruction: &Instruction<'_>) {
        match instruction {
            Instruction::LocalGet(a) => {
                self.wrapped
                    .instruction(&Instruction::LocalGet(if *a >= self.num_params {
                        *a + self.num_globals
                    } else {
                        *a
                    }))
            }
            Instruction::LocalSet(a) => {
                self.wrapped
                    .instruction(&Instruction::LocalSet(if *a >= self.num_params {
                        *a + self.num_globals
                    } else {
                        *a
                    }))
            }
            Instruction::LocalTee(a) => {
                self.wrapped
                    .instruction(&Instruction::LocalTee(if *a >= self.num_params {
                        *a + self.num_globals
                    } else {
                        *a
                    }))
            }
            Instruction::GlobalGet(a) => self
                .wrapped
                .instruction(&Instruction::LocalGet(*a + self.num_params)),
            Instruction::GlobalSet(a) => self
                .wrapped
                .instruction(&Instruction::LocalSet(*a + self.num_params)),
            i @ (Instruction::Call(_)
            | Instruction::CallIndirect { .. }
            | Instruction::CallRef(_)) => {
                for g in 0..self.num_globals {
                    self.wrapped
                        .instruction(&Instruction::LocalGet(g + self.num_params));
                }
                self.wrapped.instruction(&i);
                for g in (0..self.num_globals).rev() {
                    self.wrapped
                        .instruction(&Instruction::LocalSet(g + self.num_params));
                }
            }
            i @ (Instruction::ReturnCall(_)
            | Instruction::ReturnCallIndirect { .. }
            | Instruction::ReturnCallRef(_)
            | Instruction::Return) => {
                for g in 0..self.num_globals {
                    self.wrapped
                        .instruction(&Instruction::LocalGet(g + self.num_params));
                }
                self.wrapped.instruction(&i);
            }
            instruction => self.wrapped.instruction(instruction),
        }
    }
}
