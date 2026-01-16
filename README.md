# Tea Language

[![CI](https://github.com/bentruyman/tea-lang/actions/workflows/pr-ci.yml/badge.svg)](https://github.com/bentruyman/tea-lang/actions/workflows/pr-ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

A strongly typed scripting language with Ruby-inspired syntax that compiles to native code.

```tea
def greet(name: String) -> String
  `Hello, ${name}!`
end

var names = ["Alice", "Bob", "Charlie"]

for person in names
  @println(greet(person))
end
```

## Features

- **Static typing with inference** – catch errors before runtime while keeping code concise
- **Familiar syntax** – indentation-based, inspired by Ruby and Python
- **Generics** – write once, use anywhere with automatic specialization
- **Native compilation** – compiles to fast, standalone native binaries
- **Rich standard library** – filesystem, JSON, path, and string utilities built-in

## Everyday Tea

Most Tea scripts compose stdlib helpers to automate builds, tooling, and ops without sacrificing native performance.

```tea
use env = "std.env"
use fs = "std.fs"
use path = "std.path"
use string = "std.string"

var root = env.cwd()
var entries = fs.read_dir(root)

@print(`Tea files in ${root}:`)

for entry in entries
  if string.ends_with(entry, ".tea")
    var absolute = path.join([root, entry])
    @print(`• ${absolute}`)
  end
end
```

Modules like `std.assert`, `std.fs`, `std.path`, and the debug built-ins (`@println`, `@type_of`, `@len`) cover quick checks and diagnostics—see [`docs/stdlib-reference.md`](docs/stdlib-reference.md) for the complete reference.

## Quick Start

### Installation

#### Automated Installation (Recommended)

For macOS and Linux, use the install script:

```bash
curl -fsSL https://tea-lang.dev/install.sh | bash
```

Or clone and run locally:

```bash
git clone https://github.com/bentruyman/tea-lang
cd tea-lang
./install.sh
```

#### Prerequisites

Before installing, ensure you have:

- **Rust** (1.70+) – Install from [rustup.rs](https://rustup.rs)
- **Bun** – Install from [bun.sh](https://bun.sh)
- **Make** – Usually pre-installed on macOS/Linux
- **LLVM** (optional but recommended) – For AOT compilation
  - macOS: `brew install llvm`
  - Ubuntu/Debian: `apt-get install llvm-dev`
  - RHEL/CentOS: `yum install llvm-devel`

#### Manual Installation

If you prefer to build manually:

```bash
git clone https://github.com/bentruyman/tea-lang
cd tea-lang
make setup              # Install dependencies and generate code
cargo build --release   # Build the compiler
make install            # Install to ~/.cargo/bin
```

Ensure `~/.cargo/bin` is in your PATH:

```bash
export PATH="$HOME/.cargo/bin:$PATH"
```

Add this to your shell profile (`~/.bashrc`, `~/.zshrc`, etc.) to make it permanent.

### Your First Program

Create `hello.tea`:

```tea
var greeting = "Hello, Tea!"
@println(greeting)

var numbers = [1, 2, 3, 4, 5]
var sum = 0

for n in numbers
  sum = sum + n
end

@println(`Sum: ${sum}`)
```

Run it:

```bash
tea hello.tea
```

Or during development (without installing):

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

Build a standalone binary:

```bash
tea build hello.tea
./bin/hello
```

The binary is fully standalone and can be distributed without any runtime dependencies.

## Examples

Explore more in the [`examples/`](examples/) directory:

- **Language basics** – variables, functions, control flow
- **Data structures** – lists, dictionaries, structs
- **Standard library** – filesystem, processes, JSON/YAML
- **Testing** – snapshots and assertions

## Documentation

- **[Getting Started Guide](docs/)** – comprehensive language reference
- **[Language Semantics](docs/reference/language/semantics.md)** – types, scoping, modules
- **[Standard Library](docs/stdlib-reference.md)** – built-ins and modules for everyday scripting
- **[Compiler Architecture](docs/explanation/aot-backend.md)** – LLVM compilation details
- **[LSP Setup](docs/how-to/lsp-setup.md)** – editor integration

## Troubleshooting

### Common Installation Issues

**`tea: command not found`**

- Ensure `~/.cargo/bin` is in your PATH
- Run `export PATH="$HOME/.cargo/bin:$PATH"`
- Add the export to your shell profile for persistence

**`LLVM not found` errors during compilation**

- LLVM is required for AOT compilation features
- Install LLVM: `brew install llvm` (macOS) or `apt-get install llvm-dev` (Ubuntu)
- Alternatively, run Tea scripts without building: `tea script.tea`

**Build fails with "failed to compile tea-runtime"**

- Ensure you have the latest Rust: `rustup update`
- Clean and rebuild: `cargo clean && cargo build --release`

**`bun: command not found` during setup**

- Install Bun: `curl -fsSL https://bun.sh/install | bash`
- Restart your shell or source your profile

For more help, open an issue at [github.com/bentruyman/tea-lang/issues](https://github.com/bentruyman/tea-lang/issues).

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
- `tea-runtime/` – Runtime support library for compiled binaries (FFI helpers, stdlib hooks)
- `tea-lsp/` – Language server for editor integration
- `spec/` – Language specification (grammar, AST, tokens)
- `examples/` – Sample Tea programs

### Contributing

Contributions are welcome! Please see:

- [Contributing Guide](docs/project/CONTRIBUTING.md) - How to contribute
- [Development Guidelines](AGENTS.md) - Coding style and workflow

## License

MIT
