//! Code transformation (lowering) passes for WASM.
//!
//! This module provides various transformations that lower or simplify WASM code:
//!
//! - [`tail_calls`]: Tail call optimization transformations
//! - [`clean_rets`]: Return statement cleanup and transformation
//! - [`globalize`]: Global variable handling and function signature transformation

use super::*;
pub mod tail_calls;
pub mod clean_rets;
pub mod globalize;