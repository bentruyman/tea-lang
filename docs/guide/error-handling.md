# Error Handling

Tea provides a robust error handling system using custom error types, `throw`, and `catch`. This approach makes errors explicit in function signatures and helps you write more reliable code.

## Error Types

Define custom error types with the `error` keyword:

```tea
error FileError {
  NotFound
  PermissionDenied
}
```

Error types can have multiple variants, similar to enums in other languages.

### Error Variants with Data

Variants can carry associated data:

```tea
error NetworkError {
  Timeout(duration: Int)
  ConnectionRefused(host: String, port: Int)
  InvalidResponse(status: Int)
}
```

This allows you to include context about what went wrong.

## Throwing Errors

Use `throw` to signal an error:

```tea
error MathError {
  DivisionByZero
}

def divide(a: Float, b: Float) -> Float ! MathError.DivisionByZero
  if b == 0.0
    throw MathError.DivisionByZero
  end

  a / b
end
```

### Function Signatures with Errors

The `!` syntax in the return type declares that a function can throw errors:

```tea
def risky_operation() -> Int ! NetworkError.Timeout
  # Function body
end
```

Multiple error types:

```tea
def complex_operation() -> String ! FileError.NotFound, NetworkError.Timeout
  # Function body
end
```

This makes errors part of the function's contract, so callers know what can go wrong.

## Catching Errors

Use `catch` to handle errors:

```tea
error FileError {
  NotFound(path: String)
}

def read_config(path: String) -> String ! FileError.NotFound
  # Simulated file read
  throw FileError.NotFound(path)
end

def load_config() -> String
  var content = read_config("config.json") catch err
    case is FileError.NotFound
      @println(`Config not found: ${err.path}`)
      return "default config"
    case _
      return "error"
  end

  content
end
```

### The Catch Block

The `catch` block receives the error and uses pattern matching to handle it:

```tea
var result = risky_function() catch err
  case is ErrorType.Variant
    # Handle this variant
  case _
    # Handle any other error
end
```

### Accessing Error Data

When error variants carry data, access it through the error variable:

```tea
error ApiError {
  RequestFailed(status: Int, message: String)
}

def call_api() -> String ! ApiError.RequestFailed
  throw ApiError.RequestFailed(status: 404, message: "Not found")
end

def handle_api_call() -> String
  var response = call_api() catch err
    case is ApiError.RequestFailed
      return `API error ${err.status}: ${err.message}`
    case _
      return "Unknown error"
  end

  response
end
```

## Pattern Matching Errors

The `case` statement matches error variants:

```tea
error DatabaseError {
  ConnectionFailed(reason: String)
  QueryFailed(query: String)
  TimeoutExpired
}

def query_database(sql: String) -> String ! DatabaseError
  # Simulated database query
  throw DatabaseError.QueryFailed(query: sql)
end

def execute_query(sql: String) -> String
  var result = query_database(sql) catch err
    case is DatabaseError.ConnectionFailed
      @println(`Connection failed: ${err.reason}`)
      return ""
    case is DatabaseError.QueryFailed
      @println(`Query failed: ${err.query}`)
      return ""
    case is DatabaseError.TimeoutExpired
      @println("Database timeout")
      return ""
    case _
      @println("Unknown database error")
      return ""
  end

  result
end
```

The `case _` pattern is a catch-all for any unhandled variants.

## Practical Examples

### File Operations

```tea
error FileError {
  NotFound(path: String)
  PermissionDenied(path: String)
  InvalidFormat
}

def read_file(path: String) -> String ! FileError
  # Simulated file reading
  if path == ""
    throw FileError.NotFound(path)
  end

  "file contents"
end

def load_document(path: String) -> String
  var contents = read_file(path) catch err
    case is FileError.NotFound
      @println(`File not found: ${err.path}`)
      return "default content"
    case is FileError.PermissionDenied
      @println(`Permission denied: ${err.path}`)
      return ""
    case _
      @println("Error reading file")
      return ""
  end

  contents
end
```

### Validation Errors

