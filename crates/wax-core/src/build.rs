//! Instruction and operator sink/source abstractions for building WASM code.
//!
//! This module provides traits and types for constructing WASM instructions in a flexible way.
//! It uses a sink/source pattern where:
//!
//! - **Sinks** consume instructions or operators (e.g., writing to a function)
//! - **Sources** emit instructions or operators (e.g., reading from a template)
//!
//! The module supports both `wasm_encoder::Instruction` and `wasmparser::Operator` types,
//! allowing seamless conversion and manipulation of WASM bytecode.

use impl_trait_for_tuples::impl_for_tuples;

use crate::*;

/// A wrapper type that converts a function or closure into a sink or source.
///
/// This type provides a convenient way to create sinks from closures without
/// manually implementing the trait. It has helper constructors for creating
/// instruction and operator sinks.
///
/// # Examples
///
/// ```rust,no_run
/// use wax_core::build::{FromFn, InstructionSink};
/// use wasm_encoder::Instruction;
///
/// let mut sink = FromFn::instruction_sink(|instr: &Instruction| {
///     println!("Got instruction: {:?}", instr);
///     Ok::<(), ()>(())
/// });
/// ```
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Default)]
#[repr(transparent)]
pub struct FromFn<T>(pub T);
impl<T: FnMut(&Instruction<'_>) -> Result<(), E>, E> FromFn<T> {
    /// Creates a new instruction sink from a closure.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use wax_core::build::FromFn;
    /// use wasm_encoder::Instruction;
    ///
    /// let sink = FromFn::instruction_sink(|instr: &Instruction| {
    ///     // Process instruction
    ///     Ok::<(), ()>(())
    /// });
    /// ```
    pub fn instruction_sink(a: T) -> Self {
        Self(a)
    }
}
impl<T: FnMut(&Operator<'_>) -> Result<(), E>, E> FromFn<T> {
    /// Creates a new operator sink from a closure.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use wax_core::build::FromFn;
    /// use wasmparser::Operator;
    ///
    /// let sink = FromFn::operator_sink(|op: &Operator| {
    ///     // Process operator
    ///     Ok::<(), ()>(())
    /// });
    /// ```
    pub fn operator_sink(a: T) -> Self {
        Self(a)
    }
}

/// A trait for types that can consume WASM instructions.
///
/// Implementors of this trait can receive and process `wasm_encoder::Instruction` values.
/// This is useful for building functions, transforming code, or analyzing instruction sequences.
///
/// # Type Parameters
///
/// * `E` - The error type that can be returned during instruction processing
pub trait InstructionSink<E> {
    /// Processes a single WASM instruction.
    ///
    /// # Arguments
    ///
    /// * `instruction` - The instruction to process
    ///
    /// # Errors
    ///
    /// Returns an error if the instruction cannot be processed.
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
/// A trait for types that can consume WASM operators.
///
/// Implementors of this trait can receive and process `wasmparser::Operator` values.
/// This is useful when working with parsed WASM bytecode.
///
/// # Type Parameters
///
/// * `E` - The error type that can be returned during operator processing
pub trait OperatorSink<E> {
    /// Processes a single WASM operator.
    ///
    /// # Arguments
    ///
    /// * `op` - The operator to process
    ///
    /// # Errors
    ///
    /// Returns an error if the operator cannot be processed.
    fn operator(&mut self, op: &Operator<'_>) -> Result<(), E>;
}
impl<E, T: FnMut(&Operator<'_>) -> Result<(), E>> OperatorSink<E> for FromFn<T> {
    fn operator(&mut self, op: &Operator<'_>) -> Result<(), E> {
        let FromFn(f) = self;
        f(op)
    }
}

/// A rewriting operator sink that transforms operators through a reencoder before sending to a sink.
///
/// This struct combines a `Reencode` implementation with an `InstructionSink` to automatically
/// convert parsed operators to encoded instructions during processing.
///
/// # Type Parameters
///
/// * `R` - A type implementing `Reencode` for operator-to-instruction conversion
/// * `S` - The sink that will receive the reencoded instructions
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Default)]
pub struct Rewrite<R, S> {
    /// The reencoder used to convert operators to instructions
    pub rewriter: R,
    /// The sink that receives converted instructions
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
/// A combined trait for types that can handle both instructions and operators.
///
/// This trait is automatically implemented for any type that implements both
/// `InstructionSink<E>` and `OperatorSink<E>`.
pub trait InstructionOperatorSink<E>: InstructionSink<E> + OperatorSink<E> {}
impl<E, T: InstructionSink<E> + OperatorSink<E> + ?Sized> InstructionOperatorSink<E> for T {}

/// A trait for types that can emit WASM instructions.
///
/// This trait allows types to generate and send instructions to a sink.
/// It's useful for templates, code generators, or instruction sequences.
pub trait InstructionSource<E>: InstructionOperatorSource<E> {
    /// Emits instructions to the provided sink.
    ///
    /// # Arguments
    ///
    /// * `sink` - The sink that will receive emitted instructions
    ///
    /// # Errors
    ///
    /// Returns an error if emission fails.
    fn emit_instruction(&self, sink: &mut (dyn InstructionSink<E> + '_)) -> Result<(), E>;
}

/// A trait for types that can emit WASM operators.
///
/// This trait allows types to generate and send operators to a sink.
/// It's useful when working with parsed WASM representations.
pub trait OperatorSource<E>: InstructionOperatorSource<E> {
    /// Emits operators to the provided sink.
    ///
    /// # Arguments
    ///
    /// * `sink` - The sink that will receive emitted operators
    ///
    /// # Errors
    ///
    /// Returns an error if emission fails.
    fn emit_operator(&self, sink: &mut (dyn OperatorSink<E> + '_)) -> Result<(), E>;
}

/// A trait for types that can emit either instructions or operators.
///
/// This base trait provides the most flexible emission mechanism,
/// allowing the source to work with any sink that handles both types.
pub trait InstructionOperatorSource<E> {
    /// Emits to a sink that can handle both instructions and operators.
    ///
    /// # Arguments
    ///
    /// * `sink` - A combined instruction/operator sink
    ///
    /// # Errors
    ///
    /// Returns an error if emission fails.
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
