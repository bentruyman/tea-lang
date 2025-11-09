# New Built-Ins & Standard Library Structure

- [Built-ins](#built-ins)
- [Standard Library](#standard-library)
  - [assert](#assert) - Assertion helpers for tests and runtime checks
  - [env](#env) - Environment variable access and working directory management
  - [fs](#fs) - Filesystem operations
  - [json](#json) - JSON encoding and decoding
  - [math](#math) - Mathematical utilities
  - [path](#path) - Path manipulation utilities
  - [string](#string) - String manipulation utilities

## Built-ins

### Debug

- `@print(value: Unknown)` - Prints the string representation of a value to stderr
- `@println(value: Unknown)` - Prints the string representation of a value to stderr, followed by a newline
- `@to_string(value: Unknown)` - Converts any value to its string representation
- `@type_of(value: Unknown)` - Returns a string representation of the type of a value

### Error Handling

- `@panic(message: String)` - Panics with a message

### Math

- `@floor(value: Float) -> Int` - Rounds a float down to the nearest integer
- `@ceil(value: Float) -> Int` - Rounds a float up to the nearest integer
- `@round(value: Float) -> Int` - Rounds a float to the nearest integer
- `@abs(value: Float) -> Float` - Get the absolute value of a float
- `@sqrt(value: Float) -> Float` - Get the square root of a float
- `@min(a: Float, b: Float) -> Float` - Get the minimum of two floats
- `@max(a: Float, b: Float) -> Float` - Get the maximum of two floats
- ``

### Utility

- `@len(value: Dict | List | String)` - Gets the number of items in a `List`, characters in a `String`, or keys in a `Dict`

## Standard Library

### assert

- `ok(value: Unknown)` - Asserts that a value is truthy
- `eq(left: Unknown, right: Unknown)` - Asserts that two values are equal
- `ne(left: Unknown, right: Unknown)` - Asserts that two values are not equal
- `snapshot(name: String, value: Unknown, path: String)` - Asserts that a value matches a saved snapshot

### env

- `get(name: String) -> String` - Get the value of an environment variable
- `set(name: String, value: String) -> Void` - Set an environment variable
- `cwd() -> String` - Get the current working directory

### fs

- `create_dir(path: String) -> Void` - Create a directory
- `read_dir(path: String) -> List[String]` - Read the contents of a directory
- `read_file(path: String) -> String` - Read the contents of a file
- `remove(path: String) -> Void` - Remove a file or directory
- `rename(source: String, target: String) -> Void` - Rename a file or directory
- `stat(path: String) -> FileInfo` - Get information about a file or directory
- `write_file(path: String, contents: String) -> Void` - Write the contents of a file

### json

### path

- `basename(path: String) -> String` - Get the filename part of a path
- `dirname(path: String) -> String` - Get the directory part of a path
- `extension(path: String) -> String` - Get the extension of a path
- `join(parts: List[String]) -> String` - Join a list of path parts into a single path
- `split(path: String) -> List[String]` - Split a path into its parts

### string

- `starts_with(text: String, prefix: String) -> Bool` - Check if a string starts with a given prefix
- `ends_with(text: String, suffix: String) -> Bool` - Check if a string ends with a given suffix
- `replace(text: String, pattern: String, replacement: String) -> String` - Replace all occurrences of a pattern in a string with a replacement
- `to_lower(text: String) -> String` - Convert a string to lowercase
- `to_upper(text: String) -> String` - Convert a string to uppercase
- `trim_start(text: String) -> String` - Trim whitespace from the start of a string
- `trim_end(text: String) -> String` - Trim whitespace from the end of a string
- `trim(text: String) -> String` - Trim whitespace from both ends of a string
- `reverse(text: String) -> String` - Reverse a string
