# Built-in Functions

Tea provides several built-in functions (also called builtins or intrinsics) that are available globally without imports. These functions are prefixed with `@` and provide essential debugging and utility functionality.

## Output Functions

### `@print(value: Any) -> Void`

Print a value to standard output without a newline.

```tea
@print("Hello")
@print("World")
# Output: HelloWorld
```

Useful for building output incrementally:

```tea
@print("Loading")
@print(".")
@print(".")
@print(".")
# Output: Loading...
```

### `@println(value: Any) -> Void`

Print a value to standard output followed by a newline.

```tea
@println("Hello, Tea!")
@println(42)
@println(true)
```

Output:

```
Hello, Tea!
42
true
```

Works with any type:

```tea
var numbers = [1, 2, 3]
@println(numbers)  # Output: [1, 2, 3]

struct Point { x: Int, y: Int }
var p = Point(x: 5, y: 10)
@println(p)  # Output: Point(x: 5, y: 10)
```

## Introspection Functions

### `@type_of(value: Any) -> String`

Get the type name of a value as a string.

```tea
var number = 42
var text = "hello"
var flag = true

@println(@type_of(number))  # Output: Int
@println(@type_of(text))    # Output: String
@println(@type_of(flag))    # Output: Bool
```

Useful for debugging and type checking:

```tea
struct User { name: String }
var user = User(name: "Alice")

@println(@type_of(user))  # Output: User
```

### `@len(collection: Any) -> Int`

Get the length of a collection or string.

**Lists:**

```tea
var numbers = [1, 2, 3, 4, 5]
@println(@len(numbers))  # Output: 5

var empty: List[Int] = []
@println(@len(empty))    # Output: 0
```

**Strings:**

```tea
var greeting = "Hello"
@println(@len(greeting))  # Output: 5

var empty = ""
@println(@len(empty))     # Output: 0
```

**In loops:**

```tea
var items = ["a", "b", "c"]
var i = 0

while i < @len(items)
  @println(items[i])
  i = i + 1
end
```

## Math Functions

### `@abs(value: Float) -> Float`

Get the absolute value of a number.

```tea
@println(@abs(42.5))   # Output: 42.5
@println(@abs(-42.5))  # Output: 42.5
@println(@abs(0.0))    # Output: 0.0
```

### `@sqrt(value: Float) -> Float`

Calculate the square root.

```tea
@println(@sqrt(16.0))  # Output: 4.0
@println(@sqrt(2.0))   # Output: 1.414...
@println(@sqrt(0.0))   # Output: 0.0
```

**Pythagorean theorem:**

```tea
def distance(x: Float, y: Float) -> Float
  @sqrt(x * x + y * y)
end

@println(distance(3.0, 4.0))  # Output: 5.0
```

### `@floor(value: Float) -> Float`

Round down to the nearest integer.

```tea
@println(@floor(3.9))   # Output: 3.0
@println(@floor(3.1))   # Output: 3.0
@println(@floor(-2.5))  # Output: -3.0
```

### `@ceil(value: Float) -> Float`

Round up to the nearest integer.

```tea
@println(@ceil(3.1))   # Output: 4.0
@println(@ceil(3.9))   # Output: 4.0
@println(@ceil(-2.5))  # Output: -2.0
```

### `@round(value: Float) -> Float`

Round to the nearest integer.

```tea
@println(@round(3.4))   # Output: 3.0
@println(@round(3.5))   # Output: 4.0
@println(@round(3.6))   # Output: 4.0
@println(@round(-2.5))  # Output: -3.0
```

### `@min(a: Float, b: Float) -> Float`

Get the minimum of two numbers.

```tea
@println(@min(5.0, 10.0))   # Output: 5.0
@println(@min(-3.0, -1.0))  # Output: -3.0
```

Find minimum in a list:

```tea
var numbers = [7.5, 3.2, 9.1, 1.8, 6.4]
var min = numbers[0]

for num of numbers
  min = @min(min, num)
end

@println(min)  # Output: 1.8
```

### `@max(a: Float, b: Float) -> Float`

Get the maximum of two numbers.

```tea
@println(@max(5.0, 10.0))   # Output: 10.0
@println(@max(-3.0, -1.0))  # Output: -1.0
```

Find maximum in a list:

```tea
var numbers = [7.5, 3.2, 9.1, 1.8, 6.4]
var max = numbers[0]

for num of numbers
  max = @max(max, num)
end

@println(max)  # Output: 9.1
```

