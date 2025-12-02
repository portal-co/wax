//! Index rewriting and shimming utilities for WASM modules.
//!
//! This module provides functionality for rewriting function and type indices in WASM
//! instructions. This is useful when merging modules, adding imports, or performing
//! other transformations that change index spaces.
//!
//! The module supports two rewriting strategies:
//! - **None**: Simple offset-based rewriting for imports
//! - **Sidecar**: Dual-index encoding to maintain both original and modified indices

use wasm_encoder::FuncType;

use super::*;

/// Configuration for rewriting WASM function and type indices.
///
/// This struct holds the rewriting strategy for both function types and functions,
/// allowing you to independently control how each index space is transformed.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Default)]
pub struct Rewrite {
    /// The rewriting strategy for function type indices
    pub function_types: RewriteKind,
    /// The rewriting strategy for function indices
    pub functions: RewriteKind,
}
/// A tracker for managing indices and their associated data during transformations.
///
/// This type helps manage index allocation and data storage when adding new items
/// (like functions, types, or globals) to a WASM module.
///
/// # Type Parameters
///
/// * `T` - The type of data associated with each index
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Default)]
pub struct Tracker<T> {
    /// The next available index
    pub idx: u32,
    /// All tracked items in order
    pub all: Vec<T>,
}
impl<T> Tracker<T> {
    /// Adds a new item and returns its assigned index.
    ///
    /// # Arguments
    ///
    /// * `a` - The item to track
    ///
    /// # Returns
    ///
    /// The index assigned to the item
    pub fn push(&mut self, a: T) -> u32 {
        let i = self.idx;
        self.idx += 1;
        self.all.push(a);
        return i;
    }
}
/// The strategy for rewriting indices in WASM instructions.
///
/// This enum defines how indices should be transformed when processing instructions.
/// Different strategies are useful for different module transformation scenarios.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum RewriteKind {
    /// Simple offset-based rewriting.
    ///
    /// Indices are shifted by the number of imports. This is suitable for
    /// straightforward module merging where imports are prepended.
    None { imports: NumImports },
    
    /// Dual-index encoding with sidecar storage.
    ///
    /// Indices are encoded to maintain both original and modified values.
    /// The encoding uses: `imports + (index << 1) | original_bit`
    Sidecar { imports: NumImports },
}
impl Default for RewriteKind {
    fn default() -> Self {
        Self::None {
            imports: Default::default(),
        }
    }
}
/// The number of imported items in a WASM module.
///
/// This is used to calculate index offsets when rewriting indices.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Default)]
pub struct NumImports {
    /// The count of imports
    pub imports: u32,
}
impl NumImports {
    /// Applies the import offset to a tracker.
    ///
    /// This advances the tracker's index by the number of imports,
    /// reserving space for imported items.
    ///
    /// # Arguments
    ///
    /// * `tracker` - The tracker to modify
    pub fn apply<T>(&self, tracker: &mut Tracker<T>) {
        tracker.idx += self.imports;
    }
}
impl RewriteKind {
    /// Rewrites an index according to this strategy.
    ///
    /// # Arguments
    ///
    /// * `a` - The original index to rewrite
    /// * `orig` - Whether this represents an original (true) or modified (false) reference
    ///
    /// # Returns
    ///
    /// The rewritten index
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
/// A trait for generating shim functions.
///
/// Shims are small wrapper functions that adapt between different calling conventions
/// or function signatures. This trait allows transformation passes to generate the
/// necessary shims for imports and exports.
///
/// # Type Parameters
///
/// * `E` - The error type for instruction emission
pub trait Shimmer<E> {
    /// Generates a shim function.
    ///
    /// # Arguments
    ///
    /// * `old` - The index of the original function to shim
    /// * `func_types` - Mapping from function index to type index
    /// * `types` - The function type definitions
    /// * `kind` - Whether this is an import or export shim
    /// * `sink` - The sink to emit shim instructions to
    ///
    /// # Errors
    ///
    /// Returns an error if instruction emission fails.
    fn shim(
        &self,
        old: u32,
        func_types: &[u32],
        types: &[FuncType],
        kind: ShimKind,
        sink: &mut (dyn InstructionSink<E> + '_),
    ) -> Result<(), E>;
}

/// The type of shim function to generate.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum ShimKind {
    /// A shim wrapping an imported function
    Import,
    /// A shim wrapping an exported function
    Export,
}
