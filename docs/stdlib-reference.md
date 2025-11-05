# Tea Standard Library Reference

This document provides a comprehensive reference for all modules and functions in the Tea standard library.

## Table of Contents

- [assert](#assert) - Assertion helpers for tests and runtime checks
- [env](#env) - Environment variable access and working directory management
- [fs](#fs) - Filesystem operations
- [json](#json) - JSON encoding and decoding
- [math](#math) - Mathematical utilities
- [path](#path) - Path manipulation utilities
- [string](#string) - String manipulation utilities
- [yaml](#yaml) - YAML encoding and decoding

---

## assert

Assertion helpers for tests and runtime checks.

### Functions

#### `assert(condition, message)`

Asserts that a condition is true. Optional message for failure.

**Examples:**

```tea
assert(1 + 1 == 2)
assert(x > 0, "x must be positive")
```

#### `eq(left, right)`

Asserts that two values are equal.

**Examples:**

```tea
assert.eq(1 + 1, 2)
assert.eq("hello", "hello")
```

#### `ne(left, right)`

Asserts that two values are not equal.

**Examples:**

```tea
assert.ne(1, 2)
assert.ne("hello", "world")
```

#### `fail(message)`

Fails immediately with a message.

**Examples:**

```tea
assert.fail("not implemented")
```

#### `snapshot(name, value, path)`

Asserts that a value matches a saved snapshot.

**Examples:**

```tea
assert.snapshot("test_name", result)
assert.snapshot("test_name", result, "custom/path")
```

#### `empty(value)`

Asserts that a string is empty.

**Examples:**

```tea
assert.empty("")
```

---

## env

Environment variable access and working directory management.

### Functions

#### `get(name: String) -> String`

Get the value of an environment variable.

**Examples:**

```tea
var path = env.get("PATH")
```

#### `set(name: String, value: String) -> Void`

Set an environment variable.

**Examples:**

```tea
env.set("MY_VAR", "my_value")
```

#### `unset(name: String) -> Void`

Unset an environment variable.

**Examples:**

```tea
env.unset("MY_VAR")
```

#### `has(name: String) -> Bool`

Check if an environment variable exists.

**Examples:**

```tea
if env.has("HOME")
  print("HOME is set")
end
```

#### `vars() -> Dict[String, String]`

Get all environment variables as a dictionary.

**Examples:**

```tea
var all_vars = env.vars()
```

#### `cwd() -> String`

Get the current working directory.

**Examples:**

```tea
var cwd = env.cwd()
```

---

## fs

Filesystem operations for reading, writing, and managing files and directories.

### Functions

#### `read_text(file_path: String) -> String`

Read a text file.

**Examples:**

```tea
var content = fs.read_text("file.txt")
```

#### `write_text(file_path: String, content: String) -> Void`

Write text to a file.

**Examples:**

```tea
fs.write_text("file.txt", "Hello, world!")
```

#### `create_dir(dir_path: String) -> Void`

Create a directory.

**Examples:**

```tea
fs.create_dir("my_dir")
```

#### `ensure_dir(dir_path: String) -> Void`

Ensure a directory exists, creating it and all parents if needed.

**Examples:**

```tea
fs.ensure_dir("path/to/nested/dir")
```

#### `remove(file_path: String) -> Void`

Remove a file or directory.

**Examples:**

```tea
fs.remove("file.txt")
```

#### `exists(file_path: String) -> Bool`

Check if a file or directory exists.

**Examples:**

```tea
if fs.exists("file.txt")
  print("File exists")
end
```

#### `list_dir(dir_path: String) -> List[String]`

List all entries in a directory.

**Examples:**

```tea
var entries = fs.list_dir(".")
```

#### `walk(dir_path: String) -> List[String]`

Walk a directory tree recursively.

**Examples:**

```tea
var all_files = fs.walk("src")
```

---

## json

JSON encoding and decoding utilities.

### Functions

#### `encode(value: Dict[String, String]) -> String`

Encode a value to JSON string.

**Examples:**

```tea
var json_str = json.encode({"name": "tea", "version": "1.0"})
```

#### `decode(json_str: String) -> Dict[String, String]`

Decode a JSON string to a value.

**Examples:**

```tea
var data = json.decode("{\"name\":\"tea\"}")
```

---

## math

Mathematical utilities for integer arithmetic operations.

All functions work with integers. Floating point operations are not yet supported.

### Functions

#### `abs(n: Int) -> Int`

Get the absolute value of a number.

**Examples:**

```tea
math.abs(-5)  # => 5
math.abs(5)   # => 5
math.abs(0)   # => 0
```

#### `sign(n: Int) -> Int`

Get the sign of a number (-1, 0, or 1).

**Examples:**

```tea
math.sign(-5)  # => -1
math.sign(0)   # => 0
math.sign(5)   # => 1
```

#### `max(a: Int, b: Int) -> Int`

Get the maximum of two integers.

**Examples:**

```tea
math.max(5, 10)  # => 10
math.max(10, 5)  # => 10
math.max(5, 5)   # => 5
```

#### `min(a: Int, b: Int) -> Int`

Get the minimum of two integers.

**Examples:**

```tea
math.min(5, 10)  # => 5
math.min(10, 5)  # => 5
math.min(5, 5)   # => 5
```

#### `pow(base: Int, exp: Int) -> Int`

Raise a base to an integer exponent.

**Examples:**

```tea
math.pow(2, 3)   # => 8
math.pow(5, 0)   # => 1
math.pow(10, 2)  # => 100
```

---

## path

Path manipulation utilities for working with file paths.

### Functions

#### `join(parts: List[String]) -> String`

Join path components into a single path.

**Examples:**

```tea
var path = path.join(["usr", "local", "bin"])  # => "usr/local/bin"
```

#### `components(file_path: String) -> List[String]`

Split a path into its components.

**Examples:**

```tea
var parts = path.components("/usr/local/bin")  # => ["usr", "local", "bin"]
var parts = path.components("usr/local/")      # => ["usr", "local"]
```

#### `dirname(file_path: String) -> String`

Get the directory part of a path.

**Examples:**

```tea
var dir = path.dirname("/usr/local/bin/tea")  # => "/usr/local/bin"
var dir = path.dirname("/usr/local/")         # => "/usr/local"
var dir = path.dirname("file.txt")            # => ""
var dir = path.dirname("/")                   # => "/"
```

#### `basename(file_path: String) -> String`

Get the filename part of a path.

**Examples:**

```tea
var name = path.basename("/usr/local/bin/tea")  # => "tea"
var name = path.basename("/usr/local/")         # => "local"
var name = path.basename("file.txt")            # => "file.txt"
var name = path.basename("/")                   # => ""
```

#### `extension(file_path: String) -> String`

Get the extension of a path.

**Examples:**

```tea
var ext = path.extension("file.tea")     # => "tea"
var ext = path.extension("file.tar.gz")  # => "gz"
var ext = path.extension(".env")         # => ""
var ext = path.extension("file")         # => ""
```

#### `normalize(file_path: String) -> String`

Normalize a path (remove . and .. components).

**Examples:**

```tea
var norm = path.normalize("./foo/../bar")       # => "bar"
var norm = path.normalize("/usr/./local/../bin")  # => "/usr/bin"
var norm = path.normalize("a/b/c/../../d")      # => "a/d"
```

#### `absolute(file_path: String) -> String`

Convert a path to absolute form.

**Examples:**

```tea
var abs = path.absolute("file.txt")  # => "/current/dir/file.txt"
```

#### `relative(from: String, to: String) -> String`

Get the relative path from one path to another.

**Examples:**

```tea
var rel = path.relative("/usr/local", "/usr/local/bin")  # => "bin"
```

#### `separator() -> String`

Get the system path separator ("/" or "\").

**Examples:**

```tea
var sep = path.separator()  # => "/" on Unix
```

---

## string

String manipulation utilities for common text operations.

### Functions

#### `starts_with(text: String, prefix: String) -> Bool`

Check if a string starts with a given prefix.

**Examples:**

```tea
string.starts_with("hello world", "hello")  # => true
string.starts_with("hello world", "world")  # => false
```

#### `ends_with(text: String, suffix: String) -> Bool`

Check if a string ends with a given suffix.

**Examples:**

```tea
string.ends_with("hello world", "world")  # => true
string.ends_with("hello world", "hello")  # => false
```

#### `trim_start(text: String) -> String`

Trim whitespace from the start of a string.

**Examples:**

```tea
string.trim_start("  hello  ")  # => "hello  "
string.trim_start("\t\nhello")  # => "hello"
```

#### `trim_end(text: String) -> String`

Trim whitespace from the end of a string.

**Examples:**

```tea
string.trim_end("  hello  ")  # => "  hello"
string.trim_end("hello\n\t")  # => "hello"
```

#### `trim(text: String) -> String`

Trim whitespace from both ends of a string.

**Examples:**

```tea
string.trim("  hello  ")     # => "hello"
string.trim("\t\nhello\n\t") # => "hello"
```

#### `reverse(text: String) -> String`

Reverse a string.

**Examples:**

```tea
string.reverse("hello")  # => "olleh"
string.reverse("ab")     # => "ba"
```

---

## yaml

YAML encoding and decoding utilities.

### Functions

#### `encode(value: Dict[String, String]) -> String`

Encode a value to YAML string.

**Examples:**

```tea
var yaml_str = yaml.encode({"name": "tea", "version": "1.0"})
```

#### `decode(yaml_str: String) -> Dict[String, String]`

Decode a YAML string to a value.

**Examples:**

```tea
var data = yaml.decode("name: tea\nversion: 1.0")
```
