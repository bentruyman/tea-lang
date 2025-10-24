# tea-lang

tea-lang is a strongly typed scripting language with Ruby-inspired surface syntax. It aims to deliver compiled speed without sacrificing the quick iteration loop you expect from a scripting language. You write terse, indentation-driven code; the tooling keeps types honest and surfaces mistakes before anything runs.

## At a Glance

```tea
use debug = "std.debug"

def inc(value: Int) -> Int
  value + 1
end

var scores = [1, 2, 3]
var point = { x: 4, y: 5 }

if scores[0] < 5
  debug.print(inc(scores[0]))
end

debug.print(point.x)
```

## Why tea-lang?

- **Helpful static checks** – annotations are optional but, when present, the compiler makes sure calls, containers, and return values line up before execution.
- **Familiar surface** – indentation-based, Ruby-inspired syntax keeps prototypes readable while retaining static guarantees.
- **Data structure safety** – list and dict literals keep element types consistent; mismatches trigger targeted diagnostics.
- **Reusable generics** – parameterise functions and structs once and specialise them anywhere, even across `use`-able modules, with the compiler emitting concrete instances for each call site.
- **Single CLI workflow** – the `tea` tool builds, inspects (tokens/AST/bytecode), and executes programs without juggling multiple binaries.

## Quickstart

Build the CLI once with `make build` (or `cargo build`) so `./bin/tea` is available, then create `examples/language/basics/basics.tea` (the directory structure keeps fundamentals grouped together):

```tea
use debug = "std.debug"

var greeting = "Hello from tea-lang"
var count = 2

if count == 2
  debug.print(greeting)
end

var total = 1 + 2 + 3
debug.print(total)
```

Every `use` must introduce a local alias (for example `use fs = "std.fs"`), and helpers exposed
by the module should be accessed through that alias (`fs.create_dir`, `assert.assert_eq`, etc.).

Run it with:

```
cargo run -p tea-cli -- examples/language/basics/basics.tea
```

Output:

```
Hello from tea-lang
6
```

### Type-Safe Feedback

Violations are reported before execution:

```
$ cargo run -p tea-cli -- examples/language/basics/bad.tea
Diagnostics:
  - error: argument 1 to 'inc': expected Int, found Bool
     --> examples/language/basics/bad.tea:5:8
      inc(true)
          ^^^^
  - error: list element 2: expected Int, found String
     --> examples/language/basics/bad.tea:9:15
      numbers = [1, "oops", 3]
                  ^^^^^^
Error: Compilation failed
```

The CLI prints the exact source line with a caret underline so you can zero in on mistakes immediately.

Once compiled, you can also execute scripts via the binary: `./bin/tea examples/language/basics/basics.tea`. Lean on these diagnostics as your primary feedback loop when iterating on programs.

### Snapshots & CLI Capture

The test harness now understands CLI-style snapshots. Capture a command, assert on its exit status, and compare stdout/stderr against golden files stored under `tests/__snapshots__/`:

```tea
use assert = "std.assert"
use cli = "support.cli"

test "tea --help emits usage"
  var result = cli.capture(["./bin/tea", "--help"])
  assert.assert_eq(result.exit, 0)
  assert.assert_snapshot("tea_help", result.stdout, "stdout")
  assert.assert_empty(result.stderr)
end
```

Run `tea test --update-snapshots` to accept new output or refresh existing files.

### CLI Argument Parsing

The `support.cli` module now includes helpers tailored for command-line tools:

- `args()` returns the current process arguments as a `List[String]`.
- `parse(spec, argv?)` consumes a command specification (dictionary of options, positionals, and metadata) and returns a `CliParseResult` struct with fields such as `ok`, `exit`, `command`, `path`, `options`, `positionals`, `scopes`, `rest`, `message`, and `help`.

See `examples/stdlib/cli/parse.tea` for a complete walkthrough that falls back to a demo argument list when no parameters are supplied.

## LLVM Backend

The CLI defaults to the LLVM ahead-of-time backend when you run `tea build`:

```
cargo run -p tea-cli -- build examples/language/basics/fib.tea
```

