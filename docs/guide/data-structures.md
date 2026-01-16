# Data Structures

Tea provides several data structures for organizing data: lists, structs, and dictionaries. This guide covers how to create, access, and manipulate these structures.

## Lists

Lists are ordered collections of elements of the same type.

### Creating Lists

Define a list using square brackets:

```tea
var numbers = [1, 2, 3, 4, 5]
var names = ["Alice", "Bob", "Charlie"]
var empty: List[Int] = []
```

Tea infers the element type from the contents. All elements must have the same type:

```tea
var valid = [1, 2, 3]              # List[Int]
var invalid = [1, "two", 3]        # Error: mixed types
```

### Type Annotations

Specify the element type explicitly:

```tea
var numbers: List[Int] = [1, 2, 3]
var names: List[String] = []  # Empty list with known type
```

### Accessing Elements

Use zero-based indexing:

```tea
var colors = ["red", "green", "blue"]

@println(colors[0])  # Output: red
@println(colors[1])  # Output: green
@println(colors[2])  # Output: blue
```

### Nested Lists

Lists can contain other lists:

```tea
var matrix = [[1, 2], [3, 4], [5, 6]]

@println(matrix[0])     # Output: [1, 2]
@println(matrix[0][0])  # Output: 1
@println(matrix[1][1])  # Output: 4
```

### List Slicing

Extract a portion of a list using range syntax:

**Exclusive Range (`..`)** - Excludes the end index:

```tea
var numbers = [1, 2, 3, 4, 5]
var slice = numbers[1..4]
@println(slice)  # Output: [2, 3, 4]
```

**Inclusive Range (`...`)** - Includes the end index:

```tea
var numbers = [10, 20, 30, 40]
var slice = numbers[1...2]
@println(slice)  # Output: [20, 30]
```

### Modifying Lists

Update elements by index:

```tea
var fruits = ["apple", "banana", "cherry"]
fruits[1] = "blueberry"
@println(fruits)  # Output: [apple, blueberry, cherry]
```

### List Length

Get the number of elements with `@len`:

```tea
var items = [1, 2, 3, 4, 5]
@println(@len(items))  # Output: 5
```

### Iterating Over Lists

Use a `for` loop:

```tea
var scores = [85, 90, 78, 92]

for score in scores
  @println(`Score: ${score}`)
end
```

### Common List Operations

**Sum elements:**

```tea
var numbers = [1, 2, 3, 4, 5]
var sum = 0

for num in numbers
  sum = sum + num
end

@println(`Sum: ${sum}`)  # Output: Sum: 15
```

**Find maximum:**

```tea
var numbers = [23, 67, 12, 89, 45]
var max = numbers[0]

for num in numbers
  if num > max
    max = num
  end
end

@println(`Max: ${max}`)  # Output: Max: 89
```

**Filter elements:**

```tea
var numbers = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]
var evens: List[Int] = []

for num in numbers
  if num % 2 == 0
    # Note: List append methods depend on stdlib
    # This is pseudocode for illustration
  end
end
```

## Structs

Structs are custom data types that group related values together.

### Defining Structs

Use the `struct` keyword:

```tea
struct Point {
  x: Int
  y: Int
}
```

Structs can have any number of fields:

```tea
struct User {
  name: String
  email: String
  age: Int
  is_active: Bool
}
```

### Creating Struct Instances

Use the struct name with named parameters:

```tea
struct Point {
  x: Int
  y: Int
}

var origin = Point(x: 0, y: 0)
var p1 = Point(x: 10, y: 20)
```

All fields must be provided:

```tea
# Error: missing field 'y'
var incomplete = Point(x: 5)
```

### Accessing Fields

Use dot notation:

```tea
struct User {
  name: String
  age: Int
}

var user = User(name: "Alice", age: 30)

@println(user.name)  # Output: Alice
@println(user.age)   # Output: 30
```

### Modifying Fields

Update fields of mutable structs:

```tea
var point = Point(x: 0, y: 0)
point.x = 10
point.y = 20
@println(`Point: (${point.x}, ${point.y})`)  # Output: Point: (10, 20)
```

### Structs in Functions

Pass structs to functions:

```tea
struct Rectangle {
  width: Float
  height: Float
}

def calculate_area(rect: Rectangle) -> Float
  rect.width * rect.height
end

var room = Rectangle(width: 12.5, height: 10.0)
var area = calculate_area(room)
@println(`Area: ${area}`)  # Output: Area: 125.0
```

Return structs from functions:

```tea
def create_point(x: Int, y: Int) -> Point
  Point(x: x, y: y)
end

var p = create_point(5, 10)
@println(`${p.x}, ${p.y}`)  # Output: 5, 10
```