## Utility Functions

### `@to_string(value: Any) -> String`

Convert any value to its string representation.

```tea
var num = 42
var flag = true

@println(@to_string(num))   # Output: "42"
@println(@to_string(flag))  # Output: "true"
```

**Note:** This is used internally for string interpolation. You typically don't need to call it directly:

```tea
var count = 5
# These are equivalent:
@println("Count: ${count}")
@println("Count: " + @to_string(count))
```

## Practical Examples

### Debugging Type Issues

```tea
def process(value: Any)
  @println("Processing ${@type_of(value)}")

  if @type_of(value) == "Int"
    @println("It's an integer: ${value}")
  else if @type_of(value) == "String"
    @println("It's a string: ${value}")
  else
    @println("Unknown type")
  end
end

process(42)
process("hello")
```

### Statistics

```tea
def calculate_stats(numbers: List[Float])
  if @len(numbers) == 0
    @println("Empty list")
    return
  end

  var sum = 0.0
  var min_val = numbers[0]
  var max_val = numbers[0]

  for num of numbers
    sum = sum + num
    min_val = @min(min_val, num)
    max_val = @max(max_val, num)
  end

  var average = sum / @to_float(@len(numbers))

  @println("Count: ${@len(numbers)}")
  @println("Sum: ${sum}")
  @println("Average: ${average}")
  @println("Min: ${min_val}")
  @println("Max: ${max_val}")
end

var scores = [85.5, 92.0, 78.5, 95.0, 88.0]
calculate_stats(scores)
```

### String Analysis

```tea
def analyze_string(text: String)
  @println("Text: ${text}")
  @println("Length: ${@len(text)}")
  @println("Type: ${@type_of(text)}")

  if @len(text) == 0
    @println("String is empty")
  else if @len(text) < 10
    @println("String is short")
  else
    @println("String is long")
  end
end

analyze_string("Hello, Tea!")
```

### Distance Calculator

```tea
struct Point {
  x: Float
  y: Float
}

def distance(p1: Point, p2: Point) -> Float
  var dx = p2.x - p1.x
  var dy = p2.y - p1.y
  @sqrt(dx * dx + dy * dy)
end

def distance_from_origin(p: Point) -> Float
  @sqrt(p.x * p.x + p.y * p.y)
end

var origin = Point(x: 0.0, y: 0.0)
var point = Point(x: 3.0, y: 4.0)

@println("Distance from origin: ${distance_from_origin(point)}")
@println("Distance between points: ${distance(origin, point)}")
```

## Best Practices

### Use for Debugging

Built-in functions are great for quick debugging:

```tea
def complex_calculation(x: Int) -> Int
  @println("Input: ${x}, Type: ${@type_of(x)}")  # Debug

  var result = x * 2 + 10

  @println("Result: ${result}")  # Debug
  return result
end
```

### Prefer `@println` for Output

For user-facing output, use `@println` or `@print`:

```tea
# Good
@println("Processing complete")

# Avoid using @type_of for user output
# @println(@type_of(value))  # Too technical for users
```

### Use `@len` for Bounds Checking

Always check lengths before indexing:

```tea
def get_first[T](list: List[T]) -> T?
  if @len(list) == 0
    return nil
  end

  list[0]
end
```

### Math Functions for Calculations

Use math builtins for numeric operations:

```tea
# Calculate bounding box
def bounding_box(numbers: List[Float]) -> String
  var min_val = numbers[0]
  var max_val = numbers[0]

  for num of numbers
    min_val = @min(min_val, num)
    max_val = @max(max_val, num)
  end

  "Range: [${min_val}, ${max_val}]"
end
```

## See Also

- **[Standard Library](standard-library.md)** - Higher-level utilities built on builtins
- **[Basics Guide](../guide/basics.md)** - Learn fundamental Tea concepts
- **[Examples](../examples.md)** - More practical examples

## Quick Reference

**Output:**

- `@print(value)` - Print without newline
- `@println(value)` - Print with newline

**Introspection:**

- `@type_of(value)` - Get type name
- `@len(collection)` - Get length

**Math:**

- `@abs(n)` - Absolute value
- `@sqrt(n)` - Square root
- `@floor(n)` - Round down
- `@ceil(n)` - Round up
- `@round(n)` - Round to nearest
- `@min(a, b)` - Minimum
- `@max(a, b)` - Maximum

**Utility:**

- `@to_string(value)` - Convert to string
