# Frequently Asked Questions

Common questions about the Tea programming language.

## General Questions

### What is Tea?

Tea is a strongly typed scripting language with Ruby-inspired syntax that compiles to native code via LLVM. It combines the expressiveness and simplicity of scripting languages with the performance and type safety of compiled languages.

### Why should I use Tea?

Use Tea when you want:

- **Type safety** without verbose annotations - catch errors before runtime
- **Native performance** - fast, standalone binaries
- **Simple syntax** - clean, readable code inspired by Ruby and Python
- **Rich standard library** - filesystem, JSON, path, and string utilities built-in
- **Quick development** - run scripts directly or compile to binaries

Tea is ideal for automation scripts, CLI tools, build systems, and utilities where you want both speed and safety.

### How does Tea compare to other languages?

| Feature            | Tea | Python | Ruby | Go  | Rust |
| ------------------ | --- | ------ | ---- | --- | ---- |
| Static typing      | ✓   | ✗      | ✗    | ✓   | ✓    |
| Type inference     | ✓   | N/A    | N/A  | ✓   | ✓    |
| Native compilation | ✓   | ✗      | ✗    | ✓   | ✓    |
| Indentation-based  | ✓   | ✓      | ✗    | ✗   | ✗    |
| Generics           | ✓   | ✗      | ✗    | ✓   | ✓    |
| Script mode        | ✓   | ✓      | ✓    | ✗   | ✗    |

### Is Tea ready for production?

Tea is currently in active development. It's suitable for personal projects, automation scripts, and prototyping. Check the project repository for the latest status and roadmap.

## Installation and Setup

### How do I install Tea?

The quickest way is using the install script:

```bash
curl -fsSL https://tea-lang.dev/install.sh | bash
```

Or manually:

```bash
git clone https://github.com/bentruyman/tea-lang
cd tea-lang
./install.sh
```

