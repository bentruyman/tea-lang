# Tea Language

A strongly typed scripting language with Ruby-inspired syntax that compiles to native code.

```tea
def greet(name: String) -> String
  "Hello, ${name}!"
end

var names = ["Alice", "Bob", "Charlie"]

for person of names
  print(greet(person))
end
```

## Features

- **Static typing with inference** – catch errors before runtime while keeping code concise
- **Familiar syntax** – indentation-based, inspired by Ruby and Python
- **Generics** – write once, use anywhere with automatic specialization
- **Native compilation** – compiles to fast, standalone native binaries
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
var greeting = "Hello, Tea!"
print(greeting)

var numbers = [1, 2, 3, 4, 5]
var sum = 0

for n of numbers
  sum = sum + n
end

print("Sum: ${sum}")
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

Build a standalone binary:

```bash
cargo run -p tea-cli -- build hello.tea
./bin/hello
```

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
- **[Compiler Architecture](docs/explanation/aot-backend.md)** – LLVM compilation details
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
- `tea-runtime/` – Runtime support library for compiled binaries (FFI helpers, stdlib hooks)
- `tea-lsp/` – Language server for editor integration
- `spec/` – Language specification (grammar, AST, tokens)
- `examples/` – Sample Tea programs

### Contributing

See [AGENTS.md](AGENTS.md) for repository guidelines, coding style, and development workflow.

## License

MIT
