//! Tail call optimization transformations.
//!
//! **⚠️ Work in Progress**: This module is currently under development and does not yet
//! contain implemented functionality. The tail call optimization transformations are
//! planned but not yet available.
//!
//! Future implementations will provide transformations to optimize tail calls
//! in WebAssembly functions.

use alloc::collections::btree_set::BTreeSet;
use wasm_encoder::Catch;

use super::*;
