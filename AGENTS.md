# Repository Guidelines
tea-lang evolves toward a statically typed language that pairs compiled speed with scripting ergonomics. Use this guide to keep contributions aligned while the compiler, runtime, and CLI mature together.

## Project Structure & Module Organization
- `tea-cli/` hosts the end-user binary; keep CLI parsing in `src/main.rs` and surface-only utilities in `src/support/`.
- `tea-compiler/` contains the compilation pipeline. Stage code under `src/lexer`, `src/parser`, `src/runtime`, and `src/aot`; favor one module per phase to keep ownership clear.
- `tea-runtime/` implements the VM and standard runtime helpers shared across compiled programs.
- `examples/` holds executable `.tea` samples; group by topic (`examples/control_flow/fib.tea`) and note expected output in comments.
- `docs/` is the home for RFCs, diagrams, and architectural notes; reference new proposals there so readers can trace design intent.

## Build, Test, and Development Commands
- `cargo build -p tea-cli` produces the CLI and refreshes `target/debug/tea`; symlink or copy it into `bin/` if you need a stable path.
- `cargo run -p tea-cli -- examples/basics.tea` executes a script directly and is the fastest loop for language experiments.
- `cargo run -p tea-cli -- fmt path/or/file` reformats sources in place (directories recurse); add `--check` to gate CI or pre-commit hooks.
- `cargo run -p tea-cli -- test` drives the harness (use `--list`, `--filter`, or `--fail-fast` while iterating).
- `cargo test --workspace` runs the Rust unit suites; scope to a crate (`-p tea-compiler`) when iterating on a single stage.
- `scripts/e2e.sh` performs an LLVM build sanity check by compiling `examples/fib.tea` and asserting the emitted binary in `bin/fib` prints `832040`.

## Issue Tracker
- `bd init` is run once per repo to create `.beads/`; skip it after the first bootstrap.
- Day-to-day commands: `bd list`, `bd ready`, `bd create "task description"`, `bd close issue-id`.
- When you start work on an issue (`bd update issue-id --status in_progress`), create a fresh git branch named after the ticket (`git checkout -b tee-lang-123`) so the work stays isolated.
- Run `bd quickstart` for the full command overview when you need a refresher.

## Coding Style & Naming Conventions
- Use two-space indentation for `.tea` sources; terminate control-flow blocks with `end`, `until`, or `unless` and omit semicolons.
- Favor `snake_case` for variables and functions, `PascalCase` for type names, and avoid camelCase in user-facing language samples.
- Run `cargo fmt` before committing Rust changes; keep module names lowercase and align file names with their module (`lexer/token.rs` → `mod token`).
- Document non-obvious design decisions inline or in `docs/` so future contributors understand trade-offs without reverse-engineering patches.

## Testing Guidelines
- Place Rust integration tests beneath `tea-compiler/tests/feature_name.rs`; prefer snapshot diagnostics or bytecode dumps when validating compiler stages.
- Treat each `.tea` example as a behavior contract; if output changes, update comments and the relevant Rust assertions together.
- Use `cargo run -p tea-cli -- test` for the full sweep; when iterating on compiler changes, pair `--skip-e2e` with focused suites and run `scripts/e2e.sh` before landing codegen work.

## Commit & Pull Request Guidelines
- Follow the `type: summary` pattern (`feat: add pattern matching`, `fix: correct tuple arity check`) and keep each commit focused on one outcome.
- PRs should include a one-paragraph summary, explicit test plan (`cargo test -p tea-compiler`, `scripts/e2e.sh`), and links to tracking issues.
- Attach terminal captures or logs whenever diagnostics, bytecode, or CLI UX change so reviewers can confirm deltas quickly.
- Raise design-affecting ideas as issues or docs entries before implementation to keep roadmap discussions discoverable.
