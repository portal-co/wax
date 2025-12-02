//! Tail call optimization transformations.
//!
//! This module provides transformations related to tail call optimization
//! in WebAssembly functions.

use alloc::collections::btree_set::BTreeSet;
use wasm_encoder::Catch;

use super::*;
