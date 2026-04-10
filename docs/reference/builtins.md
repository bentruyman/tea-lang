# Built-in Functions

Tea exposes a small set of global functions prefixed with `@`. They are always available without a `use` statement.

## Output

### `@print(value: Any) -> Void`

Write a value to stdout without a trailing newline.

### `@println(value: Any) -> Void`

Write a value to stdout with a trailing newline.

### `@eprint(value: Any) -> Void`

Write a value to stderr without a trailing newline.

### `@eprintln(value: Any) -> Void`

Write a value to stderr with a trailing newline.

## Introspection And Control

### `@type_of(value: Any) -> String`

Return the runtime type name for a value.

### `@to_string(value: Any) -> String`

Convert a value to its string representation.

### `@len(value: Any) -> Int`

Return the length of a string, list, or dict.

### `@append(list: List[T], value: T) -> Void`

Append a value to a mutable list in place.

### `@args() -> List`

Return the command-line arguments passed to the program.

### `@panic(message: String) -> Void`

Abort execution immediately with an error message.

### `@exit(code: Int) -> Void`

Terminate the process with the provided exit code.

## Standard Input

### `@read_line() -> String`

Read one line from stdin without the trailing newline.

### `@read_all() -> String`

Read all remaining stdin content into a string.

### `@is_tty() -> Bool`

Return true when stdin is connected to an interactive terminal.

## Math

### `@floor(value: Float) -> Int`

Round a float down to the nearest integer.

### `@ceil(value: Float) -> Int`

Round a float up to the nearest integer.

### `@round(value: Float) -> Int`

Round a float to the nearest integer.

### `@abs(value: Float) -> Float`

Return the absolute value of a float.

### `@sqrt(value: Float) -> Float`

Return the square root of a float.

### `@min(left: Float, right: Float) -> Float`

Return the smaller of two floats.

### `@max(left: Float, right: Float) -> Float`

Return the larger of two floats.

## See Also

- **[Standard Library](standard-library.md)** for source-backed modules such as `std.fs`, `std.path`, and `std.string`
