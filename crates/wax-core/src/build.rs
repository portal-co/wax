use impl_trait_for_tuples::impl_for_tuples;

use crate::*;
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Default)]
#[repr(transparent)]
pub struct FromFn<T>(pub T);
impl<T: FnMut(&Instruction<'_>)> FromFn<T> {
    pub fn instruction_sink(a: T) -> Self {
        Self(a)
    }
}
impl<T: FnMut(&Operator<'_>)> FromFn<T> {
    pub fn operator_sink(a: T) -> Self {
        Self(a)
    }
}
pub trait InstructionSink {
    fn instruction(&mut self, instruction: &Instruction<'_>);
}
impl<T: FnMut(&Instruction<'_>)> InstructionSink for FromFn<T> {
    fn instruction(&mut self, instruction: &Instruction<'_>) {
        let FromFn(a) = self;
        a(instruction);
    }
}
impl<T: InstructionSink + ?Sized> InstructionSink for &'_ mut T {
    fn instruction(&mut self, instruction: &Instruction<'_>) {
        (&mut **self).instruction(instruction);
    }
}
impl<T: OperatorSink + ?Sized> OperatorSink for &'_ mut T {
    fn operator(&mut self, op: &Operator<'_>) {
        (&mut **self).operator(op);
    }
}
impl<T: InstructionSink + ?Sized> InstructionSink for Box<T> {
    fn instruction(&mut self, instruction: &Instruction<'_>) {
        (&mut **self).instruction(instruction);
    }
}
impl<T: OperatorSink + ?Sized> OperatorSink for Box<T> {
    fn operator(&mut self, op: &Operator<'_>) {
        (&mut **self).operator(op);
    }
}
impl InstructionSink for wasm_encoder::Function {
    fn instruction(&mut self, instruction: &Instruction<'_>) {
        wasm_encoder::Function::instruction(self, instruction);
    }
}
pub trait OperatorSink {
    fn operator(&mut self, op: &Operator<'_>);
}
impl<T: FnMut(&Operator<'_>)> OperatorSink for FromFn<T> {
    fn operator(&mut self, op: &Operator<'_>) {
        let FromFn(f) = self;
        f(op);
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Default)]
pub struct Rewrite<R, S> {
    pub rewriter: R,
    pub sink: S,
}
impl<R: Reencode, S: InstructionSink> OperatorSink for Rewrite<R, S> {
    fn operator(&mut self, op: &Operator<'_>) {
        if let Ok(a) = self.rewriter.instruction(op.clone()) {
            self.sink.instruction(&a);
        }
    }
}
impl<R, S: InstructionSink> InstructionSink for Rewrite<R, S> {
    fn instruction(&mut self, instruction: &Instruction<'_>) {
        self.sink.instruction(instruction);
    }
}
pub trait InstructionOperatorSink: InstructionSink + OperatorSink {}
impl<T: InstructionSink + OperatorSink + ?Sized> InstructionOperatorSink for T {}
#[impl_for_tuples(12)]
pub trait InstructionSource: InstructionOperatorSource {
    fn emit_instruction(&self, sink: &mut (dyn InstructionSink + '_));
}
#[impl_for_tuples(12)]
pub trait OperatorSource: InstructionOperatorSource {
    fn emit_operator(&self, sink: &mut (dyn InstructionSink + '_));
}
#[impl_for_tuples(12)]
pub trait InstructionOperatorSource {
    fn emit(&self, sink: &mut (dyn InstructionOperatorSink + '_));
}