See the [Getting Started Guide](getting-started.md#installation) for details.

### What are the system requirements?

- **Rust** 1.70+ - For building the compiler
- **Bun** - For code generation tooling
- **Make** - Usually pre-installed on macOS/Linux
- **LLVM** (optional but recommended) - For AOT compilation

### I get `tea: command not found` after installing

Ensure `~/.cargo/bin` is in your PATH:

```bash
export PATH="$HOME/.cargo/bin:$PATH"
```

Add this to your shell profile (`~/.bashrc`, `~/.zshrc`, etc.) to make it permanent, then restart your shell or run `source ~/.bashrc`.

### How do I update Tea?

Pull the latest changes and rebuild:

```bash
cd tea-lang
git pull
cargo build --release
make install
```

## Language Features

### Does Tea have a package manager?

Not yet. Currently, you import the standard library with `use` statements. Third-party package management is on the roadmap.

### Can I use Tea for web development?

Tea is currently focused on command-line tools, automation, and scripting. Web framework support is not yet available but may be added in the future.

### Does Tea support async/await?

Not currently. Async/await is a planned feature for future releases.

### How do I handle optional values?

Use the `?` type modifier for optional values:

```tea
var maybe_name: String? = nil

# Check before use
if maybe_name != nil
  print(maybe_name!)
end

# Or use nil coalescing
var name = maybe_name ?? "default"
```

See [Basics - Nil and Optional Types](guide/basics.md#nil-and-optional-types).

### How do I work with mutable vs immutable data?

Use `var` for mutable variables and `const` for immutable values:

```tea
var mutable = 5
mutable = 10  # OK

const immutable = 5
# immutable = 10  # Error!
```

See [Basics - Variables](guide/basics.md#variables).

### Does Tea have break and continue?

Not currently. Use conditional logic and boolean flags to control loop flow:

```tea
var found = false
var i = 0

while i < @len(items) && !found
  if items[i] == target
    found = true
  end
  i = i + 1
end
```

## Working with Tea

### How do I run a Tea program?

Directly execute a script:

```bash
tea script.tea
```

Or compile to a binary first:

```bash
tea build script.tea
./bin/script
```

### How do I pass arguments to my script?

Access command-line arguments via the environment:

```tea
use env = "std.env"

var args = env.args()
if @len(args) > 1
  var first_arg = args[1]
  print(first_arg)
end
```

### How do I read user input?

Tea doesn't have built-in stdin reading yet. You can work around this by accepting arguments:

```bash
tea script.tea "user input"
```

### How do I print without a newline?

Use `@print` instead of `@println`:

```tea
@print("Loading")
@print(".")
@print(".")
@print(".")
# Output: Loading...
```

### How do I debug my Tea code?

Use `@println` and `@type_of` for debugging:

```tea
var value = compute_something()
@println("Value: ${value}")
@println("Type: ${@type_of(value)}")
```

You can also use the `--emit ast` flag to inspect the AST:

```bash
tea --emit ast script.tea
```

### How do I format my code?

Use the built-in formatter:

```bash
tea fmt script.tea
```

Or format all files in a directory:

```bash
tea fmt .
```

## Errors and Troubleshooting

### I get type errors when compiling

Tea uses static typing. Make sure your types match:

```tea
var number: Int = 42
# number = "not a number"  # Error!

def add(a: Int, b: Int) -> Int
  a + b
end

# add(5, "10")  # Error!
```

See [Basics - Types](guide/basics.md#types) for more on type safety.

### How do I fix "file not found" errors?

Use absolute paths or ensure you're running from the correct directory:

```tea
use env = "std.env"
use path = "std.path"

var current = env.cwd()
var file_path = path.join([current, "file.txt"])
```

### My program crashes with a nil error

Check for nil before unwrapping optional values:

```tea
# Unsafe - crashes if nil
var value: Int? = nil
# var x = value!  # Crash!

# Safe - check first
if value != nil
  var x = value!
end

# Or use nil coalescing
var x = value ?? 0
```

### How do I report bugs or request features?

Open an issue on the [GitHub repository](https://github.com/bentruyman/tea-lang/issues).

## Performance

### Is Tea fast?

Yes! Tea compiles to native code via LLVM, giving you performance comparable to languages like C, Go, and Rust. Compiled binaries run at native speed with no runtime overhead.

### Should I use script mode or compiled binaries?

- **Script mode** (`tea script.tea`) - Fast for development and iteration
- **Compiled binaries** (`tea build script.tea`) - Best for production and distribution

Use script mode during development, then compile to a binary for deployment.

### How do I optimize my Tea code?

1. **Use the right data structures** - Choose appropriate types for your use case
2. **Avoid unnecessary allocations** - Reuse variables when possible
3. **Compile to release mode** - Use `tea build` for optimized binaries
4. **Profile your code** - Use timing to identify bottlenecks

## Standard Library

### What modules are available in the standard library?

Tea includes these modules:

- `std.assert` - Testing assertions
- `std.env` - Environment variables
- `std.fs` - Filesystem operations
- `std.json` - JSON encoding/decoding
- `std.path` - Path manipulation
- `std.string` - String utilities

See the [Standard Library Reference](reference/standard-library.md) for details.

### How do I work with JSON?

Use the `std.json` module:

```tea
use json = "std.json"
use fs = "std.fs"

# Read and parse
var json_str = fs.read_file("data.json")
var data = json.decode(json_str)

# Encode and write
var output = { "name": "tea", "version": "1.0" }
var json_out = json.encode(output)
fs.write_file("output.json", json_out)
```

### How do I manipulate file paths?

Use the `std.path` module:

```tea
use path = "std.path"

var parts = ["usr", "local", "bin"]
var full_path = path.join(parts)  # "usr/local/bin"

var filename = path.basename("/usr/local/bin/tea")  # "tea"
var directory = path.dirname("/usr/local/bin/tea")  # "/usr/local/bin"
```

## Testing

### How do I write tests?

Use `test` blocks with assertions:

```tea
use assert = "std.assert"

def add(a: Int, b: Int) -> Int
  a + b
end

test "addition works"
  assert.eq(add(2, 3), 5)
  assert.eq(add(0, 0), 0)
end
```

Run tests with:

```bash
tea test script.tea
```

See [Advanced Topics - Testing](guide/advanced.md#testing) for more.

### What assertion functions are available?

The `std.assert` module provides:

- `ok(value)` - Assert truthy
- `eq(a, b)` - Assert equal
- `ne(a, b)` - Assert not equal
- `gt(a, b)` - Assert greater than
- `lt(a, b)` - Assert less than
- `snapshot(name, value)` - Snapshot testing

See [Standard Library - assert](reference/standard-library.md#stdassert).

## Community and Support

### Where can I get help?

- **Documentation** - Start with the [Getting Started Guide](getting-started.md)
- **GitHub Issues** - For bugs and feature requests
- **Examples** - Check the [examples/](../examples/) directory

### How can I contribute?

Contributions are welcome! See the contributing guide in `docs/archive/project/CONTRIBUTING.md` for details.

### Is there a community forum or chat?

Check the GitHub repository for links to community resources.

## Miscellaneous

### What does "Tea" stand for?

Tea is just tea! The name reflects the language's focus on simplicity and clarity.

### What license is Tea under?

Tea is licensed under the MIT License. See the [LICENSE](../LICENSE) file for details.

### Where can I see more examples?

- **[Examples Guide](examples.md)** - Common patterns and recipes
- **[Examples Directory](../examples/)** - Complete example programs
- **Project Repository** - Real-world usage in tests and tools

## Can't find an answer?

Check these resources:

- **[Getting Started Guide](getting-started.md)** - Installation and first steps
- **[Language Guide](guide/basics.md)** - Comprehensive language reference
- **[Standard Library](reference/standard-library.md)** - Module documentation
- **[GitHub Issues](https://github.com/bentruyman/tea-lang/issues)** - Ask questions or report issues
