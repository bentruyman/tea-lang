# Getting Started with Tea

Welcome to Tea! This guide will help you install Tea and write your first program.

## What is Tea?

Tea is a strongly typed scripting language with a Ruby-inspired syntax that compiles to native code. It combines the expressiveness of scripting languages with the performance and safety of compiled languages.

**Key Features:**

- **Static typing with inference** - Type safety without verbose annotations
- **Familiar syntax** - Clean, indentation-based syntax inspired by Ruby and Python
- **Native compilation** - Compiles to fast, standalone binaries via LLVM
- **Rich standard library** - Built-in modules for filesystem, JSON, strings, and more
- **Generics** - Write reusable, type-safe code

## Installation

### Prerequisites

Before installing Tea, you'll need:

- **Rust** (1.70+) - Install from [rustup.rs](https://rustup.rs)
- **Bun** - Install from [bun.sh](https://bun.sh)
- **Make** - Usually pre-installed on macOS/Linux
- **LLVM** (optional but recommended) - For AOT compilation
  - macOS: `brew install llvm`
  - Ubuntu/Debian: `apt-get install llvm-dev`
  - RHEL/CentOS: `yum install llvm-devel`

### Quick Install

The fastest way to get started:

```bash
curl -fsSL https://tea-lang.dev/install.sh | bash
```

Or clone and install locally:

```bash
git clone https://github.com/bentruyman/tea-lang
cd tea-lang
./install.sh
```

This will:

1. Install dependencies
2. Build the Tea compiler
3. Install the `tea` command to `~/.cargo/bin`

Make sure `~/.cargo/bin` is in your PATH:

```bash
export PATH="$HOME/.cargo/bin:$PATH"
```

Add this to your shell profile (`~/.bashrc`, `~/.zshrc`, etc.) to make it permanent.

### Manual Installation

If you prefer more control:

```bash
git clone https://github.com/bentruyman/tea-lang
cd tea-lang
make setup              # Install dependencies and generate code
cargo build --release   # Build the compiler
make install            # Install to ~/.cargo/bin
```

### Verify Installation

Check that Tea is installed:

```bash
tea --version
```

You should see the Tea version number.

## Your First Program

Let's write a classic "Hello, World!" program.

Create a file called `hello.tea`:

```tea
var greeting = "Hello, Tea!"
print(greeting)
```

Run it:

```bash
tea hello.tea
```

You should see:

```
Hello, Tea!
```

### What's Happening?

- `var greeting = "Hello, Tea!"` declares a variable. Tea infers that `greeting` is a `String`.
- `print(greeting)` outputs the string to the console.

## A Quick Tour

Let's explore Tea's core features with a simple example.

Create `quickstart.tea`:

```tea
# Variables and type inference
var name = "Alice"
var age = 30
var score = 95.5

print("Name: ${name}")
print("Age: ${age}")

# Lists
var numbers = [1, 2, 3, 4, 5]
var sum = 0

for n of numbers
  sum = sum + n
end

print("Sum: ${sum}")

# Functions with types
def greet(person: String) -> String
  "Hello, ${person}!"
end

print(greet(name))

# Structs
struct Point {
  x: Int
  y: Int
}

var origin = Point(x: 0, y: 0)
print("Origin: (${origin.x}, ${origin.y})")
```

Run it:

```bash
tea quickstart.tea
```

### Key Concepts

**String Interpolation:** Use `${}` to embed expressions in strings:

```tea
var count = 5
print("Count: ${count}")  # Output: Count: 5
```

**Type Annotations:** While Tea infers types, you can be explicit:

```tea
var age: Int = 30
var names: List[String] = ["Alice", "Bob"]
```

**Functions:** Define with `def`, specify parameter and return types:

```tea
def add(a: Int, b: Int) -> Int
  a + b
end
```

**Structs:** Define custom data types:

```tea
struct User {
  name: String
  email: String
}

var user = User(name: "Alice", email: "alice@example.com")
```

## Compiling to Native

Tea can compile your scripts to standalone native binaries:

```bash
tea build quickstart.tea
```

This creates a binary in `./bin/quickstart`. Run it directly:

```bash
./bin/quickstart
```

The binary is fully self-contained with no runtime dependencies. You can distribute it to any compatible system.

## Running Tests

Tea has built-in testing support. Add test blocks to your code:

```tea
def add(a: Int, b: Int) -> Int
  a + b
end

test "addition works"
  assert(add(2, 3) == 5)
  assert(add(-1, 1) == 0)
end
```

Run tests:

```bash
tea test quickstart.tea
```

## Editor Support

Tea has a Language Server Protocol (LSP) implementation for editor integration. See the [Editor Setup Guide](reference/editor-setup.md) for configuration instructions.

## Common Issues

**`tea: command not found`**

- Ensure `~/.cargo/bin` is in your PATH
- Run `source ~/.bashrc` (or your shell profile) to reload

**LLVM errors during build**

- Install LLVM: `brew install llvm` (macOS) or `apt-get install llvm-dev` (Ubuntu)
- You can still run scripts without building: `tea script.tea`

**Build fails with "failed to compile tea-runtime"**

- Update Rust: `rustup update`
- Clean and rebuild: `cargo clean && cargo build --release`

## Next Steps

Now that you have Tea running, explore these resources:

- **[Language Guide](guide/basics.md)** - Learn Tea's features in depth
- **[Standard Library](reference/standard-library.md)** - Explore built-in modules
- **[Examples](examples.md)** - See common patterns and recipes
- **[FAQ](faq.md)** - Answers to frequently asked questions

Ready to dive deeper? Start with the [Basics Guide](guide/basics.md).
