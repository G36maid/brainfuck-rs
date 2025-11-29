# Brainfuck-rs

A minimalist Brainfuck toolchain written in Rust, featuring an optimizing interpreter (`bfi`) and a transpiler-based compiler (`bfc`).

## Features

Both the interpreter (`bfi`) and compiler (`bfc`) share a common optimization pipeline that includes:

- **Instruction Folding (Run-Length Encoding)**: Merges consecutive identical operations (e.g., `>>>` becomes a single `PtrAdd(3)`).
- **Offset Optimization (Lazy Pointer)**: Defers pointer movements (`<`, `>`) to merge subsequent value updates (`+`, `-`) into single operations with a pointer offset. This significantly reduces the total number of instructions.
- **Parallel Assignment (Bulk Operations)**: Batches consecutive `ValAdd`, `ValSub`, and `Clear` operations into single `BulkAdd`/`BulkClear` instructions, improving performance by processing multiple memory updates at once.
- **Dead Code Elimination (DCE)**: Removes unreachable code, such as loops that will never be entered or redundant clear operations.
- **Clear Loop Optimization**: Replaces common patterns like `[-]` or `[+]` with a single `Clear` operation.
- **Move Loop Optimization**: Transforms multiplication loops like `[->+<]` into a series of efficient `MulAdd` operations, which calculate the result directly.
- **Scan Loop Optimization**: Replaces simple scan loops like `[<]` or `[>]` with a single `ScanLeft`/`ScanRight` operation to quickly find the next zero cell.

### Interpreter (`bfi`) vs. Compiler (`bfc`)

- **`bfi` (Interpreter)**: Executes the optimized instructions directly. It's fast, portable, and ideal for immediate execution without a separate compile step.
- **`bfc` (Compiler)**: Transpiles the optimized instructions into Rust code. The Rust compiler (`rustc`) then performs further, deeper optimizations (like auto-vectorization, loop unrolling, etc.) by leveraging the LLVM backend, resulting in the fastest possible native executable.

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