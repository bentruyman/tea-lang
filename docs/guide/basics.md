# Basics

This guide introduces Tea's fundamental concepts: variables, types, and functions. By the end, you'll understand how to write basic Tea programs.

## Variables

Tea has two ways to declare variables: `var` for mutable variables and `const` for immutable values.

### Mutable Variables

Use `var` to declare variables that can change:

```tea
var count = 0
count = count + 1
print(count)  # Output: 1
```

### Constants

Use `const` for values that never change:

```tea
const pi = 3.14159
const greeting = "Hello!"

# This would be an error:
# pi = 3.14  # Error: cannot assign to const
```

Constants must be assigned when declared and cannot be reassigned.

## Types

Tea is statically typed but uses type inference to reduce verbosity. The compiler determines types automatically in most cases.

### Basic Types

Tea has several built-in types:

- **Int** - Signed integers: `42`, `-10`, `0`
- **Float** - Floating-point numbers: `3.14`, `-0.5`, `2.0`
- **String** - Text: `"hello"`, `"world"`
- **Bool** - Boolean values: `true`, `false`

### Type Inference

The compiler infers types from the initial value:

```tea
var age = 30              # Inferred as Int
var price = 19.99         # Inferred as Float
var name = "Alice"        # Inferred as String
var is_active = true      # Inferred as Bool
```

### Explicit Type Annotations

You can specify types explicitly:

```tea
var count: Int = 0
var temperature: Float = 98.6
var message: String = "Hello"
var flag: Bool = false
```

This is useful for:

- Documenting your intent
- Catching type errors early
- Declaring variables without an initial value

### Type Checking

Tea prevents type mismatches at compile time:

```tea
var number = 42
number = "not a number"  # Error: expected Int, found String

def add(a: Int, b: Int) -> Int
  a + b
end

add(5, "10")  # Error: expected Int, found String
```

## Strings

Strings are sequences of characters enclosed in double quotes.

### String Literals

```tea
var simple = "Hello, Tea!"
var empty = ""
var multiline = "This is
a multiline
string"
```

### String Interpolation

Embed expressions in strings using `${}`:

```tea
var name = "Alice"
var age = 30

print("Name: ${name}")           # Output: Name: Alice
print("Age: ${age}")             # Output: Age: 30
print("Next year: ${age + 1}")   # Output: Next year: 31
```

Any expression can go inside `${}`:

```tea
var x = 5
var y = 10
print("Sum: ${x + y}")           # Output: Sum: 15
print("Product: ${x * y}")       # Output: Product: 50
```

### Escape Sequences

Use backslashes for special characters:

```tea
print("Line 1\nLine 2")          # Newline
print("Tab\tseparated")          # Tab
print("Quote: \"Hello\"")        # Escaped quotes
print("Backslash: \\")           # Backslash
```

## Numbers

Tea has two numeric types: integers and floating-point numbers.

### Integers

Whole numbers without decimal points:

```tea
var positive = 42
var negative = -10
var zero = 0
```

Standard arithmetic operations:

```tea
var sum = 5 + 3       # 8
var diff = 10 - 4     # 6
var product = 6 * 7   # 42
var quotient = 20 / 4 # 5
var remainder = 17 % 5 # 2
```

### Floats

Numbers with decimal points:

```tea
var pi = 3.14159
var temperature = -40.0
var tiny = 0.0001
```

Float operations:

```tea
var sum = 3.5 + 2.1       # 5.6
var product = 2.5 * 4.0   # 10.0
var quotient = 7.5 / 2.0  # 3.75
```

### Type Safety

Integers and floats are distinct types:

```tea
var int_val = 5
var float_val = 5.0

# These are different types!
# int_val = float_val  # Would be an error
```

## Booleans

Boolean values represent true or false:

```tea
var is_open = true
var is_closed = false
```

### Boolean Operations

Logical operators work with booleans:

```tea
var a = true
var b = false

print(a && b)    # false (logical AND)
print(a || b)    # true (logical OR)
print(!a)        # false (logical NOT)
```

### Comparisons

Comparison operators produce booleans:

```tea
var x = 5
var y = 10

print(x == y)    # false (equal)
print(x != y)    # true (not equal)
print(x < y)     # true (less than)
print(x <= y)    # true (less than or equal)
print(x > y)     # false (greater than)
print(x >= y)    # false (greater than or equal)
```