### Lists of Structs

Structs work naturally with lists:

```tea
struct Person {
  name: String
  age: Int
}

var people = [
  Person(name: "Alice", age: 30),
  Person(name: "Bob", age: 25),
  Person(name: "Charlie", age: 35)
]

for person in people
  @println(`${person.name} is ${person.age} years old`)
end
```

### Nested Structs

Structs can contain other structs:

```tea
struct Address {
  street: String
  city: String
  zip: String
}

struct Contact {
  name: String
  address: Address
}

var contact = Contact(
  name: "Alice",
  address: Address(
    street: "123 Main St",
    city: "Springfield",
    zip: "12345"
  )
)

@println(contact.address.city)  # Output: Springfield
```

### Documentation Comments

Document struct fields with `##` comments:

```tea
## A user account in the system
struct User {
  ## The user's display name
  name: String
  ## The user's email address
  email: String
  ## Whether the account is currently active
  is_active: Bool
}
```

## Dictionaries

Dictionaries (also called maps or hash maps) store key-value pairs.

### Creating Dictionaries

Use curly braces with key-value pairs:

```tea
var scores = { "alice": 95, "bob": 87, "charlie": 92 }
var point = { x: 10, y: 20 }
```

### Accessing Values

Use dot notation:

```tea
var scores = { "alice": 95, "bob": 87 }
@println(scores.bob)  # Output: 87

var point = { x: 10, y: 20 }
@println(point.x)     # Output: 10
```

### Dictionary Types

Dictionaries in Tea are typed based on their keys and values:

```tea
# String keys, Int values
var scores = { "alice": 95, "bob": 87 }

# Symbol keys, values of various types
var config = {
  host: "localhost",
  port: 8080,
  debug: true
}
```

## Generic Collections

Lists and other collections can hold any type, including generics.

### Generic Lists

```tea
# List of integers
var numbers: List[Int] = [1, 2, 3]

# List of strings
var names: List[String] = ["Alice", "Bob"]

# List of optional integers
var maybe_numbers: List[Int?] = [1, nil, 3]
```

### Lists of Custom Types

```tea
struct Task {
  title: String
  completed: Bool
}

var tasks: List[Task] = [
  Task(title: "Write docs", completed: false),
  Task(title: "Review code", completed: true)
]
```

## Practical Examples

### Building a To-Do List

```tea
struct Task {
  description: String
  completed: Bool
}

def create_task(description: String) -> Task
  Task(description: description, completed: false)
end

def complete_task(task: Task)
  task.completed = true
end

def print_tasks(tasks: List[Task])
  var index = 0

  for task in tasks
    var status = if(task.completed) "✓" else "○"
    @println(`${index}. ${status} ${task.description}`)
    index = index + 1
  end
end

# Create tasks
var tasks = [
  create_task("Write documentation"),
  create_task("Fix bug in parser"),
  create_task("Add tests")
]

# Complete a task
complete_task(tasks[0])

# Display tasks
print_tasks(tasks)
```

### Working with Coordinates

```tea
struct Point {
  x: Float
  y: Float
}

def distance(p1: Point, p2: Point) -> Float
  var dx = p2.x - p1.x
  var dy = p2.y - p1.y
  # Note: sqrt would be from stdlib in real code
  (dx * dx + dy * dy)  # Squared distance
end

def midpoint(p1: Point, p2: Point) -> Point
  Point(
    x: (p1.x + p2.x) / 2.0,
    y: (p1.y + p2.y) / 2.0
  )
end

var p1 = Point(x: 0.0, y: 0.0)
var p2 = Point(x: 10.0, y: 10.0)
var mid = midpoint(p1, p2)

@println(`Midpoint: (${mid.x}, ${mid.y})`)  # Output: Midpoint: (5.0, 5.0)
```

## Next Steps

Now that you understand data structures, explore:

- **[Error Handling](error-handling.md)** - Errors, throw, and catch
- **[Advanced Topics](advanced.md)** - Generics, modules, and more
- **[Standard Library](../reference/standard-library.md)** - Built-in collection utilities

## Quick Reference

**Lists:**

```tea
var list = [1, 2, 3]
var element = list[0]
var slice = list[1..3]        # Exclusive
var inclusive = list[1...3]   # Inclusive
var length = @len(list)
```

**Structs:**

```tea
struct Name {
  field: Type
}

var instance = Name(field: value)
var value = instance.field
instance.field = new_value
```

**Dictionaries:**

```tea
var dict = { "key": value }
var dict2 = { symbol: value }
var value = dict.key
```

**Iteration:**

```tea
for item in collection
  # process item
end
```
