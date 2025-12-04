# Control Flow

Control flow determines the order in which code executes. Tea provides several constructs for controlling program flow: conditionals, loops, and pattern matching.

## Conditionals

### If Statements

The most basic form of conditional execution:

```tea
var temperature = 75

if temperature > 80
  print("It's hot!")
end
```

### If-Else

Provide an alternative when the condition is false:

```tea
var score = 85

if score >= 90
  print("Grade: A")
else
  print("Grade: B or lower")
end
```

### Else-If Chains

Check multiple conditions:

```tea
var score = 85

if score >= 90
  print("Grade: A")
else if score >= 80
  print("Grade: B")
else if score >= 70
  print("Grade: C")
else
  print("Grade: D or F")
end
```

### If Expressions

Tea also has if-expressions that return values, similar to ternary operators in other languages:

```tea
var is_active = true
var status = if(is_active) "online" else "offline"
print(status)  # Output: online
```

If-expressions are useful for inline conditionals:

```tea
var age = 18
var category = if(age >= 18) "adult" else "minor"

var x = 10
var result = x + if(x > 5) 100 else 0
print(result)  # Output: 110
```

If-expressions can be nested (though it's often clearer to use if-else chains):

```tea
var score = 85
var grade = if(score >= 90) "A" else if(score >= 80) "B" else "C"
```

## Loops

### While Loops

Repeat code while a condition is true:

```tea
var count = 0

while count < 5
  print("Count: ${count}")
  count = count + 1
end
```

Be careful to update the condition, or you'll create an infinite loop:

```tea
# Infinite loop - don't do this!
var x = 0
while x < 10
  print("This loops forever!")
  # x is never updated!
end
```

### For Loops

Iterate over collections with `for`:

```tea
var numbers = [1, 2, 3, 4, 5]

for num of numbers
  print("Number: ${num}")
end
```

This works with any collection type:

```tea
var names = ["Alice", "Bob", "Charlie"]

for name of names
  print("Hello, ${name}!")
end
```

### Break and Continue

Tea doesn't currently have `break` and `continue` keywords. To exit loops early, use conditional logic:

```tea
var numbers = [1, 2, 3, 4, 5]
var found = false
var target = 3
var index = 0

while index < @len(numbers) && !found
  if numbers[index] == target
    found = true
    print("Found ${target} at index ${index}")
  end
  index = index + 1
end
```

## Pattern Matching

Tea supports pattern matching with `case` statements, which is particularly useful for error handling.

### Basic Case Matching

```tea
error NetworkError {
  Timeout
  ConnectionRefused
  NotFound
}

def handle_error(err: NetworkError) -> String
  case err
  case is NetworkError.Timeout
    return "Request timed out"
  case is NetworkError.ConnectionRefused
    return "Connection refused"
  case is NetworkError.NotFound
    return "Resource not found"
  case _
    return "Unknown error"
  end
end
```

The `case _` is a catch-all pattern that matches anything.

### Matching with Data

When error variants carry data, you can extract it:

```tea
error FileError {
  NotFound(path: String)
  PermissionDenied(path: String)
  IOError(message: String)
}

def describe_file_error(err: FileError) -> String
  case err
  case is FileError.NotFound
    return `File not found: ${err.path}`
  case is FileError.PermissionDenied
    return `Permission denied: ${err.path}`
  case is FileError.IOError
    return `IO error: ${err.message}`
  case _
    return "Unknown file error"
  end
end
```

See the [Error Handling](error-handling.md) guide for more details on errors and pattern matching.

## Boolean Logic

Combine conditions using logical operators:

### AND (&&)

Both conditions must be true:

```tea
var age = 25
var has_license = true

if age >= 18 && has_license
  print("Can drive")
end
```

### OR (||)

At least one condition must be true:

```tea
var is_weekend = true
var is_holiday = false

if is_weekend || is_holiday
  print("No work today!")
end
```

### NOT (!)

Invert a boolean:

```tea
var is_raining = false

if !is_raining
  print("Good weather for a walk")
end
```

### Combining Operators

Use parentheses for complex conditions:

```tea
var age = 25
var is_student = false
var has_coupon = true

if (age < 18 || is_student) && has_coupon
  print("Eligible for discount")
end
```

## Comparison Operators

Tea provides standard comparison operators:

- `==` Equal to
- `!=` Not equal to
- `<` Less than
- `<=` Less than or equal to
- `>` Greater than
- `>=` Greater than or equal to

```tea
var x = 10
var y = 20

print(x == y)   # false
print(x != y)   # true
print(x < y)    # true
print(x <= y)   # true
print(x > y)    # false
print(x >= y)   # false
```

Comparisons work with numbers, strings, and booleans:

```tea
# Strings
var name1 = "Alice"
var name2 = "Bob"
print(name1 == name2)  # false

# Booleans
var flag1 = true
var flag2 = false
print(flag1 == flag2)  # false
```

## Optional Checking

Check if an optional value is nil:

```tea
var maybe_name: String? = nil

if maybe_name == nil
  print("No name provided")
else
  print("Name: ${maybe_name!}")
end
```

Or check if it has a value:

```tea
var maybe_age: Int? = 25

if maybe_age != nil
  print("Age: ${maybe_age!}")
end
```

## Practical Examples

### Finding Maximum

```tea
def max(a: Int, b: Int) -> Int
  if a > b
    return a
  else
    return b
  end
end

print(max(10, 20))  # Output: 20
```

Or using an if-expression:

```tea
def max(a: Int, b: Int) -> Int
  if(a > b) a else b
end
```

### Validating Input

```tea
def validate_age(age: Int) -> Bool
  if age < 0
    print("Age cannot be negative")
    return false
  end

  if age > 150
    print("Age seems unrealistic")
    return false
  end

  return true
end

var age = 25
if validate_age(age)
  print("Age is valid: ${age}")
end
```

### Processing a List

```tea
var numbers = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]
var sum = 0
var even_count = 0

for num of numbers
  sum = sum + num

  if num % 2 == 0
    even_count = even_count + 1
  end
end

print("Sum: ${sum}")
print("Even numbers: ${even_count}")
```

### Searching

```tea
var names = ["Alice", "Bob", "Charlie", "Diana"]
var search_name = "Charlie"
var found = false
var index = 0

while index < @len(names) && !found
  if names[index] == search_name
    found = true
    print("Found ${search_name} at position ${index}")
  end
  index = index + 1
end

if !found
  print("${search_name} not found")
end
```

## Next Steps

Now that you understand control flow, explore:

- **[Data Structures](data-structures.md)** - Lists, structs, and dictionaries
- **[Error Handling](error-handling.md)** - Errors, throw, and catch
- **[Advanced Topics](advanced.md)** - Generics, modules, and more

## Quick Reference

**If Statements:**

```tea
if condition
  # code
else if condition
  # code
else
  # code
end
```

**If Expressions:**

```tea
var value = if(condition) expr1 else expr2
```

**While Loop:**

```tea
while condition
  # code
end
```

**For Loop:**

```tea
for item of collection
  # code
end
```

**Pattern Matching:**

```tea
case value
case is Type.Variant
  # code
case _
  # default
end
```

**Logical Operators:**

- `&&` - AND
- `||` - OR
- `!` - NOT

**Comparison Operators:**

- `==`, `!=` - Equality
- `<`, `<=`, `>`, `>=` - Ordering
