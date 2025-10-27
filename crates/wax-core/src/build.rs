use impl_trait_for_tuples::impl_for_tuples;

use crate::*;
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Default)]
#[repr(transparent)]
pub struct FromFn<T>(pub T);
impl<T: FnMut(&Instruction<'_>) -> Result<(), E>, E> FromFn<T> {
    pub fn instruction_sink(a: T) -> Self {
        Self(a)
    }
}
impl<T: FnMut(&Operator<'_>) -> Result<(), E>, E> FromFn<T> {
    pub fn operator_sink(a: T) -> Self {
        Self(a)
    }
}
pub trait InstructionSink<E> {
    fn instruction(&mut self, instruction: &Instruction<'_>) -> Result<(), E>;
}
impl<E, T: FnMut(&Instruction<'_>) -> Result<(), E>> InstructionSink<E> for FromFn<T> {
    fn instruction(&mut self, instruction: &Instruction<'_>) -> Result<(), E> {
        let FromFn(a) = self;
        a(instruction)
    }
}
impl<E, T: InstructionSink<E> + ?Sized> InstructionSink<E> for &'_ mut T {
    fn instruction(&mut self, instruction: &Instruction<'_>) -> Result<(), E> {
        (&mut **self).instruction(instruction)
    }
}
impl<E, T: OperatorSink<E> + ?Sized> OperatorSink<E> for &'_ mut T {
    fn operator(&mut self, op: &Operator<'_>) -> Result<(), E> {
        (&mut **self).operator(op)
    }
}
impl<E, T: InstructionSink<E> + ?Sized> InstructionSink<E> for Box<T> {
    fn instruction(&mut self, instruction: &Instruction<'_>) -> Result<(), E> {
        (&mut **self).instruction(instruction)
    }
}
impl<E, T: OperatorSink<E> + ?Sized> OperatorSink<E> for Box<T> {
    fn operator(&mut self, op: &Operator<'_>) -> Result<(), E> {
        (&mut **self).operator(op)
    }
}
impl<E> InstructionSink<E> for wasm_encoder::Function {
    fn instruction(&mut self, instruction: &Instruction<'_>) -> Result<(), E> {
        wasm_encoder::Function::instruction(self, instruction);
        Ok(())
    }
}
pub trait OperatorSink<E> {
    fn operator(&mut self, op: &Operator<'_>) -> Result<(), E>;
}
impl<E, T: FnMut(&Operator<'_>) -> Result<(), E>> OperatorSink<E> for FromFn<T> {
    fn operator(&mut self, op: &Operator<'_>) -> Result<(), E> {
        let FromFn(f) = self;
        f(op)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Default)]
pub struct Rewrite<R, S> {
    pub rewriter: R,
    pub sink: S,
}
impl<R: Reencode, S: InstructionSink<E>, E: From<wasm_encoder::reencode::Error<R::Error>>>
    OperatorSink<E> for Rewrite<R, S>
{
    fn operator(&mut self, op: &Operator<'_>) -> Result<(), E> {
        self.sink
            .instruction(&self.rewriter.instruction(op.clone())?)
    }
}
impl<R, S: InstructionSink<E>, E> InstructionSink<E> for Rewrite<R, S> {
    fn instruction(&mut self, instruction: &Instruction<'_>) -> Result<(), E> {
        self.sink.instruction(instruction)
    }
}
pub trait InstructionOperatorSink<E>: InstructionSink<E> + OperatorSink<E> {}
impl<E, T: InstructionSink<E> + OperatorSink<E> + ?Sized> InstructionOperatorSink<E> for T {}
// #[impl_for_tuples(12)]
pub trait InstructionSource<E>: InstructionOperatorSource<E> {
    fn emit_instruction(&self, sink: &mut (dyn InstructionSink<E> + '_)) -> Result<(), E>;
}
// #[impl_for_tuples(12)]
pub trait OperatorSource<E>: InstructionOperatorSource<E> {
    fn emit_operator(&self, sink: &mut (dyn OperatorSink<E> + '_)) -> Result<(), E>;
}

pub trait InstructionOperatorSource<E> {
    fn emit(&self, sink: &mut (dyn InstructionOperatorSink<E> + '_)) -> Result<(), E>;
}
#[impl_for_tuples(12)]
impl<E> InstructionOperatorSource<E> for Tuple {
    for_tuples!(where #(Tuple: InstructionOperatorSource<E>)*);
    fn emit(&self, sink: &mut (dyn InstructionOperatorSink<E> + '_)) -> Result<(), E> {
        for_tuples!(#(Tuple.emit(sink)?;)*);
        Ok(())
    }
}
#[impl_for_tuples(12)]
impl<E> InstructionSource<E> for Tuple {
    for_tuples!(where #(Tuple: InstructionSource<E>)*);
    fn emit_instruction(&self, sink: &mut (dyn InstructionSink<E> + '_)) -> Result<(), E> {
        for_tuples!(#(Tuple.emit_instruction(sink)?;)*);
        Ok(())
    }
}
#[impl_for_tuples(12)]
impl<E> OperatorSource<E> for Tuple {
    for_tuples!(where #(Tuple: OperatorSource<E>)*);
    fn emit_operator(&self, sink: &mut (dyn OperatorSink<E> + '_)) -> Result<(), E> {
        for_tuples!(#(Tuple.emit_operator(sink)?;)*);
        Ok(())
    }
}
