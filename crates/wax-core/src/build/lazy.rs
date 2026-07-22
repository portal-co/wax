//! Deferred instruction-source transformations.
//!
//! This module is a body-composition primitive, not a function graph scheduler.
//! A demand planner owns function/snapshot identity and invokes a wrapper only
//! for a selected body facet.

use super::{
    InstructionOperatorSink, InstructionOperatorSource, InstructionSink, InstructionSource,
};

/// A transformation applied only when a wrapped source is emitted.
///
/// Implementations must treat `source` as deferred input. Construction and
/// planning may inspect immutable transform metadata, but must not emit or
/// iterate the source. Mutable `Context` is supplied only at emission time.
pub trait LazyInstructionTransform<Context, E, Source> {
    fn emit(
        &self,
        ctx: &mut Context,
        source: &Source,
        sink: &mut (dyn InstructionOperatorSink<Context, E> + '_),
    ) -> Result<(), E>;

    fn emit_instruction(
        &self,
        ctx: &mut Context,
        source: &Source,
        sink: &mut (dyn InstructionSink<Context, E> + '_),
    ) -> Result<(), E>;
}

/// A zero-work-until-emission wrapper for a WASM body transformation.
#[derive(Clone, Debug)]
pub struct LazyTransform<Source, Transform> {
    source: Source,
    transform: Transform,
}

impl<Source, Transform> LazyTransform<Source, Transform> {
    /// Wrap `source` without iterating or emitting it.
    pub const fn new(source: Source, transform: Transform) -> Self {
        Self { source, transform }
    }

    /// The deferred input source.
    pub fn source(&self) -> &Source {
        &self.source
    }

    /// Immutable transform metadata/configuration.
    pub fn transform(&self) -> &Transform {
        &self.transform
    }

    pub fn into_parts(self) -> (Source, Transform) {
        (self.source, self.transform)
    }
}

impl<Context, E, Source, Transform> InstructionOperatorSource<Context, E>
    for LazyTransform<Source, Transform>
where
    Transform: LazyInstructionTransform<Context, E, Source>,
{
    fn emit(
        &self,
        ctx: &mut Context,
        sink: &mut (dyn InstructionOperatorSink<Context, E> + '_),
    ) -> Result<(), E> {
        self.transform.emit(ctx, &self.source, sink)
    }
}

impl<Context, E, Source, Transform> InstructionSource<Context, E>
    for LazyTransform<Source, Transform>
where
    Transform: LazyInstructionTransform<Context, E, Source>,
{
    fn emit_instruction(
        &self,
        ctx: &mut Context,
        sink: &mut (dyn InstructionSink<Context, E> + '_),
    ) -> Result<(), E> {
        self.transform.emit_instruction(ctx, &self.source, sink)
    }
}

#[cfg(test)]
extern crate std;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::build::OperatorSink;
    use core::cell::Cell;
    use wasm_encoder::Instruction;
    use wasmparser::Operator;

    struct Source<'a>(&'a Cell<u8>);

    impl InstructionOperatorSource<(), ()> for Source<'_> {
        fn emit(
            &self,
            _: &mut (),
            sink: &mut (dyn InstructionOperatorSink<(), ()> + '_),
        ) -> Result<(), ()> {
            self.0.set(self.0.get() + 1);
            sink.instruction(&mut (), &Instruction::Nop)
        }
    }

    impl InstructionSource<(), ()> for Source<'_> {
        fn emit_instruction(
            &self,
            _: &mut (),
            sink: &mut (dyn InstructionSink<(), ()> + '_),
        ) -> Result<(), ()> {
            self.0.set(self.0.get() + 1);
            sink.instruction(&mut (), &Instruction::Nop)
        }
    }

    struct Transform<'a>(&'a Cell<u8>);

    impl LazyInstructionTransform<(), (), Source<'_>> for Transform<'_> {
        fn emit(
            &self,
            ctx: &mut (),
            source: &Source<'_>,
            sink: &mut (dyn InstructionOperatorSink<(), ()> + '_),
        ) -> Result<(), ()> {
            self.0.set(self.0.get() + 1);
            source.emit(ctx, sink)
        }

        fn emit_instruction(
            &self,
            ctx: &mut (),
            source: &Source<'_>,
            sink: &mut (dyn InstructionSink<(), ()> + '_),
        ) -> Result<(), ()> {
            self.0.set(self.0.get() + 1);
            source.emit_instruction(ctx, sink)
        }
    }

    struct Sink;

    impl InstructionSink<(), ()> for Sink {
        fn instruction(&mut self, _: &mut (), _: &Instruction<'_>) -> Result<(), ()> {
            Ok(())
        }
    }

    impl OperatorSink<(), ()> for Sink {
        fn operator(&mut self, _: &mut (), _: &Operator<'_>) -> Result<(), ()> {
            Ok(())
        }
    }

    #[test]
    fn construction_does_not_emit_or_iter_source() {
        let source_calls = Cell::new(0);
        let transform_calls = Cell::new(0);
        let lazy = LazyTransform::new(Source(&source_calls), Transform(&transform_calls));
        assert_eq!(source_calls.get(), 0);
        assert_eq!(transform_calls.get(), 0);

        lazy.emit_instruction(&mut (), &mut Sink).unwrap();
        assert_eq!(source_calls.get(), 1);
        assert_eq!(transform_calls.get(), 1);
    }
}
