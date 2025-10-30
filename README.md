# Tea Language

A strongly typed scripting language with Ruby-inspired syntax that compiles to native code or runs on a VM.

```tea
use debug = "std.debug"

def greet(name: String) -> String
  "Hello, ${name}!"
end

var names = ["Alice", "Bob", "Charlie"]

for person of names
  debug.print(greet(person))
end
```

## Features

- **Static typing with inference** – catch errors before runtime while keeping code concise
- **Familiar syntax** – indentation-based, inspired by Ruby and Python
- **Generics** – write once, use anywhere with automatic specialization
- **Dual backends** – fast iteration with VM or native binaries via LLVM
- **Rich standard library** – filesystem, processes, JSON/YAML, and CLI helpers built-in

## Quick Start

### Installation

```bash
git clone https://github.com/bentruyman/tea-lang
cd tea-lang
make setup  # Installs dependencies and generates code
```

### Your First Program

Create `hello.tea`:

```tea
use debug = "std.debug"

var greeting = "Hello, Tea!"
debug.print(greeting)

var numbers = [1, 2, 3, 4, 5]
var sum = 0

for n of numbers
  sum = sum + n
end

debug.print("Sum: ${sum}")
```

Run it:

```bash
cargo run -p tea-cli -- hello.tea
```

### Type Safety

Tea catches errors before runtime:

```tea
var numbers: List[Int] = [1, 2, "three"]  # Error: list element 2: expected Int, found String

def add(a: Int, b: Int) -> Int
  a + b
end

add(5, true)  # Error: expected Int, found Bool
```

### Compile to Native

Build a standalone binary with the LLVM backend:

```bash
# First-time setup: install build tools (if not already installed)
brew install cmake ninja git

# Build vendored LLVM/LLD (30-60 min, one-time only)
make vendor  # macOS arm64 only for now

# Build tea with static LLVM
cargo build -p tea-cli --release --features tea-cli/llvm-aot

# Compile Tea to native binary (no third-party deps needed!)
./target/release/tea-cli build hello.tea
./bin/hello
```

**Note**: Static LLVM embedding currently supports macOS arm64. The compiled `tea` binary includes everything needed to produce native executables with **ZERO third-party dependencies** - no LLVM, no Clang, no Rustc required!

**Important**: Without building the vendored LLVM, `tea-cli` will use your system LLVM (if installed), which means the binary won't be truly self-contained.

## Examples

Explore more in the [`examples/`](examples/) directory:

- **Language basics** – variables, functions, control flow
- **Data structures** – lists, dictionaries, structs
- **Standard library** – filesystem, processes, JSON/YAML
- **Testing** – snapshots and assertions

## Documentation

- **[Getting Started Guide](docs/)** – comprehensive language reference
- **[Language Semantics](docs/reference/language/semantics.md)** – types, scoping, modules
- **[Standard Library](docs/roadmap/cli-stdlib.md)** – available modules and roadmap
- **[AOT Backend](docs/explanation/aot-backend.md)** – LLVM compilation details
- **[Zero-Dependency Implementation](docs/explanation/zero-dependency-implementation.md)** – complete guide to single-binary distribution
- **[LSP Setup](docs/how-to/lsp-setup.md)** – editor integration

## Development

### Building from Source

```bash
make setup    # Install dependencies and generate code
make build    # Build the compiler and CLI
make test     # Run test suite
```

### Project Structure

- `tea-cli/` – Command-line interface
- `tea-compiler/` – Lexer, parser, typechecker, and codegen
- `tea-runtime/` – VM and runtime support for compiled binaries
- `tea-lsp/` – Language server for editor integration
- `tea-llvm-vendor/` – Vendored static LLVM + LLD libraries
- `spec/` – Language specification (grammar, AST, tokens)
- `examples/` – Sample Tea programs
- `scripts/llvm/` – Build scripts for vendored LLVM artifacts

### Contributing

See [AGENTS.md](AGENTS.md) for repository guidelines, coding style, and development workflow.

## License

MIT
