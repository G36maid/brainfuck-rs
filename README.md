# Brainfuck-rs

A minimalist Brainfuck toolchain written in Rust, featuring both an optimizing interpreter and a transpiler-based compiler.

## Components

1.  **bfi (Interpreter)**: A lightweight interpreter featuring **Instruction Folding (Run-Length Encoding)**, **Offset Optimization (Lazy Pointer)**, **Dead Code Elimination (DCE)**, **Clear Loop Optimization**, **Move Loop Optimization**, **Scan Loop Optimization**, **Parallel Assignment (Bulk Operations)**, and pre-computed jump targets. It defers pointer movements to merge operations, removes unreachable code/loops, folds consecutive identical operations (e.g., `>>>`, `+++`), optimizes `[-]` loops, transforms linear loops like `[->+<]` into optimized arithmetic, optimizes scan loops like `[<]` and `[>]`, and batches consecutive updates into parallel assignments.
2.  **bfc (Compiler)**: A transpiler that converts Brainfuck code to Rust. It performs **Instruction Folding**, **Offset Optimization**, **Dead Code Elimination**, **Clear Loop Optimization**, **Move Loop Optimization**, **Scan Loop Optimization**, and **Parallel Assignment** during transpilation, utilizing LLVM (`rustc`) for heavy optimizations (auto-vectorization, loop unrolling, etc.).

## Usage

### Prerequisites
* Rust toolchain (cargo, rustc)
* `hyperfine` (optional, for running benchmarks)

### 1. Interpreter (`bfi`)
Run a `.bf` file directly using the optimized interpreter.

```bash
cargo run --release --bin bfi -- examples/helloworld.bf
```

### 2. Compiler (`bfc`)
Transpile Brainfuck to optimized native machine code.

```bash
# 1. Transpile BF to Rust
cargo run --release --bin bfc < examples/helloworld.bf > hello.rs

# 2. Compile Rust to Machine Code (with optimizations)
rustc -O hello.rs -o hello

# 3. Run
./hello
```

## Benchmarks

A `mandelbrot.bf` generator was used to compare the performance of the interpreter versus the native compiler.

Run the benchmark script:
```bash
bash script/bench.sh
```

**Typical Results (Mandelbrot):**

| Implementation | Time (mean) | Speedup |
|----------------|-------------|---------|
| **Interpreter (bfi)** | ~3.50 s | 1x |
| **Compiler (bfc)** | ~0.52 s | ~6.7x |

*System: Linux, Rust 1.x*