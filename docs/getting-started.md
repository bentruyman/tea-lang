# Getting Started with Tea

Welcome to Tea! This guide will help you install Tea and write your first program.

## What is Tea?

Tea is a strongly typed scripting language with familiar syntax that compiles to native code. It combines the expressiveness of scripting languages with the performance and safety of compiled languages.

**Key Features:**

- **Static typing with inference** - Type safety without verbose annotations
- **Familiar syntax** - clean, indentation-based, designed for readable scripting
- **Native compilation** - Compiles to fast, standalone binaries via LLVM
- **Rich standard library** - Built-in modules for filesystem, JSON, strings, and more
- **Generics** - Write reusable, type-safe code

## Installation

### Quick Install

The fastest way to get started:

```bash
curl -fsSL https://tea-lang.dev/install | bash
```

The installer downloads a prebuilt Tea release, verifies its checksum, and installs `tea` to `~/.local/bin` by default.

Before running it, make sure a host C toolchain is available:

- **macOS** - `xcode-select --install`
- **Linux** - install `cc` or `clang` with your package manager

You can customize the installer with:

- `TEA_VERSION=v0.1.0` - Pin a specific release
- `TEA_INSTALL_DIR=/custom/bin` - Change the install location
- `TEA_GITHUB_REPO=owner/fork` - Install from a fork or staging repo

Or clone and run the same installer locally:

```bash
git clone https://github.com/bentruyman/tea-lang
cd tea-lang
./scripts/install.sh
```

This will:

1. Download the matching release artifact for your machine
2. Verify the release checksum
3. Install the `tea` command to `~/.local/bin`

Make sure `~/.local/bin` is in your PATH:

```bash
export PATH="$HOME/.local/bin:$PATH"
```

Add this to your shell profile (`~/.bashrc`, `~/.zshrc`, etc.) to make it permanent.

### Manual Installation

If you prefer to build Tea from source:

```bash
git clone https://github.com/bentruyman/tea-lang
cd tea-lang
./scripts/setup-worktree.sh  # Bootstrap a fresh dev checkout/worktree
cargo build --release        # Build the compiler
make install                 # Install to ~/.local/bin
```

Source builds require Rust, Bun, Make, and LLVM 17 on the local machine.

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
@println(greeting)
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
- `@println(greeting)` outputs the string to the console.

## A Quick Tour

Let's explore Tea's core features with a simple example.

Create `quickstart.tea`:

```tea
# Variables and type inference
var name = "Alice"
var age = 30
var score = 95.5

@println(`Name: ${name}`)
@println(`Age: ${age}`)

# Lists
var numbers = [1, 2, 3, 4, 5]
var sum = 0

for n in numbers
  sum = sum + n
end

@println(`Sum: ${sum}`)

# Functions with types
def greet(person: String) -> String
  `Hello, ${person}!`
end

@println(greet(name))

# Structs
struct Point {
  x: Int
  y: Int
}

var origin = Point(x: 0, y: 0)
@println(`Origin: (${origin.x}, ${origin.y})`)
```

Run it:

```bash
tea quickstart.tea
```

### Key Concepts

**String Interpolation:** Use `${}` to embed expressions in strings:

```tea
var count = 5
@println(`Count: ${count}`)  # Output: Count: 5
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

- Ensure `~/.local/bin` is in your PATH
- Run `source ~/.bashrc` (or your shell profile) to reload

**`cc` or linker errors while running Tea**

- Tea uses the host C toolchain to link executables
- On macOS, run `xcode-select --install`
- On Linux, install `build-essential clang` or the equivalent packages for your distro

**LLVM errors during source build**

- Install LLVM 17: `brew install llvm@17` (macOS) or `apt-get install llvm-17-dev` (Ubuntu)
- The prebuilt installer does not need local LLVM, but source builds do

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
