# Advanced Topics

This guide covers Tea's more advanced features: generics, modules, lambdas, and compilation options.

## Generics

Generics allow you to write reusable code that works with multiple types while maintaining type safety.

### Generic Functions

Define type parameters in square brackets:

```tea
def identity[T](value: T) -> T
  value
end

var num = identity[Int](42)
var text = identity[String]("hello")
```

The compiler generates specialized versions for each type you use.

### Generic Structs

Structs can also be generic:

```tea
struct Box[T] {
  value: T
}

var int_box = Box[Int](value: 42)
var string_box = Box[String](value: "hello")

@println(int_box.value)     # Output: 42
@println(string_box.value)  # Output: hello
```

### Multiple Type Parameters

Use multiple generic parameters:

```tea
struct Pair[A, B] {
  first: A
  second: B
}

var pair = Pair[String, Int](first: "age", second: 30)
@println(`${pair.first}: ${pair.second}`)  # Output: age: 30
```

### Generic Functions with Constraints

Create generic functions for collections:

```tea
def first[T](list: List[T]) -> T?
  if @len(list) == 0
    return nil
  end

  list[0]
end

var numbers = [1, 2, 3]
var first_num = first[Int](numbers)

var names = ["Alice", "Bob"]
var first_name = first[String](names)
```

### Practical Example: Generic Container

```tea
struct Container[T] {
  items: List[T]
}

def create_container[T]() -> Container[T]
  Container[T](items: [])
end

def add_item[T](container: Container[T], item: T)
  # Note: Actual implementation would use stdlib list methods
  @println("Adding item to container")
end

var int_container = create_container[Int]()
var string_container = create_container[String]()
```

## Modules

Modules help organize code into reusable units and manage namespaces.

### Using Standard Library Modules

Import stdlib modules with `use`:

```tea
use fs = "std.fs"
use path = "std.path"
use env = "std.env"

var current_dir = env.cwd()
var files = fs.read_dir(current_dir)

for file in files
  var full_path = path.join([current_dir, file])
  @println(full_path)
end
```

The syntax is: `use alias = "module.path"`

### Common Standard Library Modules

- `std.fs` - Filesystem operations
- `std.path` - Path manipulation
- `std.env` - Environment variables
- `std.string` - String utilities
- `std.json` - JSON parsing and serialization
- `std.assert` - Assertions for testing

### Creating Your Own Modules

Define public functions with `pub`:

**helpers.tea:**

```tea
def private_helper(x: Int) -> Int
  x * 2
end

pub def double(x: Int) -> Int
  private_helper(x)
end

pub def triple(x: Int) -> Int
  x * 3
end
```

Only functions marked `pub` are visible to other modules.

### Using Custom Modules

Import your own modules:

```tea
use helpers = "./helpers"

var result = helpers.double(5)
@println(result)  # Output: 10
```

### Module Organization

Structure larger projects with modules:

```
project/
├── main.tea
├── math/
│   ├── geometry.tea
│   └── stats.tea
└── utils/
    ├── string.tea
    └── file.tea
```

Import from subdirectories:

```tea
use geometry = "./math/geometry"
use stats = "./math/stats"
```

## Lambdas

Lambdas (anonymous functions) let you create inline functions.

### Basic Lambda Syntax

```tea
var add = |a: Int, b: Int| => a + b

var result = add(5, 3)
@println(result)  # Output: 8
```

The syntax is: `|parameters| => expression`

### Lambda Type Annotations

Specify lambda types explicitly:

```tea
var multiply: Func(Int, Int) -> Int = |a: Int, b: Int| => a * b
```

### Lambdas with Multiple Statements

Use block syntax for complex lambdas:

```tea
var process = |x: Int| => {
  var doubled = x * 2
  var squared = doubled * doubled
  squared
}
```

### Higher-Order Functions

Functions that accept or return functions:

```tea
def apply_twice(f: Func(Int) -> Int, value: Int) -> Int
  f(f(value))
end

var increment = |x: Int| => x + 1
var result = apply_twice(increment, 5)
@println(result)  # Output: 7
```

### Closures

Lambdas capture variables from their surrounding scope:

```tea
def make_adder(base: Int) -> Func(Int) -> Int
  |value: Int| => base + value
end

var add_10 = make_adder(10)
@println(add_10(5))   # Output: 15
@println(add_10(20))  # Output: 30
```

### Practical Example: Filtering

