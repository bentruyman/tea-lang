# Tea Language Examples

Welcome to the Tea examples collection! These programs serve as executable documentation, demonstrating both the language features and standard library capabilities.

## 📂 Directory Structure

### `language/` - Core Language Features

Learn Tea's syntax and semantics through practical examples:

#### **`basics/`** - Getting Started

- **`basics.tea`** - Variables, functions, strings, and basic I/O
- Learn fundamental syntax and program structure
- Great starting point for newcomers

#### **`control_flow/`** - Conditionals and Loops

- **`if_expressions.tea`** - If-else as expressions (they return values!)
- **`for_loops.tea`** - Iterating over collections
- **`while_loops.tea`** - Condition-based iteration

#### **`collections/`** - Data Structures

- **`lists.tea`** - List operations, iteration, and methods
- **`dicts.tea`** - Dictionary/hash map usage
- **`tuples.tea`** - Fixed-size heterogeneous collections

#### **`functions/`** - Functions and Closures

- **`lambdas.tea`** - Anonymous functions and closures
- **`recursion.tea`** - Recursive function patterns

#### **`types/`** - Type System

- **`generics.tea`** - Generic functions and type parameters
- **`structs.tea`** - Custom data types

#### **`modules/`** - Code Organization

- **`another.tea`** + **`using_another.tea`** - Cross-file imports
- Learn how to organize larger programs

#### **`numeric/`** - Numbers and Math

- **`integers.tea`** - Integer arithmetic
- **`floats.tea`** - Floating-point operations

#### **`strings/`** - Text Processing

- String manipulation
- Interpolation with backticks
- Common string operations

#### **`optionals/`** - Null Safety

- Working with optional/nullable values
- Pattern matching on optionals

### `stdlib/` - Standard Library Modules

Explore Tea's built-in capabilities:

#### **`testing/`** - Test Framework

- Snapshot testing
- Assertions
- Test organization patterns

### `full/` - Complete Programs

Real-world examples that combine multiple concepts:

- **`team_scoreboard.tea`** - Complete application demonstrating structs, collections, and formatting

## 🚀 Running Examples

### Using the Tea CLI (Recommended)

After installing Tea:

```bash
tea examples/language/basics/basics.tea
```

### During Development

Without installing, use cargo:

```bash
cargo run -p tea-cli -- examples/language/basics/basics.tea
```

### Building Examples

Compile to standalone native binaries:

```bash
tea build examples/language/basics/basics.tea
./bin/basics
```

## 📚 Learning Path

If you're new to Tea, follow this recommended order:

1. **Start with basics**: `language/basics/basics.tea`
2. **Learn control flow**: `language/control_flow/if_expressions.tea`
3. **Explore collections**: `language/collections/lists.tea`
4. **Understand functions**: `language/functions/lambdas.tea`
5. **Study types**: `language/types/structs.tea`
6. **Try a full example**: `full/team_scoreboard.tea`

## 💡 Tips

- **Read the code comments** - Examples include inline documentation
- **Modify and experiment** - Change values and see what happens
- **Check expected output** - Comments note what each example should print
- **Compare to other languages** - If you know Ruby/Python/Rust, you'll see familiar patterns

## 🐛 Found an Issue?

If an example doesn't work as described:

1. Ensure you have the latest version: `tea --version`
2. Check if tests pass: `make test`
3. Open an issue with the example name and error message

## 🤝 Contributing Examples

We welcome new examples! When adding one:

- Include clear comments explaining what's happening
- Note expected output in comments
- Keep examples focused on one concept
- Place in the appropriate directory
- Update this README with a brief description

See [CONTRIBUTING.md](../docs/project/CONTRIBUTING.md) for details.

## 📖 More Resources

- [Language Documentation](../docs/)
- [Standard Library Reference](../docs/stdlib-reference.md)
- [Language Semantics](../docs/reference/language/semantics.md)
