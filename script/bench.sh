#!/usr/bin/env bash
set -e

# 1. Prepare examples
if [ ! -f examples/mandelbrot.bf ]; then
    echo "Downloading mandelbrot.bf for benchmarking..."
    curl -s https://raw.githubusercontent.com/erikdubbelboer/brainfuck-jit/master/mandelbrot.bf -o examples/mandelbrot.bf
fi

# 2. Building (Release Mode)
echo "Building project..."
cargo build --release --quiet

# 3. Prepare the compiler version (BFC) executable
# First, use bfc to transpile bf to Rust, then use rustc to compile to machine code
echo "Compiling Brainfuck to Native Machine Code..."
./target/release/bfc < examples/mandelbrot.bf > target/mandelbrot_transpiled.rs
rustc -O -C opt-level=3 target/mandelbrot_transpiled.rs -o target/mandelbrot_native

# 4. Benchmark (Interpreter vs Native)
echo "Running Benchmark..."
hyperfine --warmup 3 \
    --export-markdown bench_results.md \
    -n "Interpreter (bfi)" "./target/release/bfi examples/mandelbrot.bf" \
    -n "Compiler (bfc -> native)" "./target/mandelbrot_native"

echo "Done! Results saved to bench_results.md"
