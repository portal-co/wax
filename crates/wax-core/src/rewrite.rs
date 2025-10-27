use super::*;
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Default)]
pub struct Rewrite {
    pub function_types: RewriteKind,
    pub functions: RewriteKind,
}
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Default)]
pub struct Tracker<T> {
    pub idx: u32,
    pub all: Vec<T>,
}
impl<T> Tracker<T>{
    pub fn push(&mut self, a: T) -> u32{
        let i = self.idx;
        self.idx += 1;
        self.all.push(a);
        return i;
    }
}
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum RewriteKind {
    None { imports: NumImports },
    Sidecar { imports: NumImports },
}
impl Default for RewriteKind {
    fn default() -> Self {
        Self::None {
            imports: Default::default(),
        }
    }
}
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Default)]
pub struct NumImports {
    pub imports: u32,
}
impl NumImports {
    pub fn apply<T>(&self, tracker: &mut Tracker<T>) {
        tracker.idx += self.imports;
    }
}
impl RewriteKind {
    pub fn rewrite(&self, a: u32, orig: bool) -> u32 {
        match self {
            RewriteKind::None { imports } => a + imports.imports,
            RewriteKind::Sidecar { imports } => {
                imports.imports + ((a << 1) | (if orig { 1 } else { 0 }))
            }
        }
    }
}
impl Rewrite {
    fn ty(&self, a: u32) -> u32 {
        self.function_types.rewrite(a, false)
    }
    fn block_ty(&self, a: BlockType) -> BlockType {
        match a {
            BlockType::FunctionType(f) => {
                BlockType::FunctionType(self.function_types.rewrite(f, true))
            }
            a => a,
        }
    }
    pub fn rewrite<T>(&self, i: &Instruction<'_>, go: impl FnOnce(&Instruction<'_>) -> T) -> T {
        match i {
            //Calls
            Instruction::ReturnCallIndirect {
                type_index,
                table_index,
            } => go(&Instruction::ReturnCallIndirect {
                type_index: self.ty(*type_index),
                table_index: *table_index,
            }),
            Instruction::ReturnCallRef(ty) => go(&Instruction::ReturnCallRef(self.ty(*ty))),
            Instruction::CallIndirect {
                type_index,
                table_index,
            } => go(&Instruction::CallIndirect {
                type_index: self.ty(*type_index),
                table_index: *table_index,
            }),
            Instruction::CallRef(ty) => go(&Instruction::CallRef(self.ty(*ty))),
            //Function calls
            Instruction::RefFunc(a) => go(&Instruction::RefFunc(self.functions.rewrite(*a, false))),
            Instruction::Call(a) => go(&Instruction::Call(self.functions.rewrite(*a, false))),
            Instruction::ReturnCall(a) => {
                go(&Instruction::ReturnCall(self.functions.rewrite(*a, false)))
            }
            //Blocks
            Instruction::If(a) => go(&Instruction::If(self.block_ty(*a))),
            Instruction::Block(a) => go(&Instruction::Block(self.block_ty(*a))),
            Instruction::Loop(a) => go(&Instruction::Loop(self.block_ty(*a))),
            //Blocks: Exceptions
            Instruction::TryTable(a, b) => go(&Instruction::TryTable(self.block_ty(*a), b.clone())),
            i => go(i),
        }
    }
}
