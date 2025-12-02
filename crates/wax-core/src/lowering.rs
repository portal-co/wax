//! Code transformation (lowering) passes for WASM.
//!
//! This module provides various transformations that lower or simplify WASM code:
//!
//! - [`tail_calls`]: Tail call optimization transformations (**⚠️ Work in Progress**)
//! - [`clean_rets`]: Return statement cleanup and transformation (**Implemented**)
//! - [`globalize`]: Global variable handling and function signature transformation (**Implemented**)
//!
//! ## Implementation Status
//!
//! - ✅ **Implemented**: [`clean_rets`], [`globalize`]
//! - 🚧 **Work in Progress**: [`tail_calls`]

use super::*;
pub mod tail_calls;
pub mod clean_rets;
pub mod globalize;