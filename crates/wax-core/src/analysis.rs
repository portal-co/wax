//! Code analysis utilities for WASM bytecode.
//!
//! This module provides various analysis passes for WASM code:
//!
//! - [`dce`]: Dead code elimination analysis
//! - [`scf`]: Structured control flow analysis

use crate::*;
pub mod dce;
pub mod scf;