```tea
error ValidationError {
  TooShort(min: Int, actual: Int)
  TooLong(max: Int, actual: Int)
  InvalidCharacter(char: String)
}

def validate_username(name: String) -> Bool ! ValidationError
  var length = @len(name)

  if length < 3
    throw ValidationError.TooShort(min: 3, actual: length)
  end

  if length > 20
    throw ValidationError.TooLong(max: 20, actual: length)
  end

  true
end

def create_user(name: String)
  var valid = validate_username(name) catch err
    case is ValidationError.TooShort
      @println(`Username too short: ${err.actual} chars (minimum: ${err.min})`)
      return
    case is ValidationError.TooLong
      @println(`Username too long: ${err.actual} chars (maximum: ${err.max})`)
      return
    case _
      @println("Invalid username")
      return
  end

  @println(`User created: ${name}`)
end
```

### Network Requests

```tea
error HttpError {
  Timeout(url: String)
  NotFound(url: String)
  ServerError(code: Int)
}

def fetch(url: String) -> String ! HttpError
  # Simulated HTTP request
  throw HttpError.NotFound(url)
end

def get_data(url: String) -> String
  var response = fetch(url) catch err
    case is HttpError.Timeout
      @println(`Request timed out: ${err.url}`)
      return ""
    case is HttpError.NotFound
      @println(`Resource not found: ${err.url}`)
      return ""
    case is HttpError.ServerError
      @println(`Server error: ${err.code}`)
      return ""
    case _
      @println("Network error")
      return ""
  end

  response
end
```

## Error Propagation

Sometimes you want to catch an error, do something, and then re-throw it:

```tea
def process_file(path: String) -> String ! FileError
  @println(`Processing: ${path}`)

  var content = read_file(path) catch err
    case is FileError.NotFound
      @println("Logging error: file not found")
      throw err  # Re-throw the error
    case _
      throw err
  end

  content
end
```

## Best Practices

### Be Specific

Create descriptive error variants:

```tea
# Good - specific errors
error AuthError {
  InvalidPassword
  UserNotFound
  AccountLocked
  SessionExpired
}

# Less helpful - generic errors
error AuthError {
  Failed
}
```

### Include Context

Add data to error variants for debugging:

```tea
# Good - includes context
error ParseError {
  UnexpectedToken(line: Int, column: Int, token: String)
}

# Less helpful - no context
error ParseError {
  SyntaxError
}
```

### Document Errors

Document what errors a function can throw:

```tea
## Read a configuration file
##
## Throws:
## - FileError.NotFound if the file doesn't exist
## - FileError.InvalidFormat if the file is malformed
def read_config(path: String) -> Config ! FileError
  # Implementation
end
```

### Handle All Cases

Always include a catch-all pattern:

```tea
var result = risky_operation() catch err
  case is SpecificError.SpecificCase
    # Handle known case
  case _  # Always include this!
    # Handle unexpected errors
end
```

## Errors vs Optionals

Use errors when you need to communicate _why_ something failed. Use optionals (`?`) for simple presence/absence:

```tea
# Use optional for simple "found or not found"
def find_user(id: Int) -> User?
  # Return User or nil
end

# Use errors when failure needs explanation
def authenticate(username: String, password: String) -> Session ! AuthError
  # Can throw InvalidPassword, UserNotFound, etc.
end
```

## Next Steps

Now that you understand error handling, explore:

- **[Advanced Topics](advanced.md)** - Generics, modules, and compilation
- **[Standard Library](../reference/standard-library.md)** - Built-in error handling utilities
- **[Examples](../examples.md)** - More error handling patterns

## Quick Reference

**Define Errors:**

```tea
error ErrorName {
  VariantName
  VariantWithData(field: Type)
}
```

**Throw Errors:**

```tea
throw ErrorType.Variant
throw ErrorType.VariantWithData(value)
```

**Catch Errors:**

```tea
var result = risky_function() catch err
  case is ErrorType.Variant
    # handle
  case _
    # default
end
```

**Function Signatures:**

```tea
def function() -> ReturnType ! ErrorType
  # body
end
```

**Access Error Data:**

```tea
case is ErrorType.Variant
  @println(err.field)  # Access variant fields
end
```
