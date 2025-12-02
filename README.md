# wax

A Rust library for WebAssembly (WASM) encoding and parsing utilities.

## Overview

`wax` provides a set of tools for working with WebAssembly bytecode at a low level. It focuses on instruction manipulation, code transformation, and analysis of WASM modules. The library is built on top of the `wasm-encoder` and `wasmparser` crates, providing higher-level abstractions for common WASM operations.

## Features

- **Instruction Building**: Flexible sink/source patterns for constructing WASM instructions
- **Code Rewriting**: Type and function index rewriting with shimming support
- **Code Analysis**: Dead code elimination and structured control flow analysis
- **Lowering Transformations**: 
  - Tail call optimization
  - Return statement cleanup
  - Global variable globalization

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
wax-core = "0.1.0"
```

## Usage

### Basic Instruction Manipulation

```rust
use wax_core::build::InstructionSink;
use wasm_encoder::{Function, Instruction};

// Create a function and add instructions
let mut func = Function::new([]);
func.instruction(&Instruction::I32Const(42));
func.instruction(&Instruction::Drop);
```

### Using Instruction Sinks

```rust
use wax_core::build::{FromFn, InstructionSink};
use wasm_encoder::Instruction;

// Create a custom instruction sink from a closure
let mut sink = FromFn::instruction_sink(|instr: &Instruction| {
    println!("Processing: {:?}", instr);
    Ok::<(), ()>(())
});

sink.instruction(&Instruction::Nop)?;
```

### Code Rewriting

```rust
use wax_core::rewrite::{Rewrite, RewriteKind, NumImports};

// Set up rewriting for function indices
let rewrite = Rewrite {
    function_types: RewriteKind::None { imports: NumImports { imports: 5 } },
    functions: RewriteKind::Sidecar { imports: NumImports { imports: 3 } },
};

// Rewrite an instruction
let original = Instruction::Call(10);
rewrite.rewrite(&original, |rewritten| {
    // Process the rewritten instruction
    println!("{:?}", rewritten);
});
```

### Dead Code Elimination

```rust
use wax_core::analysis::dce::{DceStack, dce};
use wasmparser::Operator;

let mut stack = DceStack::new();

// Check if an instruction is dead code
let is_dead = dce(&mut stack, &Operator::Nop);
```

## Modules

### `build`
Provides traits and types for building WASM instructions:
- `InstructionSink` and `OperatorSink`: Traits for consuming instructions
- `InstructionSource` and `OperatorSource`: Traits for emitting instructions
- `FromFn`: Wrapper for creating sinks from closures

### `rewrite`
Handles rewriting of WASM function and type indices:
- `Rewrite`: Configuration for index rewriting
- `Tracker`: Tracks and manages indices during transformation
- `Shimmer`: Trait for generating shim functions

### `analysis`
Code analysis utilities:
- `dce`: Dead code elimination analysis
- `scf`: Structured control flow analysis

### `lowering`
Code transformation passes:
- `tail_calls`: Tail call optimization
- `clean_rets`: Return statement cleanup and transformation
- `globalize`: Global variable handling and function signature transformation

## Requirements

- Rust 2024 edition or later
- `no_std` compatible (uses `alloc`)

## License

This project is licensed under the MPL-2.0 license.

## Contributing

Contributions are welcome! Please feel free to submit pull requests or open issues for bugs and feature requests.

## Design Philosophy

The library is designed to be `no_std` compatible, making it suitable for embedded environments and WebAssembly targets. It uses the `alloc` crate for dynamic allocations and provides zero-cost abstractions where possible.
