# Tea Standard Library Reference

This document provides a comprehensive reference for all built-in functions and standard library modules in Tea.

## Table of Contents

- [Built-ins](#built-ins)
  - [Debug](#debug)
  - [Error Handling](#error-handling)
  - [Math](#math)
  - [Utility](#utility)
- [Standard Library](#standard-library)
  - [assert](#assert) - Assertion helpers for tests and runtime checks
  - [env](#env) - Environment variable access and working directory management
  - [fs](#fs) - Filesystem operations
  - [json](#json) - JSON encoding and decoding
  - [path](#path) - Path manipulation utilities
  - [string](#string) - String manipulation utilities

---

## Built-ins

Built-in functions are available globally without imports and use the `@` prefix.

### Debug

#### `@print(value: Unknown) -> Void`

Prints the string representation of a value to stderr.

**Examples:**

```tea
@print("Hello, world!")
@print(42)
@print([1, 2, 3])
```

#### `@println(value: Unknown) -> Void`

Prints the string representation of a value to stderr, followed by a newline.

**Examples:**

```tea
@println("Hello, world!")
@println(42)
```

#### `@to_string(value: Unknown) -> String`

Converts any value to its string representation.

**Examples:**

```tea
var s = @to_string(42)        # => "42"
var s = @to_string([1, 2, 3]) # => "[1, 2, 3]"
```

#### `@type_of(value: Unknown) -> String`

Returns a string representation of the type of a value.

**Examples:**

```tea
@type_of(42)        # => "Int"
@type_of("hello")   # => "String"
@type_of([1, 2, 3]) # => "List"
```

### Error Handling

#### `@panic(message: String) -> Void`

Terminates the program immediately with an error message.

**Examples:**

```tea
@panic("Something went wrong!")
```

### Math

#### `@floor(value: Float) -> Int`

Rounds a float down to the nearest integer.

**Examples:**

```tea
@floor(3.7)  # => 3
@floor(3.2)  # => 3
@floor(-3.7) # => -4
```

#### `@ceil(value: Float) -> Int`

Rounds a float up to the nearest integer.

**Examples:**

```tea
@ceil(3.2)  # => 4
@ceil(3.7)  # => 4
@ceil(-3.2) # => -3
```

#### `@round(value: Float) -> Int`

Rounds a float to the nearest integer.

**Examples:**

```tea
@round(3.2)  # => 3
@round(3.7)  # => 4
@round(3.5)  # => 4
@round(-3.5) # => -4
```

#### `@abs(value: Float) -> Float`

Returns the absolute value of a float.

**Examples:**

```tea
@abs(3.5)   # => 3.5
@abs(-3.5)  # => 3.5
@abs(0.0)   # => 0.0
```

#### `@sqrt(value: Float) -> Float`

Returns the square root of a float.

**Examples:**

```tea
@sqrt(4.0)  # => 2.0
@sqrt(9.0)  # => 3.0
@sqrt(2.0)  # => 1.414...
```

#### `@min(a: Float, b: Float) -> Float`

Returns the minimum of two floats.

**Examples:**

```tea
@min(5.0, 10.0)  # => 5.0
@min(10.0, 5.0)  # => 5.0
@min(5.0, 5.0)   # => 5.0
```

#### `@max(a: Float, b: Float) -> Float`

Returns the maximum of two floats.

**Examples:**

```tea
@max(5.0, 10.0)  # => 10.0
@max(10.0, 5.0)  # => 10.0
@max(5.0, 5.0)   # => 5.0
```

### Utility

#### `@len(value: Dict | List | String) -> Int`

Returns the number of items in a `List`, characters in a `String`, or keys in a `Dict`.

**Examples:**

```tea
@len("hello")      # => 5
@len([1, 2, 3])    # => 3
@len({"a": 1})     # => 1
```

---

## Standard Library

Standard library modules must be imported with `use` statements before use.

### assert

Assertion helpers for tests and runtime checks.

#### `ok(value: Unknown) -> Void`

Asserts that a value is truthy. Panics if the value is falsy.

**Examples:**

```tea
use assert = "std.assert"

assert.ok(1 + 1 == 2)
assert.ok(x > 0)
```

#### `eq(left: Unknown, right: Unknown) -> Void`

Asserts that two values are equal. Panics if they are not equal.

**Examples:**

```tea
use assert = "std.assert"

assert.eq(1 + 1, 2)
assert.eq("hello", "hello")
```

#### `ne(left: Unknown, right: Unknown) -> Void`

Asserts that two values are not equal. Panics if they are equal.

**Examples:**

```tea
use assert = "std.assert"

assert.ne(1, 2)
assert.ne("hello", "world")
```

#### `snapshot(name: String, value: Unknown, path: String) -> Void`

Asserts that a value matches a saved snapshot. Creates the snapshot if it doesn't exist.

**Examples:**

```tea
use assert = "std.assert"

assert.snapshot("test_name", result)
assert.snapshot("test_name", result, "custom/path")
```

---

### env

Environment variable access and working directory management.

#### `get(name: String) -> String`

Gets the value of an environment variable.

**Examples:**

```tea
use env = "std.env"

var path = env.get("PATH")
var home = env.get("HOME")
```

#### `set(name: String, value: String) -> Void`

Sets an environment variable for the current process.

**Examples:**

```tea
use env = "std.env"

env.set("MY_VAR", "my_value")
```

#### `cwd() -> String`

Gets the current working directory.

**Examples:**

```tea
use env = "std.env"

var cwd = env.cwd()
```

#### `vars() -> Dict[String, String]`

Gets all environment variables as a dictionary.

**Examples:**

```tea
use env = "std.env"

var all_vars = env.vars()
for key of @keys(all_vars)
  @println(`{key}={all_vars[key]}`)
end
```

---

### fs

Filesystem operations for reading, writing, and managing files and directories.

#### `read_file(path: String) -> String`

Reads the entire contents of a text file.

**Examples:**

```tea
use fs = "std.fs"

var content = fs.read_file("file.txt")
```

#### `write_file(path: String, contents: String) -> Void`

Writes a string to a file, replacing existing contents.

**Examples:**

```tea
use fs = "std.fs"

fs.write_file("file.txt", "Hello, world!")
```

#### `create_dir(path: String) -> Void`

Creates a directory. Parent directories must already exist.

**Examples:**

```tea
use fs = "std.fs"

fs.create_dir("my_dir")
```

#### `remove(path: String) -> Void`

Removes a file or directory recursively.

**Examples:**

```tea
use fs = "std.fs"

fs.remove("file.txt")
fs.remove("my_dir")
```

#### `read_dir(path: String) -> List[String]`

Lists all entries in a directory.

**Examples:**

```tea
use fs = "std.fs"

var entries = fs.read_dir(".")
for entry of entries
  @println(entry)
end
```

#### `rename(source: String, target: String) -> Void`

Renames or moves a file or directory.

**Examples:**

```tea
use fs = "std.fs"

fs.rename("old.txt", "new.txt")
fs.rename("file.txt", "subdir/file.txt")
```

---

### json

JSON encoding and decoding utilities.

#### `encode(value: Dict[String, String]) -> String`

Encodes a value to a JSON string.

**Examples:**

```tea
use json = "std.json"

var json_str = json.encode({"name": "tea", "version": "1.0"})
```

#### `decode(json_str: String) -> Dict[String, String]`

Decodes a JSON string to a value.

**Examples:**

```tea
use json = "std.json"

var data = json.decode("{\"name\":\"tea\"}")
```

---

### path

Path manipulation utilities for working with file paths.

#### `join(parts: List[String]) -> String`

Joins path components into a single path.

**Examples:**

```tea
use path = "std.path"

var p = path.join(["usr", "local", "bin"])  # => "usr/local/bin"
```

#### `split(file_path: String) -> List[String]`

Splits a path into its components.

**Examples:**

```tea
use path = "std.path"

var parts = path.split("/usr/local/bin")  # => ["usr", "local", "bin"]
var parts = path.split("usr/local/")      # => ["usr", "local"]
```

#### `dirname(file_path: String) -> String`

Gets the directory part of a path.

**Examples:**

```tea
use path = "std.path"

var dir = path.dirname("/usr/local/bin/tea")  # => "/usr/local/bin"
var dir = path.dirname("/usr/local/")         # => "/usr/local"
var dir = path.dirname("file.txt")            # => ""
var dir = path.dirname("/")                   # => "/"
```

#### `basename(file_path: String) -> String`

Gets the filename part of a path.

**Examples:**

```tea
use path = "std.path"

var name = path.basename("/usr/local/bin/tea")  # => "tea"
var name = path.basename("/usr/local/")         # => "local"
var name = path.basename("file.txt")            # => "file.txt"
var name = path.basename("/")                   # => ""
```

#### `extension(file_path: String) -> String`

Gets the extension of a path (without the dot).

**Examples:**

```tea
use path = "std.path"

var ext = path.extension("file.tea")     # => "tea"
var ext = path.extension("file.tar.gz")  # => "gz"
var ext = path.extension(".env")         # => ""
var ext = path.extension("file")         # => ""
```

---

### string

String manipulation utilities for common text operations.

#### `starts_with(text: String, prefix: String) -> Bool`

Checks if a string starts with a given prefix.

**Examples:**

```tea
use string = "std.string"

string.starts_with("hello world", "hello")  # => true
string.starts_with("hello world", "world")  # => false
```

#### `ends_with(text: String, suffix: String) -> Bool`

Checks if a string ends with a given suffix.

**Examples:**

```tea
use string = "std.string"

string.ends_with("hello world", "world")  # => true
string.ends_with("hello world", "hello")  # => false
```

#### `replace(text: String, pattern: String, replacement: String) -> String`

Replaces all occurrences of a pattern in a string with a replacement.

**Examples:**

```tea
use string = "std.string"

string.replace("hello world", "world", "tea")  # => "hello tea"
string.replace("aaa", "a", "b")                # => "bbb"
```

#### `to_lower(text: String) -> String`

Converts a string to lowercase.

**Examples:**

```tea
use string = "std.string"

string.to_lower("HELLO")       # => "hello"
string.to_lower("Hello World") # => "hello world"
```

#### `to_upper(text: String) -> String`

Converts a string to uppercase.

**Examples:**

```tea
use string = "std.string"

string.to_upper("hello")       # => "HELLO"
string.to_upper("Hello World") # => "HELLO WORLD"
```

#### `trim_start(text: String) -> String`

Trims whitespace from the start of a string.

**Examples:**

```tea
use string = "std.string"

string.trim_start("  hello  ")  # => "hello  "
string.trim_start("\t\nhello")  # => "hello"
```

#### `trim_end(text: String) -> String`

Trims whitespace from the end of a string.

**Examples:**

```tea
use string = "std.string"

string.trim_end("  hello  ")  # => "  hello"
string.trim_end("hello\n\t")  # => "hello"
```

#### `trim(text: String) -> String`

Trims whitespace from both ends of a string.

**Examples:**

```tea
use string = "std.string"

string.trim("  hello  ")     # => "hello"
string.trim("\t\nhello\n\t") # => "hello"
```

#### `reverse(text: String) -> String`

Reverses a string.

**Examples:**

```tea
use string = "std.string"

string.reverse("hello")  # => "olleh"
string.reverse("ab")     # => "ba"
```
