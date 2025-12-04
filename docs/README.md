# Tea Language Documentation

Welcome to the Tea language documentation! Tea is a strongly typed scripting language with Ruby-inspired syntax that compiles to native code.

## Quick Links

- **New to Tea?** Start with the [Getting Started Guide](getting-started.md)
- **Learning the language?** Check out the [Language Guide](#language-guide)
- **Looking for reference docs?** See the [Reference](#reference) section
- **Need examples?** Browse [Examples and Patterns](examples.md)
- **Have questions?** Check the [FAQ](faq.md)

---

## Getting Started

**[Getting Started Guide](getting-started.md)**

Install Tea and write your first program. Covers installation, basic concepts, and your first steps with the language.

---

## Language Guide

Comprehensive tutorial-style guides covering Tea from basics to advanced topics:

### 1. [Basics](guide/basics.md)

Learn fundamental concepts:

- Variables (`var` and `const`)
- Types and type inference
- Functions and return values
- Strings, numbers, and booleans
- Comments
- Nil and optional types

### 2. [Control Flow](guide/control-flow.md)

Control program execution:

- If statements and if-expressions
- While and for loops
- Pattern matching with `case`
- Boolean logic and comparisons

### 3. [Data Structures](guide/data-structures.md)

Organize your data:

- Lists and arrays
- Structs for custom types
- Dictionaries (maps)
- Nested and generic collections

### 4. [Error Handling](guide/error-handling.md)

Handle errors gracefully:

- Defining error types
- Throwing and catching errors
- Pattern matching errors
- Best practices

### 5. [Advanced Topics](guide/advanced.md)

Take your Tea skills further:

- Generics for reusable code
- Modules and imports
- Lambdas and closures
- Compilation options
- Testing and formatting

---

## Reference

Quick reference documentation for looking up specific features:

### [Built-in Functions](reference/builtins.md)

Global functions available everywhere:

- Output: `@print`, `@println`
- Introspection: `@type_of`, `@len`
- Math: `@abs`, `@sqrt`, `@floor`, `@ceil`, `@round`, `@min`, `@max`
- Utility: `@to_string`

### [Standard Library](reference/standard-library.md)

Modules for common tasks:

- `std.assert` - Test assertions
- `std.env` - Environment variables
- `std.fs` - Filesystem operations
- `std.json` - JSON encoding/decoding
- `std.path` - Path manipulation
- `std.string` - String utilities

---

## Examples

**[Examples and Common Patterns](examples.md)**

Real-world examples and best practices:

- File processing
- Command-line tools
- Data processing
- Configuration management
- Testing strategies
- Error handling patterns

---

## FAQ

**[Frequently Asked Questions](faq.md)**

Common questions and answers about:

- Getting started with Tea
- Language features
- Performance
- Troubleshooting
- Testing
- Community and support

---

## Learning Paths

### Beginner Path

1. Start with [Getting Started](getting-started.md) to install Tea
2. Read [Basics](guide/basics.md) to learn variables, types, and functions
3. Learn [Control Flow](guide/control-flow.md) for if/while/for loops
4. Explore [Data Structures](guide/data-structures.md) for lists and structs
5. Check out [Examples](examples.md) for practical code

### Intermediate Path

1. Master [Error Handling](guide/error-handling.md)
2. Learn [Advanced Topics](guide/advanced.md) - generics, modules, testing
3. Study the [Standard Library](reference/standard-library.md)
4. Build projects from [Examples](examples.md)

### Reference Path

Already know Tea? Use these for quick lookups:

- [Built-in Functions](reference/builtins.md) - `@println`, `@len`, math functions
- [Standard Library](reference/standard-library.md) - `std.fs`, `std.path`, etc.
- [FAQ](faq.md) - Common questions and solutions

---

## Example Code

Here's a taste of Tea:

**Variables and Functions:**

```tea
var name = "Alice"
var age = 30

def greet(person: String) -> String
  "Hello, ${person}!"
end

print(greet(name))
```

**Lists and Loops:**

```tea
var numbers = [1, 2, 3, 4, 5]
var sum = 0

for num of numbers
  sum = sum + num
end

print("Sum: ${sum}")
```

**Structs and Types:**

```tea
struct Point {
  x: Int
  y: Int
}

var origin = Point(x: 0, y: 0)
print("Point: (${origin.x}, ${origin.y})")
```

**Error Handling:**

```tea
error FileError {
  NotFound(path: String)
}

def read_config(path: String) -> String ! FileError
  if !file_exists(path)
    throw FileError.NotFound(path)
  end

  read_file(path)
end

var config = read_config("config.json") catch err
  case is FileError.NotFound
    return "default config"
  case _
    return ""
end
```

**Standard Library:**

```tea
use fs = "std.fs"
use path = "std.path"
use env = "std.env"

var root = env.cwd()
var entries = fs.read_dir(root)

for entry of entries
  var full_path = path.join([root, entry])
  print(full_path)
end
```

---

## Quick Command Reference

**Run a script:**

```bash
tea script.tea
```

**Compile to binary:**

```bash
tea build script.tea
./bin/script
```

**Run tests:**

```bash
tea test script.tea
```

**Format code:**

```bash
tea fmt script.tea
```

---

## External Resources

- **[Main README](../README.md)** - Project overview and installation
- **[Examples Directory](../examples/)** - Complete example programs
- **[GitHub Repository](https://github.com/bentruyman/tea-lang)** - Source code and issues

---

## Contributing

Found an issue with the documentation? Want to improve something? Contributions are welcome!

See the contributing guide in [`archive/project/CONTRIBUTING.md`](archive/project/CONTRIBUTING.md) for details on how to contribute to Tea.

---

## About These Docs

These docs are organized for **users of the Tea language**. If you're looking for maintainer or contributor documentation, check the [`archive/`](archive/) directory.

**Documentation Structure:**

- `getting-started.md` - Installation and first steps
- `guide/` - Tutorial-style language guides
- `reference/` - Quick reference documentation
- `examples.md` - Practical patterns and recipes
- `faq.md` - Frequently asked questions
- `archive/` - Maintainer and historical documentation
