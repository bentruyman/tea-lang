<h1 align="center">
  <img src="./www/public/tea-logo.svg" alt="Tea logo" width="96" /><br>
  Tea Language
</h1>

<p align="center">
  <sup>A strongly typed scripting language with familiar syntax that compiles to native code.</sup>
</p>

<p align="center">
  <a href="https://github.com/bentruyman/tea-lang/actions/workflows/pr-ci.yml">
    <img src="https://github.com/bentruyman/tea-lang/actions/workflows/pr-ci.yml/badge.svg" alt="CI" />
  </a>
  <a href="LICENSE">
    <img src="https://img.shields.io/badge/License-MIT-blue.svg" alt="License: MIT" />
  </a>
</p>

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
- **Familiar syntax** – clean, indentation-based, designed for readable scripting
- **Generics** – write once, use anywhere with automatic specialization
- **Native compilation** – compiles to fast, standalone native binaries
- **Rich standard library** – filesystem, path, process, regex, and string utilities built-in

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
    @print(`• ${entry}`)
  end
end
```

Modules like `std.assert`, `std.fs`, `std.path`, and the debug built-ins (`@println`, `@type_of`, `@len`) cover quick checks and diagnostics; see [`docs/reference/standard-library.md`](docs/reference/standard-library.md) for the current reference.

## Quick Start

### Installation

#### Automated Installation (Recommended)

For macOS and Linux, use the install script:

```bash
curl -fsSL https://tea-lang.dev/install | bash
```

The installer downloads the latest GitHub Release, verifies its checksum, and installs `tea` to `~/.local/bin` by default.

Before running it, make sure a host C toolchain is available:

- **macOS**: `xcode-select --install`
- **Linux**: install `cc`/`clang` with your package manager (for example `sudo apt-get install build-essential clang`)

You can override the install behavior with:

- `TEA_VERSION=v0.1.0` to pin a release
- `TEA_INSTALL_DIR=/custom/bin` to change the install directory
- `TEA_GITHUB_REPO=owner/fork` to install from a fork or staging repo

Or clone and run the same installer locally:

```bash
git clone https://github.com/bentruyman/tea-lang
cd tea-lang
./scripts/install.sh
```

#### Manual Installation

If you prefer to build from source:

```bash
git clone https://github.com/bentruyman/tea-lang
cd tea-lang
./scripts/setup-worktree.sh  # Bootstrap a fresh dev checkout/worktree
cargo build --release        # Build the compiler
make install                 # Install to ~/.local/bin
```

Source builds need the Rust toolchain, Bun, Make, and LLVM 17 available locally.

Ensure `~/.local/bin` is in your PATH:

```bash
export PATH="$HOME/.local/bin:$PATH"
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
- **[Standard Library](docs/reference/standard-library.md)** – built-ins and modules for everyday scripting
- **[Compiler Architecture](docs/explanation/aot-backend.md)** – LLVM compilation details
- **[LSP Setup](docs/how-to/lsp-setup.md)** – editor integration

## Troubleshooting

### Common Installation Issues

**`tea: command not found`**

- Ensure `~/.local/bin` is in your PATH
- Run `export PATH="$HOME/.local/bin:$PATH"`
- Add the export to your shell profile for persistence

**`cc: command not found` or linker errors while running `tea`**

- Tea uses the host C toolchain to link executables
- On macOS, run `xcode-select --install`
- On Linux, install `build-essential clang` or the equivalent packages for your distro

**Source build fails with LLVM errors**

- LLVM 17 is required when building Tea from source
- Install LLVM 17: `brew install llvm@17` (macOS) or `apt-get install llvm-17-dev` (Ubuntu)
- Clean and rebuild: `cargo clean && cargo build --release`

**`bun: command not found` during setup**

- Install Bun: `curl -fsSL https://bun.sh/install | bash`
- Restart your shell or source your profile

For more help, open an issue at [github.com/bentruyman/tea-lang/issues](https://github.com/bentruyman/tea-lang/issues).

## Development

### Building from Source

```bash
./scripts/setup-worktree.sh  # Fresh worktree bootstrap (deps + codegen + docs install)
make build                   # Build the compiler and CLI
make test                    # Run test suite
```

### Preparing a Release

```bash
make release 0.0.1      # Update versioned manifests and lockfiles
git add .
git commit -m "chore(release): 0.0.1"
make release-tag 0.0.1  # Create annotated tag v0.0.1 on clean HEAD
make release-push-tag 0.0.1  # Publish v0.0.1 to origin so GitHub can see it
TEA_REF=v0.0.1 ./scripts/install.sh
```

If you run any GitHub-side release automation, do it after the tag exists on the remote.

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