## Functions

Functions encapsulate reusable code. They're defined with the `def` keyword.

### Basic Functions

```tea
def greet()
  print("Hello, Tea!")
end

greet()  # Call the function
```

### Parameters

Functions can accept parameters:

```tea
def greet(name: String)
  print("Hello, ${name}!")
end

greet("Alice")  # Output: Hello, Alice!
```

Multiple parameters:

```tea
def introduce(name: String, age: Int)
  print("${name} is ${age} years old")
end

introduce("Bob", 25)  # Output: Bob is 25 years old
```

### Return Values

Use `->` to specify the return type:

```tea
def add(a: Int, b: Int) -> Int
  a + b
end

var result = add(5, 3)
print(result)  # Output: 8
```

The last expression in a function is automatically returned. You can also use `return` explicitly:

```tea
def subtract(a: Int, b: Int) -> Int
  return a - b
end
```

### Early Returns

Use `return` to exit early:

```tea
def divide(a: Int, b: Int) -> Float
  if b == 0
    print("Error: division by zero")
    return 0.0
  end

  a / b
end
```

### No Return Value

Functions without a return type return nothing:

```tea
def log_message(message: String)
  print("LOG: ${message}")
end

log_message("Application started")
```

## Comments

Use `#` for single-line comments:

```tea
# This is a comment
var x = 5  # Inline comment
```

Use `##` for documentation comments:

```tea
## Calculate the sum of two integers
def add(a: Int, b: Int) -> Int
  a + b
end
```

Documentation comments are often used above functions, structs, and modules to describe their purpose.

## Nil and Optional Types

Tea has `nil` to represent the absence of a value. Variables can only hold `nil` if their type is marked optional with `?`.

### Optional Types

Add `?` to make a type optional:

```tea
var maybe_name: String? = nil
var maybe_age: Int? = nil
```

Optional values must be checked before use:

```tea
var value: Int? = 42

if value != nil
  print("Value: ${value!}")  # Use ! to unwrap
end
```

### Nil Coalescing

Use `??` to provide a default value:

```tea
var maybe_count: Int? = nil
var count = maybe_count ?? 0  # Use 0 if nil
print(count)  # Output: 0
```

### Force Unwrap

Use `!` to unwrap an optional (be careful - this crashes if the value is nil):

```tea
var name: String? = "Alice"
print(name!)  # Output: Alice

var empty: String? = nil
# print(empty!)  # Would crash!
```

Only use `!` when you're certain the value is not nil.

## Putting It Together

Here's a complete example using what we've learned:

```tea
## Calculate the area of a rectangle
def calculate_area(width: Float, height: Float) -> Float
  width * height
end

## Format a measurement with units
def format_measurement(value: Float, unit: String) -> String
  "${value} ${unit}"
end

# Constants
const room_width = 12.5
const room_height = 10.0

# Calculate
var area = calculate_area(room_width, room_height)

# Display result
print("Room dimensions:")
print("  Width: ${format_measurement(room_width, "feet")}")
print("  Height: ${format_measurement(room_height, "feet")}")
print("  Area: ${format_measurement(area, "square feet")}")
```

## Next Steps

Now that you understand the basics, learn about:

- **[Control Flow](control-flow.md)** - if/else, loops, and pattern matching
- **[Data Structures](data-structures.md)** - Lists, structs, and dictionaries
- **[Error Handling](error-handling.md)** - Errors, throw, and catch

## Quick Reference

**Variables:**

- `var name = value` - Mutable variable
- `const NAME = value` - Immutable constant

**Types:**

- `Int`, `Float`, `String`, `Bool` - Basic types
- `Type?` - Optional type (can be nil)

**Functions:**

```tea
def name(param: Type) -> ReturnType
  # body
end
```

**Comments:**

- `# Single line comment`
- `## Documentation comment`

**Operators:**

- Arithmetic: `+`, `-`, `*`, `/`, `%`
- Comparison: `==`, `!=`, `<`, `<=`, `>`, `>=`
- Logical: `&&`, `||`, `!`
- Optional: `??` (coalesce), `!` (unwrap)
