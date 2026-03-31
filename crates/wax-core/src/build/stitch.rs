use super::*;

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
pub trait OperatorToInstructionStitchFn<'a, Context, E>: Fn(Operator<'a>) -> Self::Source
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
    fn stitch<'a>(&self, op: Operator<'a>) -> Self::Source<'a> {
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
impl<Context, E, A: OperatorStitch<Context, E>, T: OperatorSource<Context, E>>
    InstructionOperatorSource<Context, E> for OperatorStitched<A, T>
{
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
                a.stitch(instruction.clone()).emit_operator(ctx, sink)
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
pub trait OperatorToInstructionStitch<Context, E> {
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
impl<Context, E, A: OperatorToInstructionStitch<Context, E>, T: InstructionSink<Context, E>>
    OperatorSink<Context, E> for CrossStitch<A, T>
{
    fn operator(&mut self, ctx: &mut Context, op: &Operator<'_>) -> Result<(), E> {
        let CrossStitch {
            stitcher: a,
            target: t,
        } = self;
        a.stitch(op.clone()).emit_instruction(ctx, t)
    }
}
impl<Context, E, A: OperatorToInstructionStitch<Context, E>, T: OperatorSource<Context, E>>
    InstructionOperatorSource<Context, E> for CrossStitch<A, T>
{
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
            &mut FromFn::operator_sink(|ctx, op| a.stitch(op.clone()).emit_instruction(ctx, sink)),
        )
    }
}
impl<Context, E, A: OperatorToInstructionStitch<Context, E>, T: OperatorSource<Context, E>>
    InstructionSource<Context, E> for CrossStitch<A, T>
{
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
pub trait InstructionIterStitch<Context, E>:
    for<'a> InstructionStitch<Context, E, Source<'a>: InstructionIterSource<Context, E>>
{
}
impl<
    Context,
    E,
    T: for<'a> InstructionStitch<Context, E, Source<'a>: InstructionIterSource<Context, E>>,
> InstructionIterStitch<Context, E> for T
{
}
pub trait OperatorIterStitch<Context, E>:
    for<'a> OperatorStitch<Context, E, Source<'a>: OperatorIterSource<Context, E>>
{
}
impl<Context, E, T: for<'a> OperatorStitch<Context, E, Source<'a>: OperatorIterSource<Context, E>>>
    OperatorIterStitch<Context, E> for T
{
}
pub trait OperatorToInstructionIterStitch<Context, E>:
    for<'a> OperatorToInstructionStitch<Context, E, Source<'a>: InstructionIterSource<Context, E>>
{
}
impl<
    Context,
    E,
    T: for<'a> OperatorToInstructionStitch<Context, E, Source<'a>: InstructionIterSource<Context, E>>,
> OperatorToInstructionIterStitch<Context, E> for T
{
}
#[cfg(feature = "gen-blocks")]
const _: () = {
    impl<Context, E, A: InstructionIterStitch<Context, E>, T: InstructionIterSource<Context, E>>
        InstructionIterSource<Context, E> for InstructionStitched<A, T>
    {
        fn instructions<'a>(
            &'a self,
            ctx: &'a mut Context,
        ) -> Box<dyn Iterator<Item = Result<Instruction<'static>, E>> + 'a>
        where
            E: 'a,
        {
            Box::new(gen_block! {
            let InstructionStitched{stitcher:a,target:t} = self;
            for target_instruction in t.instructions(ctx){
            match target_instruction{
            Ok(instruction) => for s in a.stitch(instruction).instructions(ctx){
            yield s;
            },
            Err(e) => yield Err(e),}
            }
                        })
        }
    }
    impl<Context, E, A: OperatorIterStitch<Context, E>, T: OperatorIterSource<Context, E>>
        OperatorIterSource<Context, E> for OperatorStitched<A, T>
    {
        fn operators<'a>(
            &'a self,
            ctx: &'a mut Context,
        ) -> Box<dyn Iterator<Item = Result<Operator<'static>, E>> + 'a>
        where
            E: 'a,
        {
            Box::new(gen_block! {
            let OperatorStitched{stitcher:a,target:t} = self;
            for target_operator in t.operators(ctx){
            match target_operator{
            Ok(op) => for s in a.stitch(op).operators(ctx){
            yield s;
            },
            Err(e) => yield Err(e),}
            }
                        })
        }
    }
    impl<
        Context,
        E,
        A: OperatorToInstructionIterStitch<Context, E>,
        T: OperatorIterSource<Context, E>,
    > InstructionIterSource<Context, E> for CrossStitch<A, T>
    {
        fn instructions<'a>(
            &'a self,
            ctx: &'a mut Context,
        ) -> Box<dyn Iterator<Item = Result<Instruction<'static>, E>> + 'a>
        where
            E: 'a,
        {
            Box::new(gen_block! {
            let CrossStitch{stitcher:a,target:t} = self;
            for target_operator in t.operators(ctx){
            match target_operator{
            Ok(op) => for s in a.stitch(op).instructions(ctx){
            yield s;
            },
            Err(e) => yield Err(e),}
            }
                        })
        }
    }
};