```tea
def filter[T](list: List[T], predicate: Func(T) -> Bool) -> List[T]
  var result: List[T] = []

  for item in list
    if predicate(item)
      # Add to result (pseudocode)
    end
  end

  result
end

var numbers = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]
var is_even = |n: Int| => n % 2 == 0

# var evens = filter[Int](numbers, is_even)
```

## Compilation

Tea compiles to native binaries via LLVM, giving you multiple execution options.

### Running Scripts Directly

Execute without explicit compilation:

```bash
tea script.tea
```

Tea compiles and runs the script in one step.

### Building Native Binaries

Create a standalone executable:

```bash
tea build app.tea
```

This produces `./bin/app`, which you can run:

```bash
./bin/app
```

The binary:

- Has no runtime dependencies
- Can be distributed to compatible systems
- Runs at native speed

### Compilation Modes

**Development Mode** (default when running scripts):

- Fast compilation
- Includes debug symbols
- Good error messages

**Release Mode** (when building):

- Optimized for performance
- Smaller binary size
- Production-ready

### Viewing Compiler Output

Inspect the LLVM IR:

```bash
tea --emit llvm-ir script.tea
```

Emit object files:

```bash
tea --emit obj script.tea
```

View tokens:

```bash
tea --dump-tokens script.tea
```

These options are useful for understanding how Tea compiles your code.

## Testing

Tea has built-in testing support.

### Writing Tests

Use `test` blocks:

```tea
def add(a: Int, b: Int) -> Int
  a + b
end

test "addition works correctly"
  assert(add(2, 3) == 5)
  assert(add(-1, 1) == 0)
  assert(add(0, 0) == 0)
end

test "addition is commutative"
  assert(add(3, 5) == add(5, 3))
end
```

### Running Tests

Execute all tests in a file:

```bash
tea test math.tea
```

Or test all files in a directory:

```bash
tea test ./tests/
```

### Assertions

Use the `assert` function:

```tea
test "string operations"
  var name = "Alice"
  assert(@len(name) == 5)
  assert(name != "Bob")
end
```

For more assertions, use the `std.assert` module:

```tea
use assert = "std.assert"

test "using assert module"
  assert.eq(5, 5)
  assert.ne(5, 10)
  assert.gt(10, 5)
  assert.lt(5, 10)
end
```

## Code Formatting

Keep your code consistently formatted with the built-in formatter:

```bash
tea fmt script.tea
```

Format all Tea files in a directory:

```bash
tea fmt .
```

The formatter:

- Fixes indentation
- Standardizes spacing
- Ensures consistent style

Run it before committing code to maintain consistency.

## Practical Examples

### Generic Data Structure with Tests

```tea
use assert = "std.assert"

struct Stack[T] {
  items: List[T]
}

def create_stack[T]() -> Stack[T]
  Stack[T](items: [])
end

def push[T](stack: Stack[T], item: T)
  # Implementation would modify stack.items
end

test "stack operations"
  var int_stack = create_stack[Int]()
  assert(@len(int_stack.items) == 0)
end
```

### Module-Based Project

**math/calculator.tea:**

```tea
pub def add(a: Int, b: Int) -> Int
  a + b
end

pub def multiply(a: Int, b: Int) -> Int
  a * b
end
```

**main.tea:**

```tea
use calc = "./math/calculator"

var sum = calc.add(5, 3)
var product = calc.multiply(4, 7)

@println(`Sum: ${sum}`)
@println(`Product: ${product}`)
```

Build it:

```bash
tea build main.tea
./bin/main
```

## Next Steps

You've now covered Tea's core features! Explore:

- **[Standard Library Reference](../reference/standard-library.md)** - Complete stdlib documentation
- **[Built-ins Reference](../reference/builtins.md)** - Debug functions and intrinsics
- **[Examples](../examples.md)** - Real-world patterns and recipes

## Quick Reference

**Generics:**

```tea
def function[T](param: T) -> T
struct Name[T] { field: T }
```

**Modules:**

```tea
use alias = "module.path"
pub def exported() { }
```

**Lambdas:**

```tea
|param: Type| => expression
var f: Func(T) -> R = |x| => x
```

**Compilation:**

```bash
tea script.tea          # Run
tea build script.tea    # Build binary
tea test script.tea     # Run tests
tea fmt script.tea      # Format code
```

**Tests:**

```tea
test "description"
  assert(condition)
end
```