The command lowers `examples/language/basics/fib.tea` to LLVM IR, produces an object file, links it with the runtime, and writes the resulting executable to `bin/fib`. You can inspect intermediate artefacts without producing a binary:

```
# Dump IR only
cargo run -p tea-cli -- --emit llvm-ir --no-run examples/language/basics/fib.tea

# Keep the object file alongside the IR
cargo run -p tea-cli -- --emit llvm-ir --emit obj --no-run examples/language/basics/fib.tea
```

Both the VM and LLVM pipelines cover integers/floats (with Int→Float promotion), comparisons, control flow (`if`, `while`, `until`), recursion, `var` locals, list/dict literals, structs, lambda literals, and generic functions/structs (including those imported from modules). See [docs/aot-backend.md](docs/aot-backend.md) for the latest capabilities and limitations.

### Packaging CLI Binaries

`tea build` accepts additional packaging switches when targeting the LLVM backend:

- `--target <triple>`, `--cpu`, and `--features` let you cross-compile without touching environment variables.
- `--bundle` writes a deterministic `tar.gz` archive (with metadata and checksums) alongside the executable; use `--bundle-output` to control its location.
- `--checksum` emits a SHA-256 sum file, and `--signature-key <path>` can produce an HMAC signature for the binary.
- `--opt-level` mirrors `rustc`'s optimisation shorthand (`0`, `1`, `2`, `3`, `s`, `z`).

Bundles embed reproducible metadata (respecting `SOURCE_DATE_EPOCH`) so teams can ship verifiable CLI artifacts with minimal ceremony.

## Maintainer Docs

Under the hood, programs move through a resolver and static type checker before bytecode generation, so undefined bindings, mismatched annotations, and container shape errors are caught before the VM ever runs. The runtime currently executes a compact stack-based bytecode format, and a feature-gated LLVM AOT backend can lower the same programs to native binaries.

### Language Capabilities

- Variable declarations via `var name = expr`
- Integer arithmetic (`+`, `-`, `*`, `/`) and equality/ordering comparisons
- Unary `-` and `not`
- `if` conditionals with optional `else`
- `while` / `until` loops
- List literals and indexing (`var xs = [1, 2, 3]`; `xs[0]`)
- Dictionary literals and member/index access (`var point = { x: 1 }`; `point.x` / `point["x"]`)
- String literals and a builtin `print` function
- CLI-focused helpers: `std.io` for streaming stdin/stdout, `std.fs` for globbing, directory walking, atomic writes, and metadata, `std.path` for join/normalize/relative utilities, `std.env` for reading/updating environment state and resolving common directories, `std.process` for spawning commands and capturing their output, plus `std.json` / `std.yaml` for encoding and decoding structured data (available in both the VM and LLVM backends). Upcoming modules for networking live in `docs/cli-stdlib-roadmap.md`.
- Type annotations on variables (`Bool`, `Int`, `Float`, `String`, `Nil`) plus container/function forms (`List[Int]`, `Dict[String, Int]`, `Func(Int) -> Int`) and required parameter annotations in function definitions
- Generic functions and structs that specialise automatically wherever they are called—even across modules brought in via `use`.

### Tooling

- Build the workspace with `cargo build`.
- Run a program: `cargo run -p tea-cli -- examples/language/basics/basics.tea`.
- Format sources (files or directories) in place: `cargo run -p tea-cli -- fmt examples`.
- Run project suites via the harness: `cargo run -p tea-cli -- test` (use `--list`, `--filter`, or `--fail-fast` for finer control).
- Inspect lexer and parser output:
  - Tokens: `cargo run -p tea-cli -- --dump-tokens --no-run examples/language/basics/basics.tea`
  - AST: `cargo run -p tea-cli -- --emit ast --no-run examples/language/basics/basics.tea`
  - Bytecode: `cargo run -p tea-cli -- --emit bytecode --no-run examples/language/basics/basics.tea`
- Execute tests: `cargo test`.

Additional language constructs (pattern matching, richer modules, native code generation) are on the roadmap.

### Further Reading

- [Tea LSP installation & editor setup](docs/tea-lsp-setup.md)
- [LLVM backend architecture notes](docs/aot-backend.md)
- [Type checker overview](docs/type-checking.md)
