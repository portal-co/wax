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
impl<T> FromFn<T> {
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
    pub fn instruction_sink<Context, E>(a: T) -> Self
    where
        T: FnMut(&mut Context, &Instruction<'_>) -> Result<(), E>,
    {
        Self(a)
    }
}
impl<T> FromFn<T> {
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
    pub fn operator_sink<Context, E>(a: T) -> Self
    where
        T: FnMut(&mut Context, &Operator<'_>) -> Result<(), E>,
    {
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
pub trait InstructionSink<Context, E> {
    /// Processes a single WASM instruction.
    ///
    /// # Arguments
    ///
    /// * `instruction` - The instruction to process
    ///
    /// # Errors
    ///
    /// Returns an error if the instruction cannot be processed.
    fn instruction(&mut self, ctx: &mut Context, instruction: &Instruction<'_>) -> Result<(), E>;
}
impl<Context, E, T: FnMut(&mut Context, &Instruction<'_>) -> Result<(), E>>
    InstructionSink<Context, E> for FromFn<T>
{
    fn instruction(&mut self, ctx: &mut Context, instruction: &Instruction<'_>) -> Result<(), E> {
        let FromFn(a) = self;
        a(ctx, instruction)
    }
}
impl<Context, E, T: InstructionSink<Context, E> + ?Sized> InstructionSink<Context, E>
    for &'_ mut T
{
    fn instruction(&mut self, ctx: &mut Context, instruction: &Instruction<'_>) -> Result<(), E> {
        (&mut **self).instruction(ctx, instruction)
    }
}
impl<Context, E, T: OperatorSink<Context, E> + ?Sized> OperatorSink<Context, E> for &'_ mut T {
    fn operator(&mut self, ctx: &mut Context, op: &Operator<'_>) -> Result<(), E> {
        (&mut **self).operator(ctx, op)
    }
}
impl<Context, E, T: InstructionSink<Context, E> + ?Sized> InstructionSink<Context, E> for Box<T> {
    fn instruction(&mut self, ctx: &mut Context, instruction: &Instruction<'_>) -> Result<(), E> {
        (&mut **self).instruction(ctx, instruction)
    }
}
impl<Context, E, T: OperatorSink<Context, E> + ?Sized> OperatorSink<Context, E> for Box<T> {
    fn operator(&mut self, ctx: &mut Context, op: &Operator<'_>) -> Result<(), E> {
        (&mut **self).operator(ctx, op)
    }
}
impl<Context, E> InstructionSink<Context, E> for wasm_encoder::Function {
    fn instruction(&mut self, ctx: &mut Context, instruction: &Instruction<'_>) -> Result<(), E> {
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
pub trait OperatorSink<Context, E> {
    /// Processes a single WASM operator.
    ///
    /// # Arguments
    ///
    /// * `op` - The operator to process
    ///
    /// # Errors
    ///
    /// Returns an error if the operator cannot be processed.
    fn operator(&mut self, ctx: &mut Context, op: &Operator<'_>) -> Result<(), E>;
}
impl<Context, E, T: FnMut(&mut Context, &Operator<'_>) -> Result<(), E>> OperatorSink<Context, E>
    for FromFn<T>
{
    fn operator(&mut self, ctx: &mut Context, op: &Operator<'_>) -> Result<(), E> {
        let FromFn(f) = self;
        f(ctx, op)
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
impl<
    Context,
    R: Reencode,
    S: InstructionSink<Context, E>,
    E: From<wasm_encoder::reencode::Error<R::Error>>,
> OperatorSink<Context, E> for Rewrite<R, S>
{
    fn operator(&mut self, ctx: &mut Context, op: &Operator<'_>) -> Result<(), E> {
        self.sink
            .instruction(ctx, &self.rewriter.instruction(op.clone())?)
    }
}
impl<Context, R, S: InstructionSink<Context, E>, E> InstructionSink<Context, E> for Rewrite<R, S> {
    fn instruction(&mut self, ctx: &mut Context, instruction: &Instruction<'_>) -> Result<(), E> {
        self.sink.instruction(ctx, instruction)
    }
}
/// A combined trait for types that can handle both instructions and operators.
///
/// This trait is automatically implemented for any type that implements both
/// `InstructionSink<E>` and `OperatorSink<E>`.
pub trait InstructionOperatorSink<Context, E>:
    InstructionSink<Context, E> + OperatorSink<Context, E>
{
}
impl<Context, E, T: InstructionSink<Context, E> + OperatorSink<Context, E> + ?Sized>
    InstructionOperatorSink<Context, E> for T
{
}

/// A trait for types that can emit WASM instructions.
///
/// This trait allows types to generate and send instructions to a sink.
/// It's useful for templates, code generators, or instruction sequences.
pub trait InstructionSource<Context, E>: InstructionOperatorSource<Context, E> {
    /// Emits instructions to the provided sink.
    ///
    /// # Arguments
    ///
    /// * `sink` - The sink that will receive emitted instructions
    ///
    /// # Errors
    ///
    /// Returns an error if emission fails.
    fn emit_instruction(
        &self,
        ctx: &mut Context,
        sink: &mut (dyn InstructionSink<Context, E> + '_),
    ) -> Result<(), E>;
}

pub trait InstructionIterSource<Context, E>: InstructionSource<Context, E> {
    fn instructions<'a>(
        &'a self,
        ctx: &'a mut Context,
    ) -> Box<dyn Iterator<Item = Result<Instruction<'static>, E>> + 'a>
    where
        E: 'a;
}

/// A trait for types that can emit WASM operators.
///
/// This trait allows types to generate and send operators to a sink.
/// It's useful when working with parsed WASM representations.
pub trait OperatorSource<Context, E>: InstructionOperatorSource<Context, E> {
    /// Emits operators to the provided sink.
    ///
    /// # Arguments
    ///
    /// * `sink` - The sink that will receive emitted operators
    ///
    /// # Errors
    ///
    /// Returns an error if emission fails.
    fn emit_operator(
        &self,
        ctx: &mut Context,
        sink: &mut (dyn OperatorSink<Context, E> + '_),
    ) -> Result<(), E>;
}

pub trait OperatorIterSource<Context, E>: OperatorSource<Context, E> {
    fn operators<'a>(
        &'a self,
        ctx: &'a mut Context,
    ) -> Box<dyn Iterator<Item = Result<Operator<'static>, E>> + 'a>
    where
        E: 'a;
}

/// A trait for types that can emit either instructions or operators.
///
/// This base trait provides the most flexible emission mechanism,
/// allowing the source to work with any sink that handles both types.
pub trait InstructionOperatorSource<Context, E> {
    /// Emits to a sink that can handle both instructions and operators.
    ///
    /// # Arguments
    ///
    /// * `sink` - A combined instruction/operator sink
    ///
    /// # Errors
    ///
    /// Returns an error if emission fails.
    fn emit(
        &self,
        ctx: &mut Context,
        sink: &mut (dyn InstructionOperatorSink<Context, E> + '_),
    ) -> Result<(), E>;
}
#[impl_for_tuples(12)]
impl<Context, E> InstructionOperatorSource<Context, E> for Tuple {
    for_tuples!(where #(Tuple: InstructionOperatorSource<Context, E>)*);
    fn emit(
        &self,
        ctx: &mut Context,
        sink: &mut (dyn InstructionOperatorSink<Context, E> + '_),
    ) -> Result<(), E> {
        for_tuples!(#(Tuple.emit(ctx, sink)?;)*);
        Ok(())
    }
}
#[impl_for_tuples(12)]
impl<Context, E> InstructionSource<Context, E> for Tuple {
    for_tuples!(where #(Tuple: InstructionSource<Context, E>)*);
    fn emit_instruction(
        &self,
        ctx: &mut Context,
        sink: &mut (dyn InstructionSink<Context, E> + '_),
    ) -> Result<(), E> {
        for_tuples!(#(Tuple.emit_instruction(ctx, sink)?;)*);
        Ok(())
    }
}
#[impl_for_tuples(12)]
impl<Context, E> OperatorSource<Context, E> for Tuple {
    for_tuples!(where #(Tuple: OperatorSource<Context, E>)*);
    fn emit_operator(
        &self,
        ctx: &mut Context,
        sink: &mut (dyn OperatorSink<Context, E> + '_),
    ) -> Result<(), E> {
        for_tuples!(#(Tuple.emit_operator(ctx, sink)?;)*);
        Ok(())
    }
}
pub trait InstructionStitchFn<'a, Context, E>: Fn(Instruction<'a>) -> Self::Source {
    type Source: InstructionSource<Context, E>;
}
impl<'a, Context, E, T: Fn(Instruction<'a>) -> S, S: InstructionSource<Context, E>>
    InstructionStitchFn<'a, Context, E> for T
{
    type Source = S;
}
pub trait OperatorStitchFn<'a, Context, E>: Fn(Operator<'a>) -> Self::Source {
    type Source: OperatorSource<Context, E>;
}
impl<'a, Context, E, T: Fn(Operator<'a>) -> S, S: OperatorSource<Context, E>>
    OperatorStitchFn<'a, Context, E> for T
{
    type Source = S;
}
pub trait OperatorToInstructionStitchFn<'a, Context, E>:
    Fn(Operator<'a>) -> Self::Source
where
    Self::Source: InstructionSource<Context, E>,
{
    type Source: InstructionSource<Context, E>;
}
impl<'a, Context, E, T: Fn(Operator<'a>) -> S, S: InstructionSource<Context, E>>
    OperatorToInstructionStitchFn<'a, Context, E> for T
{
    type Source = S;
}
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Default)]
pub struct OperatorStitched<A, T> {
    pub stitcher: A,
    pub target: T,
}
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Default)]
pub struct InstructionStitched<A, T> {
    pub stitcher: A,
    pub target: T,
}
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Default)]
pub struct CrossStitch<A, T> {
    pub stitcher: A,
    pub target: T,
}
pub trait InstructionStitch<Context, E> {
    type Source<'a>: InstructionSource<Context, E>;
    fn stitch<'a>(&self, instruction: Instruction<'a>) -> Self::Source<'a>;
}
impl<Context, E, T: for<'a> InstructionStitchFn<'a, Context, E>> InstructionStitch<Context, E>
    for T
{
    type Source<'a> = <T as InstructionStitchFn<'a, Context, E>>::Source;
    fn stitch<'a>(&self, instruction: Instruction<'a>) -> Self::Source<'a> {
        self(instruction)
    }
}
impl<Context, E, A: InstructionStitch<Context, E>, T: InstructionSink<Context, E>>
    InstructionSink<Context, E> for InstructionStitched<A, T>
{
    fn instruction(&mut self, ctx: &mut Context, instruction: &Instruction<'_>) -> Result<(), E> {
        let InstructionStitched {
            stitcher: a,
            target: t,
        } = self;
        a.stitch(instruction.clone()).emit_instruction(ctx, t)
    }
}
impl<Context, E, A: InstructionStitch<Context, E>, T: InstructionSource<Context, E>>
    InstructionOperatorSource<Context, E> for InstructionStitched<A, T>
{
    fn emit(
        &self,
        ctx: &mut Context,
        sink: &mut (dyn InstructionOperatorSink<Context, E> + '_),
    ) -> Result<(), E> {
        let InstructionStitched {
            stitcher: a,
            target: t,
        } = self;
        t.emit_instruction(
            ctx,
            &mut FromFn::instruction_sink(|ctx, instruction| {
                a.stitch(instruction.clone()).emit_instruction(ctx, sink)
            }),
        )
    }
}
impl<Context, E, A: InstructionStitch<Context, E>, T: InstructionSource<Context, E>>
    InstructionSource<Context, E> for InstructionStitched<A, T>
{
    fn emit_instruction(
        &self,
        ctx: &mut Context,
        sink: &mut (dyn InstructionSink<Context, E> + '_),
    ) -> Result<(), E> {
        let InstructionStitched {
            stitcher: a,
            target: t,
        } = self;
        t.emit_instruction(
            ctx,
            &mut FromFn::instruction_sink(|ctx, instruction| {
                a.stitch(instruction.clone()).emit_instruction(ctx, sink)
            }),
        )
    }
}
pub trait OperatorStitch<Context, E> {
    type Source<'a>: OperatorSource<Context, E>;
    fn stitch<'a>(&self, op: Operator<'a>) -> Self::Source<'a>;
}
impl<Context, E, T: for<'a> OperatorStitchFn<'a, Context, E>> OperatorStitch<Context, E> for T {
    type Source<'a> = <T as OperatorStitchFn<'a, Context, E>>::Source;
    fn stitch<'a>(&     self, op: Operator<'a>) -> Self::Source<'a> {
        self(op)
    }
}
impl<Context, E, A: OperatorStitch<Context, E>, T: OperatorSink<Context, E>>
    OperatorSink<Context, E> for OperatorStitched<A, T>
{
    fn operator(&mut self, ctx: &mut Context, op: &Operator<'_>) -> Result<(), E> {
        let OperatorStitched {
            stitcher: a,
            target: t,
        } = self;
        a.stitch(op.clone()).emit_operator(ctx, t)
    }
}
impl<Context, E, A: OperatorStitch<Context,E>, T:OperatorSource<Context,E>> InstructionOperatorSource<Context, E> for OperatorStitched<A, T> {
    fn emit(
        &self,
        ctx: &mut Context,
        sink: &mut (dyn InstructionOperatorSink<Context, E> + '_),
    ) -> Result<(), E> {
        let OperatorStitched {
            stitcher: a,
            target: t,
        } = self;
        t.emit_operator(
            ctx,
            &mut FromFn::operator_sink(|ctx, instruction| {
                a.stitch(instruction.clone())
                    .emit_operator(ctx, sink)
            }),
        )
    }
}
impl<Context, E, A: OperatorStitch<Context, E>, T: OperatorSource<Context, E>>
    OperatorSource<Context, E> for OperatorStitched<A, T>
{
    fn emit_operator(
        &self,
        ctx: &mut Context,
        sink: &mut (dyn OperatorSink<Context, E> + '_),
    ) -> Result<(), E> {
        let OperatorStitched {
            stitcher: a,
            target: t,
        } = self;
        t.emit_operator(
            ctx,
            &mut FromFn::operator_sink(|ctx, op| a.stitch(op.clone()).emit_operator(ctx, sink)),
        )
    }
}
pub trait OperatorToInstructionStitch<Context,E>{
    type Source<'a>: InstructionSource<Context, E>;
    fn stitch<'a>(&self, op: Operator<'a>) -> Self::Source<'a>;
}
impl<Context, E, T: for<'a> OperatorToInstructionStitchFn<'a, Context, E>>
    OperatorToInstructionStitch<Context, E> for T
{
    type Source<'a> = <T as OperatorToInstructionStitchFn<'a, Context, E>>::Source;
    fn stitch<'a>(&self, op: Operator<'a>) -> Self::Source<'a> {
        self(op)
    }
}
impl<Context, E, A: OperatorToInstructionStitch<Context, E>, T: InstructionSink<Context, E>> OperatorSink<Context, E> for CrossStitch<A, T> {
    fn operator(&mut self, ctx: &mut Context, op: &Operator<'_>) -> Result<(), E> {
        let CrossStitch {
            stitcher: a,
            target: t,
        } = self;
        a.stitch(op.clone()).emit_instruction(ctx, t)
    }
}
impl<Context, E, A: OperatorToInstructionStitch<Context, E>, T: OperatorSource<Context, E>> InstructionOperatorSource<Context, E> for CrossStitch<A, T>{
    fn emit(
        &self,
        ctx: &mut Context,
        sink: &mut (dyn InstructionOperatorSink<Context, E> + '_),
    ) -> Result<(), E> {
        let CrossStitch {
            stitcher: a,
            target: t,
        } = self;
        t.emit_operator(
            ctx,
            &mut FromFn::operator_sink(|ctx, op| {
                a.stitch(op.clone())
                    .emit_instruction(ctx, sink)
            }),
        )
    }
}
impl<Context, E, A: OperatorToInstructionStitch<Context, E>, T: OperatorSource<Context, E>> InstructionSource<Context, E> for CrossStitch<A, T>{
    fn emit_instruction(
        &self,
        ctx: &mut Context,
        sink: &mut (dyn InstructionSink<Context, E> + '_),
    ) -> Result<(), E> {
        let CrossStitch {
            stitcher: a,
            target: t,
        } = self;
        t.emit_operator(
            ctx,
            &mut FromFn::operator_sink(|ctx, op| a.stitch(op.clone()).emit_instruction(ctx, sink)),
        )   
    }
}
   
#[cfg(feature = "gen-blocks")]
macro_rules! gen_block {
    ($($e:expr)*) => {
        gen move { $($e)* }
    };
}
#[cfg(feature = "gen-blocks")]
#[impl_for_tuples(12)]
impl<Context, E> InstructionIterSource<Context, E> for Tuple {
    for_tuples!(where #(Tuple: InstructionIterSource<Context, E>)*);
    fn instructions<'a>(
        &'a self,
        ctx: &'a mut Context,
    ) -> Box<dyn Iterator<Item = Result<Instruction<'static>, E>> + 'a>
    where
        E: 'a,
    {
        Box::new(gen_block! {
            for_tuples!(#(for op in Tuple.instructions(ctx){
                yield op;
            });*)
        })
    }
}
#[cfg(feature = "gen-blocks")]
#[impl_for_tuples(12)]
impl<Context, E> OperatorIterSource<Context, E> for Tuple {
    for_tuples!(where #(Tuple: OperatorIterSource<Context, E>)*);
    fn operators<'a>(
        &'a self,
        ctx: &'a mut Context,
    ) -> Box<dyn Iterator<Item = Result<Operator<'static>, E>> + 'a>
    where
        E: 'a,
    {
        Box::new(gen_block! {
            for_tuples!(#(for op in Tuple.operators(ctx){
                yield op;
            });*)
        })
    }
}
