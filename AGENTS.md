# Agent Guidelines

## Commands
- **Build/Test:** Use `cargo build` to compile, `cargo test` to run all tests, or `cargo test <name>` for a specific test. Use `cargo check` for fast type-checking.
- **Lint/Format:** Ensure code quality with `cargo clippy -- -D warnings` and format consistently using `cargo fmt`.
- **Verify:** Before submitting, run `bash script/bench.sh` to benchmark performance and verify correctness.

## Code Style & Conventions
- **Formatting:** Always run `cargo fmt` before committing. Use `snake_case` for variables and functions, and `PascalCase` for types and structs.
- **Imports:** Organize imports by grouping: standard library (`std`), external crates, then internal modules.
- **Safety:** Avoid using `.unwrap()` or `.expect()` in library code. Prefer returning `Result` and using the `?` operator for error handling.
- **Types:** Favor type inference for local variables, but always specify explicit types in function signatures for clarity.

## Workflow
1. Format your code with `cargo fmt`.
2. Run linter: `cargo clippy -- -D warnings`.
3. Verify benchmarks and correctness: `bash script/bench.sh`.
4. Commit changes using Conventional Commits (e.g., `<type>[optional scope]: <description>`).
   Accepted types include: fix:, feat:, build:, chore:, ci:, docs:, style:, refactor:, perf:, test:, and others.
